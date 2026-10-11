//! Decoration rules from mm1.wdl: `InitSkyMenu` (mm1.wdl:3379), `InitDecor`
//! (mm1.wdl:3461), `InitArbre` (mm1.wdl:1652-1724), `OnGround` (mm1.wdl:3911) and
//! `InitMdl` (mm1.wdl:4817-4868). They adjust the WED object and say how it is drawn.

use formats::wmb::{Object, entity_flags};

use crate::level::Presentation;

/// A sky dome kept centred on the camera horizontally at a fixed height offset, spinning
/// (pan/tilt/roll degrees per tick).
#[derive(Clone, Copy, Debug)]
pub struct SkyFollow {
    /// Z offset from the camera.
    pub dz: f32,
    pub spin: [f32; 3],
    pub angles: [f32; 3],
}

/// Default sky height (`-3300 + Skill3 × 10`).
const SKY_BASE: f32 = -3300.0;
const SKY_STEP: f32 = 10.0;
/// Default sky size when WED left the scale at 1.
const SKY_SCALE: f32 = 5.0;
const SKY_HEIGHT_SCALE: f32 = 3.0;

/// `InitSkyMenu`: returns the follow behaviour unless Flag1 ("ciel immobile") is set.
pub fn init_sky_menu(obj: &mut Object, pov_z: f32) -> (Presentation, Option<SkyFollow>) {
    if obj.scale[0] == 1.0 {
        obj.scale[0] = SKY_SCALE;
        obj.scale[1] = SKY_SCALE;
    }
    obj.scale[2] = if obj.skill(4) == 0.0 { SKY_HEIGHT_SCALE } else { obj.skill(4) };
    let mut follow = None;
    if !obj.flag(1) {
        obj.origin[2] = SKY_BASE + obj.skill(3) * SKY_STEP;
        // Original bug (docs/gameplay.md): tests Skill5/6/7 but spins by Skill2/3/4.
        let spin = [
            if obj.skill(5) != 0.0 { obj.skill(2) } else { 0.0 },
            if obj.skill(6) != 0.0 { obj.skill(3) } else { 0.0 },
            if obj.skill(7) != 0.0 { obj.skill(4) } else { 0.0 },
        ];
        follow = Some(SkyFollow { dz: obj.origin[2] - pov_z, spin, angles: obj.angles });
    }
    (Presentation { sky: true, ..Default::default() }, follow)
}

/// `InitDecor`: background decor at a fixed height (default -500), enlarged.
pub fn init_decor(obj: &mut Object) {
    obj.origin[2] = if obj.skill(3) == 0.0 { -500.0 } else { obj.skill(3) };
    if obj.scale[0] == 1.0 {
        obj.scale[0] = 1.75;
        obj.scale[1] = 1.75;
    }
    obj.scale[2] = if obj.skill(4) == 0.0 { 1.5 } else { obj.skill(4) };
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
pub fn init_arbre(obj: &mut Object) -> Presentation {
    let s = obj.skills.clone();
    let mut init = Presentation { ground: Some(-4.0), oriented: obj.angles != [0.0; 3], ..Default::default() };
    if s[6] != 0.0 {
        init.alpha = Some(s[6]);
    }
    if obj.flag(1) {
        init.facing = true;
        init.ground = Some(0.0);
    }
    if obj.flag(2) {
        init.oriented = true;
        init.ground = Some(0.0);
    }
    if obj.flag(4) {
        // Random X/Y scale in [0.5, 1.5); the height offset keeps the unscaled half height.
        obj.scale[0] *= 0.5 + spawn_random(obj, 1);
        obj.scale[1] *= 0.5 + spawn_random(obj, 2);
        init.ground = Some(-4.0);
    }
    init
}

/// `OnGround` (mm1.wdl:3911-3933).
pub fn on_ground(obj: &mut Object) -> Presentation {
    Presentation { ground: Some(-2.0), alpha: (obj.skill(1) != 0.0).then_some(obj.skill(1)), ..Default::default() }
}

/// `InitMdl` (mm1.wdl:4817-4868, the live definition): static decoration models.
pub fn init_mdl(obj: &mut Object) -> Presentation {
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
    Presentation {
        skin: (s[0] != 0.0).then_some(s[0] as u32),
        frame: Some(frame),
        overlay: obj.flag(1),
        solid_box: obj.flags & entity_flags::PASSABLE == 0,
        ..Default::default()
    }
}

