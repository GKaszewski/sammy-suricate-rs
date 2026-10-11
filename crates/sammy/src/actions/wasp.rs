//! Sammy's wasps (docs/wasps.md): ammo pickups (`InitAmmo1`, bullets.wdl:21-177),
//! firing (`PlayerTire`, player.wdl:271-384), flight and hits (`InitWasp`/`Wasp`/
//! `Wasp_Hit`, bullets.wdl:203-747) and weak doors (`InitDoorWeak`, mm1.wdl:4466-4715).

use bevy::prelude::*;
use formats::wmb::Object;

use super::pickups::{PickupPos, Shrinking};
use super::player::{Player, State, TICKS};
use crate::audio::Sfx;
use crate::collision::{Brush, CollisionWorld};
use crate::coords;
use crate::game::{GameState, Rng};
use crate::level::{GameAssets, LevelObject};
use crate::model::ModelAnimator;

/// Pickup defaults by type 1..=7 (docs/wasps.md §1.3): (poison, scale).
const PICKUP: [(f32, Option<f32>); 7] =
    [(3.0, None), (2.0, Some(0.8)), (1.0, Some(0.65)), (1.0, Some(0.65)), (4.0, Some(0.8)), (6.0, Some(0.8)), (1.0, Some(0.65))];

/// Fired wasp by type 1..=7: (scale, speed q/tick, homing range, tilt gain, bounce force, lifetime).
const FIRED: [(f32, f32, f32, f32, f32, f32); 7] = [
    (1.0, 32.0, 500.0, 0.1, 1.0, 128.0),
    (0.7, 100.0, 0.0, 0.0, 2.0, 128.0),
    (0.5, 100.0, 0.0, 0.0, 2.0, 128.0),
    (0.5, 100.0, 500.0, 0.1, 2.0, 128.0),
    (0.7, 36.0, 700.0, 0.3, 3.0, 128.0),
    (1.0, 38.4, 1000.0, 0.2, 2.0, 200.0),
    (0.5, 100.0, 0.0, 0.0, 2.0, 128.0),
];

/// TirA speed factor by type (player.wdl:287-314).
const FIRE_FACTOR: [f32; 7] = [1.0, 2.2, 1.5, 1.2, 1.2, 1.0, 3.0];

#[derive(Component)]
pub struct AmmoPickup {
    kind: u32,
    amount: i32,
    poison: f32,
}

/// Spin of an idle pickup (pan += 1/tick).
#[derive(Component)]
pub struct Spinning {
    angles: [f32; 3],
}

#[derive(Component)]
pub struct Wasp {
    pub kind: u32,
    pub poison: f32,
    pub pan: f32,
    pub tilt: f32,
    pub pos: [f32; 3],
    speed: f32,
    pub homing_range: f32,
    pub tilt_gain: f32,
    pub bounce_force: f32,
    life: f32,
    /// Type 5/6 climb, hold and ease timers and dive rate (Skill9/10/11/16).
    climb: Option<[f32; 4]>,
    /// Last frame's displacement (lions read it as knockback).
    pub last_disp: [f32; 3],
    /// 16-tick cooldown of the fear tag (`FindEnnemy`).
    pub find_cd: f32,
    /// The thrower: ignored by the wasp's MOVE (`YOU = shooter`, bullets.wdl:377).
    shooter: Option<Entity>,
}

impl Wasp {
    /// Homing only runs once the climb/hold/ease phase is over.
    pub fn can_home(&self) -> bool {
        self.homing_range > 0.0 && self.climb.is_none_or(|c| c[2] <= 0.0)
    }
}

/// A wasp hit something solid (sent to targets such as lions).
#[derive(Message, Clone, Copy)]
pub struct WaspHit {
    pub target: Entity,
    pub poison: f32,
    pub bounce_force: f32,
    pub wasp_pan: f32,
    pub wasp_disp: [f32; 3],
}

/// Shots still to fire in a 3-wasp fan: (ticks left, pan offset).
#[derive(Component)]
pub struct PendingShots {
    shots: Vec<(f32, f32)>,
    spawn: [f32; 3],
}

