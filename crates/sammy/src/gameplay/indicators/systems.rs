#[allow(unused_imports)]
use super::*;

/// `LightRange1`: start range of the warning light.
pub(crate) const WARNING_RANGE: f32 = 300.0;

pub fn warning_light(
    time: Res<Time>,
    mut commands: Commands,
    mut msgs: MessageReader<WarningAttack>,
    mut player: Query<(Entity, Option<&mut DynamicLight>, Has<WarningFade>), With<Player>>,
) {
    let Ok((e, light, fading)) = player.single_mut() else { return };
    let warn = msgs.read().count() > 0;
    let t = time.delta_secs() * TICKS;
    match light {
        Some(mut l) if fading => {
            l.range -= 5.0 * t;
            if l.range <= 100.0 {
                l.range = 0.0;
                commands.entity(e).remove::<WarningFade>();
            }
        }
        // Only when Sammy carries no light already (`lightrange == 0`).
        Some(mut l) if warn && l.range == 0.0 => {
            *l = DynamicLight { color: [250.0, 100.0, 0.0], range: WARNING_RANGE };
            commands.entity(e).insert(WarningFade);
        }
        None if warn => {
            commands.entity(e).insert((DynamicLight { color: [250.0, 100.0, 0.0], range: WARNING_RANGE }, WarningFade));
        }
        _ => {}
    }
}

pub(crate) fn shadow_material(assets: &GameAssets, materials: &mut Assets<StandardMaterial>, file: &str) -> Option<(Handle<StandardMaterial>, [f32; 2])> {
    let (mesh, mat, _) = assets.sprite(file)?;
    let _ = mesh;
    let mut m = materials.get(&mat)?.clone();
    m.alpha_mode = AlphaMode::Blend;
    m.base_color = Color::srgba(1.0, 1.0, 1.0, 0.5); // `transparent`
    m.unlit = true;
    m.double_sided = true;
    m.cull_mode = None;
    m.depth_bias = 1.0;
    let size = match file {
        "shadlio2.pcx" => [48.0, 48.0],
        _ => [64.0, 32.0],
    };
    Some((materials.add(m), size))
}

pub fn place_shadows(
    mut commands: Commands,
    owners: Query<(Option<&Enemy>, Option<&Player>, &Transform), Without<GroundShadow>>,
    mut shadows: Query<(Entity, &GroundShadow, &mut Transform, &mut Visibility)>,
) {
    for (e, s, mut tf, mut vis) in &mut shadows {
        let Ok((enemy, player, otf)) = owners.get(s.owner) else {
            commands.entity(e).despawn();
            continue;
        };
        let (floor, hidden, pan) = match (enemy, player) {
            (Some(en), _) => (en.floor(), en.gone(), en.pan + 90.0),
            (_, Some(p)) => (p.floor, p.state == State::Dead, p.pan),
            _ => continue,
        };
        let x = otf.translation;
        tf.translation = Vec3::new(x.x, floor / QUANTS_PER_UNIT + 0.5 / QUANTS_PER_UNIT, x.z);
        tf.rotation = Quat::from_rotation_y(pan.to_radians());
        *vis = if hidden { Visibility::Hidden } else { Visibility::Inherited };
    }
}

pub fn run_eye(
    time: Res<Time>,
    current: Res<crate::flow::CurrentLevel>,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut eyes: Query<(&mut EyePickup, &PickupPos, &mut Transform, &mut Visibility), Without<SnapToGround>>,
    mut particles: ResMut<crate::render::particles::Particles>,
) {
    use sammy_core::eye::EyeStage;
    let t = time.delta_secs() * TICKS;
    let Ok(p) = player.single() else { return };
    sammy_core::eye::wear_off(&mut game, p.state == State::Dead, t);
    for (mut eye, pos, mut tf, mut vis) in &mut eyes {
        if eye.spin(t) {
            tf.rotation = coords::rotation(eye.angles);
        }
        let reach = crate::flow::playing(&current) && p.state != State::Dead && sammy_core::pickups::in_scan(p.pos, p.scan_radius, pos.0);
        match eye.step(&mut game, reach, t) {
            EyeStage::Idle => {
                for _ in 0..particles.ticks {
                    crate::render::effects::eye_idle(&mut particles, pos.0);
                }
            }
            EyeStage::Taken => {
                for _ in 0..particles.ticks {
                    crate::render::effects::eye_idle(&mut particles, pos.0);
                }
                sfx.play("EyeTruth_Son");
                crate::render::effects::take_eye(&mut particles, pos.0);
            }
            EyeStage::Shrinking => tf.scale *= sammy_core::eye::SHRINK_RATE.powf(t),
            EyeStage::Hidden => *vis = Visibility::Hidden,
            EyeStage::Back => {
                *vis = Visibility::Inherited;
                tf.scale = eye.scale;
            }
        }
    }
}

