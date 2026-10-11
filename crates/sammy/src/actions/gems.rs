//! Gem machinery beyond the plain pickup (docs/level-objects.md §2.1-2.5):
//! thrown-back keys (`LaunchYouKey`), gems launched by objects (`EjectNewKey` /
//! `LaunchNewKey`: boulders, Sammy's death gem), lifetimes with blinking,
//! gravity/hover for Flag7 gems, surprise colours, boulders, and the gem that
//! flies into a permanent door (`CreateGemKeyAnim` / `AnimGemKey`).

use crate::collision::Hull;
use bevy::prelude::*;
use formats::wmb::Object;

use super::pickups::{GemSprites, KeyGem, LevelKey, PickupPos, Respawnable, Shrinking};
use super::player::{JumpParams, Player, TICKS};
use super::platform::Platform;
use super::wasp::WaspHit;
use crate::audio::Sfx;
use crate::collision::CollisionWorld;
use crate::coords;
use crate::game::Rng;
use crate::level::{Billboard, SpriteAnim};

/// `HeightGem`.
const HEIGHT_GEM: f32 = 25.0;
/// `GemVerticalSpeed` / `GemVerticalFriction` (InitLvl.wdl:63-64).
const GEM_VSPEED: f32 = 10.0;
const GEM_VFRIC: f32 = 2.5;
/// `SpeedGemKey`: gem flying into a door, quants per tick.
const SPEED_GEM_KEY: f32 = 12.0;

/// The gem can be picked up by Sammy's scan (`InitNewKey` ran).
#[derive(Component)]
pub struct Collectable;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FlightKind {
    /// `LaunchYouKey`: a key Sammy threw back.
    Thrown,
    /// `LaunchNewKey`: launched by a boulder or on Sammy's death. `fate` = 0 decorative,
    /// 1-999 lifetime, 1000 stays, 1001 stays and respawns.
    Launched { fate: f32 },
}

/// A gem in flight with damped bounces (per-frame loops of gems.wdl:784-934 / 1394-1530).
#[derive(Component)]
pub struct FlyingGem {
    kind: FlightKind,
    pos: [f32; 3],
    pan: f32,
    speed: f32,
    vz: f32,
    vz_org: f32,
    fric: f32,
    bounces: u32,
    /// Thrown keys never rise above their launch height + 32.
    ceiling: f32,
    stuck: f32,
    last_good: [f32; 3],
    good_timer: f32,
    /// Lifetime inherited by a thrown key.
    life: f32,
}

/// Remaining lifetime (Skill9) of a runtime gem; counts down once it rests.
#[derive(Component)]
pub struct GemLifetime {
    ticks: f32,
    /// Blink: scale toggles 1.5 ↔ 0.75 each half animation cycle, speeding up.
    small: bool,
    speed: f32,
}

impl GemLifetime {
    pub fn ticks(&self) -> f32 {
        self.ticks
    }
}

/// Flag7 gem: falls at `Gravite` per tick down to its rest height, rides platforms.
#[derive(Component)]
pub struct GemGravity {
    rest: f32,
}

impl GemGravity {
    pub fn new(rest: f32) -> Self {
        Self { rest }
    }

    pub fn set_rest(&mut self, rest: f32) {
        self.rest = rest;
    }
}

/// Surprise gem (Skill5): not collectable for 1-8 ticks, then its colour climbs.
#[derive(Component)]
pub struct Surprise {
    pub add: f32,
    delay: f32,
}

/// Decorative gem (fate 0): shrinks and bursts once it stops.
#[derive(Component)]
pub struct Decorative;

fn sprite_bundle(sprites: &GemSprites, color: u32, pos: [f32; 3], scale: f32) -> Option<impl Bundle> {
    let (mesh, mat, frames) = sprites.0.get(&color).cloned()?;
    Some((
        crate::flow::LevelScoped,
        Transform { translation: coords::pos(pos), scale: Vec3::splat(scale), ..default() },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Billboard,
        SpriteAnim { frames, frame: 0.0, fps: 48.0 },
        KeyGem { color },
        PickupPos(pos),
    ))
}

