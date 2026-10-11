#[allow(unused_imports)]
use super::*;

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum CameraMode {
    #[default]
    Follow,
    Fly,
}

/// Per-level camera settings ([`sammy_core::camera::CameraParams`]).
#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
pub struct CameraParams(pub sammy_core::camera::CameraParams);

/// Camera globals that change during play ([`sammy_core::camera::CameraRuntime`]).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct CameraRuntime(pub sammy_core::camera::CameraRuntime);
