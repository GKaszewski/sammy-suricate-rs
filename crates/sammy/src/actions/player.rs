//! Sammy: `InitMarche` (player.wdl:1-260), the per-tick movement of
//! `HandleSpeedPlayer` (player.wdl:1115-1488) and `PlayerJump`/`FinJump`
//! (player.wdl:385-520). First pass: tank controls, walking, gravity, jumping
//! and BSP collision. Slopes, platforms, knockback, boost, firing and the
//! U-turn are not ported yet.

use crate::collision::Hull;
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
const HULL: Hull = Hull::Fat;

// Animation speeds in % of the scene per tick (mm1.wdl:39-85, anims.wdl).
const ANIM_STAND: f32 = 2.0;
const ANIM_WALK: f32 = 0.5; // × forward speed
const ANIM_TURN: f32 = 14.0;
const ANIM_DEBUT_JUMP: f32 = 30.0;
const ANIM_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP: f32 = 14.0;
const ANIM_FIN_JUMP_FAST: f32 = 18.0;

pub use crate::game::BOOST_MAX;
const MEDIUM_SLOPE: f32 = 2.0;
const SPEED_ROTATION_PENTE: f32 = 0.05;

/// Per-level player tuning (InitLvl.wdl defaults; `InitGame5` turning and slopes,
/// `InitGame4` knockback; docs/level-objects.md §2.9).
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayerTuning {
    /// `SpeedRotationPlayer` / `SpeedRotationPlayerMove` (degrees per tick).
    pub turn_stand: f32,
    pub turn_move: f32,
    pub slope_angle_min: f32,
    pub acc_pente: f32,
    pub force_pente: f32,
    pub amorti_pente_end: f32,
    pub big_slope: f32,
    /// `IntensiteBlobBounce` / `AmortiBouncePlayer`.
    pub bounce: f32,
    pub amorti_bounce: f32,
}

impl Default for PlayerTuning {
    fn default() -> Self {
        Self {
            turn_stand: 10.0,
            turn_move: 7.0,
            slope_angle_min: 0.92,
            acc_pente: 1.5,
            force_pente: 100.0,
            amorti_pente_end: 0.3,
            big_slope: 10.0,
            bounce: 30.0,
            amorti_bounce: 10.0,
        }
    }
}

impl PlayerTuning {
    /// `InitGame5` (mm1.wdl:1912-1924): Skill1/2 turning, Skill4-8 slopes.
    pub fn apply_game5(&mut self, s: &[f32]) {
        let set = |v: &mut f32, x: f32| {
            if x != 0.0 {
                *v = x;
            }
        };
        set(&mut self.turn_stand, s[0]);
        set(&mut self.turn_move, s[1]);
        set(&mut self.slope_angle_min, s[3]);
        set(&mut self.acc_pente, s[4]);
        set(&mut self.force_pente, s[5]);
        set(&mut self.amorti_pente_end, s[6]);
        set(&mut self.big_slope, s[7]);
    }

    /// `InitGame4` (mm1.wdl:1891-1910): Skill5/6 knockback strength and decay.
    pub fn apply_game4(&mut self, s: &[f32]) {
        if s[4] != 0.0 {
            self.bounce = s[4];
        }
        if s[5] != 0.0 {
            self.amorti_bounce = s[5];
        }
    }
}

/// Collision geometry of the current level.
#[derive(Resource, Clone)]
pub struct LevelCollision(pub Arc<Wmb>);

