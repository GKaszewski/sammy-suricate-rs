#[allow(unused_imports)]
use super::*;

pub fn init_boost(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((BoostPickup { boost: sammy_core::health::BoostPickup::from_object(obj), scale: wed_scale(obj) }, PickupPos(obj.origin)));
}

pub fn init_tile_death(entity: &mut EntityCommands) {
    entity.insert(DeathTile);
}

pub fn init_lava(entity: &mut EntityCommands) {
    entity.insert(LavaTile);
}

pub fn init_small_heart(obj: &Object, entity: &mut EntityCommands) {
    let heart = sammy_core::health::SmallHeart::from_object(obj);
    let snap = heart.snap;
    entity.insert((SmallHeart { heart, scale: wed_scale(obj) }, PickupPos(obj.origin)));
    if snap {
        entity.insert(SnapToGround);
    }
}

pub fn init_bonus_vie(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((BigHeart(sammy_core::health::BigHeart::from_object(obj)), PickupPos(obj.origin)));
}

pub fn init_restart(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((Checkpoint(sammy_core::health::Checkpoint::from_object(obj)), PickupPos(obj.origin), Respawnable));
}

/// Records the level spawn as the first respawn point.
pub fn init_start(pos: [f32; 3], pan: f32, entity: &mut EntityCommands) {
    entity.commands().queue(move |w: &mut World| w.resource_mut::<GameState>().start = Some((pos, pan)));
}
