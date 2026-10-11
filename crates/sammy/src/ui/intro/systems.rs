#[allow(unused_imports)]
use super::*;

/// The fog quad's edge, and its distance from the camera, in quants.
const FOG_QUAD_QUANTS: f32 = 400.0;

/// The logo ahead of the camera, facing it (`PlaceTitle`), and the black fog as a
/// near-layer quad behind it ([`sammy_present::intro`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn place_logo(
    mut commands: Commands,
    game: Res<Game>,
    mut loader: Loader,
    cam: Query<&Transform, (With<MainCamera>, Without<LogoView>, Without<FogQuad>)>,
    mut logos: Query<(Entity, &LogoView, &mut Transform, &MeshMaterial3d<StandardMaterial>), Without<FogQuad>>,
    mut fog: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>, &mut Visibility), With<FogQuad>>,
) {
    let Ok(cam) = cam.single() else { return };
    let (logo, fog_alpha) = sammy_present::intro::overlay(&game, coords::native(cam.translation), coords::native(*cam.forward()));
    let shown = logo.map_or(0, |l| l.id);
    // Respawn the entity when the logo changes.
    for (e, v, ..) in &logos {
        if v.0 != shown {
            commands.entity(e).despawn();
        }
    }
    if let Some(l) = logo
        && !logos.iter().any(|(_, v, ..)| v.0 == shown)
    {
        let unlit = |materials: &mut Assets<StandardMaterial>, base: StandardMaterial| {
            materials.add(StandardMaterial { unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..base })
        };
        let spawned = match l.art {
            LogoArt::Image(file) => loader.files.read(file).ok().and_then(|d| formats::tga::parse(&d)).map(|img| {
                let tex = loader.images.add(crate::render::image::to_bevy(&img, true, false));
                let size = img.width as f32 / coords::QUANTS_PER_UNIT;
                (loader.meshes.add(Rectangle::new(size, size)), unlit(&mut loader.materials, StandardMaterial { base_color_texture: Some(tex), ..default() }))
            }),
            LogoArt::Model(file) => loader.model(file).map(|m| {
                let base = loader.materials.get(&m.skin(1)).cloned().unwrap_or_default();
                (loader.meshes.add(m.build_mesh(0)), unlit(&mut loader.materials, base))
            }),
        };
        if let Some((mesh, mat)) = spawned {
            commands.spawn((LogoView(shown), Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_scale(Vec3::splat(l.scale)), NEAR_LAYER));
        }
    }
    if let Some(l) = logo {
        for (_, v, mut tf, mat) in &mut logos {
            tf.translation = coords::pos(l.pos);
            // Sprites face the camera; models are turned to face it (like the lose title).
            tf.rotation = if matches!(l.art, LogoArt::Image(_)) { cam.rotation } else { cam.rotation * coords::rotation([0.0, 0.0, 0.0]) };
            if v.0 == shown
                && let Some(mut m) = loader.materials.get_mut(&mat.0)
            {
                m.base_color = m.base_color.with_alpha(l.alpha);
            }
        }
    }
    // Fog quad.
    if fog.is_empty() {
        let mat = loader.materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        commands.spawn((FogQuad, Mesh3d(loader.meshes.add(Rectangle::new(FOG_QUAD_QUANTS, FOG_QUAD_QUANTS))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden, NEAR_LAYER));
        return;
    }
    for (mut tf, mat, mut vis) in &mut fog {
        *vis = if fog_alpha > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
        tf.translation = cam.translation + *cam.forward() * (FOG_QUAD_QUANTS / coords::QUANTS_PER_UNIT);
        tf.rotation = cam.rotation;
        if let Some(mut m) = loader.materials.get_mut(&mat.0) {
            m.base_color = Color::BLACK.with_alpha(fog_alpha);
        }
    }
}
