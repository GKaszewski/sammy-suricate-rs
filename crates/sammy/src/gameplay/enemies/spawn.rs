#[allow(unused_imports)]
use super::*;

pub fn init_path(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(PathNode(sammy_core::enemies::PathNode::from_object(obj)));
}

pub fn init_lion(obj: &mut Object, entity: &mut EntityCommands) -> Option<u32> {
    let (e, skin) = sammy_core::enemies::init_lion(obj);
    entity.insert(Enemy(e));
    skin
}

/// `InitWaspEnnemy` / `InitWaspExtro`: returns the skin (the scale override is applied
/// to `obj`).
pub fn init_wasp_ennemy(obj: &mut Object, entity: &mut EntityCommands, extro: bool) -> Option<u32> {
    let (e, skin, scale) = sammy_core::enemies::init_wasp_ennemy(obj, extro);
    if let Some(k) = scale {
        obj.scale = [k; 3];
    }
    entity.insert(Enemy(e));
    skin
}

pub fn init_start_ennemy(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(StartTrigger(sammy_core::enemies::StartTrigger::from_object(obj)));
}
