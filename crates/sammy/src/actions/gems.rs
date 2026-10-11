//! Gem machinery beyond the plain pickup (docs/level-objects.md §2.1-2.5):
//! thrown-back keys (`LaunchYouKey`), gems launched by objects (`EjectNewKey` /
//! `LaunchNewKey`: boulders, Sammy's death gem), lifetimes with blinking,
//! gravity/hover for Flag7 gems, surprise colours, boulders, and the gem that
//! flies into a permanent door (`CreateGemKeyAnim` / `AnimGemKey`).

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::gems::{HEIGHT_GEM, Rest};

use super::pickups::{GemSprites, KeyGem, LevelKey, PickupPos, Respawnable, Shrinking};
use super::player::{JumpParams, Player, TICKS};
use super::platform::Platform;
use super::wasp::WaspHit;
use crate::audio::Sfx;
use crate::collision::{CollisionWorld, Hull};
use crate::coords;
use crate::game::Rng;
use crate::level::{Billboard, SpriteAnim};

/// The gem can be picked up by Sammy's scan (`InitNewKey` ran).
#[derive(Component)]
pub struct Collectable;

/// A gem in flight ([`sammy_core::gems::FlyingGem`]).
#[derive(Component, Deref, DerefMut)]
pub struct FlyingGem(pub sammy_core::gems::FlyingGem);

/// Remaining lifetime of a runtime gem ([`sammy_core::gems::GemLifetime`]).
#[derive(Component, Deref, DerefMut)]
pub struct GemLifetime(pub sammy_core::gems::GemLifetime);

/// Flag7 gem gravity ([`sammy_core::gems::GemGravity`]).
#[derive(Component, Deref, DerefMut)]
pub struct GemGravity(pub sammy_core::gems::GemGravity);

impl GemGravity {
    pub fn new(rest: f32) -> Self {
        Self(sammy_core::gems::GemGravity { rest })
    }
}

/// Surprise gem ([`sammy_core::gems::Surprise`]).
#[derive(Component, Deref, DerefMut)]
pub struct Surprise(pub sammy_core::gems::Surprise);

impl Surprise {
    pub fn new(add: f32, seed: f32) -> Self {
        Self(sammy_core::gems::Surprise::new(add, seed))
    }
}

/// Decorative gem (fate 0): shrinks and bursts once it stops.
#[derive(Component)]
pub struct Decorative;

/// Gem sprite animation speed (frames per second), backwards for door gems.
const GEM_FPS: f32 = 48.0;
/// Size of a flying / runtime gem, and while blinking small.
const GEM_SCALE: f32 = 1.5;
const BLINK_SCALE: f32 = 0.75;

fn sprite_bundle(sprites: &GemSprites, color: u32, pos: [f32; 3], scale: f32) -> Option<impl Bundle> {
    let (mesh, mat, frames) = sprites.0.get(&color).cloned()?;
    Some((
        crate::flow::LevelScoped,
        Transform { translation: coords::pos(pos), scale: Vec3::splat(scale), ..default() },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Billboard,
        SpriteAnim { frames, frame: 0.0, fps: GEM_FPS },
        KeyGem { color },
        PickupPos(pos),
    ))
}

/// `EjectOldKey` + `LaunchYouKey`: the old key flies forward at 2× Sammy's walk speed.
pub fn throw_back_key(commands: &mut Commands, sprites: &GemSprites, color: u32, p: &Player, life: f32) {
    let gem = sammy_core::gems::FlyingGem::thrown(p.pos, p.pan, p.speed, life);
    let Some(b) = sprite_bundle(sprites, color, gem.pos, GEM_SCALE) else { return };
    commands.spawn((Name::new("thrown key"), b, FlyingGem(gem)));
}

/// `EjectNewKey` → `LaunchNewKey`: a gem launched from `pos` (boulders, Sammy's death).
pub fn launch_gem(commands: &mut Commands, sprites: &GemSprites, jump: &JumpParams, l: sammy_core::gems::Launch) {
    let gem = sammy_core::gems::FlyingGem::launched(l.pos, l.pan, l.hspeed, l.vz, l.fate, jump);
    let Some(b) = sprite_bundle(sprites, l.color, l.pos, GEM_SCALE) else { return };
    commands.spawn((Name::new("launched gem"), b, FlyingGem(gem)));
}

