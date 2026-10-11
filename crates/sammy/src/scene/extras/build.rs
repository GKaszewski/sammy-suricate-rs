//! Entities and materials for each kind of item.

use super::*;

pub(super) fn material(loader: &mut Loader, cache: &mut Materials, file: &'static str, skin: u32, blend: Blend) -> Option<Handle<StandardMaterial>> {
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
pub(super) fn spawn(commands: &mut Commands, loader: &mut Loader, cache: &mut Materials, kind: Kind) -> Option<Entity> {
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
