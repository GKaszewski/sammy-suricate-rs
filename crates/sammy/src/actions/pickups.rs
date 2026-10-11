//! Diamonds (`InitBonus3`/`Bonus3_Hit`, mm1.wdl:1200-1291) and colour key gems
//! (`InitKey`/`Key_Hit`/`EjectOldKey`/`LaunchYouKey`, gems.wdl:40-960).
//! Both are picked up by Sammy's scan (id 1, radius `BlobZoneScan`).

use std::collections::HashMap;

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::gems::HEIGHT_GEM;
use sammy_core::keys::{KeyOutcome, take_key};
use sammy_core::pickups::{KeySpec, NARROW_FLOOR_OFFSET, SHRINK_GONE, SHRINK_PER_TICK, SNAP_START, in_scan};

use super::player::{Player, TICKS};
use crate::audio::Sfx;
use crate::collision::{CollisionWorld, Hull};
use crate::coords;
use crate::game::{GameState, MASTER_KEY, ResetScan};

/// A diamond ([`sammy_core::pickups::Diamond`]).
#[derive(Component, Deref)]
pub struct Diamond(pub sammy_core::pickups::Diamond);

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

pub use sammy_core::keys::gem_sprite as gem_file;
use super::is_playing;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

/// Diamonds, key gems (with the gem machinery of `gems`) and boulders.
pub struct PickupsPlugin;

impl Plugin for PickupsPlugin {
    fn build(&self, app: &mut App) {
        use super::gems;
        app.init_resource::<GemSprites>()
            .add_systems(Update, shrink.in_set(GameSet::Shrink))
            .add_systems(
                Update,
                (
                    gems::surprise_gems.in_set(GameSet::SurpriseGems),
                    gems::gem_gravity.in_set(GameSet::GemGravity),
                    gems::gem_lifetimes.in_set(GameSet::GemLifetimes),
                    gems::fly_gems.in_set(GameSet::GemFlight),
                ),
            )
            .add_systems(Update, (collect.run_if(is_playing).in_set(GameSet::Collect), snap_pickups.in_set(GameSet::SnapPickups)))
            .add_systems(Update, reset_keys.in_set(GameSet::KeyReset))
            .add_systems(Update, gems::run_boulders.run_if(is_playing).in_set(GameSet::Boulders));
    }
}


pub fn init_bonus3(obj: &Object, entity: &mut EntityCommands) {
    let d = sammy_core::pickups::Diamond::from_object(obj);
    entity.insert((Diamond(d), PickupPos(obj.origin)));
    if d.snap {
        entity.insert(SnapToGround);
    }
}

pub fn init_key(obj: &Object, entity: &mut EntityCommands) {
    let spec = KeySpec::from_object(obj);
    entity.insert((
        KeyGem { color: spec.color },
        PickupPos(obj.origin),
        LevelKey { home: None, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs() },
        Respawnable,
    ));
    match spec.surprise {
        // Surprise gem: not collectable until its colour has been rolled.
        Some(add) => entity.insert(super::gems::Surprise::new(add, obj.origin[0] + obj.origin[1])),
        None => entity.insert(super::gems::Collectable),
    };
    if spec.gravity {
        entity.insert(super::gems::GemGravity::new(HEIGHT_GEM));
    }
    if spec.hover_from_wed {
        entity.insert(HoverFromWed);
    }
    if spec.snap {
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
        let tr = world.trace(Hull::Narrow, [x, y, z + SNAP_START], [x, y, z - 2000.0], None).trace;
        let floor = tr.end[2] - NARROW_FLOOR_OFFSET;
        if hover && let Some(mut g) = gravity {
            // Flag5: keep the WED height above the floor (+HeightGem) as the rest height.
            g.rest = z - floor + HEIGHT_GEM;
        }
        if tr.hit() && !tr.start_solid {
            p.0[2] = floor + HEIGHT_GEM + extra.map_or(0.0, |h| h.0);
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
    let near = |q: &[f32; 3]| in_scan(p.pos, p.scan_radius, *q);

    for (e, pos, d) in &diamonds {
        if near(&pos.0) {
            commands.entity(e).insert(Shrinking);
            let at = pos.0;
            crate::effects::queue(&mut commands, move |ps| crate::effects::diamond_burst(ps, at));
            sfx.play("Diam_son");
            if d.has_popup() {
                sfx.play("clog3");
                crate::hud3d::queue_popup(&mut commands, pos.0, d.points);
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
        let (at, color, held) = (pos.0, gem.color, game.key);
        crate::effects::queue(&mut commands, move |ps| crate::effects::gem_catch(ps, at, color));
        let (key, outcome) = take_key(game.key, gem.color);
        game.key = key;
        match outcome {
            KeyOutcome::Mixed => {
                sfx.play("GemMixing_son");
                // AnimGemMix1 end: each source primary bursts 70 stars (gems.wdl:596-605).
                let sammy = [p.pos[0], p.pos[1], p.pos[2] + 40.0];
                crate::effects::queue(&mut commands, move |ps| {
                    crate::effects::gem_mix(ps, sammy, held);
                    crate::effects::gem_mix(ps, sammy, color);
                });
            }
            KeyOutcome::Ejected(old) => super::gems::throw_back_key(&mut commands, &sprites, old, p, life.map_or(0.0, |l| l.ticks)),
            KeyOutcome::Taken => {}
        }
        break; // one gem per scan
    }
}

pub fn shrink(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, Has<Respawnable>), With<Shrinking>>,
) {
    let k = SHRINK_PER_TICK.powf(time.delta_secs() * TICKS);
    for (e, mut tf, respawnable) in &mut q {
        tf.scale *= k;
        if tf.scale.x < SHRINK_GONE {
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
                        s.rearm(&mut rng.0);
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
