#[allow(unused_imports)]
use super::*;

pub fn init_boulder(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Boulder(sammy_core::gems::Boulder::from_object(obj)));
}

pub fn spawn_door_gem(commands: &mut Commands, sprites: &GemSprites, door: Entity, color: u32, sammy: [f32; 3]) {
    let gem = sammy_core::gems::DoorGem::new(color, sammy);
    let Some((mesh, mat, frames)) = sprites.0.get(&color).cloned() else { return };
    commands.spawn((
        Name::new("door gem"),
        crate::flow::LevelScoped,
        Transform::from_translation(coords::pos(gem.pos)),
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Billboard,
        SpriteAnim { frames, frame: 0.0, fps: -GEM_FPS },
        DoorGem { door, gem },
    ));
}
