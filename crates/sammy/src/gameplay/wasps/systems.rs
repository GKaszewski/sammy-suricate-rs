#[allow(unused_imports)]
use super::*;

pub fn spin_pickups(time: Res<Time>, mut q: Query<(&mut Spinning, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (mut s, mut tf) in &mut q {
        s.angles[0] += PICKUP_SPIN * t;
        tf.rotation = coords::rotation(s.angles);
    }
}

pub fn collect_ammo(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    pickups: Query<(Entity, &PickupPos, &AmmoPickup), Without<Shrinking>>,
) {
    let Ok(p) = player.single() else { return };
    if p.state == State::Dead {
        return;
    }
    for (e, pos, a) in &pickups {
        if !sammy_core::pickups::in_scan(p.pos, p.scan_radius, pos.0) {
            continue;
        }
        a.take(&mut game);
        sfx.play("clog3");
        sfx.play("TakeWasp_son");
        let color = AMMO_COLORS[(a.kind.clamp(1, 7) - 1) as usize];
        commands.entity(e).insert((Shrinking, crate::render::lights::flash(color, 300.0, 15.0, 100.0)));
    }
}

/// `PlayerTire`: fire when Sammy is in a normal ground state.
#[allow(clippy::too_many_arguments)]
pub fn fire(
    time: Res<Time>,
    controls: Res<crate::platform::input::Controls>,
    current: Res<crate::flow::CurrentLevel>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut player: Query<(Entity, &mut Player, &mut ModelAnimator, Option<&mut PendingShots>)>,
    mut shots: Local<Vec<sammy_core::wasps::Shot>>,
) {
    let Ok((e, mut p, mut anim, pending)) = player.single_mut() else { return };
    let t = time.delta_secs() * TICKS;
    let kind = game.ammo_type.clamp(1, 7);
    // Debug: SAMMY_AUTOFIRE=1 fires whenever possible.
    let trigger = controls.fire_hit || std::env::var_os("SAMMY_AUTOFIRE").is_some();
    let fired = sammy_core::wasps::fire(&mut p, &mut anim.anim, &mut game, trigger, crate::flow::playing(&current));
    if fired.empty {
        sfx.play("Error1");
    }
    if let Some(new) = fired.thrown {
        commands.entity(e).insert(PendingShots(new));
    }
    // Release due shots of the current fan.
    if let Some(mut pending) = pending {
        shots.clear();
        if pending.release(kind, game.ammo_poison, p.pan, t, &mut shots) {
            commands.entity(e).remove::<PendingShots>();
        }
        for shot in shots.drain(..) {
            spawn_wasp(&mut commands, &assets, &mut meshes, &mut sfx, shot, e);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fly_wasps(
    time: Res<Time>,
    world: CollisionWorld,
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    mut hits: MessageWriter<WaspHit>,
    mut doors: Query<&mut WeakDoor>,
    player: Query<&Player>,
    mut wasps: Query<(Entity, &mut Wasp, &mut Transform, &mut Visibility)>,
) {
    let t = time.delta_secs() * TICKS;
    let sammy = player.single().map(|p| p.pos).ok();
    for (e, mut w, mut tf, mut vis) in &mut wasps {
        let step = w.step(&world, sammy, t);
        if w.visible && *vis == Visibility::Hidden {
            *vis = Visibility::Inherited;
        }
        tf.translation = coords::pos(w.pos);
        tf.rotation = coords::rotation([w.pan, w.tilt, 0.0]);
        if let WaspStep::Burst { hit } = step {
            if let Some(target) = hit.map(entity) {
                if let Ok(mut door) = doors.get_mut(target) {
                    door.hit(w.poison);
                } else {
                    hits.write(WaspHit { target, poison: w.poison, bounce_force: w.bounce_force, wasp_pan: w.pan, wasp_disp: w.last_disp });
                }
            }
            let (at, kind) = (w.pos, w.kind);
            crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::wasp_explode(ps, at, kind, 2.0));
            sfx.play(sammy_core::wasps::burst_sound(&mut rng));
            commands.entity(e).despawn();
        }
    }
}

/// `ExplodeDoorWeak`: HP 3/2/1 morph to doorwk2/3/4; otherwise the door breaks into planks.
#[allow(clippy::too_many_arguments)]
pub fn update_weak_doors(
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut doors: Query<(Entity, &WeakDoor, &mut Brush, &LevelObject, Option<&Children>), Changed<WeakDoor>>,
) {
    for (e, door, mut brush, obj, children) in &mut doors {
        match door.damage() {
            DoorDamage::Untouched => {}
            DoorDamage::Morph(file) => {
                sfx.play("plank1_Son");
                let Some((wmb, _)) = assets.prefab(file) else { continue };
                if !std::sync::Arc::ptr_eq(&brush.wmb, &wmb) {
                    crate::gameplay::doors::morph(&mut commands, e, &mut brush, children, &assets, file);
                }
            }
            DoorDamage::Destroyed => {
                // The door vanishes and six planks fly out (`LaunchPlank`).
                commands.entity(e).despawn();
                for file in PLANKS {
                    let Some(model) = assets.model(file) else { continue };
                    let plank = sammy_core::wasps::Plank::burst(obj.0.origin, &mut rng.0);
                    commands.spawn((
                        Name::new("plank"),
                        crate::flow::LevelScoped,
                        Mesh3d(meshes.add(model.build_mesh(0))),
                        MeshMaterial3d(model.material.clone()),
                        Transform { translation: coords::pos(plank.pos), rotation: coords::rotation(plank.angles), scale: Vec3::ONE },
                        Plank(plank),
                    ));
                }
            }
        }
    }
}

pub fn fly_planks(time: Res<Time>, world: CollisionWorld, mut sfx: Sfx, mut commands: Commands, mut q: Query<(Entity, &mut Plank, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut p, mut tf) in &mut q {
        let (sound, gone) = p.step(|a, b| world.trace(Hull::Point, a, b, None).trace, t);
        if sound {
            sfx.play("plank1_Son");
        }
        tf.translation = coords::pos(p.pos);
        tf.rotation = coords::rotation(p.angles);
        if gone {
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_hives(
    time: Res<Time>,
    mut commands: Commands,
    game: Res<GameState>,
    mut rng: ResMut<Rng>,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut hives: Query<(Entity, &mut Hive, &Transform)>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut h, tf) in &mut hives {
        let Some((l, exhausted)) = h.step(&game, &mut rng.0, t) else { continue };
        let Some(model) = assets.model("wasp1.mdl") else { continue };
        let mut anim = ModelAnimator::new(model.clone(), Vec::new(), 8.0);
        anim.play("walk", true, 0.0);
        anim.set_speed_pct(sammy_core::wasps::HIVE_WASP_ANIM, TICKS);
        commands.spawn((
            Name::new("hive wasp"),
            crate::flow::LevelScoped,
            Mesh3d(meshes.add(model.build_mesh(0))),
            MeshMaterial3d(model.skin(l.kind)),
            Transform { translation: coords::pos(h.pos), rotation: coords::rotation([l.pan, l.tilt, 0.0]), scale: Vec3::splat(l.scale) },
            anim,
            HiveWasp(sammy_core::wasps::HiveWasp::new(&l, h.pos)),
        ));
        if exhausted {
            // Exhausted: the hive bursts (bullets.wdl:1255-1272).
            let at = crate::render::coords::native(tf.translation);
            crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::hive_burst(ps, at));
            commands.entity(e).despawn();
        }
    }
}

/// `LaunchNewWasp` flight: ballistic hops with damped bounces until it settles, then
/// it becomes a 1-ammo pickup.
pub fn fly_hive_wasps(
    time: Res<Time>,
    world: CollisionWorld,
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    mut q: Query<(Entity, &mut HiveWasp, &mut Transform)>,
    mut sounds: Local<Vec<&'static str>>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut w, mut tf) in &mut q {
        let settled = w.step(|a, b| world.trace_static(Hull::Point, a, b).trace, &mut rng.0, t, &mut sounds);
        for s in sounds.drain(..) {
            sfx.play(s);
        }
        tf.translation = coords::pos(w.pos);
        tf.rotation = coords::rotation([w.pan, w.tilt, 0.0]);
        if let Some((pickup, lifetime)) = settled {
            let mut ent = commands.entity(e);
            ent.remove::<HiveWasp>().insert((AmmoPickup(pickup), PickupPos(w.pos), Spinning { angles: [w.pan, PICKUP_TILT, 0.0] }));
            if let Some(l) = lifetime {
                ent.insert(PickupLifetime(l));
            }
        }
    }
}

/// Hive pickups fade over their last ticks, then burst.
pub fn expire_pickups(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut PickupLifetime, &mut Transform, Option<&AmmoPickup>)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut life, mut tf, ammo) in &mut q {
        life.0 -= t;
        if life.0 < PICKUP_FADE {
            tf.scale *= PICKUP_FADE_SHRINK.powf(t);
        }
        if life.0 <= 0.0 {
            // `InitWaspWed` countdown over: the wasp bursts (bullets.wdl:80-108).
            if let Some(a) = ammo {
                let (at, kind) = (crate::render::coords::native(tf.translation), a.kind);
                crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::wasp_explode(ps, at, kind, 2.0));
            }
            commands.entity(e).despawn();
        }
    }
}
