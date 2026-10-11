#[allow(unused_imports)]
use super::*;

pub fn init_zone(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(CameraZone(sammy_core::camera::CameraZone::from_object(obj)));
}
