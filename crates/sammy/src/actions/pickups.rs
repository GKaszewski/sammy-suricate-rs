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

impl LevelKey {
    pub fn at(home: [f32; 3], scale: Vec3) -> Self {
        Self { home: Some(home), scale }
    }
}

/// Flag5 gems hover at their WED height above the floor (+HeightGem) instead of 25.
#[derive(Component)]
pub struct HoverFromWed;

/// Restored by the reset scan instead of being despawned after shrinking.
#[derive(Component)]
pub struct Respawnable;

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
    let flag = |n: u32| obj.flags & (1 << (n - 1)) != 0;
    entity.insert((
        KeyGem { color: obj.skills[5] as u32 },
        PickupPos(obj.origin),
        LevelKey { home: None, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs() },
        Respawnable,
    ));
    if obj.skills[4] != 0.0 {
        // Surprise gem: not collectable until its colour has been rolled.
        entity.insert(super::gems::Surprise::new(obj.skills[4], obj.origin[0] + obj.origin[1]));
    } else {
        entity.insert(super::gems::Collectable);
    }
    if flag(7) {
        entity.insert(super::gems::GemGravity::new(HEIGHT_GEM));
        if flag(5) {
            entity.insert(HoverFromWed);
        }
    }
    if !flag(6) {
        entity.insert(SnapToGround);
    }
}

pub fn snap_pickups(
    world: CollisionWorld,
    mut commands: Commands,
    mut q: Query<
        (Entity, &mut PickupPos, &mut Transform, Option<&super::fx::ExtraHeight>, Option<&mut super::gems::GemGravity>, Has<HoverFromWed>),
        With<SnapToGround>,
    >,
) {
    if !world.ready() {
        return;
    }
    for (e, mut p, mut tf, extra, gravity, hover) in &mut q {
        let [x, y, z] = p.0;
        let tr = world.trace(1, [x, y, z + 8.0], [x, y, z - 2000.0], None).trace;
        if hover && let Some(mut g) = gravity {
            // Flag5: keep the WED height above the floor (+HeightGem) as the rest height.
            g.set_rest(z - (tr.end[2] - 16.0) + HEIGHT_GEM);
        }
        if tr.hit() && !tr.start_solid {
            p.0[2] = tr.end[2] - 16.0 + HEIGHT_GEM + extra.map_or(0.0, |h| h.0);
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
        (Entity, &PickupPos, &KeyGem, Option<&mut LevelKey>, Option<&super::gems::GemLifetime>, Has<super::gems::Collectable>),
        (Without<Shrinking>, Without<SnapToGround>, Without<Collected>),
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
        for (e, _, _, _, _, collectable) in &keys {
            if collectable {
                commands.entity(e).insert(Shrinking);
            }
        }
        return;
    }
    // A gem flying into a door blocks pickups (`PlayerOldKey == 999`).
    if game.key_in_flight {
        return;
    }
    // Key_Hit: take the gem; mix primaries, otherwise throw the old key back out.
    for (e, pos, gem, level_key, life, collectable) in &mut keys {
        if !collectable || !near(&pos.0) {
            continue;
        }
        commands.entity(e).insert((Shrinking, crate::lights::flash(crate::lights::key_color(gem.color), 300.0, 15.0, 100.0)));
        if let Some(mut lk) = level_key {
            lk.home.get_or_insert(pos.0);
        }
        sfx.play("ColoredGem_son");
        let (key, outcome) = take_key(game.key, gem.color);
        game.key = key;
        match outcome {
            KeyOutcome::Mixed => sfx.play("GemMixing_son"),
            KeyOutcome::Ejected(old) => super::gems::throw_back_key(&mut commands, &sprites, old, p, life.map_or(0.0, |l| l.ticks())),
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
    game: Res<GameState>,
    mut rng: ResMut<crate::game::Rng>,
    mut keys: Query<(Entity, &mut PickupPos, &mut Transform, Option<&LevelKey>, Option<&mut super::gems::Surprise>), With<KeyGem>>,
) {
    if reset.read().count() == 0 {
        return;
    }
    for (e, mut pos, mut tf, level_key, surprise) in &mut keys {
        match level_key {
            Some(lk) if game.key != MASTER_KEY => {
                if let Some(home) = lk.home {
                    pos.0 = home;
                }
                tf.translation = coords::pos(pos.0);
                tf.scale = lk.scale;
                let mut ent = commands.entity(e);
                ent.remove::<(Collected, Shrinking)>().insert(Visibility::Inherited);
                // InitKey runs again: a surprise gem re-rolls (its colour keeps climbing).
                match surprise {
                    Some(mut s) => {
                        s.rearm(&mut rng);
                        ent.remove::<super::gems::Collectable>();
                    }
                    None => {
                        ent.insert(super::gems::Collectable);
                    }
                }
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
