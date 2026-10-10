//! Sammy: `InitMarche` (player.wdl:1-260), the per-tick movement of
//! `HandleSpeedPlayer` (player.wdl:1115-1488) and `PlayerJump`/`FinJump`
//! (player.wdl:385-520). First pass: tank controls, walking, gravity, jumping
//! and BSP collision. Slopes, platforms, knockback, boost, firing and the
//! U-turn are not ported yet.

use std::sync::Arc;

use bevy::prelude::*;
use formats::wmb::{Object, Wmb};

use crate::collision::CollisionWorld;
use crate::coords;
use crate::model::ModelAnimator;

/// Acknex ticks per second.
pub const TICKS: f32 = 16.0;
/// `heightFootPlayer` (mm1.wdl:36) × model scale 8: origin height above the floor.
pub const BLOB_HAUT: f32 = 4.4 * 8.0;
/// Model scale forced by `InitMarche` (player.wdl:34-36).
pub const SCALE: f32 = 8.0;
/// Floors stop hull-1 traces with the origin this far above them (±16 cube).
const HULL1_HALF: f32 = 16.0;
/// Sammy collides with the "fat" hull (`my.fat=1`).
const HULL: usize = 2;

// Animation speeds in % of the scene per tick (mm1.wdl:39-85, anims.wdl).
const ANIM_STAND: f32 = 2.0;
const ANIM_WALK: f32 = 0.5; // × forward speed
const ANIM_TURN: f32 = 14.0;
const ANIM_DEBUT_JUMP: f32 = 30.0;
const ANIM_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP_FAST: f32 = 18.0;

/// Collision geometry of the current level.
#[derive(Resource, Clone)]
pub struct LevelCollision(pub Arc<Wmb>);

/// Vertical physics globals (InitLvl.wdl:60-61 defaults, `InitGame3` overrides, mm1.wdl:1846).
#[derive(Resource, Clone, Copy)]
pub struct JumpParams {
    /// `BlobBallVerticalSpeed`: jump impulse.
    pub jump_speed: f32,
    /// `BlobBallVerticalFriction`: deceleration during a jump.
    pub jump_fric: f32,
    /// `Gravite`: initial vertical speed when walking off a ledge.
    pub gravity: f32,
    /// `AccGravite`: deceleration when falling off a ledge.
    pub acc_gravity: f32,
}

impl Default for JumpParams {
    fn default() -> Self {
        Self { jump_speed: 30.0, jump_fric: 3.0, gravity: -2.0, acc_gravity: 1.5 }
    }
}

impl JumpParams {
    pub fn apply_init_game3(&mut self, obj: &Object) {
        let s = &obj.skills;
        for (v, i) in [(&mut self.jump_speed, 0), (&mut self.jump_fric, 1), (&mut self.gravity, 4), (&mut self.acc_gravity, 5)] {
            if s[i] != 0.0 {
                *v = s[i];
            }
        }
    }
}

/// `PlayerEtat` / `PlayerAnimEtat`, reduced to what is ported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Stand,
    Walk,
    Turn,
    /// Standing wind-up ("sautA") before leaving the ground.
    JumpStart,
    /// In the air ("sautB", played once).
    Air,
    /// Landing ("sautC", played once).
    Land,
}

#[derive(Component)]
pub struct Player {
    /// Native Acknex position (quants).
    pub pos: [f32; 3],
    /// Degrees, counter-clockwise from +X.
    pub pan: f32,
    /// Walk speed in quants/tick (WED Skill2 → `SpeedBalle`, default 25).
    pub speed: f32,
    /// `BlobVspeed` (quants/tick) and `BlobFricVspeed`.
    pub vz: f32,
    pub fric: f32,
    pub grounded: bool,
    /// Brush entity Sammy stands on (None = level geometry).
    pub ground_entity: Option<Entity>,
    /// `BlobZoneScan` (WED Skill5, default 104): pickup scan radius.
    pub scan_radius: f32,
    pub state: State,
    snapped: bool,
}

