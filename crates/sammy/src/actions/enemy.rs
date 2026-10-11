//! Lions, scorpions and the elephant (`lion.wdl`, `anims.wdl:2-350`), ported from
//! docs/enemy-ai.md. The AI runs at a fixed 16 Hz with `time = 1` (the only safe
//! reading of the original, §2.9); models are interpolated between ticks.
//!
//! Ported: idle (0/4), walk/patrol (1), chase (2), roar (3), lethal hit (6), dying (7),
//! random mode (8/13), swipe (10), hit reaction (11), flee (12), path nodes, fear,
//! stuck roar, muzzle wall probe, wasp homing. Not yet: friend calls, Eye of Truth
//! rings, teleports/platforms/lava for enemies, respawn (no shipped lion has >1 life).

use bevy::prelude::*;
use formats::wmb::Object;

use super::player::{Player, PlayerTouched, State as PlayerState, TICKS};
use super::wasp::{Wasp, WaspHit};
use crate::audio::Sfx;
use crate::collision::{BoxCollider, CollisionWorld};
use crate::coords;
use crate::game::{GameState, Rng};
use crate::model::ModelAnimator;

/// Class ids (`IdEnnemy` after init).
const LION: u32 = 8888;
/// `Win.MDL` in the extro (`IdEnnemy == 99`): stand cycle at 6 %/tick, never a target.
const WIN_MODEL: u32 = 99;
const ELEPHANT: u32 = 1111;
/// Enemies collide with the fat hull.
const HULL: usize = 2;

/// Per-level tuning globals (`InitLevelglobal` values, §1.5), overridable by
/// `InitLionSkills1/2` and `InitGame6/7`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct LionGlobals {
    pub rotation_all: f32,
    pub rotation_near: f32,
    pub rotation_far: f32,
    pub delta_dest_pan: f32,
    pub love_patrol: f32,
    pub lion_force: f32,
    pub near_target_path: f32,
    pub far_target_path: f32,
    pub intensite_bounce: f32,
    pub amorti_bounce: f32,
    pub limit_time_without_patrol: f32,
    pub random_attack: f32,
    pub hargne: f32,
    pub lachete: f32,
    pub frightened: f32,
    pub dist_muzzle: f32,
    pub gravite: f32,
    pub acc_gravite: f32,
}

impl Default for LionGlobals {
    fn default() -> Self {
        Self {
            rotation_all: 0.2,
            rotation_near: 0.02,
            rotation_far: 0.2,
            delta_dest_pan: 5.0,
            love_patrol: 0.05,
            lion_force: 2.0,
            near_target_path: 100.0,
            far_target_path: 150.0,
            intensite_bounce: 15.0,
            amorti_bounce: 10.0,
            limit_time_without_patrol: 64.0,
            random_attack: 50.0,
            hargne: 2.0,
            lachete: 0.5,
            frightened: 0.05,
            dist_muzzle: 32.0,
            gravite: -4.0,
            acc_gravite: 3.0,
        }
    }
}

impl LionGlobals {
    /// mm1.wdl:1928-1962.
    pub fn apply_lion_skills1(&mut self, s: &[f32]) {
        let set = |v: &mut f32, x: f32| {
            if x != 0.0 {
                *v = x;
            }
        };
        set(&mut self.rotation_near, s[0]);
        set(&mut self.rotation_far, s[1]);
        set(&mut self.rotation_all, s[2]);
        set(&mut self.delta_dest_pan, s[3]);
        set(&mut self.love_patrol, s[4]);
        set(&mut self.lion_force, s[5]);
        set(&mut self.near_target_path, s[6]);
        set(&mut self.far_target_path, s[7]);
    }

    pub fn apply_lion_skills2(&mut self, s: &[f32]) {
        let set = |v: &mut f32, x: f32| {
            if x != 0.0 {
                *v = x;
            }
        };
        set(&mut self.intensite_bounce, s[0]);
        set(&mut self.amorti_bounce, s[1]);
        set(&mut self.limit_time_without_patrol, s[2]);
        set(&mut self.random_attack, s[3]);
    }

    /// `InitGame6` (mm1.wdl:1866-1889): Skill3-6.
    pub fn apply_game6(&mut self, s: &[f32]) {
        if s[2] != 0.0 {
            self.hargne = s[2];
        }
        if s[3] != 0.0 {
            self.lachete = s[3];
        }
        if s[4] != 0.0 {
            self.frightened = s[4];
        }
        if s[5] != 0.0 {
            self.dist_muzzle = if s[5] == 999.0 { 0.0 } else { s[5] };
        }
    }

    /// `InitGame7` Skill5: enemy gravity.
    pub fn apply_game7(&mut self, s: &[f32]) {
        if s[4] != 0.0 {
            self.gravite = s[4];
        }
    }
}

/// Friend calls (`CallFriendsToHelp`, lion.wdl:998-1070; §4.5). These globals are
/// not reset per level in the original (`AnswerFriends` never is).
#[derive(Resource, Debug)]
pub struct FriendCalls {
    /// `AttackCall`: ticks left of an active call.
    pub attack_call: f32,
    /// `FriendCall`: caller's `spec + weight`.
    friend_call: f32,
    /// `AnswerFriends`: 0 locks friend calls for the rest of the game.
    answer: i32,
    pub tempo_call: f32,
    pub range_call: f32,
}

impl FriendCalls {
    /// `InitGame6`: Skill7 = TempoCall; Skill8 ≠ 0 sets RangeCall to *Skill7* (original bug).
    pub fn apply_game6(&mut self, s: &[f32]) {
        if s[6] != 0.0 {
            self.tempo_call = s[6];
        }
        if s[7] != 0.0 {
            self.range_call = s[6];
        }
    }
}

impl Default for FriendCalls {
    fn default() -> Self {
        Self { attack_call: 0.0, friend_call: 0.0, answer: 1, tempo_call: 128.0, range_call: 10000.0 }
    }
}

/// A call ring spawned by a calling lion: (delay ticks, z offset, skin).
#[derive(Component)]
pub struct CallRing {
    pub delay: f32,
    pub scale: f32,
    pub frame: f32,
    pub origin: [f32; 3],
    pub skin: u32,
}

/// A lion started a chase: Sammy gets the orange warning light (`Warning_attack`).
#[derive(Message, Clone, Copy)]
pub struct WarningAttack;

/// Patrol path node (`InitPath`, lion.wdl:774-780).
#[derive(Component)]
pub struct PathNode {
    id: f32,
    next: f32,
    pos: [f32; 3],
    pan: f32,
    tilt: f32,
}

pub fn init_path(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(PathNode { id: obj.skills[0], next: obj.skills[1], pos: obj.origin, pan: obj.angles[0], tilt: obj.angles[1] });
}

