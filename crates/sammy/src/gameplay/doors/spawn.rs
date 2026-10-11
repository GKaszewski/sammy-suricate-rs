#[allow(unused_imports)]
use super::*;

pub fn init_door(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Door(sammy_core::doors::Door::from_object(obj)));
}

pub fn init_exit(entity: &mut EntityCommands) {
    entity.insert(ExitTile::default());
}
