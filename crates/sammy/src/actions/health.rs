//! Health, lives and hazards:
//! - `InitTileDeath`/`TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava`/`Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitRestart` checkpoints (mm1.wdl:3771)

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::health::{BIG_HEART_SPIN, BOOST_AWAY_SHRINK, BOOST_HIDDEN, DeathEvent, HEART_AWAY_SHRINK, HEART_HIDDEN_BELOW, HEART_REGROW, Victim};

use super::pickups::{Collected, GemSprites, PickupPos, Respawnable, Shrinking, SnapToGround};
use super::player::{Player, State, TICKS};
use crate::audio::Sfx;
use crate::camera::FollowState;
use crate::coords;
use crate::game::{GameState, ResetScan};
use crate::model::ModelAnimator;
use super::is_playing;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

/// Hazards, hearts, boosts, checkpoints and the death/respawn sequence.
pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, detect_deaths.run_if(is_playing).in_set(GameSet::Hazards))
            .add_systems(Update, collect_health.run_if(is_playing).in_set(GameSet::HealthPickups))
            .add_systems(Update, (spin_pickups.in_set(GameSet::SpinHealth), animate_hearts.in_set(GameSet::AnimateHearts), reset_checkpoints.in_set(GameSet::ResetCheckpoints)))
            .add_systems(Update, run_death.in_set(GameSet::Death));
    }
}


#[derive(Component)]
pub struct DeathTile;

#[derive(Component)]
pub struct LavaTile;

/// A small heart ([`sammy_core::health::SmallHeart`]) and its full size.
#[derive(Component, Deref, DerefMut)]
pub struct SmallHeart {
    #[deref]
    heart: sammy_core::health::SmallHeart,
    scale: Vec3,
}

#[derive(Component, Deref)]
pub struct BigHeart(sammy_core::health::BigHeart);

/// Boost pickup ([`sammy_core::health::BoostPickup`]) and its full size.
#[derive(Component, Deref, DerefMut)]
pub struct BoostPickup {
    #[deref]
    boost: sammy_core::health::BoostPickup,
    scale: Vec3,
}

#[derive(Component, Deref, DerefMut)]
pub struct Checkpoint(sammy_core::health::Checkpoint);

/// Running death/respawn sequence on Sammy ([`sammy_core::health::Dying`]).
#[derive(Component, Deref, DerefMut)]
pub struct Dying(sammy_core::health::Dying);

fn wed_scale(obj: &Object) -> Vec3 {
    Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs()
}

pub fn init_boost(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((BoostPickup { boost: sammy_core::health::BoostPickup::from_object(obj), scale: wed_scale(obj) }, PickupPos(obj.origin)));
}

pub fn init_tile_death(entity: &mut EntityCommands) {
    entity.insert(DeathTile);
}

pub fn init_lava(entity: &mut EntityCommands) {
    entity.insert(LavaTile);
}

pub fn init_small_heart(obj: &Object, entity: &mut EntityCommands) {
    let heart = sammy_core::health::SmallHeart::from_object(obj);
    let snap = heart.snap;
    entity.insert((SmallHeart { heart, scale: wed_scale(obj) }, PickupPos(obj.origin)));
    if snap {
        entity.insert(SnapToGround);
    }
}

pub fn init_bonus_vie(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((BigHeart(sammy_core::health::BigHeart::from_object(obj)), PickupPos(obj.origin)));
}

pub fn init_restart(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((Checkpoint(sammy_core::health::Checkpoint::from_object(obj)), PickupPos(obj.origin), Respawnable));
}

/// Records the level spawn as the first respawn point.
pub fn init_start(pos: [f32; 3], pan: f32, entity: &mut EntityCommands) {
    entity.commands().queue(move |w: &mut World| w.resource_mut::<GameState>().start = Some((pos, pan)));
}

/// Hazards under Sammy's feet and running out of health start a death.
pub fn detect_deaths(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    player: Query<(Entity, &Player), Without<Dying>>,
    tiles: Query<(Has<DeathTile>, Has<LavaTile>)>,
) {
    let Ok((e, p)) = player.single() else { return };
    // `Player_Kaputt` returns at once while `PlayerEtat == 3` (extro scripted Sammy).
    if matches!(p.state, State::Dead | State::Scripted) {
        return;
    }
    let (death, lava) = p.ground().filter(|_| p.grounded && p.vz == 0.0).and_then(|g| tiles.get(g).ok()).unwrap_or_default();
    if let Some(kind) = sammy_core::health::check_death(&mut game, death, lava, time.delta_secs() * TICKS) {
        commands.entity(e).insert(Dying(sammy_core::health::Dying::new(kind)));
    }
}