#[derive(Component, Debug)]
pub struct Enemy {
    // --- configuration (§1.1-1.2)
    class: u32,
    /// 1 lion family, 2 elephant, 3 scorpion.
    spec: u32,
    weight: f32,
    head_vertex: usize,
    resistance: f32,
    resistance_org: f32,
    speed: f32,
    /// `BounceForce`: push multiplier given to whoever touches this enemy.
    push_force: f32,
    attack_range: f32,
    attack_range_bis: f32,
    react_range: f32,
    path_id: f32,
    path_range: f32,
    random_org: f32,
    random_org2: f32,
    accel_attack: f32,
    time_for_nothing: f32,
    flags: u32,
    scale: f32,
    // --- runtime (§1.4)
    pub pos: [f32; 3],
    prev_pos: [f32; 3],
    pub pan: f32,
    prev_pan: f32,
    tilt: f32,
    floor: f32,
    hover: f32,
    initialised: bool,
    etat: i32,
    etat_bis: i32,
    guard_stuck: f32,
    delta_pan: f32,
    dest_pan: f32,
    ball_accel: f32,
    anim_step: f32,
    anim_speed: f32,
    random_time: f32,
    random_time2: f32,
    target: [f32; 3],
    target_pan: f32,
    target_tilt: f32,
    time_without_patrol: f32,
    pub attacked: f32,
    bounce: [f32; 3],
    vspeed: f32,
    fric_vspeed: f32,
    vertical: f32,
    /// `EnnemySpeed`: last velocity in quants/tick.
    pub vel: [f32; 3],
    /// Swipe not yet landed (`BaffeDonne == -999`).
    swipe_pending: bool,
    /// Cumulative blink time in state 7.
    dying: f32,
    /// World position of the head vertex (wasps home on it).
    pub muzzle: Option<[f32; 3]>,
    /// Model-space bbox (native, scaled) for the collision box.
    half: [f32; 3],
    center_z: f32,
    enable_block: bool,
    /// Result of the last line-of-sight test (`EnnemyAvuPlayer`).
    avu: bool,
    /// Range-ring height offset (`CircleZ = 8 + random(12)`).
    pub circle_z: f32,
    /// Distance to Sammy from the last `TestDistLionPlayer`.
    pub dist: f32,
    /// Entity the gravity trace stands on (platform, teleport pad...).
    floor_entity: Option<Entity>,
    /// Teleport pad in use: (pad, destination, fading in?).
    teleport: Option<(Entity, [f32; 3], bool)>,
    /// Opacity in percent (teleport fades).
    pub alpha: f32,
    /// Editor/script `passable` (wasp enemies): never blocks movers.
    passable: bool,
    /// Wasp enemy: keeps its editor height instead of snapping to the floor.
    flying_init: bool,
    /// `IdWaspSpec` of wasp enemies (`InitWaspEnnemy`), 0 for ground enemies.
    pub wasp_spec: u32,
}

impl Enemy {
    /// One-line state for debug logs.
    pub fn debug_line(&self) -> String {
        format!(
            "etat {} pos {:?} pan {:.0} res {:.1} attack {:.0} react {:.0} fear {:.1}",
            self.etat,
            self.pos.map(|v| v.round()),
            self.pan,
            self.resistance,
            self.attack_range,
            self.react_range,
            self.attacked
        )
    }

    /// `DisplayEnnemyRange` (lion.wdl:1586-1653): (skin, frame %, radius) of the
    /// range ring while the Eye of Truth is active, or None when hidden.
    pub fn ring(&self, display_mode: f32) -> Option<(u32, f32, f32)> {
        if self.etat == 7 || self.class == 0 {
            return None;
        }
        let d = self.dist.max(1.0);
        let calm = |e: &Enemy| {
            let skin = if d < e.attack_range + (e.react_range - e.attack_range) * 0.5 { 2 } else { 1 };
            (skin, (100.0 / (e.attack_range / d)).min(99.0).max(60.0), e.attack_range)
        };
        if display_mode > 1.0 {
            return Some(if self.etat == 2 { self.chase_ring(d) } else { calm(self) });
        }
        if self.class != LION || !self.avu {
            return None;
        }
        if self.etat == 2 {
            return Some(self.chase_ring(d));
        }
        (self.etat == 1 || self.random_org2 != 0.0).then(|| calm(self))
    }

    fn chase_ring(&self, d: f32) -> (u32, f32, f32) {
        let mut radius = self.attack_range;
        let mut skin = 2;
        if self.avu {
            skin = 3;
            if self.attack_range < d {
                radius = d + 128.0;
            }
        }
        (skin, (100.0 / (radius / d)).min(99.0).max(55.0), radius)
    }

    pub fn alive(&self) -> bool {
        !matches!(self.etat, 6 | 7) && (self.class == LION || self.class == ELEPHANT)
    }

    /// `IdEnnemy`.
    pub fn class(&self) -> u32 {
        self.class
    }

    /// `IdEnnemySpec` (1-3 ground enemies, 101-107 wasps).
    pub fn spec_id(&self) -> u32 {
        self.spec
    }

    /// Flag5: scans id 25 every tick (camera zones, credit titles).
    pub fn scans_25(&self) -> bool {
        self.flag(5) && self.initialised && !self.gone()
    }

    pub fn is_class_target(&self) -> bool {
        self.class == LION || self.class == ELEPHANT
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn floor(&self) -> f32 {
        self.floor
    }

    /// Dead and gone (blinked out): no shadow, no ring.
    pub fn gone(&self) -> bool {
        self.class == 0 || (self.etat == 7 && self.dying >= 33.0)
    }

    fn flag(&self, n: u32) -> bool {
        self.flags & (1 << (n - 1)) != 0
    }

    fn walk_speed(&self) -> f32 {
        0.82 * self.speed * if self.spec == 3 { 3.0 } else { 1.0 }
    }

    fn run_speed(&self) -> f32 {
        0.3 * self.speed * self.accel_attack
    }

    fn set_state(&mut self, etat: i32, anim_speed: f32) {
        self.etat = etat;
        self.anim_step = 0.0;
        self.anim_speed = anim_speed;
    }
}

/// f32 `frc()` (§1.1: unpack with f32 maths to reproduce the shipped data).
fn frc(x: f32) -> f32 {
    x - x.trunc()
}

fn ang(a: f32) -> f32 {
    let a = (a + 180.0).rem_euclid(360.0) - 180.0;
    if a == -180.0 { 180.0 } else { a }
}

/// `To_Angle`: (pan, tilt, length) of a vector.
fn to_angle(v: [f32; 3]) -> (f32, f32, f32) {
    let h = (v[0] * v[0] + v[1] * v[1]).sqrt();
    (v[1].atan2(v[0]).to_degrees(), v[2].atan2(h).to_degrees(), (h * h + v[2] * v[2]).sqrt())
}

fn forward(pan: f32, tilt: f32) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

pub fn init_lion(obj: &mut Object, entity: &mut EntityCommands) -> Option<u32> {
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
    let patrol = obj.flags & (1 << 2) != 0;
    let (path_id, path_range, random_org, random_org2) = unpack_path(&s, patrol);
    let attack = if s[2] == 0.0 { 500.0 } else { s[2] };
    let react = if s[5] == 0.0 { 1000.0 } else { s[5] };
    entity.insert(build_enemy(obj, &s, (class, spec, weight, head, resistance), push, attack, react, (path_id, path_range, random_org, random_org2)));
    skin
}

#[allow(clippy::type_complexity)]
fn build_enemy(
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
        alpha: 100.0,
        passable: false,
        flying_init: false,
        wasp_spec: 0,
    }
}

