//! Spawning, rebuilding and despawning mirrored entities.

use super::*;

/// Spawns, rebuilds and despawns mirrored entities.
pub(in crate::scene) fn spawn(mut commands: Commands, game: Res<Game>, mut mirror: ResMut<Mirror>, mut loader: Loader) {
    let w = &game.world;
    mirror.0.retain(|id, (e, _)| {
        let keep = w.looks.contains_key(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    for (id, look) in &w.looks {
        if let Some((e, rev)) = mirror.0.get(id) {
            if *rev == look.rev {
                continue;
            }
            commands.entity(*e).despawn();
        }
        let e = build(&mut commands, &mut loader, &game, *id, look);
        mirror.0.insert(*id, (e, look.rev));
    }
}

pub(super) fn build(commands: &mut Commands, loader: &mut Loader, game: &Game, id: Id, look: &Look) -> Entity {
    let w = &game.world;
    let name = w.bodies.get(&id).map_or(String::new(), |b| b.name.clone());
    let mut e = commands.spawn((Mirrored(id), Name::new(name), Transform::default(), Visibility::Hidden));
    if look.sky {
        e.insert(SKY_LAYER);
    }
    if look.near {
        e.insert(NEAR_LAYER);
    }
    if look.shadow {
        e.insert(CastsShadow);
    }
    match &look.visual {
        Visual::Model { file, skin, overlay, frame } => {
            let Some(model) = loader.model(file) else { return e.id() };
            let base = if *overlay { model.overlay_skin(*skin, &mut loader.images, &mut loader.materials) } else { model.skin(*skin) };
            let material = if look.sky {
                loader.unlit(&base)
            } else if look.flare {
                // `flare`: drawn additively.
                let mut m = loader.materials.get(&base).cloned().unwrap_or_default();
                m.alpha_mode = AlphaMode::Add;
                loader.materials.add(m)
            } else {
                base
            };
            let animated = w.animators.get(&id).filter(|_| frame.is_none());
            let first = frame.or_else(|| animated.and_then(|a| a.anim.sample()).map(|(a, _, _)| a)).unwrap_or(0);
            e.insert((Mesh3d(loader.meshes.add(model.build_mesh(first))), MeshMaterial3d(material.clone())));
            e.insert(MaterialView { base: material, own: None, base_mode: AlphaMode::Opaque, blend: false, frames: 1 });
            if animated.is_some() {
                e.insert(ModelView { model, shown: None });
            }
        }
        Visual::Prefab { file } => {
            let parts = loader.prefab(file).unwrap_or_default();
            e.with_children(|c| {
                for (mesh, mat) in parts {
                    c.spawn((Mesh3d(mesh), MeshMaterial3d(mat)));
                }
            });
        }
        Visual::Sprite { file, facing } => {
            let Some(sprite) = loader.sprite(file) else { return e.id() };
            e.insert((Mesh3d(sprite.mesh), MeshMaterial3d(sprite.material.clone())));
            e.insert(MaterialView { base: sprite.material, own: None, base_mode: AlphaMode::Opaque, blend: look.blend, frames: sprite.frames });
            match facing {
                SpriteFacing::Billboard => {
                    e.insert(Billboard);
                }
                SpriteFacing::Screen => {
                    e.insert(ScreenAligned);
                }
                SpriteFacing::Oriented => {}
            }
        }
    }
    e.id()
}
