//! Decoration actions from mm1.wdl: `InitSkyMenu` (mm1.wdl:3379) and `InitDecor` (mm1.wdl:3461).

use bevy::prelude::*;
use formats::wmb::Object;

use super::ActionInit;
use crate::coords;

/// Acknex ticks per second (`time` is measured in 1/16 s ticks).
const TICKS_PER_SEC: f32 = 16.0;

/// Keeps a sky dome centred on the camera horizontally at a fixed height offset
/// and spins it (pan/tilt/roll degrees per tick).
#[derive(Component)]
pub struct SkyFollow {
    /// Acknex Z offset from the camera (`skill11`).
    pub dz: f32,
    pub spin: [f32; 3],
    pub angles: [f32; 3],
}

pub fn init_sky_menu(obj: &mut Object, pov_z: f32, entity: &mut EntityCommands) -> ActionInit {
    let s = &obj.skills;
    if obj.scale[0] == 1.0 {
        obj.scale[0] = 5.0;
        obj.scale[1] = 5.0;
    }
    obj.scale[2] = if s[3] == 0.0 { 3.0 } else { s[3] };
    // Flag1 set = static sky ("ciel immobile"), left where the editor put it.
    if obj.flags & 1 == 0 {
        obj.origin[2] = -3300.0 + s[2] * 10.0;
        // Original bug (docs/gameplay.md): tests skill5/6/7 but spins by skill2/3/4.
        let spin = [
            if s[4] != 0.0 { s[1] } else { 0.0 },
            if s[5] != 0.0 { s[2] } else { 0.0 },
            if s[6] != 0.0 { s[3] } else { 0.0 },
        ];
        entity.insert(SkyFollow { dz: obj.origin[2] - pov_z, spin, angles: obj.angles });
    }
    ActionInit { sky: true, ..default() }
}

pub fn init_decor(obj: &mut Object) {
    let s = &obj.skills;
    obj.origin[2] = if s[2] == 0.0 { -500.0 } else { s[2] };
    if obj.scale[0] == 1.0 {
        obj.scale[0] = 1.75;
        obj.scale[1] = 1.75;
    }
    obj.scale[2] = if s[3] == 0.0 { 1.5 } else { s[3] };
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
