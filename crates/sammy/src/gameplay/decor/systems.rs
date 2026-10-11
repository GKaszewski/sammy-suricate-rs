#[allow(unused_imports)]
use super::*;

/// Acknex ticks per second (`time` is measured in 1/16 s ticks).
pub(crate) const TICKS_PER_SEC: f32 = sammy_core::time::TICKS_PER_SEC;

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
