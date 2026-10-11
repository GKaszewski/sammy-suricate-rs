#[allow(unused_imports)]
use super::*;

pub fn init_walker(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((Walker(sammy_core::intro::Walker::from_object(obj)), crate::render::shadow::CastsShadow));
}

pub fn init_node(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(PathNode(sammy_core::intro::PathNode::from_object(obj)));
}