/// [`sammy_core::physics::JumpParams`] as a resource.
#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
pub struct JumpParams(pub sammy_core::physics::JumpParams);

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
    /// Level won: "Iwin" while spinning (`AnimGagnee`).
    Win,
    /// Dying / respawning (`PlayerEtat == 3`): no input, animation owned by health.rs.
    Dead,
    /// Throwing a wasp: "TirA" wind-up, then "TirB" follow-through (`PlayerEtat == 2`).
    FireA,
    FireB,
    /// Extro `SetAnimPlayer` (`PlayerEtat` 2/3): no input, cannot die, animation scripted.
    Scripted,
    /// Quick 180° turn (`RetourneSammy`, `PlayerEtat == 6`).
    UTurn,
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
    /// `BounceForce1`: knockback velocity (quants/tick), decays each tick.
    pub knock: [f32; 3],
    /// Last frame's movement in quants/tick (`PlayerSpeed`), what others get pushed by.
    pub vel: [f32; 3],
    /// Floor height below Sammy (`BlobHautSol`), for his shadow.
    pub floor: f32,
    /// `PlayerPenteX/Y/Z`: slide velocity on slopes (quants/tick).
    pub pente: [f32; 3],
    /// Lean on slopes (visual tilt/roll, degrees).
    tilt: f32,
    roll: f32,
    /// U-turn target pan and whether the first (×0.25) step was taken.
    uturn: Option<(f32, bool)>,
    /// Landing after a fall faster than 70/tick (`ForceChute < -70`).
    pub hard_landing: bool,
    /// Shift-boosting this frame (used by platforms: ×3 while riding).
    pub boosting: bool,
    pub state: State,
    snapped: bool,
}

impl Player {
    /// Moves Sammy to a respawn point; he is re-snapped to the floor next frame.
    pub fn teleport(&mut self, pos: [f32; 3], pan: f32) {
        self.pos = pos;
        self.pan = pan;
        self.vz = 0.0;
        self.grounded = true;
        self.snapped = false;
    }
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
        knock: [0.0; 3],
        vel: [0.0; 3],
        floor: pos[2],
        pente: [0.0; 3],
        tilt: 0.0,
        roll: 0.0,
        uturn: None,
        hard_landing: false,
        boosting: false,
        state: State::Stand,
        snapped: false,
    });
}

/// Height of the floor below `p` (sonar from z+8, `playerClimb`) and the brush it
/// belongs to, or None over the void.
fn floor_below(world: &CollisionWorld, p: [f32; 3]) -> Option<(f32, Option<Entity>, [f32; 3])> {
    let start = [p[0], p[1], p[2] + 8.0];
    let hit = world.trace(Hull::Narrow, start, [p[0], p[1], p[2] - 2000.0], None);
    (hit.trace.hit() && !hit.trace.start_solid).then(|| (hit.trace.end[2] - HULL1_HALF, hit.entity, hit.trace.normal))
}

/// Sammy touched an entity while moving (EVENT_ENTITY on Sammy / EVENT_IMPACT on it).
#[derive(Message, Clone, Copy)]
pub struct PlayerTouched(pub Entity);

/// Moves along `delta`, sliding along walls (approximates A4's `MOVE` glide).
/// Returns the new position and the last entity bumped into.
pub fn slide_move(world: &CollisionWorld, pos: [f32; 3], delta: [f32; 3], ignore: Option<Entity>) -> ([f32; 3], Option<Entity>) {
    let (p, t) = slide_move_n(world, pos, delta, ignore);
    (p, t.map(|(e, _)| e))
}