pub fn init_player(obj: &mut Object, entity: &mut EntityCommands) {
    obj.scale = [SCALE; 3];
    let speed = obj.skills.get(1).copied().filter(|&s| s > 0.0).unwrap_or(25.0);
    let scan_radius = obj.skills.get(4).copied().filter(|&s| s > 0.0).unwrap_or(104.0);
    // Debug: SAMMY_START="x,y,z[,pan]" overrides the spawn point.
    let (mut pos, mut pan) = (obj.origin, obj.angles[0]);
    if let Ok(v) = std::env::var("SAMMY_START") {
        let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if n.len() >= 3 {
            pos = [n[0], n[1], n[2]];
        }
        if let Some(&a) = n.get(3) {
            pan = a;
        }
    }
    entity.insert(Player {
        pos,
        pan,
        speed,
        vz: 0.0,
        fric: 0.0,
        grounded: true,
        ground_entity: None,
        scan_radius,
        state: State::Stand,
        snapped: false,
    });
}

/// Height of the floor below `p` (sonar from z+8, `playerClimb`) and the brush it
/// belongs to, or None over the void.
fn floor_below(world: &CollisionWorld, p: [f32; 3]) -> Option<(f32, Option<Entity>)> {
    let start = [p[0], p[1], p[2] + 8.0];
    let hit = world.trace(1, start, [p[0], p[1], p[2] - 2000.0], None);
    (hit.trace.hit() && !hit.trace.start_solid).then(|| (hit.trace.end[2] - HULL1_HALF, hit.entity))
}