#[derive(Component)]
pub struct WeakDoor {
    hp: f32,
}

#[derive(Component)]
pub struct Plank {
    v: [f32; 3],
    spin: [f32; 2],
    angles: [f32; 3],
    pos: [f32; 3],
    still: f32,
}

pub fn init_ammo(obj: &mut Object, entity: &mut EntityCommands) -> (u32, f32) {
    let s = &obj.skills;
    let mut kind = s[0] as u32;
    let mut poison = s[2];
    let mut skin = s[6] as u32;
    if kind <= 1 {
        kind = 1;
        poison = 3.0;
    } else if skin == 0 {
        let (p, scale) = PICKUP[(kind.min(7) - 1) as usize];
        skin = kind;
        poison = p;
        if let Some(sc) = scale {
            obj.scale = [sc; 3];
        }
    }
    if poison == 0.0 {
        poison = 1.0;
    }
    obj.angles = [0.0, 20.0, 0.0];
    entity.insert((
        AmmoPickup { kind, amount: s[1].max(1.0) as i32, poison },
        PickupPos(obj.origin),
        Spinning { angles: obj.angles },
    ));
    (skin, 3.0)
}

pub fn init_door_weak(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    let hp = if s[0] == 0.0 || s[0] == 3333.0 { s[1] } else { s[0] };
    entity.insert(WeakDoor { hp });
}