/// `EjectOldKey` + `LaunchYouKey`: the old key flies forward at 2× Sammy's walk speed.
pub fn throw_back_key(commands: &mut Commands, sprites: &GemSprites, color: u32, p: &Player, life: f32) {
    let pos = [p.pos[0], p.pos[1], p.pos[2] + 50.0];
    let Some(b) = sprite_bundle(sprites, color, pos, 1.5) else { return };
    commands.spawn((
        Name::new("thrown key"),
        b,
        FlyingGem {
            kind: FlightKind::Thrown,
            pos,
            pan: p.pan,
            speed: p.speed * 2.0,
            vz: GEM_VSPEED,
            vz_org: GEM_VSPEED,
            fric: GEM_VFRIC,
            bounces: 0,
            ceiling: pos[2] + 32.0,
            stuck: 0.0,
            last_good: pos,
            good_timer: 0.0,
            life,
        },
    ));
}

/// `EjectNewKey` → `LaunchNewKey`: a gem launched from `pos` (boulders, Sammy's death).
#[allow(clippy::too_many_arguments)]
pub fn launch_gem(commands: &mut Commands, sprites: &GemSprites, jump: &JumpParams, color: u32, pos: [f32; 3], pan: f32, hspeed: f32, vz: f32, fate: f32) {
    let Some(b) = sprite_bundle(sprites, color, pos, 1.5) else { return };
    commands.spawn((
        Name::new("launched gem"),
        b,
        FlyingGem {
            kind: FlightKind::Launched { fate },
            pos,
            pan,
            speed: (hspeed + 5.0).max(25.0),
            vz,
            vz_org: vz,
            fric: jump.acc_gravity,
            bounces: 0,
            ceiling: f32::MAX,
            stuck: 0.0,
            last_good: pos,
            good_timer: 0.0,
            life: 0.0,
        },
    ));
}

