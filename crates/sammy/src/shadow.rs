//! A4 engine model shadows (`my.shadow = 1`): Sammy at `AverageFps > 40`
//! (player.wdl:197-207, so always on modern machines) and entities with the editor
//! SHADOW flag. The animated mesh is flattened onto the floor below the owner along
//! `sun_angle` (pan 135, tilt 85 in levels, mm1.wdl:618-620; pan 90 in the extro,
//! Extro.wdl:81). The exact A4 projection and darkness are [U]; 50 % black is used.

use bevy::asset::embedded_asset;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, CompareFunction, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

use crate::collision::CollisionWorld;
use crate::coords;

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shadow.wesl");
        app.add_plugins(MaterialPlugin::<ShadowMaterial>::default())
            .init_resource::<SunAngle>()
            .add_systems(Update, (spin_sun, attach_shadows, update_shadows).chain());
    }
}

/// `sun_angle` (pan, tilt) in degrees: the direction the sun is seen from.
#[derive(Resource, Clone, Copy)]
pub struct SunAngle(pub f32, pub f32);

impl Default for SunAngle {
    fn default() -> Self {
        Self(135.0, 85.0)
    }
}

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
        "embedded://sammy/shadow.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://sammy/shadow.wesl".into()
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
        let mat = materials.add(ShadowMaterial { params: ShadowParams { sun: Vec4::new(0.0, -1.0, 0.0, 0.0), color: Vec4::new(0.0, 0.0, 0.0, 0.5) } });
        let child = commands.spawn((ModelShadow { owner: e }, Mesh3d(mesh.0.clone()), MeshMaterial3d(mat), NotShadowCaster, Transform::default(), Visibility::Inherited, ChildOf(e))).id();
        commands.entity(e).insert(ModelShadowLink(child));
    }
}

#[derive(Component)]
pub struct ModelShadowLink(#[allow(dead_code)] Entity);

/// Win/lose screen: `Sun_Angle.pan += 10 * time` (mm1.wdl:966), so the shadow circles Sammy.
fn spin_sun(time: Res<Time>, current: Res<crate::flow::CurrentLevel>, mut sun: ResMut<SunAngle>) {
    use crate::flow::Phase;
    if matches!(current.phase, Phase::Won(_) | Phase::Lost(_)) {
        sun.0 += 10.0 * time.delta_secs() * crate::actions::player::TICKS;
    }
}

fn update_shadows(
    world: CollisionWorld,
    sun: Res<SunAngle>,
    mut materials: ResMut<Assets<ShadowMaterial>>,
    owners: Query<&GlobalTransform>,
    q: Query<(&ModelShadow, &MeshMaterial3d<ShadowMaterial>)>,
) {
    if !world.ready() {
        return;
    }
    let (p, t) = (sun.0.to_radians(), sun.1.to_radians());
    // Light travels from the sun toward the ground.
    let to_sun = [t.cos() * p.cos(), t.cos() * p.sin(), t.sin()];
    let dir = coords::dir([-to_sun[0], -to_sun[1], -to_sun[2]]);
    for (s, mat) in &q {
        let Ok(gt) = owners.get(s.owner) else { continue };
        let o = coords::native(gt.translation());
        let tr = world.trace_static(0, [o[0], o[1], o[2] + 8.0], [o[0], o[1], o[2] - 2000.0]).trace;
        let floor = if tr.hit() && !tr.start_solid { tr.end[2] } else { o[2] - 2000.0 };
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.params.sun = dir.extend((floor + 0.5) / coords::QUANTS_PER_UNIT);
        }
    }
}