pub fn spin_pickups(time: Res<Time>, mut q: Query<(&mut Spinning, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (mut s, mut tf) in &mut q {
        s.angles[0] += t;
        tf.rotation = coords::rotation(s.angles);
    }
}

pub fn collect_ammo(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    pickups: Query<(Entity, &PickupPos, &AmmoPickup), Without<Shrinking>>,
) {
    let Ok(p) = player.single() else { return };
    if p.state == State::Dead {
        return;
    }
    for (e, pos, a) in &pickups {
        let d = (0..3).map(|i| (pos.0[i] - p.pos[i]).powi(2)).sum::<f32>().sqrt();
        if d > p.scan_radius {
            continue;
        }
        // A different type replaces the old ammo (the old wasps "escape").
        if game.ammo_type != a.kind {
            if game.ammo_type != 0 {
                game.ammo = 0;
            }
            game.ammo_type = a.kind;
            game.ammo_poison = a.poison;
        }
        game.ammo += a.amount;
        sfx.play("clog3");
        sfx.play("TakeWasp_son");
        // GemLight in the type colour (orange, yellow, blue, green, magenta, red, white).
        let color = [[250.0, 100.0, 0.0], [250.0, 250.0, 0.0], [0.0, 0.0, 250.0], [0.0, 250.0, 0.0], [250.0, 0.0, 250.0], [250.0, 0.0, 0.0], [150.0, 150.0, 150.0]]
            [(a.kind.clamp(1, 7) - 1) as usize];
        commands.entity(e).insert((Shrinking, crate::lights::flash(color, 300.0, 15.0, 100.0)));
    }
}

/// `PlayerTire`: Space fires when Sammy is in a normal ground state.
#[allow(clippy::too_many_arguments)]
pub fn fire(
    time: Res<Time>,
    controls: Res<crate::input::Controls>,
    current: Res<crate::flow::CurrentLevel>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut player: Query<(Entity, &mut Player, &mut ModelAnimator, Option<&mut PendingShots>)>,
) {
    let Ok((e, mut p, mut anim, pending)) = player.single_mut() else { return };
    let t = time.delta_secs() * TICKS;
    let kind = game.ammo_type.clamp(1, 7);

    // Debug: SAMMY_AUTOFIRE=1 fires whenever possible.
    let trigger = controls.fire_hit || std::env::var_os("SAMMY_AUTOFIRE").is_some();
    if trigger && crate::flow::playing(&current) && matches!(p.state, State::Stand | State::Walk | State::Turn) {
        if game.ammo <= 0 {
            sfx.play("Error1");
        } else {
            p.state = State::FireA;
            anim.play("TirA", false, 0.0);
            anim.set_speed_pct(20.0 * FIRE_FACTOR[(kind - 1) as usize], TICKS);
        }
    }
    if p.state == State::FireA && anim.finished() {
        // The wasp leaves 40 quants behind Sammy, 50 up, flying his way.
        let (s, c) = (p.pan + 180.0).to_radians().sin_cos();
        let spawn = [p.pos[0] + c * 40.0, p.pos[1] + s * 40.0, p.pos[2] + 50.0];
        game.ammo -= 1;
        let shots = if matches!(kind, 3 | 4) {
            // 3-wasp fan, 2 ticks apart, each extra wasp costing ammo while there is some.
            let mut shots = vec![(0.0, -5.0)];
            for (i, off) in [0.0, 5.0].into_iter().enumerate() {
                game.ammo -= 1;
                if game.ammo < 0 {
                    break;
                }
                shots.push((2.0 * (i + 1) as f32, off));
            }
            shots
        } else {
            vec![(0.0, 0.0)]
        };
        if game.ammo <= 0 {
            game.ammo = 0;
            game.ammo_type = 0;
            game.ammo_poison = 0.0;
        }
        commands.entity(e).insert(PendingShots { shots, spawn });
        p.state = State::FireB;
        anim.play("TirB", false, 10.0);
        anim.set_speed_pct(20.0, TICKS);
    } else if p.state == State::FireB && anim.finished() {
        p.state = State::Stand;
    }

    // Release due shots of the current fan.
    if let Some(mut pending) = pending {
        let poison = game.ammo_poison.max(1.0);
        let spawn = pending.spawn;
        let pan = p.pan;
        pending.shots.retain_mut(|(left, off)| {
            *left -= t;
            if *left > 0.0 {
                return true;
            }
            spawn_wasp(&mut commands, &assets, &mut meshes, &mut sfx, kind, poison, spawn, pan + *off, e);
            false
        });
        if pending.shots.is_empty() {
            commands.entity(e).remove::<PendingShots>();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_wasp(
    commands: &mut Commands,
    assets: &GameAssets,
    meshes: &mut Assets<Mesh>,
    sfx: &mut Sfx,
    kind: u32,
    poison: f32,
    pos: [f32; 3],
    pan: f32,
    shooter: Entity,
) {
    let Some(model) = assets.model("wasp1.mdl") else { return };
    let (scale, speed, homing_range, tilt_gain, bounce_force, life) = FIRED[(kind - 1) as usize];
    let climb = match kind {
        5 => Some([3.0, 5.0, 5.0, 0.2]),
        6 => Some([4.0, 8.0, 10.0, 0.1]),
        _ => None,
    };
    let mut anim = ModelAnimator::new(model.clone(), Vec::new(), 8.0);
    anim.play("walk", true, 0.0);
    anim.set_speed_pct(0.5 * speed, TICKS);
    sfx.play("Wasp_son");
    commands.spawn((
        Name::new("wasp"),
        crate::flow::LevelScoped,
        Mesh3d(meshes.add(model.build_mesh(anim.frames.first().copied().unwrap_or(0)))),
        MeshMaterial3d(model.skin(kind)),
        Transform { translation: coords::pos(pos), rotation: coords::rotation([pan, 0.0, 0.0]), scale: Vec3::splat(scale) },
        Visibility::Hidden,
        anim,
        Wasp { kind, poison, pan, tilt: 0.0, pos, speed, homing_range, tilt_gain, bounce_force, life, climb, last_disp: [0.0; 3], find_cd: 0.0, shooter: Some(shooter) },
    ));
}

fn forward(pan: f32, tilt: f32) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

#[allow(clippy::too_many_arguments)]
pub fn fly_wasps(
    time: Res<Time>,
    world: CollisionWorld,
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    mut hits: MessageWriter<WaspHit>,
    mut doors: Query<&mut WeakDoor>,
    player: Query<&Player>,
    mut wasps: Query<(Entity, &mut Wasp, &mut Transform, &mut Visibility)>,
) {
    let t = time.delta_secs() * TICKS;
    let sammy = player.single().map(|p| p.pos).ok();
    for (e, mut w, mut tf, mut vis) in &mut wasps {
        // Types 5/6: climb, hold, ease to level, then a slow dive (HandleVspeed, bullets.wdl:922-948).
        if let Some(mut c) = w.climb {
            if c[0] > 0.0 {
                c[0] -= t;
                w.tilt += 20.0 * t;
            } else if c[1] > 0.0 {
                c[1] -= t;
            } else if c[2] > 0.0 && w.tilt > 1.0 {
                c[2] -= t;
                w.tilt -= w.tilt * (0.2 * t).min(0.2);
                if c[2] <= 0.0 {
                    w.tilt = 0.0;
                }
            } else {
                w.tilt -= c[3] * t;
            }
            w.climb = Some(c);
        }
        let dir = forward(w.pan, w.tilt);
        let target = [0, 1, 2].map(|i| w.pos[i] + dir[i] * w.speed * t);
        let hit = world.trace(0, w.pos, target, w.shooter);
        w.last_disp = [0, 1, 2].map(|i| hit.trace.end[i] - w.pos[i]);
        w.pos = hit.trace.end;
        // Invisible until 80 quants from Sammy (it starts behind him).
        if *vis == Visibility::Hidden && sammy.is_some_and(|s| (0..3).map(|i| (s[i] - w.pos[i]).powi(2)).sum::<f32>().sqrt() > 80.0) {
            *vis = Visibility::Inherited;
        }
        tf.translation = coords::pos(w.pos);
        tf.rotation = coords::rotation([w.pan, w.tilt, 0.0]);
        w.life -= t;
        w.find_cd -= t;
        let died = if hit.trace.hit() || hit.trace.start_solid {
            if let Some(target) = hit.entity {
                if let Ok(mut door) = doors.get_mut(target) {
                    door.hp = (door.hp - w.poison).trunc();
                } else {
                    hits.write(WaspHit { target, poison: w.poison, bounce_force: w.bounce_force, wasp_pan: w.pan, wasp_disp: w.last_disp });
                }
            }
            true
        } else {
            w.life <= 0.0
        };
        if died {
            let (at, kind) = (w.pos, w.kind);
            crate::effects::queue(&mut commands, move |ps| crate::effects::wasp_explode(ps, at, kind, 2.0));
            let n = rng.random(3.0) as usize;
            sfx.play(["WaspKaputt_son", "WaspKaputt2_son", "WaspKaputt3_son"][n.min(2)]);
            commands.entity(e).despawn();
        }
    }
}

/// `ExplodeDoorWeak`: HP 3/2/1 morph to doorwk2/3/4; otherwise the door breaks into planks.
#[allow(clippy::too_many_arguments)]
pub fn update_weak_doors(
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut doors: Query<(Entity, &WeakDoor, &mut Brush, &LevelObject, Option<&Children>), Changed<WeakDoor>>,
) {
    for (e, door, mut brush, obj, children) in &mut doors {
        let morph = match door.hp as i32 {
            1 => Some("doorwk4.wmb"),
            2 => Some("doorwk3.wmb"),
            3 => Some("doorwk2.wmb"),
            4.. => continue, // untouched (first change is the insert)
            _ => None,
        };
        if let Some(file) = morph {
            sfx.play("plank1_Son");
            let Some((wmb, parts)) = assets.prefab(file) else { continue };
            if !std::sync::Arc::ptr_eq(&brush.wmb, &wmb) {
                brush.wmb = wmb;
                for c in children.into_iter().flatten() {
                    commands.entity(*c).despawn();
                }
                commands.entity(e).with_children(|c| {
                    for (mesh, mat) in parts {
                        c.spawn((Mesh3d(mesh), MeshMaterial3d(mat)));
                    }
                });
            }
            continue;
        }
        // Destroyed: the door vanishes and six planks fly out (bullets of LaunchPlank).
        let o = obj.0.origin;
        commands.entity(e).despawn();
        for file in ["plank1.mdl", "plank2.mdl", "plank3.mdl", "plank1.mdl", "plank2.mdl", "plank3.mdl"] {
            let Some(model) = assets.model(file) else { continue };
            let pos = [o[0] + rng.random(100.0) - 50.0, o[1] + rng.random(100.0) - 50.0, o[2] + rng.random(300.0)];
            let angles = [rng.random(360.0), 0.0, 0.0];
            commands.spawn((
                Name::new("plank"),
                crate::flow::LevelScoped,
                Mesh3d(meshes.add(model.build_mesh(0))),
                MeshMaterial3d(model.material.clone()),
                Transform { translation: coords::pos(pos), rotation: coords::rotation(angles), scale: Vec3::ONE },
                Plank {
                    v: [rng.random(40.0) - 20.0, rng.random(40.0) - 20.0, 30.0],
                    spin: [30.0 * rng.random(2.0) - 1.0, 30.0 * rng.random(2.0) - 1.0],
                    angles,
                    pos,
                    still: 24.0,
                },
            ));
        }
    }
}

/// Simplified `LaunchPlank`: ballistic tumble with bounces, settle flat, then vanish.
pub fn fly_planks(time: Res<Time>, world: CollisionWorld, mut sfx: Sfx, mut commands: Commands, mut q: Query<(Entity, &mut Plank, &mut Transform)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut p, mut tf) in &mut q {
        p.v[2] -= 1.5 * t;
        let target = [0, 1, 2].map(|i| p.pos[i] + p.v[i] * t);
        let hit = world.trace(0, p.pos, target, None).trace;
        p.pos = hit.end;
        if hit.hit() && hit.normal[2] > 0.5 {
            if p.v[2].abs() > 6.0 {
                sfx.play("plank1_Son");
            }
            p.v[2] = -p.v[2] * 0.5;
            p.v[0] *= 0.6;
            p.v[1] *= 0.6;
        }
        let speed = p.v[0].abs() + p.v[1].abs();
        if speed < 2.0 && hit.hit() {
            p.still -= t;
            p.angles[1] += (90.0 - p.angles[1]) * (0.5 * t).min(1.0);
        } else {
            p.angles[1] += p.spin[0] * t;
            p.angles[2] += p.spin[1] * t;
        }
        tf.translation = coords::pos(p.pos);
        tf.rotation = coords::rotation(p.angles);
        if p.still <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// Hive (`InitHive`/`EmitWasp`, bullets.wdl:1197-1321; docs/wasps.md §8).
#[derive(Component)]
pub struct Hive {
    /// Skill1: launched wasp fate (0 decorative, 1-999 lifetime ticks, 1000 stays, 1001 frozen).
    fate: f32,
    /// Skill2: random pan spread.
    spread: f32,
    /// int(Skill3): wasps left (9990 = unlimited); frac×100: cap on Sammy's ammo.
    total: f32,
    cap: i32,
    /// Skill4 / Skill7: vertical and horizontal launch speed.
    vz: f32,
    hspeed: f32,
    /// Skill6: wasp type (999 = random 1-6).
    kind: u32,
    /// Skill8: emission period (0 = never).
    period: f32,
    timer: f32,
    pan: f32,
    pos: [f32; 3],
}

pub fn init_hive(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    let cap = ((s[2] - s[2].trunc()) * 100.0 + 0.5).trunc() as i32;
    let total = s[2].trunc();
    entity.insert(Hive {
        fate: s[0],
        spread: s[1],
        total: if total == 0.0 { 9990.0 } else { total },
        cap: if cap == 0 { 10 } else { cap },
        vz: s[3],
        hspeed: s[6],
        kind: s[5] as u32,
        period: s[7],
        timer: 32.0 + s[7], // `waitt 32` before the first period
        pan: obj.angles[0],
        pos: obj.origin,
    });
}

/// A wasp launched by a hive, bouncing until it settles into a pickup.
#[derive(Component)]
pub struct HiveWasp {
    pub kind: u32,
    pos: [f32; 3],
    pan: f32,
    tilt: f32,
    speed: f32,
    vz: f32,
    vz_org: f32,
    bounces: f32,
    fate: f32,
}

impl HiveWasp {
    /// `Particle_WaspHive` trail while it still rises (`BlobVspeedOrg > 5`, bullets.wdl:1139).
    pub fn trail(&self) -> Option<(u32, [f32; 3])> {
        (self.vz_org > 5.0).then_some((self.kind, self.pos))
    }
}

/// Remaining lifetime of a hive-emitted pickup (fades over its last 16 ticks).
#[derive(Component)]
pub struct PickupLifetime(f32);

/// Hive wasp pickup table (`initSkinScaleWaspHive2`): (poison, scale).
const HIVE_PICKUP: [(f32, f32); 7] = [(3.0, 1.0), (2.0, 1.0), (1.0, 0.8), (1.0, 0.8), (4.0, 1.0), (5.0, 1.0), (1.0, 0.8)];

#[allow(clippy::too_many_arguments)]
pub fn run_hives(
    time: Res<Time>,
    mut commands: Commands,
    game: Res<GameState>,
    mut rng: ResMut<Rng>,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut hives: Query<(Entity, &mut Hive, &Transform)>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut h, tf) in &mut hives {
        if h.period <= 0.0 {
            continue;
        }
        h.timer -= t;
        if h.timer > 0.0 {
            continue;
        }
        h.timer += h.period;
        if game.ammo >= h.cap && game.ammo_type == h.kind {
            continue; // capped: the hive looks empty
        }
        let kind = if h.kind == 999 { rng.random(6.0) as u32 + 1 } else { h.kind.clamp(1, 7) };
        let pan = if h.spread != 0.0 { h.pan + rng.random(h.spread) - rng.random(h.spread * 0.5) } else { h.pan };
        let launch = [h.hspeed + rng.random(5.0), h.vz + rng.random(4.0)];
        h.total -= 1.0;
        let Some(model) = assets.model("wasp1.mdl") else { continue };
        let mut anim = ModelAnimator::new(model.clone(), Vec::new(), 8.0);
        anim.play("walk", true, 0.0);
        anim.set_speed_pct(20.0, TICKS);
        let scale = HIVE_PICKUP[(kind - 1) as usize].1;
        let tilt = rng.random(20.0) + 20.0;
        commands.spawn((
            Name::new("hive wasp"),
            crate::flow::LevelScoped,
            Mesh3d(meshes.add(model.build_mesh(0))),
            MeshMaterial3d(model.skin(kind)),
            Transform { translation: coords::pos(h.pos), rotation: coords::rotation([pan, tilt, 0.0]), scale: Vec3::splat(scale) },
            anim,
            HiveWasp {
                kind,
                pos: h.pos,
                pan,
                tilt,
                speed: (launch[0].abs() + 5.0).max(15.0),
                vz: launch[1],
                vz_org: launch[1],
                bounces: h.fate, // bounce counter shares the lifetime slot (original quirk)
                fate: h.fate,
            },
        ));
        if h.total <= 0.0 {
            // Exhausted: the hive bursts (bullets.wdl:1255-1272).
            let at = crate::coords::native(tf.translation);
            crate::effects::queue(&mut commands, move |ps| crate::effects::hive_burst(ps, at));
            commands.entity(e).despawn();
        }
    }
}

/// `LaunchNewWasp` flight: ballistic hops with damped bounces until it settles,
/// then it becomes a 1-ammo pickup with a lifetime of fate + bounces.
pub fn fly_hive_wasps(
    time: Res<Time>,
    world: CollisionWorld,
    mut commands: Commands,
    mut sfx: Sfx,
    mut rng: ResMut<Rng>,
    mut q: Query<(Entity, &mut HiveWasp, &mut Transform)>,
) {
    let t = time.delta_secs() * TICKS;
    for (e, mut w, mut tf) in &mut q {
        if w.fate == 1001.0 {
            continue; // frozen at spawn forever
        }
        let ground = {
            let tr = world.trace_static(0, w.pos, [w.pos[0], w.pos[1], w.pos[2] - 2000.0]).trace;
            if tr.hit() { tr.end[2] } else { w.pos[2] - 2000.0 }
        };
        if w.vz == 0.0 && w.pos[2] - ground > 26.0 {
            w.vz = -2.0;
        }
        let mut vstep = 0.0;
        if w.vz != 0.0 {
            w.vz -= 1.5 * t;
            if w.pos[2] < ground + 25.0 {
                w.bounces += 1.0;
                if w.speed > 0.5 {
                    if w.vz_org > 15.0 {
                        sfx.play(["WaspKaputt_son", "WaspKaputt2_son", "WaspKaputt3_son"][(rng.random(3.0) as usize).min(2)]);
                    }
                    w.vz_org *= 0.8;
                    w.vz = w.vz_org - 1.5 * t;
                    vstep = w.vz * t;
                } else {
                    vstep = ground + 25.0 - w.pos[2];
                    w.vz = 0.0;
                }
            } else {
                vstep = w.vz * t;
            }
        }
        w.tilt -= if w.bounces == 0.0 { 2.0 * t } else { w.tilt * 0.1 * t };
        let (s, c) = w.pan.to_radians().sin_cos();
        let d = [c * w.speed * t.min(1.0), s * w.speed * t.min(1.0), vstep];
        let hit = world.trace_static(0, w.pos, [w.pos[0] + d[0], w.pos[1] + d[1], w.pos[2] + d[2]]);
        let before = w.pos;
        w.pos = hit.trace.end;
        if hit.trace.hit() && hit.trace.normal[2].abs() < 0.7 {
            // Wasp_Bounce: turn off the wall, lose speed.
            let n = hit.trace.normal;
            w.pan = n[1].atan2(n[0]).to_degrees();
            w.speed = (w.speed - 8.0).max(0.1);
        }
        w.speed -= 0.05 * t.min(1.0);
        let moved = (w.pos[0] - before[0]).abs() + (w.pos[1] - before[1]).abs();
        let mut settled = false;
        if moved < 1.0 {
            w.vz_org -= 0.5 * t.min(1.0);
            settled |= w.vz_org.abs() < 1.5;
        } else {
            w.vz_org -= 0.02 * t.min(1.0);
        }
        if w.speed < 1.0 {
            w.speed = 1.0;
            settled |= w.vz_org.abs() < 1.5;
        }
        tf.translation = coords::pos(w.pos);
        tf.rotation = coords::rotation([w.pan, w.tilt, 0.0]);
        if settled {
            let (poison, _) = HIVE_PICKUP[(w.kind - 1) as usize];
            let mut ent = commands.entity(e);
            ent.remove::<HiveWasp>().insert((
                AmmoPickup { kind: w.kind, amount: 1, poison },
                PickupPos(w.pos),
                Spinning { angles: [w.pan, 20.0, 0.0] },
            ));
            if w.fate > 0.0 && w.fate < 1000.0 {
                ent.insert(PickupLifetime(w.fate + w.bounces));
            }
        }
    }
}

/// Hive pickups fade over their last 16 ticks, then disappear.
pub fn expire_pickups(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut PickupLifetime, &mut Transform, Option<&AmmoPickup>)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut life, mut tf, ammo) in &mut q {
        life.0 -= t;
        if life.0 < 16.0 {
            tf.scale *= 0.85f32.powf(t);
        }
        if life.0 <= 0.0 {
            // `InitWaspWed` countdown over: the wasp bursts (bullets.wdl:80-108).
            if let Some(a) = ammo {
                let (at, kind) = (crate::coords::native(tf.translation), a.kind);
                crate::effects::queue(&mut commands, move |ps| crate::effects::wasp_explode(ps, at, kind, 2.0));
            }
            commands.entity(e).despawn();
        }
    }
}
