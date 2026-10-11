//! Moving platforms (`InitEnginePlatForm` / `InitPlatForm` / `InitBodyPlatForm`,
//! mm1.wdl:2238-2629) and teleport pads (`InitTeleport` / `Teleport_Hit`, mm1.wdl:4099-4300).
//!
//! An invisible *engine* runs along its local +X at Skill2 quants/tick through
//! hidden rails in the level. On impact it either reflects (Flag3) or turns by
//! Skill4/5/7 and pauses Skill3 ticks. Platforms and bodies with the same link id
//! (Skill1) copy the engine's motion at a fixed offset, carrying whatever stands
//! on them. An engine with Skill6 is locked until Sammy stands on its platform
//! holding a matching key.

use crate::collision::Hull;
use bevy::prelude::*;
use formats::wmb::Object;

use super::health::set_alpha;
use super::player::{LevelCollision, Player, State, TICKS};
use crate::audio::Sfx;
use crate::camera::FollowState;
use crate::collision::Brush;
use crate::coords;
use crate::game::{GameState, MASTER_KEY};

/// Default link id (`8465`).
const DEFAULT_LINK: f32 = 8465.0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum EngineState {
    Moving,
    Paused(f32),
}

#[derive(Component)]
pub struct Engine {
    link: f32,
    pos: [f32; 3],
    angles: [f32; 3],
    /// Skill2: quants per tick along local +X.
    speed: f32,
    /// Skill3: pause after an impact.
    pause: f32,
    /// Skill4/5/7: pan/tilt/roll change on impact.
    turn: [f32; 3],
    /// Flag3: reflect off walls instead of turning.
    reflect: bool,
    /// Skill6: key colour that unlocks it (0 = always running); Flag7 = exact colour.
    lock: u32,
    lock_exact: bool,
    unlocked: bool,
    state: EngineState,
    /// Motion this frame.
    delta: [f32; 3],
}

#[derive(Component)]
pub struct Platform {
    link: f32,
    engine: Option<Entity>,
    offset: [f32; 3],
    /// Motion this frame (carried onto riders).
    pub delta: [f32; 3],
}

