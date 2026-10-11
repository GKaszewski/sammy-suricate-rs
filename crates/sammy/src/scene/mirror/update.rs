//! Keeping mirrored entities in step: transforms, visibility, lights, animation.

use super::*;

/// Transforms, visibility, lights and animation of every mirrored entity.
pub(in crate::scene) fn update(
    mut commands: Commands,
    game: Res<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        Entity,
        &Mirrored,
        &mut Transform,
        &mut Visibility,
        Option<&mut ModelView>,
        Option<&mut MaterialView>,
        Option<&mut MeshMaterial3d<StandardMaterial>>,
        Option<&Mesh3d>,
        Option<&mut DynamicLight>,
        Has<Billboard>,
        Has<ScreenAligned>,
    )>,
) {
    let w = &game.world;
    for (e, m, mut tf, mut vis, model, view, mat, mesh, light, billboard, screen) in &mut q {
        let id = m.0;
        let Some(b) = w.bodies.get(&id) else {
            *vis = Visibility::Hidden;
            continue;
        };
        tf.translation = coords::pos(b.pos);
        if !billboard && !screen {
            tf.rotation = coords::rotation(b.angles);
        }
        tf.scale = Vec3::new(b.scale[1], b.scale[2], b.scale[0]);
        let shown = if b.hidden { Visibility::Hidden } else { Visibility::Inherited };
        if *vis != shown {
            *vis = shown;
        }
        // Model animation: blend the two vertex frames.
        if let (Some(mut mv), Some(mesh), Some(a)) = (model, mesh, w.animators.get(&id))
            && let Some(blend) = a.anim.sample()
            && mv.shown != Some(blend)
            && let Some(mut mesh) = meshes.get_mut(&mesh.0)
        {
            mv.shown = Some(blend);
            let (pos, norm) = mv.model.blend(blend);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(pos));
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::Float32x3(norm));
        }
        // Opacity, tint and sprite frame on the entity's own material copy.
        if let (Some(mut view), Some(mut mat)) = (view, mat) {
            let dark = w.menu_items.get(&id).is_some_and(|i| i.dark);
            let frame = w.sprite_anims.get(&id).map(|a| a.frame);
            let needs_own = view.blend || b.alpha < OPAQUE || dark || (view.frames > 1 && frame.is_some());
            match (needs_own, view.own.clone()) {
                (true, None) => {
                    let base = materials.get(&view.base).cloned().unwrap_or_default();
                    view.base_mode = base.alpha_mode;
                    let own = materials.add(base);
                    view.own = Some(own.clone());
                    mat.0 = own;
                }
                (false, Some(_)) => {
                    view.own = None;
                    mat.0 = view.base.clone();
                }
                _ => {}
            }
            if let Some(own) = &view.own
                && let Some(mut m) = materials.get_mut(own)
            {
                let a = (b.alpha / OPAQUE).clamp(0.0, 1.0);
                let blend = view.blend || a < 1.0;
                let mode = if blend { AlphaMode::Blend } else { view.base_mode };
                if m.alpha_mode != mode {
                    m.alpha_mode = mode;
                }
                let tint = if dark { DARK_TINT } else { 1.0 };
                let color = Color::srgba(tint, tint, tint, a);
                if m.base_color != color {
                    m.base_color = color;
                }
                if let Some(f) = frame {
                    let offset = f.floor() / view.frames as f32;
                    if m.uv_transform.translation.x != offset {
                        m.uv_transform = bevy::math::Affine2::from_translation(Vec2::new(offset, 0.0));
                    }
                }
            }
        }
        // Dynamic light.
        match (w.lights.get(&id), light) {
            (Some(l), Some(mut dl)) => {
                if dl.color != l.color || dl.range != l.range {
                    *dl = DynamicLight { color: l.color, range: l.range };
                }
            }
            (Some(l), None) => {
                commands.entity(e).insert(DynamicLight { color: l.color, range: l.range });
            }
            (None, Some(_)) => {
                commands.entity(e).remove::<DynamicLight>();
            }
            (None, None) => {}
        }
    }
}