/// Patrol path / random mode unpacking shared by lions and wasp enemies.
fn unpack_path(s: &[f32], patrol: bool) -> (f32, f32, f32, f32) {
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
pub fn init_wasp_ennemy(obj: &mut Object, entity: &mut EntityCommands, extro: bool) -> Option<u32> {
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
    if let Some(k) = scale {
        obj.scale = [k; 3];
    }
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
    let path = unpack_path(&s, obj.flags & (1 << 2) != 0);
    let mut e = build_enemy(obj, &s, (class, wasp_spec, weight, 0, resistance), push, attack, react, path);
    e.passable = true;
    e.wasp_spec = wasp_spec;
    e.flying_init = true;
    if extro {
        e.etat = 1;
        e.anim_speed = 3.0;
    }
    entity.insert(e);
    skin
}

/// 16 Hz clock for the enemy AI.
#[derive(Resource, Default)]
pub struct AiClock {
    acc: f32,
    /// Ticks to run this frame.
    pub ticks: u32,
    /// `ticks1`: frame counter wrapping every 64 (mm1.wdl:893-903).
    pub ticks1: u32,
}

pub fn advance_clock(time: Res<Time>, current: Res<crate::flow::CurrentLevel>, mut clock: ResMut<AiClock>) {
    // Enemies wait for `PartieEncours == 1` (lion.wdl:143-148).
    if !crate::flow::playing(&current) {
        clock.ticks = 0;
        return;
    }
    clock.acc += time.delta_secs() * TICKS;
    clock.ticks = (clock.acc.floor() as u32).min(4);
    clock.acc -= clock.ticks as f32;
    clock.acc = clock.acc.min(1.0);
}

/// Scan cone test (§2.5): within `range`, ±h/2 of `pan`, ±v/2 of `tilt`.
fn in_cone(from: [f32; 3], pan: f32, tilt: f32, h: f32, v: f32, range: f32, p: [f32; 3]) -> bool {
    let (dp, dt, d) = to_angle([p[0] - from[0], p[1] - from[1], p[2] - from[2]]);
    d <= range && ang(dp - pan).abs() <= h / 2.0 && ang(dt - tilt).abs() <= v / 2.0
}

/// Context shared by one enemy's tick.
struct Ctx<'a, 'w1, 's1, 'w2, 's2> {
    g: &'a LionGlobals,
    rng: &'a mut Rng,
    sfx: &'a mut Sfx<'w1, 's1>,
    world: &'a CollisionWorld<'w2, 's2>,
    sammy: Option<(Entity, [f32; 3], [f32; 3])>,
    ticks1: u32,
    me: Entity,
    calls: &'a mut FriendCalls,
    warn: &'a mut bool,
    /// Call rings to spawn: (position, delay).
    call_rings: &'a mut Vec<[f32; 3]>,
}

#[allow(clippy::too_many_arguments)]
pub fn tick_enemies(
    clock: Res<AiClock>,
    globals: Res<LionGlobals>,
    mut rng: ResMut<Rng>,
    mut sfx: Sfx,
    world: CollisionWorld,
    nodes: Query<&PathNode>,
    mut game: ResMut<GameState>,
    mut player: Query<(Entity, &mut Player)>,
    mut touches: MessageReader<PlayerTouched>,
    mut wasp_hits: MessageReader<WaspHit>,
    mut enemies: Query<(Entity, &mut Enemy, &mut ModelAnimator, &mut Visibility)>,
    mut calls: ResMut<FriendCalls>,
    mut warnings: MessageWriter<WarningAttack>,
    mut commands: Commands,
    (platforms, mut pads, pov): (Query<&super::platform::Platform>, Query<&mut super::platform::Teleport>, Res<super::extro::Pov>),
) {
    let mut warn = false;
    let mut call_rings = Vec::new();
    let touched: Vec<Entity> = touches.read().map(|t| t.0).collect();
    let hits: Vec<WaspHit> = wasp_hits.read().copied().collect();
    // Distances and line of sight are measured to `pov_Syn` (lion.wdl:948-950).
    let sammy = pov.entity.map(|e| (e, pov.pos, pov.vel));
    let sammy_entity = player.single().ok().map(|(e, _)| e);
    let sammy_alive = player.single().is_ok_and(|(_, p)| p.state != PlayerState::Dead);
    let nodes: Vec<&PathNode> = nodes.iter().collect();

    // Wasp hits (EVENT_IMPACT on the lion, §5.1) are applied as they arrive.
    for h in &hits {
        if let Ok((_, mut e, ..)) = enemies.get_mut(h.target) {
            wasp_hit(&mut e, h, &globals, &mut rng);
        }
    }

    for _ in 0..clock.ticks {
        for (me, mut e, mut anim, mut vis) in &mut enemies {
            if !e.initialised {
                init_runtime(&mut e, &anim, &world, me, &mut rng);
            }
            e.prev_pos = e.pos;
            e.prev_pan = e.pan;
            // Teleport pads with Flag2 (mm1.wdl:4236-4285): fade out −2/tick, jump, fade in
            // +1.5/tick, forced idle and intangible throughout.
            if let Some((pad, dest, fading_in)) = e.teleport {
                e.etat = 0;
                if !fading_in {
                    e.alpha -= 2.0;
                    if e.alpha <= 0.0 {
                        e.alpha = 0.0;
                        e.pos = dest;
                        e.prev_pos = dest;
                        e.teleport = Some((pad, dest, true));
                    }
                } else {
                    e.alpha += 1.5;
                    if e.alpha >= 100.0 {
                        e.alpha = 100.0;
                        e.teleport = None;
                        if let Ok(mut p) = pads.get_mut(pad) {
                            p.busy = false;
                        }
                    }
                }
                continue;
            }
            let grounded = e.vspeed == 0.0 && e.pos[2] <= e.floor + e.hover + 41.0;
            if grounded && e.alive() && let Some(fe) = e.floor_entity {
                if let Ok(mut pad) = pads.get_mut(fe)
                    && pad.lions
                    && !pad.busy
                {
                    pad.busy = true;
                    e.teleport = Some((fe, pad.dest, false));
                    continue;
                }
                // Moving platforms carry the lion (`LionForce` = platform motion).
                if let Ok(pl) = platforms.get(fe) {
                    for i in 0..3 {
                        e.pos[i] += pl.delta[i];
                    }
                }
            }
            // Contacts with Sammy (either mover): the swipe rules (§4.1).
            let touching = sammy_entity.is_some_and(|s| touched.contains(&me) || e_touches(&world, &e, me, s));
            if touching && sammy_alive && e.class == LION {
                if let Ok((_, mut p)) = player.single_mut() {
                    contact_sammy(&mut e, &mut p, &mut game, &globals, &mut sfx, &mut rng);
                }
            }
            {
                let mut ctx = Ctx {
                    g: &globals,
                    rng: &mut rng,
                    sfx: &mut sfx,
                    world: &world,
                    sammy,
                    ticks1: clock.ticks1,
                    me,
                    calls: &mut calls,
                    warn: &mut warn,
                    call_rings: &mut call_rings,
                };
                enemy_behavior(&mut e, &mut ctx, &nodes, &anim);
                anim_lion(&mut e, &mut ctx, &mut vis);
            }
            sync_anim(&e, &mut anim);
        }
        // `AttackCall -= time` while a call is running.
        calls.attack_call = (calls.attack_call - 1.0).max(0.0);
    }
    if warn {
        warnings.write(WarningAttack);
    }
    // Three expanding rings per call: z+50 (skin 1), +85 after 8 ticks (skin 3), +150 after 18 more (skin 1).
    for p in call_rings {
        for (dz, delay, skin) in [(50.0, 0.0, 1), (85.0, 8.0, 3), (150.0, 26.0, 1)] {
            commands.spawn((CallRing { delay, scale: 0.01, frame: 1.0, origin: [p[0], p[1], p[2] + dz], skin }, crate::flow::LevelScoped));
        }
    }
}

/// Lion moving into Sammy is detected by a tiny probe along its last velocity.
fn e_touches(world: &CollisionWorld, e: &Enemy, me: Entity, sammy: Entity) -> bool {
    let v = e.vel;
    if v[0].abs() + v[1].abs() < 0.01 {
        return false;
    }
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    let end = [e.pos[0] + v[0] / len * 4.0, e.pos[1] + v[1] / len * 4.0, e.pos[2]];
    world.trace(HULL, e.pos, end, Some(me)).entity == Some(sammy)
}