pub fn set_alpha(materials: &mut Assets<StandardMaterial>, mat: &MeshMaterial3d<StandardMaterial>, alpha: f32) {
    if let Some(mut m) = materials.get_mut(&mat.0) {
        let a = (alpha / 100.0).clamp(0.0, 1.0);
        m.alpha_mode = if a < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque };
        m.base_color.set_alpha(a);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_death(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    sprites: Res<GemSprites>,
    jump: Res<super::player::JumpParams>,
    mut reset: MessageWriter<ResetScan>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut follow: Query<&mut FollowState>,
    mut player: Query<(Entity, &mut Player, &mut Dying, &mut Visibility, &MeshMaterial3d<StandardMaterial>, Option<&mut ModelAnimator>)>,
    mut events: Local<Vec<DeathEvent>>,
) {
    let Ok((e, mut p, mut d, mut vis, mat, mut anim)) = player.single_mut() else { return };
    let t = time.delta_secs() * TICKS;
    let victim = Victim { pos: p.pos, pan: p.pan, vel: p.vel };
    events.clear();
    // Opacity is applied on frames that run a fade stage (not on the frame one starts).
    let fading = d.fading();
    d.step(&mut game, anim.as_deref_mut().map(|a| &mut a.anim), victim, t, &mut events);
    if fading {
        set_alpha(&mut materials, mat, d.alpha);
    }
    for ev in events.drain(..) {
        match ev {
            DeathEvent::Started => p.state = State::Dead,
            DeathEvent::DropKey(l) => super::gems::launch_gem(&mut commands, &sprites, &jump, l),
            DeathEvent::Sound(s) => sfx.play(s),
            DeathEvent::CameraLookDown { t } => {
                for mut f in follow.iter_mut() {
                    f.tilt_scale = Some(sammy_core::health::look_down(f.tilt_scale.unwrap_or(1.0), t));
                }
            }
            DeathEvent::GameOver => {
                for mut f in follow.iter_mut() {
                    f.tilt_scale = Some(0.0);
                    f.orbit = true;
                }
            }
            DeathEvent::Respawn(at) => {
                reset.write(ResetScan);
                if let Some((pos, pan)) = at {
                    p.teleport(pos, pan);
                    let snap = sammy_core::platforms::CAMERA_SNAP;
                    for mut f in follow.iter_mut() {
                        f.snap_to([pos[0] + snap[0], pos[1] + snap[1], pos[2] + snap[2]]);
                    }
                }
            }
            DeathEvent::Visible(v) => *vis = if v { Visibility::Inherited } else { Visibility::Hidden },
            DeathEvent::Finished => {
                p.state = State::Stand;
                commands.entity(e).remove::<Dying>();
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn collect_health(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut hearts: Query<(Entity, &PickupPos, &mut SmallHeart), (Without<Shrinking>, Without<SnapToGround>)>,
    big: Query<(Entity, &PickupPos, &BigHeart), Without<Shrinking>>,
    checkpoints: Query<(Entity, &PickupPos, &Checkpoint), (Without<Shrinking>, Without<Collected>)>,
    mut boosts: Query<(Entity, &PickupPos, &mut BoostPickup, &mut Transform, &mut Visibility), Without<Shrinking>>,
) {
    let Ok(p) = player.single() else { return };
    if p.state == State::Dead {
        return;
    }
    let near = |q: &[f32; 3]| sammy_core::pickups::in_scan(p.pos, p.scan_radius, *q);

    for (e, pos, mut heart) in &mut hearts {
        if heart.away.is_some() || !near(&pos.0) {
            continue;
        }
        if !sammy_core::health::take_small_heart(&mut heart, &mut game) {
            continue; // full: duck away and come back
        }
        sfx.play("Gagne1");
        sfx.play("Chimes2");
        let at = pos.0;
        crate::effects::queue(&mut commands, move |ps| crate::effects::small_heart(ps, at));
        commands.entity(e).insert(Shrinking);
    }
    for (e, pos, b) in &big {
        if near(&pos.0) && sammy_core::health::take_big_heart(b, &mut game) {
            sfx.play("Getlife_son");
            // One `particle_Getlife` every 2 ticks while the light shrinks (mm1.wdl:2696-2706).
            let at = pos.0;
            crate::effects::queue(&mut commands, move |ps| {
                for i in 0..6 {
                    crate::effects::get_life(ps, at, i as f32 * 2.0);
                }
            });
            commands.entity(e).insert((Shrinking, crate::lights::flash([50.0, 250.0, 10.0], 300.0, 40.0, 64.0)));
        }
    }
    for (e, pos, mut b, mut tf, mut vis) in &mut boosts {
        if let Some(left) = b.away {
            // Ducked away because boost was full: shrink, hide, grow back.
            let t = time.delta_secs() * TICKS;
            let left = left - t;
            if left > BOOST_HIDDEN {
                tf.scale *= BOOST_AWAY_SHRINK.powf(t);
            } else {
                *vis = Visibility::Hidden;
            }
            if left <= 0.0 {
                *vis = Visibility::Inherited;
                tf.scale = b.scale;
                b.away = None;
            } else {
                b.away = Some(left);
            }
            continue;
        }
        if !near(&pos.0) || !sammy_core::health::take_boost(&mut b, &mut game) {
            continue;
        }
        sfx.play("chimes2");
        commands.entity(e).insert((Shrinking, crate::lights::flash([150.0, 50.0, 100.0], 200.0, 10.0, 64.0)));
    }
    for (e, pos, c) in &checkpoints {
        if near(&pos.0) {
            sfx.play("Restart_Son");
            sammy_core::health::take_checkpoint(c, &mut game);
            commands.entity(e).insert(Shrinking);
        }
    }
}

/// Small hearts that ducked away (health was full) shrink, wait and grow back.
pub fn animate_hearts(time: Res<Time>, mut q: Query<(&mut SmallHeart, &mut Transform, &mut Visibility), Without<Shrinking>>) {
    let t = time.delta_secs() * TICKS;
    for (mut h, mut tf, mut vis) in &mut q {
        let Some(left) = h.away else { continue };
        let left = left - t;
        if left > 0.0 {
            tf.scale *= HEART_AWAY_SHRINK.powf(t);
            if tf.scale.x < HEART_HIDDEN_BELOW {
                *vis = Visibility::Hidden;
            }
            h.away = Some(left);
        } else {
            *vis = Visibility::Inherited;
            tf.scale = (tf.scale.max(Vec3::splat(HEART_HIDDEN_BELOW)) * HEART_REGROW.powf(t)).min(h.scale);
            if tf.scale.x >= h.scale.x {
                h.away = None;
            }
        }
    }
}

/// Big hearts spin; checkpoints spin when Flag3 is set.
pub fn spin_pickups(
    time: Res<Time>,
    mut hearts: Query<(&mut Transform, &PickupPos), (With<BigHeart>, Without<Checkpoint>)>,
    mut checkpoints: Query<(&mut Transform, &mut Checkpoint), Without<BigHeart>>,
) {
    let t = time.delta_secs() * TICKS;
    for (mut tf, _) in &mut hearts {
        tf.rotate_y((BIG_HEART_SPIN * t).to_radians());
    }
    for (mut tf, mut c) in &mut checkpoints {
        if let Some(spin) = c.spin {
            for i in 0..3 {
                c.angles[i] += spin[i] * t;
            }
            tf.rotation = coords::rotation(c.angles);
        }
    }
}

/// Reset scan for checkpoints (`RestartWaitForRestart`): taken ones come back.
pub fn reset_checkpoints(
    mut reset: MessageReader<ResetScan>,
    mut commands: Commands,
    mut q: Query<(Entity, &Checkpoint, &mut Transform), With<Collected>>,
) {
    if reset.read().count() == 0 {
        return;
    }
    for (e, c, mut tf) in &mut q {
        tf.translation = coords::pos(c.home);
        tf.scale = Vec3::ONE;
        commands.entity(e).remove::<Collected>().insert(Visibility::Inherited);
    }
}
