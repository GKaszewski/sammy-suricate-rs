//! A4 engine model shadows (`my.shadow = 1`): Sammy at `AverageFps > 40`
//! (player.wdl:197-207, so always on modern machines) and entities with the editor
//! SHADOW flag. The animated mesh is flattened onto the floor below the owner along
//! `sun_angle` ([`sammy_present::extras::Extras::sun`]). The exact A4 projection and darkness are [U]; 50 % black is used.

use bevy::asset::embedded_asset;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, CompareFunction, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use sammy_core::collision::Hull;

use crate::game::{Game, GameSystems};
use crate::render::coords;

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shadow.wesl");
        app.add_plugins(MaterialPlugin::<ShadowMaterial>::default())
            .add_systems(Update, (attach_shadows, update_shadows).chain().after(GameSystems::Tick));
    }
}

/// Shadow colour alpha: 50 % black ([U] in the original).
const SHADOW_ALPHA: f32 = 0.5;
/// Floor probe starts this many quants above the owner origin.
const FLOOR_PROBE_UP: f32 = 8.0;
/// Floor probe reaches this many quants below the owner origin.
const FLOOR_PROBE_DOWN: f32 = 2000.0;
/// Quants the shadow plane is lifted above the floor, against z-fighting.
const FLOOR_LIFT: f32 = 0.5;

#[derive(Clone, Copy, ShaderType, Default, Debug)]
pub struct ShadowParams {
    pub sun: Vec4,
    pub color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ShadowMaterial {
    #[uniform(0)]
    pub params: ShadowParams,
}

impl Material for ShadowMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://sammy/render/shadow.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://sammy/render/shadow.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // All fragments lie on one plane: write depth and use a strict test so that
        // overlapping triangles of the silhouette darken the floor only once.
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(true);
            ds.depth_compare = Some(CompareFunction::Greater);
        }
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Owner of an engine shadow (needs a `Mesh3d`).
#[derive(Component)]
pub struct CastsShadow;

#[derive(Component)]
pub struct ModelShadow {
    owner: Entity,
}

fn attach_shadows(
    mut commands: Commands,
    mut materials: ResMut<Assets<ShadowMaterial>>,
    q: Query<(Entity, &Mesh3d), (With<CastsShadow>, Without<ModelShadowLink>)>,
) {
    for (e, mesh) in &q {
        let mat = materials.add(ShadowMaterial { params: ShadowParams { sun: Vec4::new(0.0, -1.0, 0.0, 0.0), color: Vec4::new(0.0, 0.0, 0.0, SHADOW_ALPHA) } });
        let child = commands.spawn((ModelShadow { owner: e }, Mesh3d(mesh.0.clone()), MeshMaterial3d(mat), NotShadowCaster, Transform::default(), Visibility::Inherited, ChildOf(e))).id();
        commands.entity(e).insert(ModelShadowLink(child));
    }
}

#[derive(Component)]
pub struct ModelShadowLink(#[allow(dead_code)] Entity);

fn update_shadows(
    game: Res<Game>,
    extras: Res<crate::scene::extras::ExtrasState>,
    mut materials: ResMut<Assets<ShadowMaterial>>,
    owners: Query<&GlobalTransform>,
    q: Query<(&ModelShadow, &MeshMaterial3d<ShadowMaterial>)>,
) {
    if game.level.is_none() {
        return;
    }
    let (p, t) = (extras.sun[0].to_radians(), extras.sun[1].to_radians());
    // Light travels from the sun toward the ground.
    let to_sun = [t.cos() * p.cos(), t.cos() * p.sin(), t.sin()];
    let dir = coords::dir([-to_sun[0], -to_sun[1], -to_sun[2]]);
    for (s, mat) in &q {
        let Ok(gt) = owners.get(s.owner) else { continue };
        let o = coords::native(gt.translation());
        let tr = game.trace_static(Hull::Point, [o[0], o[1], o[2] + FLOOR_PROBE_UP], [o[0], o[1], o[2] - FLOOR_PROBE_DOWN]).trace;
        let floor = if tr.hit() && !tr.start_solid { tr.end[2] } else { o[2] - FLOOR_PROBE_DOWN };
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.params.sun = dir.extend((floor + FLOOR_LIFT) / coords::QUANTS_PER_UNIT);
        }
    }
}
