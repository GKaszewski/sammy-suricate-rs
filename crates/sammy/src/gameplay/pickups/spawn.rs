#[allow(unused_imports)]
use super::*;

pub fn init_bonus3(obj: &Object, entity: &mut EntityCommands) {
    let d = sammy_core::pickups::Diamond::from_object(obj);
    entity.insert((Diamond(d), PickupPos(obj.origin)));
    if d.snap {
        entity.insert(SnapToGround);
    }
}

pub fn init_key(obj: &Object, entity: &mut EntityCommands) {
    let spec = KeySpec::from_object(obj);
    entity.insert((
        KeyGem { color: spec.color },
        PickupPos(obj.origin),
        LevelKey { home: None, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs() },
        Respawnable,
    ));
    match spec.surprise {
        // Surprise gem: not collectable until its colour has been rolled.
        Some(add) => entity.insert(crate::gameplay::gems::Surprise::new(add, obj.origin[0] + obj.origin[1])),
        None => entity.insert(crate::gameplay::gems::Collectable),
    };
    if spec.gravity {
        entity.insert(crate::gameplay::gems::GemGravity::new(HEIGHT_GEM));
    }
    if spec.hover_from_wed {
        entity.insert(HoverFromWed);
    }
    if spec.snap {
        entity.insert(SnapToGround);
    }
}
