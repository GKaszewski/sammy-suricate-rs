#[allow(unused_imports)]
use super::*;

pub fn init_trigger(action: &str, obj: &Object, entity: &mut EntityCommands) {
    let kind = match action {
        "initsetpovsyn" => TriggerKind::PovSyn,
        "initsetanimplayer" => TriggerKind::AnimPlayer,
        _ => TriggerKind::FinExtro,
    };
    entity.insert(ExtroTrigger(sammy_core::extro::Trigger::from_object(kind, obj)));
}
