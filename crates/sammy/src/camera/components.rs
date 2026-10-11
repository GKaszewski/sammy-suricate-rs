#[allow(unused_imports)]
use super::*;

/// The world-rendering camera.
#[derive(Component)]
pub struct MainCamera;

/// Secondary cameras (sky pass, near pass) that mirror the main camera's transform.
#[derive(Component)]
pub struct SkyCamera;

#[derive(Component, Default)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
}
