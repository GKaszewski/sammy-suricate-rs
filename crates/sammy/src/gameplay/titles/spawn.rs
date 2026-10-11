#[allow(unused_imports)]
use super::*;

pub fn init_title(obj: &mut Object, entity: &mut EntityCommands) {
    let title = sammy_core::titles::Title::from_object(obj);
    // The sign is drawn where its trigger centre is.
    obj.origin = title.center;
    entity.insert((Title(title), Visibility::Hidden));
}
