//! Draws [`sammy_present::extras`]: enemy ground shadows, range and call rings, score
//! popups and the lose title as world entities, and the HUD wasp / Eye of Truth plate
//! glued to the camera next to the 2D HUD. Each list position keeps its entity while
//! it shows the same kind of thing.

use std::collections::HashMap;

use bevy::prelude::*;
use sammy_present::extras::{Blend, Extras, HudModel, Item, View};

use super::{Billboard, Loader, NEAR_LAYER};
use crate::camera::MainCamera;
use crate::game::{Game, GameEvent};
use crate::render::coords::{self, QUANTS_PER_UNIT};
use crate::render::model::ModelAnimator;
use crate::render::particles::Particles;

mod build;
mod hud;
mod world;

pub(super) use self::{hud::*, world::*};
use self::build::*;
use sammy_core::time::Secs;

/// The extras' state as a resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ExtrasState(pub Extras);

/// What a pooled entity shows; another kind needs a new entity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Kind {
    Model(&'static str, Blend),
    Title(&'static str),
    Decal(&'static str),
    Sprite(&'static str),
    Hud(&'static str, Blend),
}

/// An entity showing one list position.
#[derive(Component)]
pub struct ExtraView;

/// Materials per (file, skin, blend, lit).
#[derive(Default)]
pub(super) struct Materials(pub(super) HashMap<(&'static str, u32, Blend), Handle<StandardMaterial>>);

/// Takes the game's events and advances the extras.
pub(super) fn update(
    time: Res<Time>,
    game: Res<Game>,
    mut events: MessageReader<GameEvent>,
    mut extras: ResMut<ExtrasState>,
    mut particles: ResMut<Particles>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
) {
    for GameEvent(e) in events.read() {
        extras.event(&game, e);
    }
    let view = cam.single().map_or(View::default(), |g| View::looking(coords::native(g.translation()), coords::native(g.forward().as_vec3())));
    extras.update(&game, view, Secs(time.delta_secs()), &mut particles);
}