fn init_runtime(e: &mut Enemy, anim: &ModelAnimator, world: &CollisionWorld, me: Entity, rng: &mut Rng) {
    e.initialised = true;
    e.circle_z = 8.0 + rng.random(12.0);
    // `Sonar my,2000`, then hover at 28×scale (15×scale for scorpions) (lion.wdl:151-173).
    let tr = world.trace(0, e.pos, [e.pos[0], e.pos[1], e.pos[2] - 2000.0], Some(me)).trace;
    if tr.hit() {
        e.floor = tr.end[2];
    }
    let h = if e.flying_init { e.pos[2] - e.floor } else if e.spec == 3 { 15.0 * e.scale } else { 28.0 * e.scale };
    e.pos[2] = e.floor + h;
    e.prev_pos = e.pos;
    e.hover = h;
    // Collision box from the model's first frame.
    if let Some(f) = anim.model.mdl.frames.first() {
        let (mn, mx) = (f.bbox_min, f.bbox_max);
        let hx = (mx[0] - mn[0]).max(mx[1] - mn[1]) * 0.5 * e.scale;
        e.half = [hx * 0.7, hx * 0.7, (mx[2] - mn[2]) * 0.5 * e.scale];
        e.center_z = (mx[2] + mn[2]) * 0.5 * e.scale;
    }
}

/// `TestDistLionPlayer` (§2.6): returns (pan, tilt, dist) to Sammy.
fn test_dist(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, g: &LionGlobals) -> (f32, f32, f32) {
    let Some((_, sp, _)) = ctx.sammy else { return (0.0, 0.0, f32::MAX) };
    if e.etat == 12 {
        return (0.0, 0.0, 1.0);
    }
    let mut d = to_angle([sp[0] - e.pos[0], sp[1] - e.pos[1], sp[2] + 15.0 - e.pos[2]]);
    e.dist = d.2;
    if ctx.calls.attack_call != 0.0 {
        // Answering a friend call: same species+subtype within RangeCall (exact float compare).
        if e.spec as f32 + e.weight == ctx.calls.friend_call && d.2 < ctx.calls.range_call && d.2 > 128.0 {
            if matches!(e.etat, 0 | 1 | 4 | 8 | 9) {
                if d.2 > e.attack_range {
                    ctx.calls.answer += 1;
                }
                e.etat_bis = 0;
                e.dest_pan = d.0;
                let r = e.run_speed();
                e.set_state(2, r);
            }
            d.2 = 64.0;
        }
        return d; // during a call nobody reverts to idle
    }
    if d.2 > e.react_range * 1.1 && (e.etat == 1 || e.etat == 2) {
        e.etat = if ctx.rng.random(100.0) > g.random_attack { 4 } else { 0 };
        e.anim_speed = 3.0;
        e.anim_step = 0.0;
    }
    d
}

/// `LionShootPlayer` (§2.6): line of sight from the lion origin to Sammy.
fn sees_sammy(e: &mut Enemy, ctx: &Ctx<'_, '_, '_, '_, '_>) -> bool {
    let Some((s, sp, _)) = ctx.sammy else { return false };
    let hit = ctx.world.trace(0, e.pos, [sp[0], sp[1], sp[2] + 15.0], Some(ctx.me));
    let seen = !hit.trace.hit() || hit.entity == Some(s);
    if hit.trace.hit() {
        e.avu = seen;
    }
    seen
}

fn second_react_range(e: &mut Enemy) {
    let v = (frc(e.react_range) * 10000.0 + 0.5).trunc();
    if v != 0.0 {
        e.react_range = v;
    }
}

fn take_attack_bis(e: &mut Enemy) {
    if e.attack_range_bis != 0.0 {
        e.attack_range = e.attack_range_bis;
        e.attack_range_bis = 0.0;
    }
}

/// `EnnemyBehavior` (§2.2), one tick.
fn enemy_behavior(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, nodes: &[&PathNode], anim: &ModelAnimator) {
    let g = *ctx.g;
    if e.etat == 7 {
        return;
    }
    if e.etat == 6 || e.etat == 3 {
        handle_speed(e, ctx, anim);
        e.ball_accel += (1.0 - e.ball_accel) * 0.1;
        return;
    }
    if e.etat == 8 || e.etat == 9 {
        let d = test_dist(e, ctx, &g);
        if d.2 < e.attack_range && e.anim_step > 50.0 && sees_sammy(e, ctx) {
            take_attack_bis(e);
            e.guard_stuck = 0.0;
            let w = e.walk_speed();
            e.set_state(1, w);
        }
        e.guard_stuck = 6.0;
        handle_speed(e, ctx, anim);
        e.random_time -= 1.0;
        randomize(e, ctx);
        e.guard_stuck -= 1.0;
        return;
    }
    if e.etat == 0 || e.etat == 4 {
        let d = test_dist(e, ctx, &g);
        e.guard_stuck = 12.0;
        if d.2 < e.react_range {
            // React by distance only, no line of sight (lion.wdl:1760).
            second_react_range(e);
            e.guard_stuck = 0.0;
            let w = e.walk_speed();
            e.set_state(1, w);
        } else {
            gravity(e, ctx);
            move_enemy(e, ctx, [0.0, 0.0, e.vertical]);
            e.guard_stuck -= 1.0;
        }
        return;
    }

    // Active states: 1, 2, 10, 11, 12, 13.
    let mut rotation = g.rotation_all;
    if e.etat == 1 && e.flag(3) {
        rotation = patrol(e, ctx, nodes, &g).unwrap_or(rotation);
    }
    handle_speed(e, ctx, anim);

    if !e.flag(4) {
        let d = test_dist(e, ctx, &g);
        let period = match (e.spec == 3, e.etat == 2) {
            (false, false) => 10,
            (false, true) => 4,
            (true, false) => 16,
            (true, true) => 8,
        };
        let vu = ctx.ticks1.is_multiple_of(period) && sees_sammy(e, ctx);
        if vu && ((e.etat == 1 && ctx.rng.random(100.0) > g.random_attack) || e.etat == 9) {
            let random_mode = e.random_org2 != 0.0;
            if d.2 <= e.attack_range && d.1.abs() < 45.0 && ((e.random_time <= 0.0 && !random_mode) || random_mode) {
                if e.spec == 1 && ctx.rng.random(100.0) > g.random_attack {
                    ctx.sfx.play("LionAttack1_son");
                }
                if e.spec == 3 && ctx.rng.random(100.0) > g.random_attack {
                    ctx.sfx.play("ScorpionAttack_son");
                }
                *ctx.warn = true; // Warning_Attack
                e.etat_bis = 0;
                e.dest_pan = d.0;
                let r = e.run_speed();
                e.set_state(2, r);
                take_attack_bis(e);
            }
        }
        if e.etat == 2 || e.etat == 10 {
            if d.2 > e.attack_range + 128.0 {
                let w = e.walk_speed();
                e.set_state(1, w);
                if e.spec == 3 {
                    ctx.sfx.play("ScorpionWalk_son");
                }
            } else {
                e.ball_accel = e.accel_attack;
                if e.random_time <= 0.0 && e.guard_stuck < 9.0 {
                    e.dest_pan = d.0; // homing
                }
            }
            if d.2 < 16.0 {
                e.dest_pan = ang(d.0 - 180.0);
                e.random_time = e.time_for_nothing * 2.0;
            }
        } else {
            if e.etat == 13 {
                e.ball_accel = e.accel_attack;
                if e.guard_stuck > 9.0 {
                    e.dest_pan = ang(e.dest_pan + ctx.rng.random(20.0) - 10.0);
                }
            }
            if !e.flag(3) && e.random_org2 != 0.0 {
                randomize(e, ctx);
            }
        }
        // Fear (§2.2 :2128).
        if e.attacked > 0.0 {
            let v = (e.resistance * e.weight * g.lachete).abs();
            e.attacked -= g.frightened * v;
            if e.attacked > v {
                if e.etat <= 1 {
                    e.etat_bis = 0;
                    e.dest_pan = ctx.rng.random(360.0);
                    let r = e.run_speed();
                    e.set_state(12, r);
                    e.random_time = e.time_for_nothing;
                }
            } else if e.etat == 12 {
                let w = e.walk_speed();
                e.set_state(1, w);
            }
        }
        if e.etat == 12 {
            e.ball_accel = e.accel_attack;
        }
    }

    // Suite0 (:2172): rotation toward the destination heading.
    e.random_time -= 1.0;
    e.delta_pan = ang(e.dest_pan - e.pan) * rotation;
    if e.flag(8) {
        // Flyers turn anywhere and pitch toward the patrol target height (:2176-2198).
        e.pan = ang(e.pan + e.delta_pan);
        if e.flag(3) && e.target[2] != f32::MAX {
            let tz = e.target[2];
            if (e.pos[2] - tz).abs() < 16.0 {
                e.tilt -= e.tilt * 0.25;
            } else if e.pos[2] > tz {
                e.tilt -= rotation;
            } else {
                e.tilt += rotation;
            }
        }
    } else if e.pos[2] - e.floor < 128.0 {
        e.pan = ang(e.pan + e.delta_pan);
    }
    e.enable_block = e.delta_pan.abs() <= 6.0;
    if e.guard_stuck > 0.0 {
        e.guard_stuck -= 0.25;
        if e.guard_stuck > 16.0 {
            if !e.flag(3) {
                e.dest_pan = ang(e.pan - 180.0);
            } else {
                e.etat_bis = 0;
            }
            e.guard_stuck = 0.0;
        }
    }
    e.ball_accel += (1.0 - e.ball_accel) * 0.1;
}

