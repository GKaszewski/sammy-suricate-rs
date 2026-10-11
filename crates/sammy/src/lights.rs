//! A4 dynamic lights (`my.lightred/green/blue`, `my.lightrange`): added on top of
//! the lightmaps by the level shader, and as Bevy point lights for models.

use bevy::prelude::*;

use crate::camera::MainCamera;
use crate::coords::QUANTS_PER_UNIT;
use crate::material::{LevelMaterial, LightData};

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (sync_point_lights, update_level_lights).after(TransformSystems::Propagate));
    }
}

/// A dynamic light on an entity: colour in A4 units (0..255), range in quants
/// (0 = off), offset from the entity origin in Bevy units.
#[derive(Component, Clone, Copy, Debug)]
pub struct DynamicLight {
    pub color: [f32; 3],
    pub range: f32,
}

/// Child point light mirroring a `DynamicLight` for models (StandardMaterial).
#[derive(Component)]
struct DynamicPointLight;

fn sync_point_lights(
    mut commands: Commands,
    lights: Query<(Entity, &DynamicLight, Option<&Children>)>,
    mut points: Query<&mut PointLight, With<DynamicPointLight>>,
) {
    for (e, l, children) in &lights {
        let child = children.and_then(|c| c.iter().find(|c| points.contains(*c)));
        let color = Color::srgb(l.color[0] / 255.0, l.color[1] / 255.0, l.color[2] / 255.0);
        let range = l.range / QUANTS_PER_UNIT;
        let intensity = if l.range > 0.0 { 2_000_000.0 * (l.range / 300.0) } else { 0.0 };
        match child.and_then(|c| points.get_mut(c).ok()) {
            Some(mut p) => {
                p.color = color;
                p.range = range.max(0.01);
                p.intensity = intensity;
            }
            None => {
                commands.entity(e).with_child((
                    DynamicPointLight,
                    PointLight { color, range: range.max(0.01), intensity, shadow_maps_enabled: false, ..default() },
                    Transform::default(),
                ));
            }
        }
    }
}

/// Feeds the 8 lights nearest the camera to every level material.
fn update_level_lights(
    cam: Query<&GlobalTransform, With<MainCamera>>,
    lights: Query<(&GlobalTransform, &DynamicLight)>,
    mut materials: ResMut<Assets<LevelMaterial>>,
    mut last: Local<LightData>,
) {
    let cam = cam.single().map(|c| c.translation()).unwrap_or_default();
    let mut active: Vec<(f32, Vec3, &DynamicLight)> = lights
        .iter()
        .filter(|(_, l)| l.range > 0.0)
        .map(|(t, l)| (t.translation().distance_squared(cam), t.translation(), l))
        .collect();
    active.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut data = LightData::default();
    for (i, (_, p, l)) in active.iter().take(8).enumerate() {
        data.data[i * 2] = p.extend(l.range / QUANTS_PER_UNIT);
        data.data[i * 2 + 1] = Vec4::new(l.color[0] / 255.0, l.color[1] / 255.0, l.color[2] / 255.0, 0.0);
    }
    if data == *last {
        return;
    }
    *last = data;
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(mut m) = materials.get_mut(id) {
            m.lights = data;
        }
    }
}
