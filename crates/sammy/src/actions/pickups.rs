//! Diamonds (`InitBonus3`/`Bonus3_Hit`, mm1.wdl:1200-1291) and colour key gems
//! (`InitKey`/`Key_Hit`/`EjectOldKey`/`LaunchYouKey`, gems.wdl:40-960).
//! Both are picked up by Sammy's scan (id 1, radius `BlobZoneScan`).

use std::collections::HashMap;

use bevy::prelude::*;
use formats::wmb::Object;

use super::player::{Player, TICKS};
use crate::audio::Sfx;
use crate::collision::CollisionWorld;
use crate::coords;
use crate::game::{GameState, ResetScan, MASTER_KEY};
use crate::level::{Billboard, SpriteAnim};

/// `HeightGem` (skills.wdl:697): gems float this far above the ground.
const HEIGHT_GEM: f32 = 25.0;

#[derive(Component)]
pub struct Diamond {
    pub points: i32,
}

#[derive(Component)]
pub struct KeyGem {
    pub color: u32,
}

/// Native position of a pickup (kept separately so ground snapping and bouncing
/// work in Acknex units).
#[derive(Component)]
pub struct PickupPos(pub [f32; 3]);

/// Snap to `floor + HEIGHT_GEM` on the first frame (`sonar my,2000`).
#[derive(Component)]
pub struct SnapToGround;

/// Picked up: shrink ×0.8 per tick, then despawn (`Bonus3_Hit`, `ScaleDownGem`).
/// Entities that can come back (level keys, checkpoints) are hidden instead.
#[derive(Component)]
pub struct Shrinking;

/// Collected and waiting for the reset scan (`GemWaitForNewStart`).
#[derive(Component)]
pub struct Collected;

/// A key placed in the level (WDL Flag3): restored at its home on each respawn.
#[derive(Component)]
pub struct LevelKey {
    home: Option<[f32; 3]>,
    scale: Vec3,
}

/// Restored by the reset scan instead of being despawned after shrinking.
#[derive(Component)]
pub struct Respawnable;

/// An ejected key in flight (`LaunchYouKey`): collectable after it settles.
#[derive(Component)]
pub struct Bouncing {
    vel: [f32; 3],
    bounces: u32,
}

/// Gem sprite assets by colour, for keys created at runtime (ejected/mixed keys).
#[derive(Resource, Default)]
pub struct GemSprites(pub HashMap<u32, (Handle<Mesh>, Handle<StandardMaterial>, u32)>);

pub fn gem_file(color: u32) -> Option<&'static str> {
    Some(match color {
        1 => "gemre+17.pcx",
        2 => "gemye+17.pcx",
        3 => "gemor+17.pcx",
        4 => "gembl+17.pcx",
        5 => "gemma+17.pcx",
        6 => "gemve+17.pcx",
        MASTER_KEY => "gemmu+17.pcx",
        _ => return None,
    })
}

pub fn init_bonus3(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((Diamond { points: obj.skills[1] as i32 }, PickupPos(obj.origin)));
    if obj.flags & (1 << 5) == 0 {
        entity.insert(SnapToGround); // Flag6 keeps the WED height
    }
}

pub fn init_key(obj: &Object, entity: &mut EntityCommands) {
    // Skill5 "surprise" gems pick a random colour; not ported yet (uses Skill6).
    entity.insert((
        KeyGem { color: obj.skills[5] as u32 },
        PickupPos(obj.origin),
        LevelKey { home: None, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs() },
        Respawnable,
    ));
    if obj.flags & (1 << 5) == 0 {
        entity.insert(SnapToGround);
    }
}

