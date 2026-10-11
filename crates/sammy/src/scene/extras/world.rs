//! The world items.

use super::*;

/// The world items, one pooled entity per list position.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(in crate::scene) fn draw(
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