/// After a flight: make the gem collectable and give it its resting behaviour.
fn settle(commands: &mut Commands, e: Entity, g: &sammy_core::gems::FlyingGem) {
    let mut ent = commands.entity(e);
    ent.remove::<FlyingGem>();
    match g.rest() {
        Rest::Collectable { lifetime } => {
            ent.insert(Collectable);
            if let Some(l) = lifetime {
                ent.insert(GemLifetime(sammy_core::gems::GemLifetime::new(l)));
            }
        }
        Rest::Decorative => {
            ent.insert((Decorative, Shrinking));
        }
        Rest::Gravity { lifetime, respawns } => {
            ent.insert((Collectable, GemGravity::new(HEIGHT_GEM)));
            if let Some(l) = lifetime {
                ent.insert(GemLifetime(sammy_core::gems::GemLifetime::new(l)));
            } else if respawns {
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
    mut sounds: Local<Vec<&'static str>>,
) {
    // The original loops run per frame with `mintime = min(time, 1)`.
    let t = (time.delta_secs() * TICKS).min(1.0);
    for (e, mut g, mut pp, mut tf, collectable) in &mut q {
        let step = g.step(collectable, &jump, &mut rng, t, |a, b| world.trace_static(Hull::Point, a, b).trace, &mut sounds);
        for s in sounds.drain(..) {
            sfx.play(s);
        }
        if step.collectable {
            commands.entity(e).insert(Collectable);
        }
        pp.0 = g.pos;
        tf.translation = coords::pos(g.pos);
        if step.done {
            settle(&mut commands, e, &g);
        }
    }
}

/// Resting runtime gems: lifetime blink and burst (GemWaitForPlayer with Skill9).
/// The blink follows the sprite animation: small in the second half of each cycle.
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
            tf.scale = Vec3::splat(BLINK_SCALE);
            sfx.play("cligno1_son");
        } else if anim.frame < half && l.small {
            l.small = false;
            l.speed += sammy_core::gems::BLINK_SPEEDUP * t;
            tf.scale = Vec3::splat(GEM_SCALE);
            sfx.play("cligno1_son");
        }
        if l.tick(t) {
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
        p.0[2] = g.fall(p.0[2], hit.trace.end[2], &jump, t);
        tf.translation = coords::pos(p.0);
    }
}

/// Surprise gems: the colour climbs after a 1-8 tick delay, then they can be collected.
pub fn surprise_gems(time: Res<Time>, mut commands: Commands, mut rng: ResMut<Rng>, mut q: Query<(Entity, &mut Surprise, &mut KeyGem)>) {
    let t = time.delta_secs() * TICKS;
    for (e, mut s, mut gem) in &mut q {
        if let Some(color) = s.step(gem.color, &mut rng, t) {
            gem.color = color;
            commands.entity(e).insert(Collectable);
        }
    }
}

/// Boulder emitting gems ([`sammy_core::gems::Boulder`]).
#[derive(Component, Deref, DerefMut)]
pub struct Boulder(pub sammy_core::gems::Boulder);

pub fn init_boulder(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(Boulder(sammy_core::gems::Boulder::from_object(obj)));
}

pub fn run_boulders(
    time: Res<Time>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    sprites: Res<GemSprites>,
    jump: Res<JumpParams>,
    mut hits: MessageReader<WaspHit>,
    mut boulders: Query<&mut Boulder>,
) {
    let t = time.delta_secs() * TICKS;
    // Impact mode: a wasp hitting the boulder launches a gem back toward the shooter.
    for h in hits.read() {
        let Ok(b) = boulders.get(h.target) else { continue };
        if let Some(l) = b.hit_by_wasp(h.wasp_pan, h.wasp_disp, &mut rng) {
            launch_gem(&mut commands, &sprites, &jump, l);
        }
    }
    // Auto mode: every Skill8 ticks.
    for mut b in &mut boulders {
        if let Some(l) = b.step(&mut rng, t) {
            launch_gem(&mut commands, &sprites, &jump, l);
        }
    }
}

/// Gem flying from Sammy into a permanent door ([`sammy_core::gems::DoorGem`]).
#[derive(Component, Deref, DerefMut)]
pub struct DoorGem {
    pub door: Entity,
    #[deref]
    pub gem: sammy_core::gems::DoorGem,
}

pub fn spawn_door_gem(commands: &mut Commands, sprites: &GemSprites, door: Entity, color: u32, sammy: [f32; 3]) {
    let gem = sammy_core::gems::DoorGem::new(color, sammy);
    let Some((mesh, mat, frames)) = sprites.0.get(&color).cloned() else { return };
    commands.spawn((
        Name::new("door gem"),
        crate::flow::LevelScoped,
        Transform::from_translation(coords::pos(gem.pos)),
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Billboard,
        SpriteAnim { frames, frame: 0.0, fps: -GEM_FPS },
        DoorGem { door, gem },
    ));
}
