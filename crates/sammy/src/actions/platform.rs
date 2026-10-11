//! Moving platforms (`InitEnginePlatForm` / `InitPlatForm` / `InitBodyPlatForm`,
//! mm1.wdl:2238-2629) and teleport pads (`InitTeleport` / `Teleport_Hit`, mm1.wdl:4099-4300).
//!
//! An invisible *engine* runs along its local +X at Skill2 quants/tick through
//! hidden rails in the level. On impact it either reflects (Flag3) or turns by
//! Skill4/5/7 and pauses Skill3 ticks. Platforms and bodies with the same link id
//! (Skill1) copy the engine's motion at a fixed offset, carrying whatever stands
//! on them. An engine with Skill6 is locked until Sammy stands on its platform
//! holding a matching key.

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::platforms::{TeleportStep, UNLOCKED_PLATFORM};

use super::health::set_alpha;
use super::player::{LevelCollision, Player, State, TICKS};
use crate::audio::Sfx;
use crate::camera::FollowState;
use crate::collision::{Brush, Hull};
use crate::coords;
use crate::game::GameState;
use super::is_playing;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

/// Engines, platforms and teleport pads.
pub struct PlatformsPlugin;

impl Plugin for PlatformsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                link_platforms.in_set(GameSet::LinkPlatforms),
                run_engines.run_if(is_playing).in_set(GameSet::RunEngines),
                follow_engines.in_set(GameSet::FollowEngines),
                ride_platforms.in_set(GameSet::RidePlatforms),
            ),
        )
            .add_systems(Update, enter_teleports.run_if(is_playing).in_set(GameSet::TeleportEntry))
            .add_systems(Update, run_teleports.in_set(GameSet::Teleports));
    }
}


/// An engine ([`sammy_core::platforms::Engine`]).
#[derive(Component, Deref, DerefMut)]
pub struct Engine(pub sammy_core::platforms::Engine);

/// A platform or body ([`sammy_core::platforms::Platform`]) and the engine it follows.
#[derive(Component, Deref, DerefMut)]
pub struct Platform {
    #[deref]
    pub platform: sammy_core::platforms::Platform,
    engine: Option<Entity>,
}

/// A teleport pad ([`sammy_core::platforms::Teleport`]).
#[derive(Component, Deref, DerefMut)]
pub struct Teleport(pub sammy_core::platforms::Teleport);

/// Sammy is being teleported from `pad` (input locked).
#[derive(Component, Deref, DerefMut)]
pub struct Teleporting {
    pad: Entity,
    #[deref]
    state: sammy_core::platforms::Teleporting,
}

pub fn init_engine(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Engine(sammy_core::platforms::Engine::from_object(obj)));
}

pub fn init_platform(obj: &Object, body: bool, entity: &mut EntityCommands) {
    entity.insert(Platform { platform: sammy_core::platforms::Platform::from_object(obj, body), engine: None });
}

pub fn init_teleport(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Teleport(sammy_core::platforms::Teleport::from_object(obj)));
}

/// `SetLinkMoteur`: each platform attaches to the engine with its link id.
pub fn link_platforms(engines: Query<(Entity, &Engine)>, mut platforms: Query<(&mut Platform, &Brush), Added<Platform>>) {
    for (mut p, brush) in &mut platforms {
        if let Some((e, eng)) = engines.iter().find(|(_, eng)| eng.link == p.link) {
            p.engine = Some(e);
            p.platform.attach(brush.origin, eng);
        }
    }
}

pub fn run_engines(
    time: Res<Time>,
    level: Option<Res<LevelCollision>>,
    player: Query<&Player>,
    platforms: Query<&Platform>,
    mut engines: Query<(Entity, &mut Engine)>,
) {
    let Some(level) = level else { return };
    let t = time.delta_secs() * TICKS;
    // Shift-boost while riding: the carrying engine runs faster (InitEnginePlatForm :2478-2491).
    let boosted = player
        .single()
        .ok()
        .filter(|p| p.boosting && p.grounded)
        .and_then(|p| p.ground())
        .and_then(|g| platforms.get(g).ok())
        .and_then(|pl| pl.engine);
    for (id, mut e) in &mut engines {
        e.step(boosted == Some(id), t, |a, b| level.0.trace(0, Hull::Point.index(), a, b));
    }
}

