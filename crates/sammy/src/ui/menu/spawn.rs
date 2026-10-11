#[allow(unused_imports)]
use super::*;

pub fn init_item(obj: &Object, title: bool, entity: &mut EntityCommands) -> ActionInit {
    let sprite = obj.file.to_lowercase().ends_with(".pcx");
    entity.insert(MenuItem {
        code: obj.skill(1) as u32,
        bob: sammy_core::menu::Bob::from_object(obj, title),
        sprite,
        alpha: sammy_core::menu::base_alpha(obj),
        scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs(),
        styled: false,
        locked: false,
        sparkle: [0.0; 2],
    });
    // `facing = 1`: sprites are screen-parallel.
    ActionInit { facing: sprite, ..default() }
}

pub fn init_marker(obj: &Object, code: u32, entity: &mut EntityCommands) -> ActionInit {
    entity.insert(MenuMarker { code, pos: obj.origin, pan: obj.angles[0], tilt: obj.angles[1] });
    ActionInit { hidden_but_alive: true, ..default() }
}
