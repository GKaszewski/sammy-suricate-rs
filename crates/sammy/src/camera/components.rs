#[allow(unused_imports)]
use super::*;

/// The world-rendering camera.
#[derive(Component)]
pub struct MainCamera;

/// Secondary cameras (sky pass, near pass) that mirror the main camera's transform.
#[derive(Component)]
pub struct SkyCamera;

/// A camera zone ([`sammy_core::camera::CameraZone`]).
#[derive(Component, Deref, DerefMut)]
pub struct CameraZone(pub(crate) sammy_core::camera::CameraZone);

/// Follow-camera state ([`sammy_core::camera::FollowState`]).
#[derive(Component, Default, Deref, DerefMut)]
pub struct FollowState(pub sammy_core::camera::FollowState);

#[derive(Component, Default)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
}