/// Patrol step (§2.5). Returns the rotation rate override.
fn patrol(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, nodes: &[&PathNode], g: &LionGlobals) -> Option<f32> {
    let scan = |e: &mut Enemy, from: [f32; 3], pan: f32, tilt: f32, h: f32, range: f32| -> bool {
        // PathScanned: the first node with level line of sight wins; failing that,
        // the last node scanned still overwrites the target.
        let old = e.target;
        let mut accepted = false;
        let path_id = e.path_id;
        for n in nodes.iter().filter(|n| n.id == path_id) {
            if accepted || (n.pos[0] == e.target[0] && n.pos[1] == e.target[1]) {
                continue;
            }
            if !in_cone(from, pan, tilt, h, 120.0, range, n.pos) {
                continue;
            }
            if n.next != 0.0 {
                e.path_id = n.next;
            }
            e.target = n.pos;
            e.target_pan = n.pan;
            e.target_tilt = ang(n.tilt);
            let tr = ctx.world.trace_static(0, n.pos, e.pos).trace;
            if !tr.hit() {
                accepted = true;
            }
        }
        e.target[0] != old[0] || e.target[1] != old[1]
    };

    e.time_without_patrol += 1.0;
    if e.time_without_patrol > g.limit_time_without_patrol {
        e.etat_bis = 0;
    }
    if e.etat_bis == 0 {
        let (pos, pan, tilt, range) = (e.pos, e.pan, e.tilt, e.path_range);
        if scan(e, pos, pan, tilt, 90.0, range) {
            e.etat_bis = 1;
            e.time_without_patrol = 0.0;
        } else {
            let d = to_angle([e.target[0] - e.pos[0], e.target[1] - e.pos[1], 0.0]);
            if e.target[0] != f32::MAX && d.2 > range * 1.8 {
                e.etat_bis = 0;
                if !scan(e, pos, pan, tilt, 100.0, range * 4.0) {
                    scan(e, pos, ang(pan - 180.0), 0.0, 100.0, range * 3.0);
                    e.time_without_patrol = 0.0;
                }
            }
        }
        return None;
    }
    let d = to_angle([e.target[0] - e.pos[0], e.target[1] - e.pos[1], 0.0]);
    let mut v = e.dest_pan;
    let mut rot = None;
    let pull = (g.love_patrol * 0.5).min(1.0);
    if d.2 < g.far_target_path && (e.etat_bis == 1 || e.etat_bis == 2) {
        v = e.target_pan;
        rot = Some(g.rotation_near);
        e.etat_bis = 2;
        e.bounce[0] = (e.target[0] - e.pos[0]) * pull;
        e.bounce[1] = (e.target[1] - e.pos[1]) * pull;
        e.tilt += ang((e.target_tilt - e.tilt) * 0.5);
    }
    if d.2 >= g.far_target_path && e.etat_bis == 1 {
        v = d.0;
        rot = Some(g.rotation_far);
    }
    if d.2 > e.path_range && e.etat_bis == 2 {
        e.etat_bis = 1;
        v = d.0;
        rot = Some(g.rotation_far);
    }
    e.dest_pan = if e.random_time <= 0.0 { ang(v) } else { ang(e.dest_pan + (v - e.dest_pan) * g.love_patrol) };
    if d.2 < g.near_target_path {
        e.etat_bis = 0;
        let (tp, tt, range) = (e.target, (e.target_pan, e.target_tilt), e.path_range);
        if scan(e, tp, tt.0, tt.1, 30.0, range) {
            e.etat_bis = 1;
            e.time_without_patrol = 0.0;
        }
    }
    rot
}

/// `RandomizeLion` (§2.7).
fn randomize(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>) {
    if e.random_time > 0.0 {
        return;
    }
    if e.random_time2 > 0.0 {
        e.random_time2 -= 1.0;
        return;
    }
    let v = ctx.rng.random(3.0) as i32 + 1;
    if v == 1 && e.etat != 8 {
        e.set_state(8, 3.0);
        e.random_time = e.random_org * 0.5;
        e.random_time2 = ctx.rng.random(e.random_org2) * 0.5;
        return;
    }
    if v == 2 {
        e.dest_pan = ctx.rng.random(360.0);
        if e.etat != 1 {
            e.anim_step = 0.0;
        }
        e.etat = 1;
        e.anim_speed = e.walk_speed();
        e.random_time = e.random_org;
        e.random_time2 = ctx.rng.random(e.random_org2) * 0.5;
        return;
    }
    if e.etat != 2 {
        e.anim_step = 0.0;
    }
    e.dest_pan = ctx.rng.random(360.0);
    e.etat = 13;
    e.anim_speed = e.run_speed();
    e.random_time = e.random_org;
    e.random_time2 = ctx.rng.random(e.random_org2);
}

/// `HandleGravityEnnemy` (§2.4): hover at a constant height over the traced floor.
fn gravity(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>) {
    let g = ctx.g;
    let hit = ctx.world.trace(0, e.pos, [e.pos[0], e.pos[1], e.pos[2] - 2000.0], Some(ctx.me));
    e.floor_entity = hit.entity.filter(|_| hit.trace.hit());
    if hit.trace.hit() {
        e.floor = hit.trace.end[2];
        e.vertical = 0.0;
    } else {
        e.vertical = 0.0;
        e.vspeed = 0.0;
    }
    if e.flag(8) {
        // Flyers: no gravity, only kept 64 above the floor (lion.wdl:1394-1403).
        if e.pos[2] < e.floor + 64.0 {
            e.vertical = (e.floor + 64.0) - e.pos[2];
            e.vspeed = 0.0;
        }
        return;
    }
    if e.vspeed == 0.0 {
        if e.pos[2] - e.floor > e.hover + 0.5 {
            let up = ctx.sammy.map_or(0.0, |(_, sp, _)| sp[2] - e.pos[2]);
            if e.pos[2] - e.floor > 200.0 && up > 100.0 && up < 400.0 {
                e.vspeed = 20.0;
                e.fric_vspeed = 3.0;
            } else {
                e.vspeed = g.gravite;
                e.fric_vspeed = g.acc_gravite;
            }
        } else {
            e.vertical = (e.floor + e.hover) - e.pos[2];
        }
    }
    if e.vspeed != 0.0 {
        e.vspeed -= e.fric_vspeed;
        if e.pos[2] < e.floor + e.hover {
            e.vertical = (e.floor + e.hover) - e.pos[2];
            e.vspeed = 0.0;
        } else {
            e.vertical = e.vspeed;
        }
    }
}

