#[allow(unused_imports)]
use super::*;

pub fn advance_clock(time: Res<Time>, current: Res<crate::flow::CurrentLevel>, mut clock: ResMut<AiClock>) {
    clock.advance(time.delta_secs() * TICKS, crate::flow::playing(&current));
}

#[allow(clippy::too_many_arguments)]
pub fn tick_enemies(
    clock: Res<AiClock>,
    globals: Res<LionGlobals>,
    mut rng: ResMut<Rng>,
    mut sfx: Sfx,
    world: CollisionWorld,
    nodes: Query<&PathNode>,
    mut game: ResMut<GameState>,
    mut player: Query<(Entity, &mut Player)>,
    mut touches: MessageReader<PlayerTouched>,
    mut wasp_hits: MessageReader<WaspHit>,
    mut enemies: Query<(Entity, &mut Enemy, &mut ModelAnimator, &mut Visibility)>,
    mut calls: ResMut<FriendCalls>,
    mut warnings: MessageWriter<WarningAttack>,
    mut commands: Commands,
    (platforms, mut pads, pov): (Query<(Entity, &crate::gameplay::platforms::Platform)>, Query<(Entity, &mut crate::gameplay::platforms::Teleport)>, Res<crate::gameplay::extro::Pov>),
) {
    let touched: Vec<sammy_core::id::Id> = touches.read().map(|t| id(t.0)).collect();
    // Wasp hits (EVENT_IMPACT on the lion, §5.1) are applied as they arrive.
    for h in wasp_hits.read() {
        if let Ok((_, mut e, ..)) = enemies.get_mut(h.target) {
            sammy_core::enemies::apply_impact(&mut e, &Impact { poison: h.poison, bounce_force: h.bounce_force, wasp_pan: h.wasp_pan, wasp_disp: h.wasp_disp });
        }
    }
    // Distances and line of sight are measured to `pov_Syn` (lion.wdl:948-950).
    let nodes: Vec<sammy_core::enemies::PathNode> = nodes.iter().map(|n| n.0.clone()).collect();
    let platforms: Vec<_> = platforms.iter().map(|(e, p)| (id(e), p.delta)).collect();
    let mut pad_list: Vec<_> = pads.iter().map(|(e, p)| (id(e), p.0.clone())).collect();
    let (sammy_entity, sammy_alive) = match player.single() {
        Ok((e, p)) => (Some(id(e)), p.state != PlayerState::Dead),
        Err(_) => (None, false),
    };
    let view = World {
        space: &world,
        sammy: pov.entity.map(|e| (e, pov.pos, pov.vel)),
        sammy_entity,
        sammy_alive,
        touched: &touched,
        nodes: &nodes,
        platforms: &platforms,
        ticks1: clock.ticks1,
    };
    let mut out = Out::default();
    for _ in 0..clock.ticks {
        for (me, mut e, mut anim, mut vis) in &mut enemies {
            let mut shared = Shared { g: &globals, rng: &mut rng.0, calls: &mut calls.0, game: &mut game.0, pads: &mut pad_list };
            let ModelAnimator { model, anim } = &mut *anim;
            let p = player.single_mut().ok().map(|(_, p)| p.into_inner());
            sammy_core::enemies::tick(&mut e, id(me), anim, &model.mdl, p.map(|p| &mut p.0), &view, &mut shared, &mut out);
            if e.blinking() {
                *vis = if e.hidden { Visibility::Hidden } else { Visibility::Inherited };
            }
        }
        sammy_core::enemies::end_tick(&mut calls);
    }
    for (pad_e, mut pad) in &mut pads {
        if let Some((_, p)) = pad_list.iter().find(|(i, _)| *i == id(pad_e)) {
            pad.busy = p.busy;
        }
    }
    for s in out.sounds {
        sfx.play(s);
    }
    for (at, n) in out.hit_stars {
        crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::hit_stars(ps, at, n));
    }
    if out.warn {
        warnings.write(WarningAttack);
    }
    for p in out.call_rings {
        for (dz, delay, skin) in CALL_RINGS {
            commands.spawn((CallRing { delay, scale: 0.01, frame: 1.0, origin: [p[0], p[1], p[2] + dz], skin }, crate::flow::LevelScoped));
        }
    }
}

