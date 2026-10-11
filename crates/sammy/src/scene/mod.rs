//! The game world on screen: the level geometry, every game entity mirrored as a Bevy
//! entity, and the extras drawn from the game state (ground shadows, range and call
//! rings, popups, the lose title, the HUD wasp and eye).

pub mod assets;
pub mod extras;
pub mod level;
pub mod mirror;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

pub use assets::{Loader, SceneAssets, SpriteAsset};
pub use mirror::{Mirror, Mirrored};

use crate::camera::MainCamera;
use crate::game::GameSystems;

/// Render layer for sky domes, drawn by a background camera before the world.
pub const SKY_LAYER: RenderLayers = RenderLayers::layer(1);

/// Render layer for `Near` entities (help signs), drawn last over a cleared depth buffer.
pub const NEAR_LAYER: RenderLayers = RenderLayers::layer(2);

/// Camera-facing sprite (rotates around the vertical axis only, like Acknex sprites).
#[derive(Component)]
pub struct Billboard;

/// Sprite parallel to the screen (WDL `Facing` + pan/tilt copied from the camera).
#[derive(Component)]
pub struct ScreenAligned;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneAssets>()
            .init_resource::<Mirror>()
            .add_systems(Update, (level::rebuild, mirror::spawn, mirror::update).chain().after(GameSystems::Tick))
            .init_resource::<extras::ExtrasState>()
            .add_systems(Update, (extras::update, extras::draw).chain().after(GameSystems::Tick))
            .add_systems(PostUpdate, extras::draw_hud.after(TransformSystems::Propagate).before(bevy::camera::visibility::VisibilitySystems::CheckVisibility))
            .add_systems(PostUpdate, face_camera.after(crate::camera::CameraSystems::Place).before(TransformSystems::Propagate));
    }
}

pub fn face_camera(
    cam: Query<&Transform, (With<MainCamera>, Without<Billboard>, Without<ScreenAligned>)>,
    mut q: ParamSet<(Query<&mut Transform, With<Billboard>>, Query<&mut Transform, With<ScreenAligned>>)>,
) {
    let Ok(cam) = cam.single() else { return };
    for mut tf in &mut q.p0() {
        let d = cam.translation - tf.translation;
        tf.rotation = Quat::from_rotation_y(d.x.atan2(d.z));
    }
    for mut tf in &mut q.p1() {
        tf.rotation = cam.rotation;
    }
}