/// MOVE with glide; returns (moved distance, wall normal if a wall was hit, entity hit).
fn move_enemy(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, mut delta: [f32; 3]) -> (f32, Option<[f32; 3]>, Option<Entity>) {
    let start = e.pos;
    let mut wall = None;
    let mut ent = None;
    for _ in 0..3 {
        if delta.iter().all(|d| d.abs() < 1e-4) {
            break;
        }
        let end = [0, 1, 2].map(|i| e.pos[i] + delta[i]);
        let hit = ctx.world.trace(HULL, e.pos, end, Some(ctx.me));
        if hit.trace.start_solid {
            break;
        }
        e.pos = hit.trace.end;
        if !hit.trace.hit() {
            break;
        }
        if hit.entity.is_some() {
            ent = hit.entity;
        } else if hit.trace.normal[2].abs() < 0.7 {
            wall = Some(hit.trace.normal);
        }
        let rest = delta.map(|d| d * (1.0 - hit.trace.fraction));
        let n = hit.trace.normal;
        let into = rest[0] * n[0] + rest[1] * n[1] + rest[2] * n[2];
        delta = [0, 1, 2].map(|i| rest[i] - n[i] * into);
    }
    let moved = (0..3).map(|i| (e.pos[i] - start[i]).powi(2)).sum::<f32>().sqrt();
    (moved, wall, ent)
}

/// `HandleSpeedLion` (§2.3).
fn handle_speed(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, anim: &ModelAnimator) {
    let mut fwd = 0.0;
    if e.etat == 1 && e.delta_pan.abs() < 10.0 {
        fwd = e.speed * e.ball_accel;
    }
    if e.etat == 2 || e.etat == 13 {
        fwd = e.speed * e.ball_accel;
    }
    if e.etat == 10 {
        fwd = e.speed * e.ball_accel * 1.5;
    }
    if e.etat == 12 {
        fwd = e.speed * e.ball_accel * 1.2;
    }
    gravity(e, ctx);
    let dir = forward(e.pan, e.tilt);
    let before = e.pos;
    let (moved, wall, ent) = move_enemy(e, ctx, [dir[0] * fwd, dir[1] * fwd, dir[2] * fwd + e.vertical]);
    e.vel = if moved == 0.0 {
        let (s, c) = e.pan.to_radians().sin_cos();
        [c * 0.1, s * 0.1, e.vel[2]]
    } else {
        [0, 1, 2].map(|i| e.pos[i] - before[i])
    };
    if let Some(n) = wall
        && e.enable_block
    {
        on_block(e, ctx, n);
    }
    if let Some(other) = ent {
        on_entity(e, ctx, other);
    }

    // Muzzle wall probe (:1122-1162): 32 quants ahead of the head vertex.
    e.muzzle = None;
    if e.head_vertex != 0 && ctx.g.dist_muzzle != 0.0 {
        if let Some(head) = head_pos(e, anim) {
            e.muzzle = Some(head);
            let f = forward(e.pan, e.tilt);
            let tip = [0, 1, 2].map(|i| head[i] + f[i] * ctx.g.dist_muzzle);
            let tr = ctx.world.trace_static(0, head, tip);
            if tr.trace.start_solid {
                let a = if e.etat == 6 { e.pan } else { e.pan - 180.0 };
                let (s, c) = a.to_radians().sin_cos();
                e.bounce[0] = c * ctx.g.intensite_bounce;
                e.bounce[1] = s * ctx.g.intensite_bounce;
            } else if tr.trace.hit() && tr.entity.is_none() {
                on_block(e, ctx, tr.trace.normal);
            }
        }
    }

    // Knockback channel (:1164-1204), decaying linearly (AverageFps reference 50).
    if e.bounce != [0.0; 3] {
        let b = e.bounce;
        move_enemy(e, ctx, b);
        let dec = ctx.g.amorti_bounce * 80.0 / 51.0;
        e.bounce = e.bounce.map(|v| if v.abs() <= dec { 0.0 } else { v - dec * v.signum() });
    }
}

/// Head vertex in world space (current frame).
fn head_pos(e: &Enemy, anim: &ModelAnimator) -> Option<[f32; 3]> {
    let frame = anim.current_frame()?;
    let v = anim.model.mdl.frames.get(frame)?.positions.get(e.head_vertex)?;
    let (s, c) = e.pan.to_radians().sin_cos();
    let (x, y, z) = (v[0] * e.scale, v[1] * e.scale, v[2] * e.scale);
    Some([e.pos[0] + x * c - y * s, e.pos[1] + x * s + y * c, e.pos[2] + z])
}

/// `EnnemyOnBlock` (§2.8): bounce off the wall and re-steer 25-45° off its normal.
fn on_block(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, n: [f32; 3]) {
    let g = *ctx.g;
    if stuck_roar(e, ctx) {
        return;
    }
    e.guard_stuck += 3.0;
    let mut v = (e.vel[0].abs() + e.vel[1].abs()) * 0.1 * e.weight * g.intensite_bounce;
    if v == 0.0 {
        v = g.intensite_bounce;
    }
    e.bounce = n.map(|c| c * v);
    let coin = ctx.rng.random(1.0) > 0.5;
    let off = ctx.rng.random(20.0) + 25.0;
    let a = to_angle(n).0 + if coin { off } else { -off };
    if e.random_time <= 0.0 && e.delta_pan.abs() < g.delta_dest_pan {
        e.dest_pan = ang(a);
    }
    if e.flag(3) {
        if e.etat != 2 && e.random_time <= 0.0 {
            e.random_time = e.time_for_nothing * 1.5;
        }
        if e.etat == 2 || e.etat == 12 {
            if e.random_time <= 0.0 {
                e.random_time = e.time_for_nothing;
            }
            e.etat = 1;
            e.anim_speed = e.walk_speed();
        }
    }
}

/// EVENT_ENTITY on the lion (§2.8 step 4) for non-Sammy entities; also the stuck roar.
fn on_entity(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, other: Entity) {
    let g = *ctx.g;
    e.guard_stuck += e.ball_accel;
    if stuck_roar(e, ctx) {
        return;
    }
    if ctx.sammy.is_some_and(|(s, _, _)| s == other) {
        e.random_time = 0.0;
        e.dest_pan = e.pan; // keep the heading into Sammy
        return;
    }
    // Bumped something else (another lion, a door): bounce and turn ±45°.
    let n = to_angle([-e.vel[0], -e.vel[1], 0.0]);
    let a = n.0 + if ctx.rng.random(1.0) > 0.5 { 45.0 } else { 315.0 };
    let (s, c) = n.0.to_radians().sin_cos();
    e.bounce = [c * g.intensite_bounce * e.weight, s * g.intensite_bounce * e.weight, 0.0];
    if e.delta_pan.abs() < g.delta_dest_pan {
        e.dest_pan = ang(a);
    }
    if e.flag(3) && e.etat != 2 && e.random_time <= 0.0 {
        e.random_time = e.time_for_nothing;
    }
}

