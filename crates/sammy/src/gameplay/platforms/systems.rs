#[allow(unused_imports)]
use super::*;

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
            crate::gameplay::doors::morph(&mut commands, g, &mut brush, children, &assets, UNLOCKED_PLATFORM);
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
