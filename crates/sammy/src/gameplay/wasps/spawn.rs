#[allow(unused_imports)]
use super::*;

/// Returns the model skin and animation speed.
pub fn init_ammo(obj: &mut Object, entity: &mut EntityCommands) -> (u32, f32) {
    let (pickup, look) = sammy_core::wasps::AmmoPickup::from_object(obj);
    if let Some(sc) = look.scale {
        obj.scale = [sc; 3];
    }
    obj.angles = [0.0, PICKUP_TILT, 0.0];
    entity.insert((AmmoPickup(pickup), PickupPos(obj.origin), Spinning { angles: obj.angles }));
    (look.skin, sammy_core::wasps::PICKUP_ANIM)
}

pub fn init_door_weak(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(WeakDoor(sammy_core::wasps::WeakDoor::from_object(obj)));
}

pub(crate) fn spawn_wasp(commands: &mut Commands, assets: &GameAssets, meshes: &mut Assets<Mesh>, sfx: &mut Sfx, shot: sammy_core::wasps::Shot, shooter: Entity) {
    let Some(model) = assets.model("wasp1.mdl") else { return };
    let w = sammy_core::wasps::Wasp::new(shot, Some(id(shooter)));
    let mut anim = ModelAnimator::new(model.clone(), Vec::new(), crate::level::MODEL_FPS);
    anim.play("walk", true, 0.0);
    anim.set_speed_pct(sammy_core::wasps::WING_ANIM * w.speed(), TICKS);
    sfx.play("Wasp_son");
    let scale = FIRED[(shot.kind - 1) as usize].scale;
    commands.spawn((
        Name::new("wasp"),
        crate::flow::LevelScoped,
        Mesh3d(meshes.add(model.build_mesh(anim.frames.first().copied().unwrap_or(0)))),
        MeshMaterial3d(model.skin(shot.kind)),
        Transform { translation: coords::pos(shot.pos), rotation: coords::rotation([shot.pan, 0.0, 0.0]), scale: Vec3::splat(scale) },
        Visibility::Hidden,
        anim,
        Wasp(w),
    ));
}

pub fn init_hive(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Hive(sammy_core::wasps::Hive::from_object(obj)));
}
