#[allow(unused_imports)]
use super::*;

pub fn init_engine(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Engine(sammy_core::platforms::Engine::from_object(obj)));
}

pub fn init_platform(obj: &Object, body: bool, entity: &mut EntityCommands) {
    entity.insert(Platform { platform: sammy_core::platforms::Platform::from_object(obj, body), engine: None });
}

pub fn init_teleport(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Teleport(sammy_core::platforms::Teleport::from_object(obj)));
}