/// Smooth rendering between 16 Hz AI ticks.
pub fn place_enemies(clock: Res<AiClock>, mut q: Query<(&Enemy, &mut Transform)>) {
    let a = clock.acc.clamp(0.0, 1.0);
    for (e, mut tf) in &mut q {
        if let Some((pos, angles)) = e.pose(a) {
            tf.translation = coords::pos(pos);
            tf.rotation = coords::rotation(angles);
        }
    }
}

/// Wasp homing and fear tagging.
pub fn home_wasps(time: Res<Time>, mut wasps: Query<&mut Wasp>, mut enemies: Query<&mut Enemy>) {
    let t = time.delta_secs() * TICKS;
    for mut w in &mut wasps {
        let mut list: Vec<Mut<Enemy>> = enemies.iter_mut().collect();
        let mut refs: Vec<&mut sammy_core::enemies::Enemy> = list.iter_mut().map(|e| &mut e.0).collect();
        sammy_core::enemies::home_wasp(&mut w, &mut refs, t);
    }
}

/// Moves enemy boxes with their owners; dead enemies stop colliding.
pub fn sync_enemy_colliders(mut q: Query<(&Enemy, &mut BoxCollider)>) {
    for (e, mut c) in &mut q {
        if let Some((center, _, solid)) = e.collider() {
            c.center = center;
            c.solid = solid;
        }
    }
}

pub fn fade_enemies(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &Enemy, &mut MeshMaterial3d<StandardMaterial>, Option<&FadeMaterial>)>,
) {
    for (ent, e, mut mat, fade) in &mut q {
        match (e.alpha < 100.0, fade) {
            (true, None) => {
                let mut m = materials.get(&mat.0).cloned().unwrap_or_default();
                m.alpha_mode = AlphaMode::Blend;
                commands.entity(ent).insert(FadeMaterial(mat.0.clone()));
                mat.0 = materials.add(m);
            }
            (true, Some(_)) => {
                if let Some(mut m) = materials.get_mut(&mat.0) {
                    m.base_color.set_alpha(e.alpha / 100.0);
                }
            }
            (false, Some(orig)) => {
                mat.0 = orig.0.clone();
                commands.entity(ent).remove::<FadeMaterial>();
            }
            (false, None) => {}
        }
    }
}

pub fn run_start_triggers(clock: Res<AiClock>, mut triggers: Query<&mut StartTrigger>, mut enemies: Query<&mut Enemy>) {
    for _ in 0..clock.ticks {
        for mut tr in &mut triggers {
            let mut list: Vec<Mut<Enemy>> = enemies.iter_mut().collect();
            let mut refs: Vec<&mut sammy_core::enemies::Enemy> = list.iter_mut().map(|e| &mut e.0).collect();
            tr.tick(&mut refs);
        }
    }
}

/// Adds the collision box once the model is known.
pub fn add_enemy_colliders(mut commands: Commands, q: Query<(Entity, &Enemy), Without<BoxCollider>>) {
    for (ent, e) in &q {
        if let Some((center, half, _)) = e.collider() {
            commands.entity(ent).insert(BoxCollider(sammy_core::collision::BoxCollider { center, half, solid: true }));
        }
    }
}

/// Sammy's own collision box (scaled model bbox), so lions and wasps can touch him.
pub fn add_player_collider(mut commands: Commands, q: Query<(Entity, &Player, &ModelAnimator), Without<BoxCollider>>) {
    for (ent, p, anim) in &q {
        let Some((half, cz)) = sammy_core::enemies::player_box(&anim.model.mdl, crate::gameplay::player::SCALE) else { continue };
        commands.entity(ent).insert((BoxCollider(sammy_core::collision::BoxCollider { center: [p.pos[0], p.pos[1], p.pos[2] + cz], half, solid: true }), PlayerBoxOffset(cz)));
    }
}

pub fn sync_player_collider(mut q: Query<(&Player, &PlayerBoxOffset, &mut BoxCollider)>) {
    for (p, off, mut c) in &mut q {
        c.center = [p.pos[0], p.pos[1], p.pos[2] + off.0];
        c.solid = p.state != PlayerState::Dead;
    }
}

pub fn bump_clock(mut clock: ResMut<AiClock>) {
    clock.bump();
}

/// `RestartSammy` sets `AttackCall = 0` (player.wdl:894).
pub fn reset_calls(mut reset: MessageReader<crate::gameplay::state::ResetScan>, mut calls: ResMut<FriendCalls>) {
    if reset.read().count() > 0 {
        calls.attack_call = 0.0;
    }
}
