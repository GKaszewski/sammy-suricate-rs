//! Sammy: `InitMarche` (player.wdl:1-260) and the per-tick movement of
//! `HandleSpeedPlayer` (player.wdl:1115-1488). Simplified first pass: tank
//! controls, walking, gravity, jumping and BSP collision. Slopes, platforms,
//! knockback, boost, firing and the U-turn are not ported yet.

use std::sync::Arc;

use bevy::prelude::*;
use formats::wmb::{Object, Wmb};

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

/// Collision geometry of the current level.
#[derive(Resource, Clone)]
pub struct LevelCollision(pub Arc<Wmb>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnimState {
    Stand,
    Walk,
    Turn,
    Jump,
}

#[derive(Component)]
pub struct Player {
    /// Native Acknex position (quants).
    pub pos: [f32; 3],
    /// Degrees, counter-clockwise from +X.
    pub pan: f32,
    /// Walk speed in quants/tick (WED Skill2 → `SpeedBalle`, default 25).
    pub speed: f32,
    /// Vertical speed (quants/tick) and its per-tick deceleration.
    pub vz: f32,
    pub fric: f32,
    pub grounded: bool,
    pub anim: AnimState,
    snapped: bool,
}

pub fn init_player(obj: &mut Object, entity: &mut EntityCommands) {
    obj.scale = [SCALE; 3];
    let speed = obj.skills.get(1).copied().filter(|&s| s > 0.0).unwrap_or(25.0);
    entity.insert(Player {
        pos: obj.origin,
        pan: obj.angles[0],
        speed,
        vz: 0.0,
        fric: 0.0,
        grounded: true,
        anim: AnimState::Stand,
        snapped: false,
    });
}

/// Height of the floor below `p` (sonar from z+8, `playerClimb`), or None over the void.
fn floor_below(wmb: &Wmb, p: [f32; 3]) -> Option<f32> {
    let start = [p[0], p[1], p[2] + 8.0];
    let tr = wmb.trace(0, 1, start, [p[0], p[1], p[2] - 2000.0]);
    (tr.hit() && !tr.start_solid).then(|| tr.end[2] - HULL1_HALF)
}

/// Moves along `delta`, sliding along walls (approximates A4's `MOVE` glide).
fn slide_move(wmb: &Wmb, mut pos: [f32; 3], mut delta: [f32; 3]) -> [f32; 3] {
    for _ in 0..4 {
        if delta.iter().all(|d| d.abs() < 1e-4) {
            break;
        }
        let end = [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]];
        let tr = wmb.trace(0, HULL, pos, end);
        if tr.start_solid {
            return pos;
        }
        pos = tr.end;
        if !tr.hit() {
            break;
        }
        // Remove the part of the remaining motion that goes into the wall.
        let left = 1.0 - tr.fraction;
        let rest = delta.map(|d| d * left);
        let into = rest[0] * tr.normal[0] + rest[1] * tr.normal[1] + rest[2] * tr.normal[2];
        delta = [0, 1, 2].map(|i| rest[i] - tr.normal[i] * into);
    }
    pos
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    level: Option<Res<LevelCollision>>,
    mut q: Query<(&mut Player, &mut Transform, Option<&mut ModelAnimator>)>,
) {
    let Some(level) = level else { return };
    let wmb = &level.0;
    let t = (time.delta_secs() * TICKS).min(4.0); // `time` in ticks, clamped like A4's time_smooth
    for (mut p, mut tf, anim) in &mut q {
        if !p.snapped {
            if let Some(f) = floor_below(wmb, p.pos) {
                p.pos[2] = f + BLOB_HAUT;
            }
            p.snapped = true;
        }

        // Intention: x = right(+)/left(-), y = forward(+)/back(-) (Key_Force).
        let axis = |pos: KeyCode, neg: KeyCode| keys.pressed(pos) as i32 as f32 - keys.pressed(neg) as i32 as f32;
        let ix = axis(KeyCode::ArrowRight, KeyCode::ArrowLeft);
        let iy = axis(KeyCode::ArrowUp, KeyCode::ArrowDown);

        // 1. Turning: 10°/tick standing, 7 moving, half in the air.
        let mut rate = if iy != 0.0 { 7.0 } else { 10.0 };
        if !p.grounded {
            rate *= 0.5;
        }
        p.pan -= ix * rate * t;

        // 2. Forward speed; air control is 75%.
        let v = if p.grounded { iy * p.speed } else { iy * p.speed * 0.75 } * t;
        let (s, c) = p.pan.to_radians().sin_cos();
        p.pos = slide_move(wmb, p.pos, [c * v, s * v, 0.0]);

        // 3. Jump (Ctrl): vz=30, deceleration 3/tick² (InitLvl.wdl:60-61).
        if p.grounded && (keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight)) {
            p.grounded = false;
            p.vz = 30.0;
            p.fric = 3.0;
        }

        // 4. Vertical motion: snap to floor when grounded, else ballistic.
        let floor = floor_below(wmb, p.pos);
        if p.grounded {
            match floor {
                Some(f) if p.pos[2] - BLOB_HAUT - f <= 1.0 => p.pos[2] = f + BLOB_HAUT,
                _ => {
                    // Walked off a ledge: gravity starts at -2, accel 1.5/tick².
                    p.grounded = false;
                    p.vz = -2.0;
                    p.fric = 1.5;
                }
            }
        }
        if !p.grounded {
            p.vz -= p.fric * t;
            let target = [p.pos[0], p.pos[1], p.pos[2] + p.vz * t];
            let tr = wmb.trace(0, HULL, p.pos, target);
            if tr.hit() && p.vz > 0.0 {
                p.vz = 0.0; // ceiling cancels the ascent
            }
            p.pos = tr.end;
            if let Some(f) = floor_below(wmb, p.pos)
                && p.vz <= 0.0
                && p.pos[2] <= f + BLOB_HAUT
            {
                p.pos[2] = f + BLOB_HAUT;
                p.grounded = true;
                p.vz = 0.0;
            }
        }

        // 5. Animation state (anims.wdl:352-497).
        let state = if !p.grounded {
            AnimState::Jump
        } else if iy != 0.0 {
            AnimState::Walk
        } else if ix != 0.0 {
            AnimState::Turn
        } else {
            AnimState::Stand
        };
        if let Some(mut a) = anim {
            // Scene speeds are in % of the scene per tick.
            let (scene, pct) = match state {
                AnimState::Stand => ("stand", 2.0),
                AnimState::Walk => ("walkB", 0.5 * p.speed * iy.abs()),
                AnimState::Turn => ("turn", 14.0),
                AnimState::Jump => ("sautB", 14.0),
            };
            if state != p.anim || a.frames.is_empty() {
                a.frames = a.model.animation(scene);
                a.time = 0.0;
            }
            a.fps = pct / 100.0 * a.frames.len() as f32 * TICKS * if iy < 0.0 { -1.0 } else { 1.0 };
        }
        p.anim = state;

        tf.translation = coords::pos(p.pos);
        tf.rotation = coords::rotation([p.pan, 0.0, 0.0]);
    }
}
