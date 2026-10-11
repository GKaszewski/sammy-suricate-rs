#[allow(unused_imports)]
use super::*;

/// `LightRange1`: start range of the warning light.
pub(crate) const WARNING_RANGE: f32 = 300.0;
/// The warning light's colour.
const WARNING_COLOR: [f32; 3] = [250.0, 100.0, 0.0];
/// The warning light shrinks this much per tick and goes out at `WARNING_FADE_END`.
const WARNING_FADE_RATE: f32 = 5.0;
const WARNING_FADE_END: f32 = 100.0;
/// Ground shadow quad sizes in quants: the lion's `shadlio2.pcx`, any other sprite.
const SHADOW_SIZE_LION: [f32; 2] = [48.0, 48.0];
const SHADOW_SIZE_DEFAULT: [f32; 2] = [64.0, 32.0];
/// Shadows sit this far above the floor (quants); enemy shadows turn with `pan` + this.
const SHADOW_LIFT: f32 = 0.5;
const ENEMY_SHADOW_PAN_OFFSET: f32 = 90.0;
/// Range rings: skins 1..=`RING_SKINS`; `circl100` is 1000 quants across at scale 1.
const RING_SKINS: u32 = 3;
const RING_DIAMETER: f32 = 1000.0;
/// `Circle_CallFriends`: scale grows `CALL_GROW` per tick until `CALL_EASE_FROM`, then
/// eases (`CALL_EASE_RATE`) toward `CALL_EASE_TARGET`; the ring ends at scale
/// `CALL_END_SCALE` or frame `CALL_END_FRAME`; frames advance `CALL_FRAME_RATE` per tick.
const CALL_GROW: f32 = 0.2;
const CALL_EASE_FROM: f32 = 3.0;
const CALL_EASE_TARGET: f32 = 5.1;
const CALL_EASE_RATE: f32 = 0.5;
const CALL_FRAME_RATE: f32 = 4.0;
const CALL_END_FRAME: f32 = 100.0;
const CALL_END_SCALE: f32 = 5.0;

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
            l.range -= WARNING_FADE_RATE * t;
            if l.range <= WARNING_FADE_END {
                l.range = 0.0;
                commands.entity(e).remove::<WarningFade>();
            }
        }
        // Only when Sammy carries no light already (`lightrange == 0`).
        Some(mut l) if warn && l.range == 0.0 => {
            *l = DynamicLight { color: WARNING_COLOR, range: WARNING_RANGE };
            commands.entity(e).insert(WarningFade);
        }
        None if warn => {
            commands.entity(e).insert((DynamicLight { color: WARNING_COLOR, range: WARNING_RANGE }, WarningFade));
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
        "shadlio2.pcx" => SHADOW_SIZE_LION,
        _ => SHADOW_SIZE_DEFAULT,
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
            (Some(en), _) => (en.floor(), en.gone(), en.pan + ENEMY_SHADOW_PAN_OFFSET),
            (_, Some(p)) => (p.floor, p.state == State::Dead, p.pan),
            _ => continue,
        };
        let x = otf.translation;
        tf.translation = Vec3::new(x.x, floor / QUANTS_PER_UNIT + SHADOW_LIFT / QUANTS_PER_UNIT, x.z);
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
            (1..=RING_SKINS)
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
        mat.0 = mats.0[(skin.clamp(1, RING_SKINS) - 1) as usize].clone();
        anim.time = (frame - 1.0).max(0.0); // A4 frame numbers are 1-based
        let s = radius / RING_DIAMETER;
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
        mat.0 = mats.0[(r.skin.clamp(1, RING_SKINS) - 1) as usize].clone();
        if r.scale < CALL_EASE_FROM {
            r.scale += CALL_GROW * t;
        } else {
            r.scale += (CALL_EASE_TARGET - r.scale) * CALL_EASE_RATE * t;
        }
        r.frame += CALL_FRAME_RATE * t;
        anim.time = (r.frame - 1.0).clamp(0.0, (anim.frames.len() - 1) as f32);
        tf.translation = coords::pos(r.origin);
        tf.scale = Vec3::new(r.scale, 1.0, r.scale);
        if r.frame >= CALL_END_FRAME || r.scale >= CALL_END_SCALE {
            commands.entity(e).despawn();
        }
    }
}
