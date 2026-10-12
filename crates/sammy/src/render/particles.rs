//! Draws the game's particles ([`sammy_present::particles`]): the effects and emitters
//! run after the game tick, and every batch of live particles sharing a bitmap and
//! blend mode is one camera-facing quad mesh, rebuilt every frame.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sammy_core::player::State;
use sammy_present::effects;
use sammy_present::particles::BatchKey;

use sammy_core::time::Secs;

use crate::camera::MainCamera;
use crate::game::{Game, GameEvent};
use crate::render::coords;
use crate::scene::Loader;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Particles>()
            .add_systems(PreUpdate, |time: Res<Time>, mut ps: ResMut<Particles>| ps.advance(Secs(time.delta_secs())))
            .add_systems(
                Update,
                (play_effects, emit.run_if(|g: Res<Game>| g.flow.phase != sammy_core::flow::Phase::Intro)).chain().after(crate::game::GameSystems::Tick),
            )
            .add_systems(PostUpdate, ((|mut ps: ResMut<Particles>| ps.update()), draw_particles).chain().before(TransformSystems::Propagate));
    }
}

/// The particles as a resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Particles(pub sammy_present::particles::Particles);

fn play_effects(mut ps: ResMut<Particles>, mut events: MessageReader<GameEvent>) {
    for GameEvent(e) in events.read() {
        effects::play(&mut ps, e);
    }
}

fn emit(mut ps: ResMut<Particles>, game: Res<Game>, mut prev: Local<Option<(State, bool)>>) {
    effects::emit_trails(&mut ps, &game);
    effects::emit_sammy(&mut ps, &game, &mut prev);
}

/// One draw batch per (bitmap, additive).
#[derive(Component)]
pub struct ParticleBatch;

#[derive(Default)]
struct Batches(HashMap<BatchKey, (Entity, Handle<Mesh>)>);

#[allow(clippy::too_many_arguments)]
fn draw_particles(
    mut commands: Commands,
    particles: Res<Particles>,
    mut loader: Loader,
    cam: Query<&GlobalTransform, With<MainCamera>>,
    mut batches: Local<Batches>,
    mut dot: Local<Option<Handle<Image>>>,
    alive: Query<(), With<ParticleBatch>>,
) {
    let Ok(cam) = cam.single() else { return };
    // Batches despawned with the level are recreated on demand.
    batches.0.retain(|_, (e, _)| alive.contains(*e));
    let groups = particles.quads(coords::native_dir(cam.right().as_vec3()), coords::native_dir(cam.up().as_vec3()));
    // Empty the batches that have no particles this frame.
    for (key, (_, mesh)) in &batches.0 {
        if !groups.contains_key(key)
            && let Some(mut m) = loader.meshes.get_mut(mesh)
        {
            m.insert_indices(Indices::U32(Vec::new()));
        }
    }
    for (key, verts) in groups {
        let pos: Vec<[f32; 3]> = verts.iter().map(|v| coords::pos(v.pos).to_array()).collect();
        let idx: Vec<u32> = (0..verts.len() as u32 / 4).flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i)).collect();
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; pos.len()])
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, verts.iter().map(|v| v.uv).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, verts.iter().map(|v| v.color).collect::<Vec<_>>())
            .with_inserted_indices(Indices::U32(idx));
        if let Some((_, handle)) = batches.0.get(&key) {
            if let Some(mut m) = loader.meshes.get_mut(handle) {
                *m = mesh;
            }
            continue;
        }
        let texture = match key.0 {
            Some(file) => loader.sprite(file).and_then(|s| loader.materials.get(&s.material).and_then(|m| m.base_color_texture.clone())),
            None => Some(dot.get_or_insert_with(|| loader.images.add(crate::render::image::to_bevy(&sammy_present::particles::dot_image(), true, false))).clone()),
        };
        let material = loader.materials.add(StandardMaterial {
            base_color_texture: texture,
            unlit: true,
            alpha_mode: if key.1 { AlphaMode::Add } else { AlphaMode::Blend },
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        let handle = loader.meshes.add(mesh);
        let e = commands
            .spawn((ParticleBatch, Mesh3d(handle.clone()), MeshMaterial3d(material), NotShadowCaster, Transform::default(), bevy::camera::visibility::NoFrustumCulling))
            .id();
        batches.0.insert(key, (e, handle));
    }
}