/// Stuck roar (Lion_Hit :319-376), checked first on every collision event: a lion
/// running and stuck (`GuardStuck > 10`) with Sammy in clear view roars, and with
/// Flag7 calls its friends.
fn stuck_roar(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>) -> bool {
    if e.guard_stuck > 10.0 && matches!(e.etat, 2 | 12 | 13) {
        e.random_time = 0.0;
        e.guard_stuck = 0.0;
        // The original traces from Sammy+100 to the lion+100 against level geometry only.
        let clear = ctx.sammy.is_some_and(|(_, sp, _)| {
            !ctx.world.trace_static(0, [sp[0], sp[1], sp[2] + 100.0], [e.pos[0], e.pos[1], e.pos[2] + 100.0]).trace.hit()
        });
        if clear {
            e.dest_pan = ang(e.pan - 180.0);
            if e.spec == 3 {
                ctx.sfx.play("ScorpionAttack_son");
            } else if e.flag(7) && ctx.calls.attack_call == 0.0 {
                ctx.sfx.play("tigre_son");
                ctx.sfx.play("LionAttack1_son");
            } else {
                ctx.sfx.play("tigre_son");
            }
            e.random_time = e.time_for_nothing * 2.0;
            e.set_state(3, 4.0);
            if e.flag(7) && e.dist < e.attack_range && ctx.calls.attack_call == 0.0 && ctx.calls.answer != 0 {
                // CallFriendsToHelp.
                ctx.calls.answer = 0;
                ctx.calls.attack_call = ctx.calls.tempo_call;
                ctx.calls.friend_call = e.spec as f32 + e.weight;
                ctx.call_rings.push(e.pos);
            }
            return true;
        }
    }
    false
}

/// Lion touching Sammy (§4.1/4.2): bump damage and swipe start, then the swipe hit.
fn contact_sammy(e: &mut Enemy, p: &mut Player, game: &mut GameState, g: &LionGlobals, sfx: &mut Sfx, rng: &mut Rng) {
    // Sammy is pushed by the lion's motion (`ObjetPoussant`): vel × 0.8 × BounceForce.
    p.knock[0] = e.vel[0] * 0.8 * e.push_force;
    p.knock[1] = e.vel[1] * 0.8 * e.push_force;
    if e.etat == 2 {
        game.hearts -= e.resistance * 0.25;
        e.set_state(10, e.anim_speed); // swipe at the inherited run speed
        e.swipe_pending = true;
    } else if e.etat == 10 && e.anim_step > 60.0 && e.swipe_pending {
        game.hearts -= e.resistance * g.hargne;
        e.swipe_pending = false;
        sfx.play(["Hit1", "Hit2", "Hit3"][(rng.random(3.0) as usize).min(2)]);
    }
}

/// Wasp impact on a lion (§5.1, IMPACT path).
fn wasp_hit(e: &mut Enemy, h: &WaspHit, g: &LionGlobals, rng: &mut Rng) {
    let _ = rng;
    if e.class != LION || matches!(e.etat, 7) {
        return;
    }
    if e.etat == 0 || e.etat == 4 {
        take_attack_bis(e);
        second_react_range(e);
    }
    e.dest_pan = h.wasp_pan;
    e.bounce = [h.wasp_disp[0] * e.weight * h.bounce_force, h.wasp_disp[1] * e.weight * h.bounce_force, 0.0];
    if e.flag(3) && e.random_time <= 0.0 {
        e.random_time = e.time_for_nothing;
    }
    e.resistance -= h.poison;
    if e.resistance <= 0.0 {
        if e.etat != 6 {
            e.pan = ang(e.dest_pan - 180.0);
            e.set_state(6, 4.0);
            e.vspeed = 20.0;
            e.fric_vspeed = 3.5;
        }
    } else if e.etat != 11 && e.etat != 6 {
        e.vspeed = 20.0 * e.weight;
        e.fric_vspeed = 3.5;
        e.attacked += h.poison;
        if e.resistance < e.resistance_org * 0.95 {
            let s = (10.0 * e.resistance * 0.3).clamp(5.0, 20.0);
            e.set_state(11, s);
        } else {
            e.bounce[0] *= 2.5;
            e.bounce[1] *= 2.5;
        }
    }
    let _ = g;
}

/// `AnimLion` (§3): advances the animation step and handles end-of-animation transitions.
fn anim_lion(e: &mut Enemy, ctx: &mut Ctx<'_, '_, '_, '_, '_>, vis: &mut Visibility) {
    // Idle states animate twice per tick (§3.2).
    let k = if matches!(e.etat, 0 | 4 | 8 | 9) { 2.0 } else { 1.0 };
    e.anim_step += e.anim_speed * k;
    if e.anim_step <= 100.0 {
        return;
    }
    match e.etat {
        0 | 4 => {
            e.anim_step = 0.0;
            if e.etat == 0 && ctx.rng.random(100.0) < 10.0 {
                e.etat = 4;
            } else if e.etat == 4 && ctx.rng.random(100.0) < 95.0 {
                e.etat = 0;
            }
        }
        3 => {
            let w = e.walk_speed();
            e.set_state(1, w);
        }
        6 => {
            // LionDeath: blink 33 ticks, then gone for good (1 life in all shipped data).
            e.etat = 7;
            e.dying = 0.0;
            e.anim_step = 100.0;
        }
        7 => {
            e.anim_step = 100.0;
            e.dying += 1.0;
            let blink = [4.0, 8.0, 12.0, 16.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0, 31.0, 33.0];
            let toggles = blink.iter().filter(|&&b| e.dying >= b).count();
            *vis = if toggles % 2 == 0 { Visibility::Inherited } else { Visibility::Hidden };
            if e.dying >= 33.0 {
                *vis = Visibility::Hidden;
                e.class = 0; // inert
            }
        }
        10 => {
            if e.spec == 3 {
                ctx.sfx.play("ScorpionAttack_son");
            } else {
                ctx.sfx.play(["Hit1", "Hit2", "Hit3"][(ctx.rng.random(3.0) as usize).min(2)]);
            }
            let r = e.run_speed();
            e.set_state(2, r);
        }
        11 => {
            ctx.sfx.play(["LionHit1_son", "LionHit2_son", "tigre_son"][(ctx.rng.random(3.0) as usize).min(2)]);
            e.attack_range *= 1.5;
            e.react_range *= 1.5;
            let r = e.run_speed();
            if e.flag(7) && e.resistance <= 0.75 * e.resistance_org {
                e.etat = 3;
                e.anim_step = 0.0; // roar keeps the hit speed (anim_speed not reset)
            } else if e.resistance <= 0.5 * e.resistance_org {
                e.dest_pan = ang(e.pan - 180.0 + ctx.rng.random(180.0));
                e.random_time = e.time_for_nothing * 1.5;
                e.set_state(12, r);
            } else {
                e.random_time = e.time_for_nothing * 2.0;
                e.set_state(2, r);
            }
        }
        _ => e.anim_step -= 100.0,
    }
}

/// Maps the WDL state + step onto the model animator.
fn sync_anim(e: &Enemy, anim: &mut ModelAnimator) {
    if e.class == WIN_MODEL {
        if anim.scene() != "stand" {
            anim.play("stand", true, 0.0);
            anim.set_speed_pct(6.0, TICKS);
        }
        return;
    }
    let (scene, looping) = match e.etat {
        0 | 8 => ("stand", true),
        4 | 9 => ("AltStand", true),
        1 => ("walk", true),
        2 | 12 | 13 => {
            if e.speed * e.ball_accel < 18.0 {
                ("walk", true)
            } else {
                ("run", true)
            }
        }
        3 => ("rugit", true),
        10 => ("attack", false),
        11 => ("Hit", false),
        6 | 7 => ("death", false),
        _ => ("stand", true),
    };
    // Wasp models only have stand/walk/death.
    let (scene, looping) = if e.wasp_spec != 0 && !matches!(scene, "stand" | "walk" | "death") {
        (if matches!(e.etat, 4 | 9) { "stand" } else { "walk" }, true)
    } else {
        (scene, looping)
    };
    if anim.scene() != scene {
        anim.play(scene, looping, e.anim_step.min(100.0));
    } else {
        anim.set_step(e.anim_step.min(100.0));
    }
    let k = if matches!(e.etat, 0 | 4 | 8 | 9) { 2.0 } else { 1.0 };
    anim.set_speed_pct(if e.etat == 7 { 0.0 } else { e.anim_speed * k }, TICKS);
}

