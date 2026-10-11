//! Draws [`sammy_present::extras`]: enemy ground shadows, range and call rings, score
//! popups and the lose title as world entities, and the HUD wasp / Eye of Truth plate
//! glued to the camera next to the 2D HUD. Each list position keeps its entity while
//! it shows the same kind of thing.

use std::collections::HashMap;

use bevy::prelude::*;
use sammy_present::extras::{Blend, Extras, HudModel, Item, View};

use super::{Billboard, Loader, NEAR_LAYER};
use crate::camera::MainCamera;
use crate::game::{Game, GameEvent};
use crate::render::coords::{self, QUANTS_PER_UNIT};
use crate::render::model::ModelAnimator;
use crate::render::particles::Particles;

/// The extras' state as a resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ExtrasState(pub Extras);

/// What a pooled entity shows; another kind needs a new entity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Kind {
    Model(&'static str, Blend),
    Title(&'static str),
    Decal(&'static str),
    Sprite(&'static str),
    Hud(&'static str, Blend),
}

/// An entity showing one list position.
#[derive(Component)]
pub struct ExtraView;

/// Materials per (file, skin, blend, lit).
#[derive(Default)]
pub(super) struct Materials(HashMap<(&'static str, u32, Blend), Handle<StandardMaterial>>);

/// Takes the game's events and advances the extras.
pub(super) fn update(
    time: Res<Time>,
    game: Res<Game>,
    mut events: MessageReader<GameEvent>,
    mut extras: ResMut<ExtrasState>,
    mut particles: ResMut<Particles>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
) {
    for GameEvent(e) in events.read() {
        extras.event(&game, e);
    }
    let view = cam.single().map_or(View::default(), |g| View::looking(coords::native(g.translation()), coords::native(g.forward().as_vec3())));
    extras.update(&game, view, time.delta_secs(), &mut particles);
}

fn material(loader: &mut Loader, cache: &mut Materials, file: &'static str, skin: u32, blend: Blend) -> Option<Handle<StandardMaterial>> {
    if let Some(m) = cache.0.get(&(file, skin, blend)) {
        return Some(m.clone());
    }
    let base = if file.ends_with(".pcx") { loader.sprite(file)?.material } else { loader.model(file)?.skin(skin) };
    let mut m = loader.materials.get(&base).cloned().unwrap_or_default();
    m.unlit = true;
    m.double_sided = true;
    m.cull_mode = None;
    if blend == Blend::Add {
        m.alpha_mode = AlphaMode::Add; // `flare`
    }
    let h = loader.materials.add(m);
    cache.0.insert((file, skin, blend), h.clone());
    Some(h)
}

/// Spawns the entity for a kind of item.
fn spawn(commands: &mut Commands, loader: &mut Loader, cache: &mut Materials, kind: Kind) -> Option<Entity> {
    let mut e = commands.spawn((ExtraView, Transform::default(), Visibility::Hidden));
    match kind {
        Kind::Model(file, blend) | Kind::Hud(file, blend) => {
            let model = loader.model(file)?;
            let mut anim = ModelAnimator::new(model.clone(), (0..model.mdl.frames.len()).collect(), 0.0);
            anim.looping = false;
            e.insert((Mesh3d(loader.meshes.add(model.build_mesh(0))), anim, MeshMaterial3d(material(loader, cache, file, 1, blend)?)));
            if matches!(kind, Kind::Hud(..)) {
                e.insert(NEAR_LAYER);
            }
        }
        Kind::Title(file) => {
            let model = loader.model(file)?;
            e.insert((Mesh3d(loader.meshes.add(model.build_mesh(0))), MeshMaterial3d(material(loader, cache, file, 1, Blend::Alpha)?), NEAR_LAYER));
        }
        Kind::Decal(file) => {
            let base = loader.sprite(file)?.material;
            let mut m = loader.materials.get(&base).cloned().unwrap_or_default();
            m.alpha_mode = AlphaMode::Blend;
            m.depth_bias = 1.0;
            e.insert((Mesh3d(loader.meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.5)))), MeshMaterial3d(loader.materials.add(m))));
        }
        Kind::Sprite(file) => {
            let sprite = loader.sprite(file)?;
            // Its own material: the strip frame is a UV shift.
            let m = loader.materials.get(&sprite.material).cloned().unwrap_or_default();
            e.insert((Billboard, Mesh3d(sprite.mesh), MeshMaterial3d(loader.materials.add(m))));
        }
    }
    Some(e.id())
}

/// The world items, one pooled entity per list position.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn draw(
    mut commands: Commands,
    game: Res<Game>,
    extras: Res<ExtrasState>,
    mut loader: Loader,
    mut cache: Local<Materials>,
    mut pool: Local<Vec<(Kind, Entity)>>,
    mut q: Query<(&mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>, Option<&mut ModelAnimator>, Option<&mut crate::render::lights::DynamicLight>)>,
) {
    let items = extras.items(&game);
    for (n, item) in items.iter().enumerate() {
        let kind = match item {
            Item::Model { file, blend, .. } => Kind::Model(file, *blend),
            Item::Title { file, .. } => Kind::Title(file),
            Item::Decal { file, .. } => Kind::Decal(file),
            Item::Sprite { file, .. } => Kind::Sprite(file),
        };
        if pool.get(n).is_none_or(|(k, _)| *k != kind) {
            let Some(e) = spawn(&mut commands, &mut loader, &mut cache, kind) else { continue };
            if let Item::Title { light: (color, range), .. } = item {
                commands.entity(e).insert(crate::render::lights::DynamicLight { color: *color, range: *range });
            }
            if let Some((_, old)) = pool.get(n) {
                commands.entity(*old).despawn();
                pool[n] = (kind, e);
            } else {
                pool.push((kind, e));
            }
            // New entities show from the next frame.
            continue;
        }
        let Ok((mut tf, mut vis, mut mat, anim, _)) = q.get_mut(pool[n].1) else { continue };
        *vis = Visibility::Inherited;
        match item {
            Item::Model { file, skin, frame, pos, angles, scale, blend } => {
                if let Some(m) = material(&mut loader, &mut cache, file, *skin, *blend) {
                    mat.0 = m;
                }
                if let Some(mut a) = anim {
                    a.time = frame.clamp(0.0, (a.frames.len().max(1) - 1) as f32);
                }
                *tf = Transform { translation: coords::pos(*pos), rotation: coords::rotation(*angles), scale: Vec3::new(scale[0], scale[2], scale[1]) };
            }
            Item::Title { pos, angles, scale, .. } => {
                *tf = Transform { translation: coords::pos(*pos), rotation: coords::rotation(*angles), scale: Vec3::splat(*scale) };
            }
            Item::Decal { pos, pan, size, alpha, .. } => {
                if let Some(mut m) = loader.materials.get_mut(&mat.0) {
                    m.base_color = Color::srgba(1.0, 1.0, 1.0, *alpha);
                }
                *tf = Transform { translation: coords::pos(*pos), rotation: Quat::from_rotation_y(pan.to_radians()), scale: Vec3::splat(size / QUANTS_PER_UNIT) };
            }
            Item::Sprite { file, frame, pos } => {
                let frames = sammy_core::sim::assets::sprite_frames(file).max(1);
                if let Some(mut m) = loader.materials.get_mut(&mat.0) {
                    m.uv_transform = bevy::math::Affine2::from_translation(Vec2::new(*frame as f32 / frames as f32, 0.0));
                }
                tf.translation = coords::pos(*pos);
            }
        }
    }
    let keep = items.len().min(pool.len());
    for (_, e) in pool.drain(keep..) {
        commands.entity(e).despawn();
    }
}

/// The original screen's vertical arc basis: half of the 60° arc, at 4:3.
const ARC_HALF_DEG: f32 = 30.0;
const ASPECT_43: f32 = 4.0 / 3.0;
/// Rays at a grazing angle to the view axis are clamped to this cosine.
const MIN_RAY_COS: f32 = 0.1;

/// The HUD models, placed next to the 2D HUD by unprojecting their mode-6 pixel at the
/// original depth.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn draw_hud(
    mut commands: Commands,
    game: Res<Game>,
    extras: Res<ExtrasState>,
    visible: Res<crate::ui::hud::HudVisible>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    mut loader: Loader,
    mut cache: Local<Materials>,
    mut pool: Local<Vec<(Kind, Entity)>>,
    cam: Query<(&Camera, &GlobalTransform, &Projection), With<MainCamera>>,
    mut q: Query<(&mut Transform, &mut GlobalTransform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>), Without<MainCamera>>,
) {
    let models: Vec<HudModel> = if visible.0 { extras.hud_models(&game) } else { Vec::new() };
    let (Ok(win), Ok((camera, cam_tf, Projection::Perspective(p)))) = (windows.single(), cam.single()) else { return };
    let (w, h) = (sammy_present::hud::W, sammy_present::hud::H);
    // Mode-6 pixel size in window pixels, as for the 2D HUD (whole-number UI scale).
    let s = ui_scale.0;
    // Same size relative to the 2D HUD as at 4:3 with the original 60° arc.
    let vfov_43 = 2.0 * (ARC_HALF_DEG.to_radians().tan() / ASPECT_43).atan();
    let size_k = (p.fov / 2.0).tan() / (vfov_43 / 2.0).tan() * s / (win.height() / h);
    for (n, m) in models.iter().enumerate() {
        let kind = Kind::Hud(m.file, m.blend);
        if pool.get(n).is_none_or(|(k, _)| *k != kind) {
            let Some(e) = spawn(&mut commands, &mut loader, &mut cache, kind) else { continue };
            if let Some((_, old)) = pool.get(n) {
                commands.entity(*old).despawn();
                pool[n] = (kind, e);
            } else {
                pool.push((kind, e));
            }
            continue;
        }
        let Ok((mut tf, mut gtf, mut vis, mut mat)) = q.get_mut(pool[n].1) else { continue };
        *vis = Visibility::Inherited;
        if let Some(h) = material(&mut loader, &mut cache, m.file, m.skin, m.blend) {
            mat.0 = h;
        }
        // Mode-6 pixel → window pixel (negative x anchors to the right edge, y to the bottom).
        let x = if m.px[0] < 0.0 { win.width() + m.px[0] * s } else { win.width() / 2.0 + (m.px[0] - w / 2.0) * s };
        let y = win.height() - (h - m.px[1]) * s;
        let Ok(ray) = camera.viewport_to_world(cam_tf, Vec2::new(x, y)) else { continue };
        let fwd = cam_tf.forward().as_vec3();
        let depth = m.depth / QUANTS_PER_UNIT;
        let pos = ray.origin + ray.direction.as_vec3() * (depth / ray.direction.dot(fwd).max(MIN_RAY_COS));
        let (_, cam_rot, _) = cam_tf.to_scale_rotation_translation();
        *tf = Transform { translation: pos, rotation: cam_rot * coords::rotation(m.angles), scale: Vec3::splat(m.scale * size_k) };
        *gtf = GlobalTransform::from(*tf);
    }
    let keep = models.len().min(pool.len());
    for (_, e) in pool.drain(keep..) {
        commands.entity(e).despawn();
    }
}