#[derive(Component)]
pub struct Teleport {
    pub dest: [f32; 3],
    pan: f32,
    /// Flag2: lions are teleported too.
    pub lions: bool,
    /// `Skill12`: busy while someone is being teleported (shared by Sammy and lions).
    pub busy: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum TeleportStage {
    /// `waitt 4` while Sammy settles onto the pad.
    Settle(f32),
    FadeOut,
    FadeIn,
}

/// Sammy is being teleported (input locked).
#[derive(Component)]
pub struct Teleporting {
    pad: Entity,
    dest: [f32; 3],
    pan: f32,
    stage: TeleportStage,
    alpha: f32,
}

pub fn init_engine(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    entity.insert(Engine {
        link: if s[0] == 0.0 { DEFAULT_LINK } else { s[0] },
        pos: obj.origin,
        angles: obj.angles,
        speed: s[1],
        pause: s[2],
        turn: [s[3], s[4], s[6]],
        reflect: obj.flags & (1 << 2) != 0,
        lock: s[5] as u32,
        lock_exact: obj.flags & (1 << 6) != 0,
        unlocked: s[5] == 0.0,
        state: EngineState::Moving,
        delta: [0.0; 3],
    });
}

pub fn init_platform(obj: &Object, body: bool, entity: &mut EntityCommands) {
    let link = if obj.skills[0] == 0.0 && body { DEFAULT_LINK } else { obj.skills[0] };
    entity.insert(Platform { link, engine: None, offset: [0.0; 3], delta: [0.0; 3] });
}

pub fn init_teleport(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    entity.insert(Teleport { dest: [s[0], s[1], s[2]], pan: s[3], lions: obj.flags & (1 << 1) != 0, busy: false });
}

/// `SetLinkMoteur`: each platform attaches to the engine with its link id.
pub fn link_platforms(engines: Query<(Entity, &Engine)>, mut platforms: Query<(&mut Platform, &Brush), Added<Platform>>) {
    for (mut p, brush) in &mut platforms {
        if let Some((e, eng)) = engines.iter().find(|(_, eng)| eng.link == p.link) {
            p.engine = Some(e);
            p.offset = [0, 1, 2].map(|i| brush.origin[i] - eng.pos[i]);
        }
    }
}

fn forward([pan, tilt, _]: [f32; 3]) -> [f32; 3] {
    let (sp, cp) = pan.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    [cp * ct, sp * ct, st]
}

fn wrap(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

pub fn run_engines(
    time: Res<Time>,
    level: Option<Res<LevelCollision>>,
    player: Query<&Player>,
    platforms: Query<&Platform>,
    mut engines: Query<(Entity, &mut Engine)>,
) {
    let Some(level) = level else { return };
    let t = time.delta_secs() * TICKS;
    // Shift-boost while riding: the carrying engine runs ×3 (InitEnginePlatForm :2478-2491).
    let boosted = player
        .single()
        .ok()
        .filter(|p| p.boosting && p.grounded)
        .and_then(|p| p.ground_entity)
        .and_then(|g| platforms.get(g).ok())
        .and_then(|pl| pl.engine);
    for (id, mut e) in &mut engines {
        let speed = if boosted == Some(id) { e.speed * 3.0 } else { e.speed };
        e.delta = [0.0; 3];
        if !e.unlocked {
            continue;
        }
        e.state = match e.state {
            EngineState::Paused(left) if left - t > 0.0 => EngineState::Paused(left - t),
            EngineState::Paused(_) => EngineState::Moving,
            EngineState::Moving => {
                let dir = forward(e.angles);
                let target = [0, 1, 2].map(|i| e.pos[i] + dir[i] * speed * t);
                let tr = level.0.trace(0, Hull::Point.index(), e.pos, target); // tiny sprite: point collision
                if tr.start_solid {
                    EngineState::Moving
                } else {
                    e.delta = [0, 1, 2].map(|i| tr.end[i] - e.pos[i]);
                    e.pos = tr.end;
                    if !tr.hit() {
                        EngineState::Moving
                    } else if e.reflect {
                        // `To_angle Angles,BOUNCE`: face along the reflected direction.
                        let n = tr.normal;
                        let d = dir[0] * n[0] + dir[1] * n[1] + dir[2] * n[2];
                        let r = [0, 1, 2].map(|i| dir[i] - 2.0 * d * n[i]);
                        e.angles[0] = r[1].atan2(r[0]).to_degrees();
                        EngineState::Moving
                    } else {
                        // `moteurBonus_Hit`; roll uses the *new tilt* (original bug, mm1.wdl:2591).
                        e.angles[0] = wrap(e.angles[0] + e.turn[0]);
                        e.angles[1] = wrap(e.angles[1] + e.turn[1]);
                        e.angles[2] = wrap(e.angles[1] + e.turn[2]);
                        if e.pause > 0.0 { EngineState::Paused(e.pause) } else { EngineState::Moving }
                    }
                }
            }
        };
    }
}

pub fn follow_engines(engines: Query<&Engine>, mut platforms: Query<(&mut Platform, &mut Brush, &mut Transform)>) {
    for (mut p, mut brush, mut tf) in &mut platforms {
        let Some(eng) = p.engine.and_then(|e| engines.get(e).ok()) else { continue };
        let target = [0, 1, 2].map(|i| eng.pos[i] + p.offset[i]);
        p.delta = [0, 1, 2].map(|i| target[i] - brush.origin[i]);
        brush.origin = target;
        tf.translation = coords::pos(target);
    }
}

/// Riders move with the platform under them (`PlayerForce` = platform motion);
/// standing on a locked platform with the right key starts its engine.
#[allow(clippy::too_many_arguments)]
pub fn ride_platforms(
    game: Res<GameState>,
    mut sfx: Sfx,
    mut commands: Commands,
    assets: Res<crate::level::GameAssets>,
    platforms: Query<&Platform>,
    mut bodies: Query<(&mut Brush, Option<&Children>)>,
    mut engines: Query<&mut Engine>,
    mut player: Query<&mut Player>,
) {
    let Ok(mut p) = player.single_mut() else { return };
    if !p.grounded {
        return;
    }
    let Some(plat) = p.ground_entity.and_then(|g| platforms.get(g).ok()) else { return };
    for i in 0..3 {
        p.pos[i] += plat.delta[i];
    }
    if let Some(mut eng) = plat.engine.and_then(|e| engines.get_mut(e).ok())
        && !eng.unlocked
    {
        let key = game.key;
        let ok = key == MASTER_KEY
            || if eng.lock_exact { key == eng.lock } else { key & eng.lock != 0 && (key == eng.lock || matches!(key, 1 | 2 | 4)) };
        if ok {
            eng.unlocked = true;
            sfx.play("platform1_son");
            // The platform turns into the plain wooden one (`morph <platfor1.wmb>`).
            if let Some(g) = p.ground_entity
                && let Ok((mut brush, children)) = bodies.get_mut(g)
            {
                super::door::morph(&mut commands, g, &mut brush, children, &assets, "platfor1.wmb");
            }
        }
    }
}

/// `Teleport_Hit`: Sammy standing on a pad is faded out, moved and faded back in.
pub fn enter_teleports(mut commands: Commands, mut pads: Query<&mut Teleport>, player: Query<(Entity, &Player), Without<Teleporting>>) {
    let Ok((e, p)) = player.single() else { return };
    if !p.grounded || p.vz != 0.0 || p.state == State::Dead {
        return;
    }
    if let Some(g) = p.ground_entity
        && let Ok(mut pad) = pads.get_mut(g)
        && !pad.busy
    {
        pad.busy = true;
        commands.entity(e).insert(Teleporting { pad: g, dest: pad.dest, pan: pad.pan, stage: TeleportStage::Settle(4.0), alpha: 100.0 });
    }
}

pub fn run_teleports(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut follow: Query<&mut FollowState>,
    mut pads: Query<&mut Teleport>,
    mut player: Query<(Entity, &mut Player, &mut Teleporting, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Ok((e, mut p, mut tp, mat)) = player.single_mut() else { return };
    let t = time.delta_secs() * TICKS;
    p.state = State::Dead; // PlayerMoveMode = 0
    tp.stage = match tp.stage {
        TeleportStage::Settle(left) if left - t > 0.0 => TeleportStage::Settle(left - t),
        TeleportStage::Settle(_) => TeleportStage::FadeOut,
        TeleportStage::FadeOut => {
            tp.alpha = (tp.alpha - 10.0 * t).max(0.0);
            if tp.alpha <= 0.0 {
                p.teleport(tp.dest, tp.pan);
                for mut f in &mut follow {
                    f.snap_to([tp.dest[0] + 32.0, tp.dest[1] + 32.0, tp.dest[2] + 150.0]);
                }
                TeleportStage::FadeIn
            } else {
                TeleportStage::FadeOut
            }
        }
        TeleportStage::FadeIn => {
            tp.alpha = (tp.alpha + 10.0 * t).min(100.0);
            if tp.alpha >= 100.0 {
                p.state = State::Stand;
                if let Ok(mut pad) = pads.get_mut(tp.pad) {
                    pad.busy = false;
                }
                commands.entity(e).remove::<Teleporting>();
            }
            TeleportStage::FadeIn
        }
    };
    set_alpha(&mut materials, mat, tp.alpha);
}