/// Moves along `delta`, sliding along walls (approximates A4's `MOVE` glide).
fn slide_move(world: &CollisionWorld, mut pos: [f32; 3], mut delta: [f32; 3]) -> [f32; 3] {
    for _ in 0..4 {
        if delta.iter().all(|d| d.abs() < 1e-4) {
            break;
        }
        let end = [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]];
        let tr = world.trace(HULL, pos, end, None).trace;
        if tr.start_solid {
            return pos;
        }
        pos = tr.end;
        if !tr.hit() {
            break;
        }
        // Remove the part of the remaining motion that goes into the wall.
        let rest = delta.map(|d| d * (1.0 - tr.fraction));
        let into = rest[0] * tr.normal[0] + rest[1] * tr.normal[1] + rest[2] * tr.normal[2];
        delta = [0, 1, 2].map(|i| rest[i] - tr.normal[i] * into);
    }
    pos
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    jump: Res<JumpParams>,
    world: CollisionWorld,
    mut q: Query<(&mut Player, &mut Transform, Option<&mut ModelAnimator>)>,
) {
    if !world.ready() {
        return;
    }
    let t = (time.delta_secs() * TICKS).min(4.0); // `time` in ticks, clamped like A4's time_smooth
    for (mut p, mut tf, mut anim) in &mut q {
        if !p.snapped {
            if let Some((f, _)) = floor_below(&world, p.pos) {
                p.pos[2] = f + BLOB_HAUT;
            }
            p.snapped = true;
        }

        // Intention: x = right(+)/left(-), y = forward(+)/back(-) (Key_Force).
        let axis = |pos: KeyCode, neg: KeyCode| keys.pressed(pos) as i32 as f32 - keys.pressed(neg) as i32 as f32;
        let ix = axis(KeyCode::ArrowRight, KeyCode::ArrowLeft);
        let mut iy = axis(KeyCode::ArrowUp, KeyCode::ArrowDown);
        if std::env::var_os("SAMMY_AUTOWALK").is_some() {
            iy = 1.0; // debug: hold forward
        }
        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let ctrl_hit = keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight);

        // 1. Turning: 10°/tick standing, 7 moving, half in the air.
        let mut rate = if iy != 0.0 { 7.0 } else { 10.0 };
        if !p.grounded {
            rate *= 0.5;
        }
        p.pan -= ix * rate * t;

        // 2. Forward speed; air control is 75%. No walking during the wind-up.
        let v = if p.state == State::JumpStart {
            0.0
        } else if p.grounded {
            iy * p.speed * t
        } else {
            iy * p.speed * 0.75 * t
        };
        let (s, c) = p.pan.to_radians().sin_cos();
        p.pos = slide_move(&world, p.pos, [c * v, s * v, 0.0]);

        // 3. Jump (PlayerJump): only from the ground in a normal state. Moving jumps take
        //    off at once; standing jumps play the "sautA" wind-up first.
        let mut take_off = false;
        if ctrl_hit && p.grounded && matches!(p.state, State::Stand | State::Walk | State::Turn) {
            if iy != 0.0 {
                take_off = true;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("sautB", false, 10.0);
                }
            } else {
                p.state = State::JumpStart;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("sautA", false, 0.0);
                    a.set_speed_pct(ANIM_DEBUT_JUMP, TICKS);
                }
            }
        }
        if p.state == State::JumpStart && anim.as_deref().is_none_or(|a| a.finished()) {
            take_off = true;
            if let Some(a) = anim.as_deref_mut() {
                a.play("sautB", false, 0.0);
            }
        }
        if take_off {
            p.state = State::Air;
            p.grounded = false;
            p.vz = jump.jump_speed;
            p.fric = jump.jump_fric;
        }

        // 4. Vertical motion: snap to the floor when grounded, else ballistic.
        let floor = floor_below(&world, p.pos);
        if p.grounded {
            match floor {
                Some((f, e)) if p.pos[2] - BLOB_HAUT - f <= 1.0 => {
                    p.pos[2] = f + BLOB_HAUT;
                    p.ground_entity = e;
                }
                _ => {
                    // Walked off a ledge (`Gravite`, `AccGravite`).
                    p.grounded = false;
                    p.vz = jump.gravity;
                    p.fric = jump.acc_gravity;
                }
            }
        }
        let mut landed = false;
        if !p.grounded && p.vz != 0.0 {
            p.vz -= p.fric * t;
            let target = [p.pos[0], p.pos[1], p.pos[2] + p.vz * t];
            let tr = world.trace(HULL, p.pos, target, None).trace;
            if tr.start_solid {
                // Stuck inside geometry (e.g. a closing door): never sink through it.
                p.vz = 0.0;
                p.grounded = true;
            } else {
                if tr.hit() && p.vz > 0.0 {
                    p.vz = 0.0; // ceiling cancels the ascent
                }
                p.pos = tr.end;
            }
            let floor = floor_below(&world, p.pos);
            let floor_z = floor.map(|(f, _)| f);
            // A real fall (more than 64 below the normal height) shows the jump animation.
            if p.state != State::Air && floor_z.is_none_or(|f| p.pos[2] - f > BLOB_HAUT + 64.0) {
                p.state = State::Air;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("sautB", false, 50.0);
                }
            }
            if let Some((f, e)) = floor
                && p.vz <= 0.0
                && p.pos[2] <= f + BLOB_HAUT
            {
                p.pos[2] = f + BLOB_HAUT;
                p.ground_entity = e;
                p.grounded = true;
                p.vz = 0.0;
                landed = true;
            }
        }

        // 5. Landing (FinJump): "sautC" once, faster when the player is already steering.
        if landed && p.state == State::Air {
            p.state = State::Land;
            if let Some(a) = anim.as_deref_mut() {
                a.play("sautC", false, 10.0);
            }
        }
        if p.state == State::Land && anim.as_deref().is_none_or(|a| a.finished()) {
            p.state = State::Stand;
        }

        // 6. Ground animation states (anims.wdl:352-497).
        if matches!(p.state, State::Stand | State::Walk | State::Turn) {
            let next = if iy != 0.0 {
                State::Walk
            } else if ix != 0.0 {
                State::Turn
            } else {
                State::Stand
            };
            if let Some(a) = anim.as_deref_mut() {
                let scene = match next {
                    State::Walk => "walkB",
                    State::Turn => "turn",
                    _ => "stand",
                };
                if next != p.state || a.frames.is_empty() || !a.looping {
                    a.play(scene, true, 0.0);
                }
                let pct = match next {
                    State::Walk => ANIM_WALK * p.speed * iy, // negative plays backwards
                    State::Turn => ANIM_TURN,
                    _ => ANIM_STAND,
                };
                a.set_speed_pct(pct, TICKS);
            }
            p.state = next;
        }
        if let Some(a) = anim.as_deref_mut() {
            match p.state {
                State::Air => a.set_speed_pct(ANIM_JUMP, TICKS),
                State::Land => {
                    let pct = if iy != 0.0 || ctrl { ANIM_FIN_JUMP_FAST * 2.0 } else if ix != 0.0 { ANIM_FIN_JUMP_FAST } else { ANIM_FIN_JUMP };
                    a.set_speed_pct(pct, TICKS);
                }
                _ => {}
            }
        }

        tf.translation = coords::pos(p.pos);
        tf.rotation = coords::rotation([p.pan, 0.0, 0.0]);
    }
}
