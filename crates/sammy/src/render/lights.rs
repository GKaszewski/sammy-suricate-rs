//! A4 dynamic lights (`my.lightred/green/blue`, `my.lightrange`): added on top of
//! the lightmaps by the level shader, and as Bevy point lights for models.

use bevy::prelude::*;

use crate::camera::MainCamera;
use crate::render::coords::QUANTS_PER_UNIT;
use crate::render::material::{LevelMaterial, LightData, MAX_LIGHTS};

/// A4 light colour channel maximum (colours are 0..255).
const COLOR_MAX: f32 = 255.0;
/// Bevy point light intensity at `POINT_LIGHT_REF_RANGE`; scales linearly with range.
const POINT_LIGHT_INTENSITY: f32 = 2_000_000.0;
/// Range in quants at which a point light has `POINT_LIGHT_INTENSITY`.
const POINT_LIGHT_REF_RANGE: f32 = 300.0;
/// Smallest Bevy point light range (units), keeps off lights valid.
const MIN_POINT_RANGE: f32 = 0.01;
/// Full-strength light colour channel of a key light.
const KEY_FULL: f32 = 250.0;
/// Orange key light green channel.
const KEY_ORANGE_G: f32 = 100.0;

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fade_lights)
            .add_systems(PostUpdate, (sync_point_lights, update_level_lights).after(TransformSystems::Propagate));
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
        let color = Color::srgb(l.color[0] / COLOR_MAX, l.color[1] / COLOR_MAX, l.color[2] / COLOR_MAX);
        let range = l.range / QUANTS_PER_UNIT;
        let intensity = if l.range > 0.0 { POINT_LIGHT_INTENSITY * (l.range / POINT_LIGHT_REF_RANGE) } else { 0.0 };
        match child.and_then(|c| points.get_mut(c).ok()) {
            Some(mut p) => {
                p.color = color;
                p.range = range.max(MIN_POINT_RANGE);
                p.intensity = intensity;
            }
            None => {
                commands.entity(e).with_child((
                    DynamicPointLight,
                    PointLight { color, range: range.max(MIN_POINT_RANGE), intensity, shadow_maps_enabled: false, ..default() },
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
    for (i, (_, p, l)) in active.iter().take(MAX_LIGHTS).enumerate() {
        data.data[i * 2] = p.extend(l.range / QUANTS_PER_UNIT);
        data.data[i * 2 + 1] = Vec4::new(l.color[0] / COLOR_MAX, l.color[1] / COLOR_MAX, l.color[2] / COLOR_MAX, 0.0);
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

/// A light whose range shrinks by `rate` quants per tick until `until`, then turns off
/// (GemLight, DoorLight, BonusVie/Boost flashes).
#[derive(Component)]
pub struct FadingLight {
    pub rate: f32,
    pub until: f32,
}

/// Light colour of a key colour bitmask (`GemLight` / `DoorLight`, gems.wdl:454-477).
pub fn key_color(key: u32) -> [f32; 3] {
    match key {
        1 => [KEY_FULL, 0.0, 0.0],
        2 => [KEY_FULL, KEY_FULL, 0.0],
        3 => [KEY_FULL, KEY_ORANGE_G, 0.0],
        4 => [0.0, 0.0, KEY_FULL],
        5 => [KEY_FULL, 0.0, KEY_FULL],
        6 => [0.0, KEY_FULL, 0.0],
        _ => [KEY_FULL, KEY_FULL, KEY_FULL],
    }
}

/// Bundle for a flash: start range, shrink rate per tick, cut-off range.
pub fn flash(color: [f32; 3], range: f32, rate: f32, until: f32) -> (DynamicLight, FadingLight) {
    (DynamicLight { color, range }, FadingLight { rate, until })
}

fn fade_lights(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut DynamicLight, &FadingLight)>) {
    let t = time.delta_secs() * crate::gameplay::player::TICKS;
    for (e, mut l, f) in &mut q {
        l.range -= f.rate * t;
        if l.range <= f.until {
            l.range = 0.0;
            commands.entity(e).remove::<FadingLight>();
        }
    }
}
