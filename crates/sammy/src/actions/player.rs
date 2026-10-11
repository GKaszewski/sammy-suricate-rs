//! Sammy as a Bevy entity: the [`sammy_core::player::Player`] component, his tuning
//! resources and the system that steps him.

use std::sync::Arc;

use bevy::prelude::*;
use formats::wmb::{Object, Wmb};
use sammy_core::player::{Frame, StepOut, Touch};

use crate::collision::{CollisionWorld, entity, id};
use crate::coords;
use crate::model::ModelAnimator;

pub use sammy_core::player::{BLOB_HAUT, SCALE, State};
pub use sammy_core::time::TICKS_PER_SEC as TICKS;

pub use crate::game::BOOST_MAX;

/// [`sammy_core::player::PlayerTuning`] as a resource.
#[derive(Resource, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct PlayerTuning(pub sammy_core::player::PlayerTuning);

/// Collision geometry of the current level.
#[derive(Resource, Clone)]
pub struct LevelCollision(pub Arc<Wmb>);

/// [`sammy_core::physics::JumpParams`] as a resource.
#[derive(Resource, Clone, Copy, Default, Deref, DerefMut)]
pub struct JumpParams(pub sammy_core::physics::JumpParams);

/// Sammy ([`sammy_core::player::Player`]).
#[derive(Component, Deref, DerefMut)]
pub struct Player(pub sammy_core::player::Player);

impl Player {
    /// The entity he stands on, if any (None = level geometry).
    pub fn ground(&self) -> Option<Entity> {
        self.ground_entity.map(entity)
    }
}

pub fn init_player(obj: &mut Object, entity: &mut EntityCommands) {
    obj.scale = [SCALE; 3];
    let mut p = sammy_core::player::Player::from_object(obj);
    // Debug: SAMMY_START="x,y,z[,pan]" overrides the spawn point.
    if let Ok(v) = std::env::var("SAMMY_START") {
        let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        let (mut pos, mut pan) = (p.pos, p.pan);
        if n.len() >= 3 {
            pos = [n[0], n[1], n[2]];
        }
        if let Some(&a) = n.get(3) {
            pan = a;
        }
        p = sammy_core::player::Player::at(pos, pan, p.speed, p.scan_radius);
    }
    entity.insert(Player(p));
}

/// Sammy touched an entity while moving (EVENT_ENTITY on Sammy / EVENT_IMPACT on it).
#[derive(Message, Clone, Copy)]
pub struct PlayerTouched(pub Entity);

#[allow(clippy::too_many_arguments)]
pub fn move_player(
    time: Res<Time>,
    controls: Res<crate::input::Controls>,
    jump: Res<JumpParams>,
    current: Res<crate::flow::CurrentLevel>,
    world: CollisionWorld,
    mut game: ResMut<crate::game::GameState>,
    mut sfx: crate::audio::Sfx,
    tuning: Res<PlayerTuning>,
    enemies: Query<(), With<super::enemy::Enemy>>,
    pushers: Query<(), Or<(With<super::platform::Platform>, With<super::wasp::Hive>)>>,
    gems: Query<(), With<super::pickups::KeyGem>>,
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
