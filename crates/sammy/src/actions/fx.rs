//! Gameplay visuals tied to the AI: the warning light on Sammy when a lion starts a
//! chase (`Warning_attack`, player.wdl:1091), ground shadows (`InitOmbreLion`,
//! `InitOmbreSammy`), the Eye of Truth pickup (`InitEyeOfTruth`, mm1.wdl:4954-5103)
//! with lion range rings (`DisplayEnnemyRange`, lion.wdl:1586-1653), and the
//! expanding rings of a friend call (`Circle_CallFriends`, lion.wdl:1015-1070).

use bevy::prelude::*;
use formats::wmb::Object;

use super::enemy::{CallRing, Enemy, WarningAttack};
use super::pickups::{PickupPos, SnapToGround};
use super::player::{Player, State, TICKS};
use crate::audio::Sfx;
use crate::coords::{self, QUANTS_PER_UNIT};
use crate::game::GameState;
use crate::level::GameAssets;
use crate::lights::DynamicLight;
use crate::model::ModelAnimator;

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
    for e in &player {
        // Sammy's blob shadow: ombblob2.pcx, scale 0.8 × 1.2.
        spawn(e, "ombblob2.pcx", 0.8, 1.2);
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

/// Eye of Truth pickup.
#[derive(Component)]
pub struct EyePickup {
    scale: Vec3,
    /// Taken: hidden for 128 ticks, then grows back.
    away: Option<f32>,
    spin: Option<[f32; 3]>,
    angles: [f32; 3],
}

/// Extra hover height added when snapping to the ground (Eye of Truth: 2 × HeightGem).
#[derive(Component)]
pub struct ExtraHeight(pub f32);

pub fn init_eye(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    entity.insert((
        EyePickup {
            scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs(),
            away: None,
            spin: (obj.flags & (1 << 2) != 0).then_some([s[1], s[2], s[3]]),
            angles: obj.angles,
        },
        PickupPos(obj.origin),
    ));
    if obj.flags & (1 << 5) == 0 {
        entity.insert((SnapToGround, ExtraHeight(25.0)));
    }
}

pub fn run_eye(
    time: Res<Time>,
    current: Res<crate::flow::CurrentLevel>,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut eyes: Query<(&mut EyePickup, &PickupPos, &mut Transform, &mut Visibility), Without<SnapToGround>>,
) {
    let t = time.delta_secs() * TICKS;
    let Ok(p) = player.single() else { return };
    if p.state == State::Dead {
        game.eye_ticks = 0.0; // Player_Kaputt clears it
    }
    game.eye_ticks = (game.eye_ticks - t).max(0.0);
    for (mut eye, pos, mut tf, mut vis) in &mut eyes {
        if let Some(spin) = eye.spin {
            for i in 0..3 {
                eye.angles[i] += spin[i] * t;
            }
            tf.rotation = coords::rotation(eye.angles);
        }
        match eye.away {
            Some(left) => {
                let left = left - t;
                if left > 128.0 {
                    tf.scale *= 0.8f32.powf(t);
                } else {
                    *vis = Visibility::Hidden;
                }
                if left <= 0.0 {
                    *vis = Visibility::Inherited;
                    tf.scale = eye.scale;
                    eye.away = None;
                } else {
                    eye.away = Some(left);
                }
            }
            None => {
                let d = (0..3).map(|i| (pos.0[i] - p.pos[i]).powi(2)).sum::<f32>().sqrt();
                if crate::flow::playing(&current) && p.state != State::Dead && d <= p.scan_radius {
                    game.eye_ticks = game.eye_duration;
                    sfx.play("EyeTruth_Son");
                    eye.away = Some(128.0 + 12.0); // shrink ~12 ticks, then 128 hidden
                }
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