pub(crate) fn ring_materials(assets: &GameAssets, materials: &mut Assets<StandardMaterial>, cache: &mut Option<Vec<Handle<StandardMaterial>>>) -> Option<Vec<Handle<StandardMaterial>>> {
    if cache.is_none() {
        let model = assets.model("circl100.mdl")?;
        *cache = Some(
            (1..=3)
                .map(|n| {
                    let mut m = materials.get(&model.skin(n)).cloned().unwrap_or_default();
                    m.alpha_mode = AlphaMode::Add; // `flare`
                    m.unlit = true;
                    m.double_sided = true;
                    m.cull_mode = None;
                    materials.add(m)
                })
                .collect(),
        );
    }
    cache.clone()
}

pub(crate) fn ring_bundle(assets: &GameAssets, meshes: &mut Assets<Mesh>) -> Option<(Mesh3d, ModelAnimator)> {
    let model = assets.model("circl100.mdl")?;
    let mut anim = ModelAnimator::new(model.clone(), (0..model.mdl.frames.len()).collect(), 0.0);
    anim.looping = false;
    Some((Mesh3d(meshes.add(model.build_mesh(0))), anim))
}

#[allow(clippy::type_complexity)]
pub fn update_rings(
    game: Res<GameState>,
    mats: Option<Res<RingMaterials>>,
    enemies: Query<&Enemy>,
    mut rings: Query<(&RangeRing, &mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>, &mut ModelAnimator)>,
) {
    let Some(mats) = mats else { return };
    for (ring, mut tf, mut vis, mut mat, mut anim) in &mut rings {
        let shown = (game.eye_ticks > 0.0)
            .then(|| enemies.get(ring.owner).ok())
            .flatten()
            .filter(|e| !e.gone())
            .and_then(|e| e.ring(game.display_range).map(|r| (e, r)));
        let Some((e, (skin, frame, radius))) = shown else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Inherited;
        mat.0 = mats.0[(skin.clamp(1, 3) - 1) as usize].clone();
        anim.time = (frame - 1.0).max(0.0); // A4 frame numbers are 1-based
        let s = radius / 1000.0;
        tf.translation = coords::pos([e.pos[0], e.pos[1], e.pos[2] + e.circle_z]);
        tf.scale = Vec3::new(s, 1.0, s);
    }
}

/// `Circle_CallFriends`: grow +0.2/tick to scale 3, then ease to 5; frame +4/tick to 100, then gone.
pub fn run_call_rings(
    time: Res<Time>,
    mut commands: Commands,
    mats: Option<Res<RingMaterials>>,
    mut rings: Query<(Entity, &mut CallRing, &mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>, &mut ModelAnimator)>,
) {
    let Some(mats) = mats else { return };
    let t = time.delta_secs() * TICKS;
    for (e, mut r, mut tf, mut vis, mut mat, mut anim) in &mut rings {
        if r.delay > 0.0 {
            r.delay -= t;
            continue;
        }
        *vis = Visibility::Inherited;
        mat.0 = mats.0[(r.skin.clamp(1, 3) - 1) as usize].clone();
        if r.scale < 3.0 {
            r.scale += 0.2 * t;
        } else {
            r.scale += (5.1 - r.scale) * 0.5 * t;
        }
        r.frame += 4.0 * t;
        anim.time = (r.frame - 1.0).clamp(0.0, (anim.frames.len() - 1) as f32);
        tf.translation = coords::pos(r.origin);
        tf.scale = Vec3::new(r.scale, 1.0, r.scale);
        if r.frame >= 100.0 || r.scale >= 5.0 {
            commands.entity(e).despawn();
        }
    }
}
