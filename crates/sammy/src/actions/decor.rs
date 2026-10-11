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

/// Deterministic stand-in for WDL `random(1)` at spawn, seeded by the editor origin.
fn spawn_random(obj: &Object, salt: u32) -> f32 {
    let mut h = obj.origin.iter().fold(0x9E37_79B9u32 ^ salt, |h, v| (h ^ v.to_bits()).wrapping_mul(0x0100_0193));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h >> 8) as f32 / (1 << 24) as f32
}

/// `InitArbre` (mm1.wdl:1652-1724): grass and tree sprites. Every shipped one gets the ×0.3
/// trunk hull, but sprite collision in A4 4.22 is unverified so none is added here.
pub fn init_arbre(obj: &mut Object) -> ActionInit {
    let flag = |n: u32| obj.flags & (1 << (n - 1)) != 0;
    let s = obj.skills.clone();
    let mut init = ActionInit { ground: Some(-4.0), oriented: obj.angles != [0.0; 3], ..default() };
    if s[6] != 0.0 {
        init.alpha = Some(s[6]);
    }
    if flag(1) {
        init.facing = true;
        init.ground = Some(0.0);
    }
    if flag(2) {
        init.oriented = true;
        init.ground = Some(0.0);
    }
    if flag(4) {
        // Random X/Y scale in [0.5, 1.5); the height offset keeps the unscaled half height.
        obj.scale[0] *= 0.5 + spawn_random(obj, 1);
        obj.scale[1] *= 0.5 + spawn_random(obj, 2);
        init.ground = Some(-4.0);
    }
    init
}

/// `OnGround` (mm1.wdl:3911-3933).
pub fn on_ground(obj: &mut Object) -> ActionInit {
    ActionInit { ground: Some(-2.0), alpha: (obj.skills[0] != 0.0).then_some(obj.skills[0]), ..default() }
}

/// `InitMdl` (mm1.wdl:4817-4868, the live definition): static decoration models.
pub fn init_mdl(obj: &mut Object) -> ActionInit {
    let s = obj.skills.clone();
    for i in 0..3 {
        if s[1 + i] != 0.0 {
            obj.scale[i] = s[1 + i];
        }
    }
    let frame = match (s[5], s[6]) {
        (0.0, _) => 1.0,
        (lo, 0.0) => lo,
        (lo, hi) => lo + spawn_random(obj, 3) * (hi - lo),
    };
    ActionInit {
        skin: (s[0] != 0.0).then_some(s[0] as u32),
        frame: Some(frame),
        overlay: obj.flags & 1 != 0,
        solid_box: obj.flags & formats::wmb::entity_flags::PASSABLE == 0,
        ..default()
    }
}

/// Snaps a sprite onto the floor below its editor origin once collision is loaded.
#[derive(Component)]
pub struct SpriteGround {
    pub origin: [f32; 3],
    /// Height offset above the floor (half sprite height + action offset).
    pub dz: f32,
}

pub fn snap_sprites(world: crate::collision::CollisionWorld, mut commands: Commands, mut q: Query<(Entity, &SpriteGround, &mut Transform)>) {
    if !world.ready() {
        return;
    }
    for (e, g, mut tf) in &mut q {
        let [x, y, z] = g.origin;
        let tr = world.trace_static(0, [x, y, z], [x, y, z - 2000.0]).trace;
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
