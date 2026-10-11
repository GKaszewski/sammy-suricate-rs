#[allow(unused_imports)]
use super::*;

pub fn init_sky_menu(obj: &mut Object, pov_z: f32, entity: &mut EntityCommands) -> ActionInit {
    let (look, follow) = sammy_core::decor::init_sky_menu(obj, pov_z);
    if let Some(f) = follow {
        entity.insert(SkyFollow(f));
    }
    look
}
