#[allow(unused_imports)]
use super::*;

/// Intro fog and logo alpha are percentages; the fog starts fully black.
const FULL_PERCENT: f32 = 100.0;
/// Model logos (`publogo`, `surilogo`) are shown at this scale; the sprite logo at 1.
const LOGO_MODEL_SCALE: f32 = 1.5;
/// The fog quad's edge, and its distance from the camera, in quants.
const FOG_QUAD_QUANTS: f32 = 400.0;
/// Uncompressed TGA: header length, image type 2 (true colour) and the descriptor's
/// top-down bit.
const TGA_HEADER_LEN: usize = 18;
const TGA_TRUECOLOR: u8 = 2;
const TGA_TOP_DOWN_BIT: u8 = 0x20;

/// Logo `Skill2` quants ahead of the camera at its height, facing it (`PlaceTitle`), and
/// the black fog as a near-layer quad behind it.
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
    let (shown, dist, alpha, fog_pct) = game.intro.overlay();
    // Respawn the entity when the logo changes.
    for (e, v, ..) in &logos {
        if v.0 != shown {
            commands.entity(e).despawn();
        }
    }
    if shown != 0 && !logos.iter().any(|(_, v, ..)| v.0 == shown) {
        let unlit = |materials: &mut Assets<StandardMaterial>, base: StandardMaterial| {
            materials.add(StandardMaterial { unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..base })
        };
        let spawned = match shown {
            1 => load_tga(&loader.files, "slogoa5.tga").map(|img| {
                let tex = loader.images.add(crate::render::image::to_bevy(&img, true, false));
                let size = img.width as f32 / coords::QUANTS_PER_UNIT;
                (loader.meshes.add(Rectangle::new(size, size)), unlit(&mut loader.materials, StandardMaterial { base_color_texture: Some(tex), ..default() }), 1.0)
            }),
            n => loader.model(if n == 2 { "publogo.mdl" } else { "surilogo.mdl" }).map(|m| {
                let base = loader.materials.get(&m.skin(1)).cloned().unwrap_or_default();
                (loader.meshes.add(m.build_mesh(0)), unlit(&mut loader.materials, base), LOGO_MODEL_SCALE)
            }),
        };
        if let Some((mesh, mat, scale)) = spawned {
            commands.spawn((LogoView(shown), Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_scale(Vec3::splat(scale)), NEAR_LAYER));
        }
    }
    let fwd = {
        let f = coords::native(*cam.forward());
        let a = f[1].atan2(f[0]);
        [a.cos(), a.sin(), 0.0]
    };
    let at = |d: f32| {
        let p = coords::native(cam.translation);
        coords::pos([p[0] + fwd[0] * d, p[1] + fwd[1] * d, p[2]])
    };
    for (_, v, mut tf, mat) in &mut logos {
        tf.translation = at(dist);
        // Sprites face the camera; models are turned to face it (like the lose title).
        tf.rotation = if v.0 == 1 { cam.rotation } else { cam.rotation * coords::rotation([0.0, 0.0, 0.0]) };
        if let Some(mut m) = loader.materials.get_mut(&mat.0) {
            m.base_color = m.base_color.with_alpha(alpha / FULL_PERCENT);
        }
    }
    // Fog quad.
    if fog.is_empty() {
        let mat = loader.materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        commands.spawn((FogQuad, Mesh3d(loader.meshes.add(Rectangle::new(FOG_QUAD_QUANTS, FOG_QUAD_QUANTS))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden, NEAR_LAYER));
        return;
    }
    for (mut tf, mat, mut vis) in &mut fog {
        let f = fog_pct;
        *vis = if f > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
        tf.translation = cam.translation + *cam.forward() * (FOG_QUAD_QUANTS / coords::QUANTS_PER_UNIT);
        tf.rotation = cam.rotation;
        if let Some(mut m) = loader.materials.get_mut(&mat.0) {
            m.base_color = Color::BLACK.with_alpha(f / FULL_PERCENT);
        }
    }
}

/// Uncompressed 24/32-bit TGA (bottom-up unless bit 5 of the descriptor); black is the
/// colour key.
pub(crate) fn load_tga(files: &GameFiles, name: &str) -> Option<formats::image::RgbaImage> {
    let d = files.read(name).ok()?;
    if d.len() < TGA_HEADER_LEN || d[2] != TGA_TRUECOLOR {
        return None;
    }
    let (w, h) = (u16::from_le_bytes([d[12], d[13]]) as u32, u16::from_le_bytes([d[14], d[15]]) as u32);
    let bpp = d[16] as usize / 8;
    let top_down = d[17] & TGA_TOP_DOWN_BIT != 0;
    let data = d.get(TGA_HEADER_LEN + d[0] as usize..)?;
    let mut img = formats::image::RgbaImage::new(w, h);
    for y in 0..h {
        let sy = if top_down { y } else { h - 1 - y };
        for x in 0..w {
            let s = ((sy * w + x) as usize) * bpp;
            let p = data.get(s..s + bpp)?;
            let rgb = [p[2], p[1], p[0]];
            let i = ((y * w + x) * 4) as usize;
            img.data[i..i + 3].copy_from_slice(&rgb);
            img.data[i + 3] = if crate::render::image::is_key(&rgb) { 0 } else { 255 };
        }
    }
    Some(img)
}
