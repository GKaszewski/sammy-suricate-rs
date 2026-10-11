//! Gameplay visuals tied to the AI: the warning light on Sammy when a lion starts a
//! chase (`Warning_attack`, player.wdl:1091), ground shadows (`InitOmbreLion`,
//! `InitOmbreSammy`), the Eye of Truth pickup (`InitEyeOfTruth`, mm1.wdl:4954-5103)
//! with lion range rings (`DisplayEnnemyRange`, lion.wdl:1586-1653), and the
//! expanding rings of a friend call (`Circle_CallFriends`, lion.wdl:1015-1070).

use bevy::prelude::*;
use formats::wmb::Object;

use super::enemies::{CallRing, Enemy, WarningAttack};
use super::pickups::{PickupPos, SnapToGround};
use super::player::{Player, State, TICKS};
use crate::platform::audio::Sfx;
use crate::render::coords::{self, QUANTS_PER_UNIT};
use crate::gameplay::state::GameState;
use crate::level::GameAssets;
use crate::render::lights::DynamicLight;
use crate::render::model::ModelAnimator;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

/// Warning light, blob shadows, the Eye of Truth and the range/call rings.
pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                warning_light.in_set(GameSet::WarningLight),
                spawn_shadows.in_set(GameSet::SpawnShadows),
                place_shadows.in_set(GameSet::PlaceShadows),
                run_eye.in_set(GameSet::Eye),
                spawn_rings.in_set(GameSet::SpawnRings),
                update_rings.in_set(GameSet::UpdateRings),
                run_call_rings.in_set(GameSet::CallRings),
            ),
        );
    }
}


/// `LightRange1`: start range of the warning light.
const WARNING_RANGE: f32 = 300.0;

/// Fading warning light on Sammy (range −5/tick down to 100, then off).
#[derive(Component)]
pub struct WarningFade;

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

/// Flat shadow decal following an owner at floor + 0.5.
#[derive(Component)]
pub struct GroundShadow {
    owner: Entity,
}

#[derive(Component)]
pub struct HasShadow;

fn shadow_material(assets: &GameAssets, materials: &mut Assets<StandardMaterial>, file: &str) -> Option<(Handle<StandardMaterial>, [f32; 2])> {
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

#[allow(clippy::type_complexity)]
pub fn spawn_shadows(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    enemies: Query<(Entity, &Enemy), Without<HasShadow>>,
    player: Query<Entity, (With<Player>, Without<HasShadow>)>,
) {
    let mut spawn = |owner: Entity, file: &str, sx: f32, sy: f32| {
        let Some((mat, size)) = shadow_material(&assets, &mut materials, file) else { return };
        let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(size[0] * sx * 0.5, size[1] * sy * 0.5) / QUANTS_PER_UNIT));
        commands.spawn((GroundShadow { owner }, Mesh3d(mesh), MeshMaterial3d(mat), Transform::default(), crate::flow::LevelScoped));
        commands.entity(owner).insert(HasShadow);
    };
    for (e, enemy) in &enemies {
        // Lion shadow: shadlio2.pcx at 1.2 × the lion's scale.
        spawn(e, "shadlio2.pcx", 1.2 * enemy.scale(), 1.2 * enemy.scale());
    }
    // Sammy's blob sprite (ombblob2.pcx) is only shown below 40 fps (player.wdl:197-207);
    // he casts the engine model shadow instead (crate::render::shadow).
    for e in &player {
        commands.entity(e).insert((HasShadow, crate::render::shadow::CastsShadow));
    }
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

/// Eye of Truth pickup ([`sammy_core::eye::EyePickup`]) and its full size.
#[derive(Component, Deref, DerefMut)]
pub struct EyePickup {
    #[deref]
    eye: sammy_core::eye::EyePickup,
    scale: Vec3,
}

/// Extra hover height added when snapping to the ground.
#[derive(Component)]
pub struct ExtraHeight(pub f32);

pub fn init_eye(obj: &Object, entity: &mut EntityCommands) {
    let eye = sammy_core::eye::EyePickup::from_object(obj);
    let snap = eye.snap;
    entity.insert((EyePickup { eye, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs() }, PickupPos(obj.origin)));
    if snap {
        entity.insert((SnapToGround, ExtraHeight(sammy_core::eye::EXTRA_HEIGHT)));
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

/// Range ring drawn around a lion while the Eye of Truth is active.
#[derive(Component)]
pub struct RangeRing {
    owner: Entity,
}

#[derive(Component)]
pub struct HasRing;

/// Additive, unlit ring materials per skin (1..=3).
#[derive(Resource)]
pub struct RingMaterials(Vec<Handle<StandardMaterial>>);

fn ring_materials(assets: &GameAssets, materials: &mut Assets<StandardMaterial>, cache: &mut Option<Vec<Handle<StandardMaterial>>>) -> Option<Vec<Handle<StandardMaterial>>> {
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

fn ring_bundle(assets: &GameAssets, meshes: &mut Assets<Mesh>) -> Option<(Mesh3d, ModelAnimator)> {
    let model = assets.model("circl100.mdl")?;
    let mut anim = ModelAnimator::new(model.clone(), (0..model.mdl.frames.len()).collect(), 0.0);
    anim.looping = false;
    Some((Mesh3d(meshes.add(model.build_mesh(0))), anim))
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_rings(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: Local<Option<Vec<Handle<StandardMaterial>>>>,
    enemies: Query<Entity, (With<Enemy>, Without<HasRing>)>,
    calls: Query<Entity, (With<CallRing>, Without<Mesh3d>)>,
) {
    let Some(mats) = ring_materials(&assets, &mut materials, &mut cache) else { return };
    commands.insert_resource(RingMaterials(mats.clone()));
    for owner in &enemies {
        let Some((mesh, anim)) = ring_bundle(&assets, &mut meshes) else { return };
        commands.spawn((RangeRing { owner }, mesh, anim, MeshMaterial3d(mats[0].clone()), Transform::default(), Visibility::Hidden, crate::flow::LevelScoped));
        commands.entity(owner).insert(HasRing);
    }
    for e in &calls {
        let Some((mesh, anim)) = ring_bundle(&assets, &mut meshes) else { return };
        commands.entity(e).insert((mesh, anim, MeshMaterial3d(mats[0].clone()), Transform::default(), Visibility::Hidden));
    }
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
