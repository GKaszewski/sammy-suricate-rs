#[allow(unused_imports)]
use super::*;

#[allow(clippy::too_many_arguments)]
pub fn move_player(
    time: Res<Time>,
    controls: Res<crate::platform::input::Controls>,
    jump: Res<JumpParams>,
    current: Res<crate::flow::CurrentLevel>,
    world: CollisionWorld,
    mut game: ResMut<crate::gameplay::state::GameState>,
    mut sfx: crate::platform::audio::Sfx,
    tuning: Res<PlayerTuning>,
    enemies: Query<(), With<crate::gameplay::enemies::Enemy>>,
    pushers: Query<(), Or<(With<crate::gameplay::platforms::Platform>, With<crate::gameplay::wasps::Hive>)>>,
    gems: Query<(), With<crate::gameplay::pickups::KeyGem>>,
    mut touched_msgs: MessageWriter<PlayerTouched>,
    mut q: Query<(Entity, &mut Player, &mut Transform, Option<&mut ModelAnimator>)>,
) {
    if !world.ready() {
        return;
    }
    let mut controls = controls.0.clone();
    if std::env::var_os("SAMMY_AUTOWALK").is_some() {
        controls.forward = 1.0; // debug: hold forward
    }
    let touch = |e: sammy_core::id::Id| {
        let e = entity(e);
        if gems.contains(e) {
            Touch::Gem
        } else if enemies.contains(e) {
            Touch::Enemy
        } else if pushers.contains(e) {
            Touch::Pusher
        } else {
            Touch::Other
        }
    };
    for (me, mut p, mut tf, mut anim) in &mut q {
        let frame = Frame {
            space: &world,
            controls: &controls,
            // Input only counts while the level is being played (`PartieEncours == 1`).
            input: crate::flow::playing(&current),
            won: matches!(current.phase, crate::flow::Phase::Won(_)),
            jump: &jump,
            tuning: &tuning,
            me: id(me),
            touch: &touch,
            t: time.delta_secs() * TICKS,
        };
        let mut out = StepOut::default();
        p.step(&mut game, anim.as_deref_mut().map(|a| &mut a.anim), &frame, &mut out);
        for s in out.sounds {
            sfx.play(s);
        }
        if let Some(e) = out.touched {
            touched_msgs.write(PlayerTouched(entity(e)));
        }
        tf.translation = coords::pos(p.pos);
        tf.rotation = coords::rotation(p.angles());
    }
}
