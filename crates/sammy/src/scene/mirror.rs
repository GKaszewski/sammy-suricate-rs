//! Mirrors the game's entities as Bevy entities: one per entity with a
//! [`Look`](sammy_core::sim::Look), spawned when it appears, rebuilt when its visual
//! is swapped, despawned when it is gone, and updated every frame (transform,
//! visibility, opacity, animation frame, light).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use sammy_core::id::Id;
use sammy_core::sim::{Look, SpriteFacing, Visual};

use super::assets::Loader;
use super::{Billboard, NEAR_LAYER, SKY_LAYER, ScreenAligned};
use crate::game::Game;
use crate::render::coords;
use crate::render::lights::DynamicLight;
use crate::render::model::LoadedModel;
use crate::render::shadow::CastsShadow;

/// Opacity of an opaque body (percent).
const OPAQUE: f32 = 100.0;
/// Grey tint of a darkened (absent) level icon in the menu.
const DARK_TINT: f32 = 0.35;

/// Game entity → Bevy entity, with the visual revision it was built from.
#[derive(Resource, Default)]
pub struct Mirror(HashMap<Id, (Entity, u32)>);

impl Mirror {
    /// The Bevy entity showing a game entity.
    pub fn entity(&self, id: Id) -> Option<Entity> {
        self.0.get(&id).map(|(e, _)| *e)
    }
}

/// The game entity a Bevy entity shows.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mirrored(pub Id);

/// An animated model: its vertex frames, and the blend shown.
#[derive(Component)]
pub struct ModelView {
    model: Arc<LoadedModel>,
    shown: Option<(usize, usize, f32)>,
}

/// The material an entity was built with, and its own copy while it is translucent,
/// tinted or plays a sprite strip.
#[derive(Component)]
pub struct MaterialView {
    base: Handle<StandardMaterial>,
    own: Option<Handle<StandardMaterial>>,
    /// The base material's alpha mode (an opaque copy goes back to it).
    base_mode: AlphaMode,
    /// Always uses its own blended copy (fading signs).
    blend: bool,
    /// Sprite strip frames (UV offsets).
    frames: u32,
}

/// Spawns, rebuilds and despawns mirrored entities.
pub(super) fn spawn(mut commands: Commands, game: Res<Game>, mut mirror: ResMut<Mirror>, mut loader: Loader) {
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

fn build(commands: &mut Commands, loader: &mut Loader, game: &Game, id: Id, look: &Look) -> Entity {
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

/// Transforms, visibility, lights and animation of every mirrored entity.
pub(super) fn update(
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