/// `slide_move` that also reports the contact normal of the touched entity.
pub fn slide_move_n(world: &CollisionWorld, mut pos: [f32; 3], mut delta: [f32; 3], ignore: Option<Entity>) -> ([f32; 3], Option<(Entity, [f32; 3])>) {
    let mut touched = None;
    for _ in 0..4 {
        if delta.iter().all(|d| d.abs() < 1e-4) {
            break;
        }
        let end = [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]];
        let hit = world.trace(HULL, pos, end, ignore);
        let tr = hit.trace;
        if tr.start_solid {
            return (pos, touched);
        }
        if let Some(e) = hit.entity
            && tr.hit()
        {
            touched = Some((e, tr.normal));
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
    (pos, touched)
}

pub fn move_player(
    time: Res<Time>,
    controls: Res<crate::input::Controls>,
    jump: Res<JumpParams>,
    current: Res<crate::flow::CurrentLevel>,
    world: CollisionWorld,
    mut game: ResMut<crate::game::GameState>,
    mut sfx: crate::audio::Sfx,
    tuning: Res<PlayerTuning>,
    enemies: Query<(), With<super::enemy::Enemy>>,
    pushers: Query<(), Or<(With<super::platform::Platform>, With<super::wasp::Hive>)>>,
    gems: Query<(), With<super::pickups::KeyGem>>,
    mut touched_msgs: MessageWriter<PlayerTouched>,
    mut q: Query<(Entity, &mut Player, &mut Transform, Option<&mut ModelAnimator>)>,
) {
    use crate::flow::Phase;
    // Input only counts while the level is being played (`PartieEncours == 1`).
    let input = crate::flow::playing(&current);
    if !world.ready() {
        return;
    }
    let t = (time.delta_secs() * TICKS).min(4.0); // `time` in ticks, clamped like A4's time_smooth
    for (me, mut p, mut tf, mut anim) in &mut q {
        let start_pos = p.pos;
        if !p.snapped {
            if let Some((f, ..)) = floor_below(&world, p.pos) {
                p.pos[2] = f + BLOB_HAUT;
            }
            p.snapped = true;
        }

        // Intention: x = right(+)/left(-), y = forward(+)/back(-) (Key_Force).
        let (mut ix, mut iy) = (controls.turn, controls.forward);
        if std::env::var_os("SAMMY_AUTOWALK").is_some() {
            iy = 1.0; // debug: hold forward
        }
        let (mut ctrl, mut ctrl_hit) = (controls.jump, controls.jump_hit);
        if !input || p.state == State::Dead {
            (ix, iy, ctrl, ctrl_hit) = (0.0, 0.0, false, false);
        }
        if matches!(current.phase, Phase::Won(_)) && p.grounded {
            // AnimGagnee: "Iwin" while spinning 12°/tick.
            if p.state != State::Win {
                p.state = State::Win;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("Iwin", true, 0.0);
                    a.set_speed_pct(3.0, TICKS);
                }
            }
            p.pan += 12.0 * t;
            tf.translation = coords::pos(p.pos);
            tf.rotation = coords::rotation([p.pan, 0.0, 0.0]);
            continue;
        }

        // Boost (HandleBoostPlayer): the engine doubles Key_Force under Shift; with charge
        // the script keeps turning ×0.75 (net ×1.5) and forward ×2, else halves it back.
        let shift = controls.boost;
        p.boosting = false;
        if shift && input && game.boost < BOOST_MAX && p.state != State::Dead {
            game.boost = (game.boost + t).min(BOOST_MAX); // LessBoost: ConsoBoost 1/tick
            ix *= 1.5;
            iy *= 2.0;
            p.boosting = true;
        }

        // U-turn (RetourneSammy): Alt in a normal state, pan eases toward +180.
        let alt = controls.uturn_hit;
        if alt && input && p.grounded && matches!(p.state, State::Stand | State::Walk | State::Turn) {
            p.state = State::UTurn;
            p.uturn = Some((p.pan + 180.0, false));
            sfx.play("Blouit_son");
            if let Some(a) = anim.as_deref_mut() {
                a.play("turn", true, 0.0);
                a.set_speed_pct(28.0, TICKS);
            }
        }
        if let (State::UTurn, Some((dest, started))) = (p.state, p.uturn) {
            let wrap = |a: f32| (a + 180.0).rem_euclid(360.0) - 180.0;
            let step = wrap((dest - p.pan) * if started { 0.4 } else { 0.25 });
            if step > 2.0 {
                p.pan += step * t;
                p.uturn = Some((dest, true));
            } else {
                p.uturn = None;
                p.state = State::Stand;
            }
        }

        // Turning and moving only in the normal and jump states (etat 1 / 4).
        let can_move = matches!(p.state, State::Stand | State::Walk | State::Turn | State::Air | State::Land);
        if !can_move {
            ix = 0.0;
            iy = 0.0;
        }
        // 1. Turning: 10°/tick standing, 7 moving (×0.6 boosting), half in the air.
        let mut rate = if iy != 0.0 { if p.boosting { 0.6 * tuning.turn_move } else { tuning.turn_move } } else { tuning.turn_stand };
        if !p.grounded {
            rate *= 0.5;
        }
        let mut angle_force = ix * rate * t;

        // 2. Forward speed; air control is 75%.
        let mut v = if p.grounded { iy * p.speed * t } else { iy * p.speed * 0.75 * t };
        let mut strafe = 0.0;
        // Slope steering: big slopes strafe and turn downhill, medium slopes damp turning.
        let big = p.pente[0].abs() > tuning.big_slope || p.pente[1].abs() > tuning.big_slope;
        let medium = p.pente[0].abs() > MEDIUM_SLOPE || p.pente[1].abs() > MEDIUM_SLOPE;
        let slope_pan = p.pente[1].atan2(p.pente[0]).to_degrees();
        if big {
            v *= 0.75;
            strafe = -ix * p.speed * 0.75 * t;
            angle_force = 0.0;
            let wrap = (slope_pan - p.pan + 180.0).rem_euclid(360.0) - 180.0;
            p.pan += wrap * SPEED_ROTATION_PENTE * t;
        } else if medium {
            v *= 0.9;
            strafe = -ix * p.speed * 0.5 * t;
            angle_force *= 0.25;
        }
        p.pan -= angle_force;
        let (s, c) = p.pan.to_radians().sin_cos();
        let (pos, touched) = slide_move_n(&world, p.pos, [c * v - s * strafe, s * v + c * strafe, 0.0], Some(me));
        p.pos = pos;
        if let Some((e, n)) = touched {
            touched_msgs.write(PlayerTouched(e));
            // Player_Bong (player.wdl:520-697): bumping an entity cancels the slope slide;
            // non-pushers knock Sammy back along the contact normal, pushers (platforms,
            // hives: no speed of their own) cancel the knockback. Lions: enemy.rs.
            if !gems.contains(e) && !enemies.contains(e) {
                p.pente = [0.0; 3];
                p.knock = if pushers.contains(e) { [0.0; 3] } else { [n[0] * tuning.bounce, n[1] * tuning.bounce, 0.0] };
            }
        }

        // handlePente: on floors steeper than ~23° Sammy slides downhill and leans.
        if let Some((f, _, n)) = floor_below(&world, p.pos)
            && n[2] > 0.0
            && n[2] <= tuning.slope_angle_min
            && p.pos[2] - f <= BLOB_HAUT + 32.0
        {
            let force = [n[0] * tuning.force_pente, n[1] * tuning.force_pente, -20.0];
            let v2 = 1.0 - n[2];
            for i in 0..3 {
                p.pente[i] += tuning.acc_pente * force[i].signum() * v2;
                if p.pente[i].abs() > force[i].abs() {
                    p.pente[i] = force[i];
                }
            }
            // Lean into the slope (local normal → tilt/roll), only for real slides.
            if p.pente[0].abs() > 4.0 || p.pente[1].abs() > 4.0 {
                let (sp, cp) = (-p.pan).to_radians().sin_cos();
                let local = [n[0] * cp - n[1] * sp, n[0] * sp + n[1] * cp];
                let want_tilt = -local[0].asin().to_degrees();
                let want_roll = -local[1].asin().to_degrees();
                if p.tilt.abs() < 12.0 {
                    p.tilt += 0.1 * (want_tilt - p.tilt) * t;
                }
                if p.roll.abs() < 6.0 {
                    p.roll += 0.1 * (want_roll - p.roll) * t;
                }
            }
            let d = p.pente.map(|x| x * t);
            p.pos = slide_move(&world, p.pos, d, Some(me)).0;
        } else {
            p.tilt -= 0.6 * p.tilt * t.min(1.0);
            p.roll -= 0.6 * p.roll * t.min(1.0);
            let k = (1.0 - tuning.amorti_pente_end * t).max(0.0);
            p.pente = p.pente.map(|x| if x.abs() < 0.01 { 0.0 } else { x * k });
            if p.pente != [0.0; 3] {
                let d = p.pente.map(|x| x * t);
                p.pos = slide_move(&world, p.pos, [d[0], d[1], 0.0], Some(me)).0;
            }
        }
        // Knockback (`BounceForce1`), separate move, decaying by AmortiBouncePlayer
        // 10 × 80/(fps+1) ≈ 15.7 per tick at the reference 50 fps.
        if p.knock != [0.0; 3] {
            let k = p.knock.map(|v| v * t);
            p.pos = slide_move(&world, p.pos, [k[0], k[1], 0.0], Some(me)).0;
            let dec = tuning.amorti_bounce * 80.0 / 51.0 * t;
            p.knock = p.knock.map(|v| if v.abs() <= dec { 0.0 } else { v - dec * v.signum() });
        }

        // 3. Jump (PlayerJump): only from the ground in a normal state. Moving jumps take
        //    off at once; standing jumps play the "sautA" wind-up first.
        let mut take_off = false;
        // No jumping off a big slope unless facing within 45° of downhill.
        let slope_ok = !big || ((slope_pan - p.pan + 180.0).rem_euclid(360.0) - 180.0).abs() <= 45.0;
        if ctrl_hit && slope_ok && p.grounded && matches!(p.state, State::Stand | State::Walk | State::Turn) {
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
        if let Some((f, ..)) = floor {
            p.floor = f;
        }
        if p.grounded {
            match floor {
                Some((f, e, _)) if p.pos[2] - BLOB_HAUT - f <= 1.0 => {
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
            let hit = world.trace(HULL, p.pos, target, Some(me));
            let tr = hit.trace;
            // Stomp (Player_Bong :584-618): landing on a lion's head re-jumps.
            if p.vz < 0.0 && tr.hit() && jump.bounce_on_enemy != 0.0 && hit.entity.is_some_and(|e| enemies.contains(e)) {
                p.vz = jump.bounce_on_enemy * jump.jump_speed;
                p.fric = jump.jump_fric;
                p.knock = p.knock.map(|k| k * 2.0);
                sfx.play("Blouit_son");
            }
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
            let floor_z = floor.map(|(f, ..)| f);
            // A real fall (more than 64 below the normal height) shows the jump animation.
            if !matches!(p.state, State::Air | State::Dead | State::Scripted) && floor_z.is_none_or(|f| p.pos[2] - f > BLOB_HAUT + 64.0) {
                p.state = State::Air;
                if let Some(a) = anim.as_deref_mut() {
                    a.play("sautB", false, 50.0);
                }
            }
            if let Some((f, e, _)) = floor
                && p.vz <= 0.0
                && p.pos[2] <= f + BLOB_HAUT
            {
                p.hard_landing = p.vz < -70.0;
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
                    let pct = if p.hard_landing {
                        ANIM_FIN_JUMP * 0.5 // hard landing: slow recovery
                    } else if iy != 0.0 || ctrl {
                        ANIM_FIN_JUMP_FAST * 2.0
                    } else if ix != 0.0 {
                        ANIM_FIN_JUMP_FAST
                    } else {
                        ANIM_FIN_JUMP
                    };
                    a.set_speed_pct(pct, TICKS);
                }
                _ => {}
            }
        }

        if t > 0.0 {
            p.vel = [0, 1, 2].map(|i| (p.pos[i] - start_pos[i]) / t);
        }
        tf.translation = coords::pos(p.pos);
        tf.rotation = coords::rotation([p.pan, p.tilt, p.roll]);
    }
}
