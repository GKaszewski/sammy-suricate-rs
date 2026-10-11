//! Building lions and wasp enemies from level objects.

use super::*;
use crate::percent::Percent;

/// `InitLion`: returns the enemy and its skin override.
pub fn init_lion(obj: &Object) -> (Enemy, Option<u32>) {
    let s = obj.skills.clone();
    let kind = s[0].trunc() as i32;
    let (class, spec, weight, head, def_res, skin) = match kind {
        10 => (LION, 1, 0.65, 109, 6.0, None),
        0 | 11 => (LION, 1, 0.62, 135, 9.0, None),
        12 => (LION, 1, 0.60, 132, 12.0, None),
        13 => (LION, 1, 0.55, 139, 15.0, None),
        14 => (LION, 1, 0.50, 209, 30.0, None),
        2 => (LION, 3, 0.75, 0, 12.0, None),
        3 => (LION, 3, 0.75, 0, 12.0, Some(2)),
        1 => (ELEPHANT, 2, 0.30, 0, 100.0, None),
        k => (k as u32, 0, 0.0, 0, 0.0, None),
    };
    let res = (frc(s[0]) * 100.0 + 0.5).trunc();
    let resistance = if res == 0.0 { def_res } else { res };
    let push = (frc(s[1]) * 100.0 + 0.5).trunc();
    let patrol = obj.flag(3);
    let (path_id, path_range, random_org, random_org2) = unpack_path(&s, patrol);
    let attack = if s[2] == 0.0 { 500.0 } else { s[2] };
    let react = if s[5] == 0.0 { 1000.0 } else { s[5] };
    (build_enemy(obj, &s, (class, spec, weight, head, resistance), push, attack, react, (path_id, path_range, random_org, random_org2)), skin)
}

#[allow(clippy::type_complexity)]
pub(super) fn build_enemy(
    obj: &Object,
    s: &[f32],
    (class, spec, weight, head, resistance): (u32, u32, f32, usize, f32),
    push: f32,
    attack: f32,
    react: f32,
    (path_id, path_range, random_org, random_org2): (f32, f32, f32, f32),
) -> Enemy {
    Enemy {
        class,
        spec,
        weight,
        head_vertex: head,
        resistance,
        resistance_org: resistance,
        speed: s[1],
        push_force: if push == 0.0 { 2.0 } else { push },
        attack_range: attack,
        attack_range_bis: (frc(s[2]) * 10000.0).trunc(),
        react_range: react,
        path_id,
        path_range,
        random_org,
        random_org2,
        accel_attack: frc(s[4]) * 10.0,
        time_for_nothing: s[7],
        flags: obj.flags,
        scale: obj.scale[2].abs(),
        pos: obj.origin,
        prev_pos: obj.origin,
        pan: obj.angles[0],
        prev_pan: obj.angles[0],
        tilt: 0.0,
        floor: obj.origin[2],
        hover: 0.0,
        initialised: false,
        etat: 0,
        etat_bis: 0,
        guard_stuck: 0.0,
        delta_pan: 0.0,
        dest_pan: obj.angles[0],
        ball_accel: 1.0,
        anim_step: 0.0,
        anim_speed: 3.0,
        random_time: random_org,
        random_time2: random_org2,
        target: [f32::MAX; 3],
        target_pan: 999.0,
        target_tilt: 0.0,
        time_without_patrol: 0.0,
        attacked: 0.0,
        bounce: [0.0; 3],
        vspeed: 0.0,
        fric_vspeed: 0.0,
        vertical: 0.0,
        vel: [0.0; 3],
        swipe_pending: false,
        dying: 0.0,
        muzzle: None,
        half: [16.0; 3],
        center_z: 0.0,
        enable_block: true,
        avu: false,
        circle_z: 8.0,
        dist: f32::MAX,
        floor_entity: None,
        teleport: None,
        alpha: Percent::FULL,
        passable: false,
        flying_init: false,
        wasp_spec: 0,
        hidden: false,
    }
}

/// Patrol path / random mode unpacking shared by lions and wasp enemies.
pub(super) fn unpack_path(s: &[f32], patrol: bool) -> (f32, f32, f32, f32) {
    if s[3] == 0.0 {
        (0.0, 0.0, 0.0, 0.0)
    } else if patrol {
        (s[3].trunc(), (frc(s[3]) * 10000.0 + 0.5).trunc(), 0.0, 0.0)
    } else {
        (0.0, 0.0, s[3].trunc(), (frc(s[3]) * 1000.0 + 0.5).trunc())
    }
}

/// `InitWaspEnnemy` (bullets.wdl:1502-1677) and `InitWaspExtro` (`:1849-1938`, `extro`):
/// a lion with the hive wasp's skin and scale (`initSkinScaleWaspHive2`, `:1411-1478`),
/// flying (Flag8) and passable. Types 0-7 become 8888; others (the "You Win" model, 99)
/// keep their id. Returns the skin.
/// Returns the enemy, its skin and its scale override.
pub fn init_wasp_ennemy(obj: &Object, extro: bool) -> (Enemy, Option<u32>, Option<f32>) {
    let mut s = obj.skills.clone();
    if extro {
        s[0] = 6.0;
    }
    let kind = s[0].trunc() as i32;
    // (scale, WaspPoison) by IdWaspSpec; the poison overwrites Skill3 = LionAttackRange.
    let (scale, poison) = match kind {
        0 | 1 => (None, 3.0),
        2 => (Some(1.0), 2.0),
        3 | 4 | 7 => (Some(0.8), 1.0),
        5 => (Some(1.0), 4.0),
        6 => (Some(1.0), 5.0),
        _ => (None, s[2]),
    };
    let skin = match kind {
        2..=7 => Some(kind as u32),
        _ => Some(s[6].max(1.0) as u32),
    };
    s[2] = poison;
    if s[5] == 0.0 {
        s[5] = 1.0;
    }
    let (class, weight, wasp_spec, def_res) = match (extro, kind) {
        (true, _) => (LION, 0.3, 101, 18.0),
        (_, 0 | 1) => (LION, 0.3, 101, 18.0),
        (_, 2) => (LION, 0.8, 102, 12.0),
        (_, 3) => (LION, 0.9, 103, 3.0),
        (_, 4) => (LION, 0.9, 104, 3.0),
        (_, 5) => (LION, 0.6, 105, 9.0),
        (_, 6) => (LION, 0.6, 106, 12.0),
        (_, 7) => (LION, 0.8, 107, 3.0),
        (_, k) => (k as u32, 0.0, k as u32, 0.0),
    };
    let res = if extro { 0.0 } else { (frc(s[0]) * 100.0 + 0.5).trunc() };
    let resistance = if res == 0.0 { def_res } else { res };
    let push = (frc(s[1]) * 100.0 + 0.5).trunc();
    let attack = if s[2] == 0.0 { 500.0 } else { s[2] };
    let react = if s[5] == 0.0 { 1000.0 } else { s[5] };
    let path = unpack_path(&s, obj.flag(3));
    let mut obj = obj.clone();
    if let Some(k) = scale {
        obj.scale = [k; 3];
    }
    let mut e = build_enemy(&obj, &s, (class, wasp_spec, weight, 0, resistance), push, attack, react, path);
    e.passable = true;
    e.wasp_spec = wasp_spec;
    e.flying_init = true;
    if extro {
        e.etat = 1;
        e.anim_speed = 3.0;
    }
    (e, skin, scale)
}