/// Smooth rendering between 16 Hz AI ticks.
pub fn place_enemies(clock: Res<AiClock>, mut q: Query<(&Enemy, &mut Transform)>) {
    let a = clock.acc.clamp(0.0, 1.0);
    for (e, mut tf) in &mut q {
        if !e.initialised {
            continue;
        }
        let p = [0, 1, 2].map(|i| e.prev_pos[i] + (e.pos[i] - e.prev_pos[i]) * a);
        tf.translation = coords::pos(p);
        tf.rotation = coords::rotation([e.prev_pan + ang(e.pan - e.prev_pan) * a, e.tilt, 0.0]);
    }
}

/// Wasp homing and fear tagging (`Wasp_Hit` EVENT_DETECT, docs/wasps.md §4.4).
pub fn home_wasps(time: Res<Time>, mut wasps: Query<&mut Wasp>, mut enemies: Query<&mut Enemy>) {
    let t = time.delta_secs() * TICKS;
    for mut w in &mut wasps {
        if !w.can_home() {
            continue;
        }
        let mut targets: Vec<(f32, Mut<Enemy>)> = enemies
            .iter_mut()
            .filter(|e| e.alive() && e.is_class_target())
            .filter_map(|e| {
                let aim = e.muzzle.map(|m| [m[0], m[1], m[2] - 32.0]).unwrap_or([e.pos[0], e.pos[1], e.pos[2] + 32.0]);
                in_cone(w.pos, w.pan, w.tilt, 120.0, 120.0, w.homing_range, aim).then(|| (to_angle([aim[0] - w.pos[0], aim[1] - w.pos[1], aim[2] - w.pos[2]]).2, e))
            })
            .collect();
        targets.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, mut e) in targets {
            if e.attacked <= 0.0 && w.find_cd <= 0.0 {
                w.find_cd = 16.0;
                e.attacked += w.poison;
            }
            let aim = e.muzzle.map(|m| [m[0], m[1], m[2] - 32.0]).unwrap_or([e.pos[0], e.pos[1], e.pos[2] + 32.0]);
            let (dp, dt, _) = to_angle([aim[0] - w.pos[0], aim[1] - w.pos[1], aim[2] - w.pos[2]]);
            w.pan += ang(dp - w.pan) * 0.4 * t;
            let gain = w.tilt_gain;
            w.tilt += ang(dt - w.tilt) * gain * t;
        }
    }
}

/// Moves enemy boxes with their owners; dead enemies stop colliding.
pub fn sync_enemy_colliders(mut q: Query<(&Enemy, &mut BoxCollider)>) {
    for (e, mut c) in &mut q {
        c.center = [e.pos[0], e.pos[1], e.pos[2] + e.center_z];
        c.solid = e.class != 0 && !e.passable && e.teleport.is_none() && !matches!(e.etat, 7) && !(e.etat == 6 && e.anim_step > 50.0);
    }
}

/// Lions fading on teleport pads get their own material copy while translucent.
#[derive(Component)]
pub struct FadeMaterial(Handle<StandardMaterial>);

pub fn fade_enemies(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &Enemy, &mut MeshMaterial3d<StandardMaterial>, Option<&FadeMaterial>)>,
) {
    for (ent, e, mut mat, fade) in &mut q {
        match (e.alpha < 100.0, fade) {
            (true, None) => {
                let mut m = materials.get(&mat.0).cloned().unwrap_or_default();
                m.alpha_mode = AlphaMode::Blend;
                commands.entity(ent).insert(FadeMaterial(mat.0.clone()));
                mat.0 = materials.add(m);
            }
            (true, Some(_)) => {
                if let Some(mut m) = materials.get_mut(&mat.0) {
                    m.base_color.set_alpha(e.alpha / 100.0);
                }
            }
            (false, Some(orig)) => {
                mat.0 = orig.0.clone();
                commands.entity(ent).remove::<FadeMaterial>();
            }
            (false, None) => {}
        }
    }
}

/// `InitStartEnnemy` (mm1.wdl:5126-5170): every 8 ticks, wake the first lion of id
/// Skill1 within Skill2 into state Skill3 with new react/attack ranges; one-shot unless Skill4.
#[derive(Component)]
pub struct StartTrigger {
    pos: [f32; 3],
    s: [f32; 8],
    timer: f32,
    active: bool,
}

pub fn init_start_ennemy(obj: &Object, entity: &mut EntityCommands) {
    let mut s = [0.0; 8];
    s.copy_from_slice(&obj.skills[..8]);
    entity.insert(StartTrigger { pos: obj.origin, s, timer: 0.0, active: true });
}

pub fn run_start_triggers(clock: Res<AiClock>, mut triggers: Query<&mut StartTrigger>, mut enemies: Query<&mut Enemy>) {
    for _ in 0..clock.ticks {
        for mut tr in &mut triggers {
            if !tr.active {
                continue;
            }
            tr.timer -= 1.0;
            if tr.timer > 0.0 {
                continue;
            }
            tr.timer = 8.0;
            let p = tr.pos;
            let s = tr.s;
            let hit = enemies.iter_mut().find(|e| {
                e.class as f32 == s[0] && e.alive() && (0..3).map(|i| (e.pos[i] - p[i]).powi(2)).sum::<f32>().sqrt() <= s[1]
            });
            if let Some(mut e) = hit {
                e.etat = s[2] as i32;
                if e.etat == 1 && e.class == LION {
                    e.anim_step = 0.0;
                    e.anim_speed = 0.82 * e.speed;
                }
                if s[4] != 0.0 {
                    e.react_range = s[4];
                }
                if s[5] != 0.0 {
                    e.attack_range = s[5];
                }
                if s[3] == 0.0 {
                    tr.active = false;
                } else {
                    tr.timer = s[3];
                }
            }
        }
    }
}

/// Adds the collision box once the model is known.
pub fn add_enemy_colliders(mut commands: Commands, q: Query<(Entity, &Enemy), Without<BoxCollider>>) {
    for (ent, e) in &q {
        if e.initialised {
            commands.entity(ent).insert(BoxCollider { center: [e.pos[0], e.pos[1], e.pos[2] + e.center_z], half: e.half, solid: true });
        }
    }
}

/// Sammy's own collision box (scaled model bbox), so lions and wasps can touch him.
pub fn add_player_collider(mut commands: Commands, q: Query<(Entity, &Player, &ModelAnimator), Without<BoxCollider>>) {
    for (ent, p, anim) in &q {
        let Some(f) = anim.model.mdl.frames.first() else { continue };
        let s = super::player::SCALE;
        let hx = (f.bbox_max[0] - f.bbox_min[0]).max(f.bbox_max[1] - f.bbox_min[1]) * 0.25 * s;
        let hz = (f.bbox_max[2] - f.bbox_min[2]) * 0.5 * s;
        let cz = (f.bbox_max[2] + f.bbox_min[2]) * 0.5 * s;
        commands.entity(ent).insert((BoxCollider { center: [p.pos[0], p.pos[1], p.pos[2] + cz], half: [hx, hx, hz], solid: true }, PlayerBoxOffset(cz)));
    }
}

#[derive(Component)]
pub struct PlayerBoxOffset(f32);

pub fn sync_player_collider(mut q: Query<(&Player, &PlayerBoxOffset, &mut BoxCollider)>) {
    for (p, off, mut c) in &mut q {
        c.center = [p.pos[0], p.pos[1], p.pos[2] + off.0];
        c.solid = p.state != PlayerState::Dead;
    }
}

pub fn bump_clock(mut clock: ResMut<AiClock>) {
    clock.ticks1 = (clock.ticks1 + clock.ticks) % 64;
}

/// `RestartSammy` sets `AttackCall = 0` (player.wdl:894).
pub fn reset_calls(mut reset: MessageReader<crate::game::ResetScan>, mut calls: ResMut<FriendCalls>) {
    if reset.read().count() > 0 {
        calls.attack_call = 0.0;
    }
}
