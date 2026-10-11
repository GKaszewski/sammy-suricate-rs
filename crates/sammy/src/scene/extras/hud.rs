//! The HUD models glued to the camera.

use super::*;

/// The original screen's vertical arc basis: half of the 60° arc, at 4:3.
pub(super) const ARC_HALF_DEG: f32 = 30.0;
pub(super) const ASPECT_43: f32 = 4.0 / 3.0;
/// Rays at a grazing angle to the view axis are clamped to this cosine.
pub(super) const MIN_RAY_COS: f32 = 0.1;

/// The HUD models, placed next to the 2D HUD by unprojecting their mode-6 pixel at the
/// original depth.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(in crate::scene) fn draw_hud(
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
