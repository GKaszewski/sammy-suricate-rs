//! Decoration ([`sammy_core::decor`]): sky domes that follow the camera and sprites
//! snapped onto the floor.

use bevy::prelude::*;
use formats::wmb::Object;

use super::ActionInit;
use crate::gameplay::collision::Hull;
use crate::render::coords;

pub use sammy_core::decor::{init_arbre, init_decor, init_mdl, on_ground};

/// Acknex ticks per second (`time` is measured in 1/16 s ticks).
const TICKS_PER_SEC: f32 = sammy_core::time::TICKS_PER_SEC;

/// A sky dome following the camera ([`sammy_core::decor::SkyFollow`]).
#[derive(Component, Deref, DerefMut)]
pub struct SkyFollow(sammy_core::decor::SkyFollow);

pub fn init_sky_menu(obj: &mut Object, pov_z: f32, entity: &mut EntityCommands) -> ActionInit {
    let (look, follow) = sammy_core::decor::init_sky_menu(obj, pov_z);
    if let Some(f) = follow {
        entity.insert(SkyFollow(f));
    }
    look
}

/// Snaps a sprite onto the floor below its editor origin once collision is loaded.
#[derive(Component)]
pub struct SpriteGround {
    pub origin: [f32; 3],
    /// Height offset above the floor (half sprite height + action offset).
    pub dz: f32,
}

pub fn snap_sprites(world: crate::gameplay::collision::CollisionWorld, mut commands: Commands, mut q: Query<(Entity, &SpriteGround, &mut Transform)>) {
    if !world.ready() {
        return;
    }
    for (e, g, mut tf) in &mut q {
        let [x, y, z] = g.origin;
        let tr = world.trace_static(Hull::Point, [x, y, z], [x, y, z - 2000.0]).trace;
        if tr.hit() && !tr.start_solid {
            tf.translation = coords::pos([x, y, tr.end[2] + g.dz]);
        }
        commands.entity(e).remove::<SpriteGround>();
    }
}

pub fn follow_sky(
    time: Res<Time>,
    cam: Query<&Transform, (With<crate::camera::MainCamera>, Without<SkyFollow>)>,
    mut q: Query<(&mut Transform, &mut SkyFollow)>,
) {
    let Ok(cam) = cam.single() else { return };
    let ticks = time.delta_secs() * TICKS_PER_SEC;
    for (mut tf, mut sky) in &mut q {
        for i in 0..3 {
            sky.angles[i] += sky.spin[i] * ticks;
        }
        tf.translation = cam.translation + Vec3::Y * sky.dz / coords::QUANTS_PER_UNIT;
        tf.rotation = coords::rotation(sky.angles);
    }
}