pub fn snap_pickups(world: CollisionWorld, mut commands: Commands, mut q: Query<(Entity, &mut PickupPos, &mut Transform), With<SnapToGround>>) {
    if !world.ready() {
        return;
    }
    for (e, mut p, mut tf) in &mut q {
        let [x, y, z] = p.0;
        let tr = world.trace(1, [x, y, z + 8.0], [x, y, z - 2000.0], None).trace;
        if tr.hit() && !tr.start_solid {
            p.0[2] = tr.end[2] - 16.0 + HEIGHT_GEM;
            tf.translation = coords::pos(p.0);
        }
        commands.entity(e).remove::<SnapToGround>();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collect(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    diamonds: Query<(Entity, &PickupPos, &Diamond), (Without<Shrinking>, Without<SnapToGround>)>,
    mut keys: Query<
        (Entity, &PickupPos, &KeyGem, Option<&mut LevelKey>),
        (Without<Shrinking>, Without<Bouncing>, Without<SnapToGround>, Without<Collected>),
    >,
    sprites: Res<GemSprites>,
) {
    let Ok(p) = player.single() else { return };
    if p.state == super::player::State::Dead {
        return;
    }
    let near = |q: &[f32; 3]| (0..3).map(|i| (q[i] - p.pos[i]).powi(2)).sum::<f32>().sqrt() <= p.scan_radius;

    for (e, pos, d) in &diamonds {
        if near(&pos.0) {
            commands.entity(e).insert(Shrinking);
            sfx.play("Diam_son");
            if d.points == 50 || d.points == 100 {
                sfx.play("clog3");
            }
            game.add_points(d.points, &mut sfx);
        }
    }

    // All key gems vanish once the goal is reached (gems.wdl:77).
    if game.goal_reached {
        for (e, ..) in &keys {
            commands.entity(e).insert(Shrinking);
        }
        return;
    }
    if game.key_in_flight {
        return;
    }
    // Key_Hit: take the gem; mix primaries, otherwise throw the old key back out.
    for (e, pos, gem, level_key) in &mut keys {
        if !near(&pos.0) {
            continue;
        }
        commands.entity(e).insert(Shrinking);
        if let Some(mut lk) = level_key {
            lk.home.get_or_insert(pos.0);
        }
        sfx.play("ColoredGem_son");
        let (key, outcome) = take_key(game.key, gem.color);
        game.key = key;
        match outcome {
            KeyOutcome::Mixed => sfx.play("GemMixing_son"),
            KeyOutcome::Ejected(old) => eject_key(&mut commands, &mut game, &sprites, old, p, 30.0),
            KeyOutcome::Taken => {}
        }
        break; // one gem per scan
    }
}

#[derive(Debug, PartialEq)]
pub enum KeyOutcome {
    Taken,
    /// Two different primaries combined (red+yellow=orange, red+blue=magenta, yellow+blue=green).
    Mixed,
    /// The previously held key is thrown back out.
    Ejected(u32),
}

/// `Key_Hit` (gems.wdl:212-350): the key held after picking up `gem` while holding `held`.
pub fn take_key(held: u32, gem: u32) -> (u32, KeyOutcome) {
    let primary = |c: u32| matches!(c, 1 | 2 | 4);
    match held {
        0 => (gem, KeyOutcome::Taken),
        _ if primary(held) && primary(gem) && held != gem => (held | gem, KeyOutcome::Mixed),
        _ => (gem, KeyOutcome::Ejected(held)),
    }
}

/// `EjectOldKey` + `LaunchYouKey`: the old key pops out behind Sammy and bounces.
/// `EjectNewKey` (on death) launches it with vz 40.
pub fn eject_key(commands: &mut Commands, game: &mut GameState, sprites: &GemSprites, color: u32, p: &Player, vz: f32) {
    let Some((mesh, mat, frames)) = sprites.0.get(&color).cloned() else { return };
    game.key_in_flight = true;
    let pos = [p.pos[0], p.pos[1], p.pos[2] + 50.0];
    let back = (p.pan + 180.0).to_radians();
    commands.spawn((
        Name::new("ejected key"),
        crate::flow::LevelScoped,
        Transform::from_translation(coords::pos(pos)),
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Billboard,
        SpriteAnim { frames, frame: 0.0, fps: 48.0 },
        KeyGem { color },
        PickupPos(pos),
        Bouncing { vel: [back.cos() * 12.0, back.sin() * 12.0, vz], bounces: 0 },
    ));
}

pub fn bounce_keys(
    time: Res<Time>,
    world: CollisionWorld,
    mut game: ResMut<GameState>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut PickupPos, &mut Transform, &mut Bouncing)>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut p, mut tf, mut b) in &mut q {
        b.vel[2] -= 4.0 * t;
        let target = [0, 1, 2].map(|i| p.0[i] + b.vel[i] * t);
        let hit = world.trace(1, p.0, target, None).trace;
        p.0 = hit.end;
        if hit.hit() {
            // Reflect off the surface, losing energy.
            let n = hit.normal;
            let d = b.vel[0] * n[0] + b.vel[1] * n[1] + b.vel[2] * n[2];
            b.vel = [0, 1, 2].map(|i| (b.vel[i] - 2.0 * d * n[i]) * 0.6);
            if n[2] > 0.7 {
                b.bounces += 1;
            }
        }
        // Collectable again after the 5th bounce (docs/gameplay.md §4.2).
        if b.bounces >= 5 {
            p.0[2] = hit.end[2] - 16.0 + HEIGHT_GEM;
            commands.entity(e).remove::<Bouncing>();
            game.key_in_flight = false;
        }
        tf.translation = coords::pos(p.0);
    }
}

pub fn shrink(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, Has<Respawnable>), With<Shrinking>>,
) {
    let k = 0.8f32.powf(time.delta_secs() * TICKS);
    for (e, mut tf, respawnable) in &mut q {
        tf.scale *= k;
        if tf.scale.x < 0.05 {
            if respawnable {
                commands.entity(e).remove::<Shrinking>().insert((Collected, Visibility::Hidden));
            } else {
                commands.entity(e).despawn();
            }
        }
    }
}

/// Reset scan (id 1000) for keys: level keys come back home (or vanish for good
/// once Sammy holds the master key); runtime keys are removed.
pub fn reset_keys(
    mut reset: MessageReader<ResetScan>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut keys: Query<(Entity, &mut PickupPos, &mut Transform, Option<&LevelKey>), With<KeyGem>>,
) {
    if reset.read().count() == 0 {
        return;
    }
    game.key_in_flight = false;
    for (e, mut pos, mut tf, level_key) in &mut keys {
        match level_key {
            Some(lk) if game.key != MASTER_KEY => {
                if let Some(home) = lk.home {
                    pos.0 = home;
                }
                tf.translation = coords::pos(pos.0);
                tf.scale = lk.scale;
                commands.entity(e).remove::<(Collected, Shrinking)>().insert(Visibility::Inherited);
            }
            _ => {
                commands.entity(e).despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mixing_follows_key_hit() {
        assert_eq!(take_key(0, 4), (4, KeyOutcome::Taken));
        assert_eq!(take_key(1, 2), (3, KeyOutcome::Mixed));
        assert_eq!(take_key(4, 1), (5, KeyOutcome::Mixed));
        assert_eq!(take_key(2, 4), (6, KeyOutcome::Mixed));
        // Same colour or a mixed key: the old one is ejected.
        assert_eq!(take_key(2, 2), (2, KeyOutcome::Ejected(2)));
        assert_eq!(take_key(3, 4), (4, KeyOutcome::Ejected(3)));
        assert_eq!(take_key(1, MASTER_KEY), (MASTER_KEY, KeyOutcome::Ejected(1)));
    }
}