pub fn follow_engines(engines: Query<&Engine>, mut platforms: Query<(&mut Platform, &mut Brush, &mut Transform)>) {
    for (mut p, mut brush, mut tf) in &mut platforms {
        let Some(eng) = p.engine.and_then(|e| engines.get(e).ok()) else { continue };
        p.platform.follow(&mut brush.origin, eng);
        tf.translation = coords::pos(brush.origin);
    }
}

/// Riders move with the platform under them (`PlayerForce` = platform motion);
/// standing on a locked platform with the right key starts its engine.
#[allow(clippy::too_many_arguments)]
pub fn ride_platforms(
    game: Res<GameState>,
    mut sfx: Sfx,
    mut commands: Commands,
    assets: Res<crate::level::GameAssets>,
    platforms: Query<&Platform>,
    mut bodies: Query<(&mut Brush, Option<&Children>)>,
    mut engines: Query<&mut Engine>,
    mut player: Query<&mut Player>,
) {
    let Ok(mut p) = player.single_mut() else { return };
    if !p.grounded {
        return;
    }
    let Some(plat) = p.ground().and_then(|g| platforms.get(g).ok()) else { return };
    for i in 0..3 {
        p.pos[i] += plat.delta[i];
    }
    if let Some(mut eng) = plat.engine.and_then(|e| engines.get_mut(e).ok())
        && !eng.unlocked
        && eng.unlocks(game.key)
    {
        eng.unlocked = true;
        sfx.play("platform1_son");
        // The platform turns into the plain wooden one (`morph <platfor1.wmb>`).
        if let Some(g) = p.ground()
            && let Ok((mut brush, children)) = bodies.get_mut(g)
        {
            super::door::morph(&mut commands, g, &mut brush, children, &assets, UNLOCKED_PLATFORM);
        }
    }
}

/// `Teleport_Hit`: Sammy standing on a pad is faded out, moved and faded back in.
pub fn enter_teleports(mut commands: Commands, mut pads: Query<&mut Teleport>, player: Query<(Entity, &Player), Without<Teleporting>>) {
    let Ok((e, p)) = player.single() else { return };
    if !p.grounded || p.vz != 0.0 || p.state == State::Dead {
        return;
    }
    if let Some(g) = p.ground()
        && let Ok(mut pad) = pads.get_mut(g)
        && !pad.busy
    {
        pad.busy = true;
        commands.entity(e).insert(Teleporting { pad: g, state: sammy_core::platforms::Teleporting::new(&pad) });
    }
}

pub fn run_teleports(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut follow: Query<&mut FollowState>,
    mut pads: Query<&mut Teleport>,
    mut player: Query<(Entity, &mut Player, &mut Teleporting, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Ok((e, mut p, mut tp, mat)) = player.single_mut() else { return };
    let t = time.delta_secs() * TICKS;
    p.state = State::Dead; // PlayerMoveMode = 0
    match tp.step(t) {
        TeleportStep::Fading => {}
        TeleportStep::Jump => {
            p.teleport(tp.dest, tp.pan);
            let snap = sammy_core::platforms::CAMERA_SNAP;
            for mut f in &mut follow {
                f.snap_to([tp.dest[0] + snap[0], tp.dest[1] + snap[1], tp.dest[2] + snap[2]]);
            }
        }
        TeleportStep::Done => {
            p.state = State::Stand;
            if let Ok(mut pad) = pads.get_mut(tp.pad) {
                pad.busy = false;
            }
            commands.entity(e).remove::<Teleporting>();
        }
    }
    set_alpha(&mut materials, mat, tp.alpha);
}