/// After a flight: make the gem collectable and give it its resting behaviour.
fn settle(commands: &mut Commands, e: Entity, g: &FlyingGem) {
    let mut ent = commands.entity(e);
    ent.remove::<FlyingGem>();
    match g.kind {
        FlightKind::Thrown => {
            ent.insert(Collectable);
            if g.life > 0.0 {
                ent.insert(GemLifetime { ticks: g.life, small: false, speed: 3.0 });
            }
        }
        FlightKind::Launched { fate } if fate <= 0.0 => {
            ent.insert((Decorative, Shrinking));
        }
        FlightKind::Launched { fate } => {
            ent.insert((Collectable, GemGravity { rest: HEIGHT_GEM }));
            if fate < 1000.0 {
                ent.insert(GemLifetime { ticks: fate, small: false, speed: 3.0 });
            } else if fate == 1001.0 {
                ent.insert((LevelKey::at(g.pos, Vec3::splat(1.0)), Respawnable));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fly_gems(
    time: Res<Time>,
    world: CollisionWorld,
    jump: Res<JumpParams>,
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    mut q: Query<(Entity, &mut FlyingGem, &mut PickupPos, &mut Transform, Has<Collectable>)>,
) {
    // The original loops run per frame with `mintime = min(time, 1)`.
    let t = (time.delta_secs() * TICKS).min(1.0);
    for (e, mut g, mut pp, mut tf, collectable) in &mut q {
        let floor = {
            let tr = world.trace_static(Hull::Point, g.pos, [g.pos[0], g.pos[1], g.pos[2] - 2000.0]).trace;
            if tr.hit() { tr.end[2] } else { g.pos[2] - 2000.0 }
        };
        let thrown = g.kind == FlightKind::Thrown;
        if g.vz == 0.0 && g.pos[2] - floor > HEIGHT_GEM + 1.0 {
            g.vz = jump.gravity;
            g.fric = jump.acc_gravity;
        }
        if thrown && g.pos[2] > g.ceiling {
            g.vz = jump.gravity;
            g.fric = jump.acc_gravity;
        }
        let mut dz = 0.0;
        let mut done = false;
        if g.vz != 0.0 {
            g.vz -= g.fric * t;
            if g.pos[2] < floor + HEIGHT_GEM {
                // Touchdown = one bounce; collectable from the 5th on.
                g.bounces += 1;
                let launched_fate = match g.kind {
                    FlightKind::Launched { fate } => Some(fate),
                    FlightKind::Thrown => None,
                };
                if g.bounces == 5 && launched_fate.is_none_or(|f| f > 0.0) && !collectable {
                    commands.entity(e).insert(Collectable);
                }
                let (min_speed, keep) = if thrown { (2.0, 0.95) } else { (0.5, 0.9) };
                if g.speed > min_speed {
                    if g.vz_org > if thrown { 7.0 } else { 5.0 } {
                        sfx.play(["ding6", "ding5", "ding3", "ding7"][(g.bounces.clamp(1, 4) - 1) as usize]);
                    }
                    if thrown {
                        g.speed -= 5.0;
                    }
                    g.vz_org *= keep;
                    g.vz = g.vz_org - g.fric * t;
                    dz = g.vz * t;
                } else {
                    dz = floor + HEIGHT_GEM - g.pos[2];
                    g.vz = 0.0;
                }
            } else {
                dz = g.vz * t;
            }
        }
        g.good_timer -= t;
        if g.good_timer <= 0.0 {
            g.good_timer = 16.0;
            g.last_good = g.pos;
        }
        let (s, c) = g.pan.to_radians().sin_cos();
        let target = [g.pos[0] + c * g.speed * t, g.pos[1] + s * g.speed * t, g.pos[2] + dz];
        let hit = world.trace_static(Hull::Point, g.pos, target);
        let before = g.pos;
        if hit.trace.start_solid {
            g.pos = g.last_good;
            done = true;
        } else {
            g.pos = hit.trace.end;
        }
        // Gem_Hit: reflect off walls while not yet collectable.
        if hit.trace.hit() && hit.trace.normal[2].abs() < 0.7 && !collectable {
            let n = hit.trace.normal;
            let d = c * n[0] + s * n[1];
            g.pan = (s - 2.0 * d * n[1]).atan2(c - 2.0 * d * n[0]).to_degrees();
            g.speed = (g.speed - 8.0).max(0.1);
        }
        let moved_z = g.pos[2] - before[2];
        if thrown {
            if dz != 0.0 && moved_z.abs() < 1e-3 {
                if g.pos[2] > floor + HEIGHT_GEM {
                    // Stuck in the air: hop off in a random direction.
                    g.stuck += 1.0;
                    if g.stuck > 24.0 {
                        done = true;
                    }
                    sfx.play("ding3");
                    g.speed = 30.0;
                    g.vz = GEM_VSPEED;
                    g.vz_org = GEM_VSPEED;
                    g.pan = rng.random(360.0);
                    g.pos[2] += 8.0;
                } else {
                    g.pos[2] = floor + HEIGHT_GEM;
                    done = true;
                }
            }
            g.speed -= 0.5 * t;
            if g.speed < 0.1 {
                g.speed = 0.1;
                if g.vz == 0.0 {
                    done = true;
                }
            }
        } else {
            g.speed -= 0.05 * t;
            let blocked = (g.pos[0] - before[0]).abs() + (g.pos[1] - before[1]).abs() < 1.0;
            g.vz_org -= if blocked { 0.5 } else { 0.02 } * t;
            if blocked && g.vz_org.abs() < jump.acc_gravity.abs() {
                done = true;
            }
            if g.speed < 1.0 {
                g.speed = 1.0;
                if g.vz_org.abs() < jump.acc_gravity.abs() {
                    done = true;
                }
            }
        }
        pp.0 = g.pos;
        tf.translation = coords::pos(g.pos);
        if done {
            settle(&mut commands, e, &g);
        }
    }
}

/// Resting runtime gems: lifetime blink and burst (GemWaitForPlayer with Skill9).
pub fn gem_lifetimes(
    time: Res<Time>,
    mut commands: Commands,
    mut sfx: Sfx,
    mut q: Query<(Entity, &mut GemLifetime, &mut Transform, &mut SpriteAnim, Option<&KeyGem>), (Without<FlyingGem>, Without<Shrinking>)>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut l, mut tf, mut anim, gem) in &mut q {
        anim.fps = l.speed * TICKS;
        let half = anim.frames as f32 / 2.0;
        if anim.frame > half && !l.small {
            l.small = true;
            tf.scale = Vec3::splat(0.75);
            sfx.play("cligno1_son");
        } else if anim.frame < half && l.small {
            l.small = false;
            l.speed += 0.1 * t;
            tf.scale = Vec3::splat(1.5);
            sfx.play("cligno1_son");
        }
        l.ticks -= t;
        if l.ticks <= 0.0 {
            sfx.play("broken4"); // GemExplose
            let (at, color) = (crate::coords::native(tf.translation), gem.map_or(0, |g| g.color));
            crate::effects::queue(&mut commands, move |ps| crate::effects::gem_explode(ps, at, color));
            commands.entity(e).despawn();
        }
    }
}

/// Flag7 gems: fall at `Gravite` per tick to floor + rest, carried by moving platforms.
pub fn gem_gravity(
    time: Res<Time>,
    world: CollisionWorld,
    jump: Res<JumpParams>,
    platforms: Query<&Platform>,
    mut q: Query<(&GemGravity, &mut PickupPos, &mut Transform), (Without<FlyingGem>, Without<Shrinking>)>,
) {
    let t = time.delta_secs() * TICKS;
    for (g, mut p, mut tf) in &mut q {
        let hit = world.trace(Hull::Point, p.0, [p.0[0], p.0[1], p.0[2] - 2000.0], None);
        if !hit.trace.hit() {
            continue;
        }
        if let Some(plat) = hit.entity.and_then(|e| platforms.get(e).ok()) {
            for i in 0..3 {
                p.0[i] += plat.delta[i];
            }
        }
        let floor = hit.trace.end[2];
        if p.0[2] > floor + g.rest {
            p.0[2] = (p.0[2] + jump.gravity * t).max(floor + g.rest);
        } else {
            p.0[2] = floor + g.rest;
        }
        tf.translation = coords::pos(p.0);
    }
}

/// Surprise gems: the colour climbs by 1..=Skill5 (max 6) after a 1-8 tick delay.
pub fn surprise_gems(time: Res<Time>, mut commands: Commands, mut rng: ResMut<Rng>, mut q: Query<(Entity, &mut Surprise, &mut KeyGem)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut s, mut gem) in &mut q {
        if s.delay <= 0.0 {
            continue;
        }
        s.delay -= t;
        if s.delay <= 0.0 {
            gem.color = (gem.color + rng.random(s.add) as u32 + 1).min(6);
            commands.entity(e).insert(Collectable);
        }
    }
}

impl Surprise {
    /// Armed with a 1-8 tick delay (derived from the gem until the reset scan re-rolls it).
    pub fn new(add: f32, seed: f32) -> Self {
        Self { add, delay: (seed.abs() as u32 % 8) as f32 + 1.0 }
    }

    /// Re-roll on respawn (the colour keeps climbing, original quirk).
    pub fn rearm(&mut self, rng: &mut Rng) {
        self.delay = rng.random(8.0).trunc() + 1.0;
    }

}

/// Boulder emitting gems (`InitBoulder`, gems.wdl:1568-1657).
#[derive(Component)]
pub struct Boulder {
    fate: f32,
    vz: f32,
    /// Id whose impact makes it emit (9999 = Sammy's wasps).
    trigger: f32,
    color: f32,
    hspeed: f32,
    period: f32,
    timer: f32,
    spread: f32,
    pan: f32,
    pos: [f32; 3],
    impact: bool,
}

pub fn init_boulder(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    let f = |n: u32| obj.flags & (1 << (n - 1)) != 0;
    let spread = 20.0 * f(3) as u8 as f32 + 45.0 * f(4) as u8 as f32 + 90.0 * f(5) as u8 as f32 + 180.0 * f(6) as u8 as f32;
    entity.insert(Boulder {
        fate: s[0],
        vz: s[3],
        trigger: s[4],
        color: s[5],
        hspeed: s[6],
        period: s[7],
        timer: 48.0 + s[7],
        spread,
        pan: obj.angles[0],
        pos: obj.origin,
        impact: !f(2),
    });
}

impl Boulder {
    fn color(&self, rng: &mut Rng) -> u32 {
        if self.color == 999.0 { rng.random(6.0) as u32 + 1 } else { self.color as u32 }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_boulders(
    time: Res<Time>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    sprites: Res<GemSprites>,
    jump: Res<JumpParams>,
    mut hits: MessageReader<WaspHit>,
    mut boulders: Query<(Entity, &mut Boulder)>,
) {
    let t = time.delta_secs() * TICKS;
    // Impact mode: a wasp (id 9999) hitting the boulder launches a gem back toward the shooter.
    for h in hits.read() {
        let Ok((ent, b)) = boulders.get(h.target) else { continue };
        let _ = ent;
        if !b.impact || b.trigger != 9999.0 {
            continue;
        }
        let pan = h.wasp_pan - 180.0 + rng.random(b.spread) - b.spread * 0.5;
        let hs = (h.wasp_disp[0] * b.hspeed).abs() + (h.wasp_disp[1] * b.hspeed).abs();
        let color = b.color(&mut rng);
        let pos = [b.pos[0] + h.wasp_disp[0], b.pos[1] + h.wasp_disp[1], b.pos[2] + 50.0];
        launch_gem(&mut commands, &sprites, &jump, color, pos, pan, hs, b.vz, b.fate);
    }
    // Auto mode: every Skill8 ticks from 150 above the boulder.
    for (_, mut b) in &mut boulders {
        if b.period <= 0.0 {
            continue;
        }
        b.timer -= t;
        if b.timer > 0.0 {
            continue;
        }
        b.timer += b.period;
        let pan = b.pan + rng.random(b.spread) - b.spread * 0.5;
        let hs = b.hspeed + rng.random(3.0);
        let vz = b.vz + rng.random(4.0);
        let color = b.color(&mut rng);
        let pos = [b.pos[0], b.pos[1], b.pos[2] + 150.0];
        launch_gem(&mut commands, &sprites, &jump, color, pos, pan, hs, vz, b.fate);
    }
}

/// Gem flying from Sammy into a permanent door (`AnimGemKey`, gems.wdl:1043-1233).
#[derive(Component)]
pub struct DoorGem {
    pub door: Entity,
    pub color: u32,
    pos: [f32; 3],
    phase: u8,
    timer: f32,
}

pub fn spawn_door_gem(commands: &mut Commands, sprites: &GemSprites, door: Entity, color: u32, sammy: [f32; 3]) {
    let pos = [sammy[0], sammy[1], sammy[2] + 40.0];
    let Some((mesh, mat, frames)) = sprites.0.get(&color).cloned() else { return };
    commands.spawn((
        Name::new("door gem"),
        crate::flow::LevelScoped,
        Transform::from_translation(coords::pos(pos)),
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Billboard,
        SpriteAnim { frames, frame: 0.0, fps: -48.0 },
        DoorGem { door, color, pos, phase: 1, timer: 80.0 },
    ));
}

/// Door-side state the gem needs: (door target position, gem still wanted, validated).
pub type DoorTarget = ([f32; 3], bool, bool);

/// Moves door gems: phase 1 at 12/tick for ≤80 ticks, phase 2 closes 70 % of the gap per
/// tick for ≤16 ticks; arrival validates the door, a recall flies back and returns the key.
/// Returns (door, arrived) events for the door system.
pub fn fly_door_gem(g: &mut DoorGem, target: DoorTarget, sammy: [f32; 3], t: f32) -> Option<bool> {
    let (door_pos, wanted, validated) = target;
    let aim = [door_pos[0], door_pos[1], door_pos[2] + 100.0];
    let step = |pos: &mut [f32; 3], to: [f32; 3], dist_step: f32| -> f32 {
        let d = [to[0] - pos[0], to[1] - pos[1], to[2] - pos[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if len > 1e-3 {
            let k = (dist_step / len).min(1.0);
            for i in 0..3 {
                pos[i] += d[i] * k;
            }
        }
        len
    };
    match g.phase {
        1 => {
            let dist = step(&mut g.pos, aim, SPEED_GEM_KEY * t);
            g.timer -= t;
            if dist <= 20.0 || !wanted || g.timer <= 0.0 || validated {
                g.phase = 2;
                g.timer = 16.0;
            }
            None
        }
        2 => {
            let d = [aim[0] - g.pos[0], aim[1] - g.pos[1], aim[2] - g.pos[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            step(&mut g.pos, aim, len * 0.7 * t);
            g.timer -= t;
            let dist = step(&mut g.pos.clone(), aim, 0.0);
            if dist <= 16.0 || g.timer <= 0.0 {
                if dist <= 20.0 && wanted {
                    return Some(true); // arrived: validate the door
                }
                g.phase = 3;
                g.timer = 80.0;
            }
            None
        }
        _ => {
            // Recalled: fly back to Sammy and return the key.
            let home = [sammy[0], sammy[1], sammy[2] + 100.0];
            let d = [home[0] - g.pos[0], home[1] - g.pos[1], home[2] - g.pos[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            g.timer -= t;
            if len <= 32.0 || g.timer <= 0.0 {
                return Some(false);
            }
            let v = if len > 128.0 { 40.0 * t } else { len * 0.9 * t };
            step(&mut g.pos, home, v);
            None
        }
    }
}

pub fn door_gem_pos(g: &DoorGem) -> [f32; 3] {
    g.pos
}
