//! Lions, scorpions, the elephant and wasp enemies as Bevy entities: the AI is
//! [`sammy_core::enemies`]; these systems feed it the frame and show the result.

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::enemies::{CALL_RINGS, Impact, Out, Shared, World};

use super::player::{Player, PlayerTouched, State as PlayerState, TICKS};
use super::wasp::{Wasp, WaspHit};
use crate::audio::Sfx;
use crate::collision::{BoxCollider, CollisionWorld, id};
use crate::coords;
use crate::game::{GameState, Rng};
use crate::model::ModelAnimator;

/// [`sammy_core::enemies::LionGlobals`] as a resource.
#[derive(Resource, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct LionGlobals(pub sammy_core::enemies::LionGlobals);

/// [`sammy_core::enemies::FriendCalls`] as a resource (not reset per level).
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct FriendCalls(pub sammy_core::enemies::FriendCalls);

/// The 16 Hz AI clock ([`sammy_core::enemies::AiClock`]).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct AiClock(pub sammy_core::enemies::AiClock);

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

/// Patrol path node ([`sammy_core::enemies::PathNode`]).
#[derive(Component, Deref)]
pub struct PathNode(pub sammy_core::enemies::PathNode);

pub fn init_path(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(PathNode(sammy_core::enemies::PathNode::from_object(obj)));
}

/// An enemy ([`sammy_core::enemies::Enemy`]).
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Enemy(pub sammy_core::enemies::Enemy);

pub fn init_lion(obj: &mut Object, entity: &mut EntityCommands) -> Option<u32> {
    let (e, skin) = sammy_core::enemies::init_lion(obj);
    entity.insert(Enemy(e));
    skin
}

/// `InitWaspEnnemy` / `InitWaspExtro`: returns the skin (the scale override is applied
/// to `obj`).
pub fn init_wasp_ennemy(obj: &mut Object, entity: &mut EntityCommands, extro: bool) -> Option<u32> {
    let (e, skin, scale) = sammy_core::enemies::init_wasp_ennemy(obj, extro);
    if let Some(k) = scale {
        obj.scale = [k; 3];
    }
    entity.insert(Enemy(e));
    skin
}

pub fn advance_clock(time: Res<Time>, current: Res<crate::flow::CurrentLevel>, mut clock: ResMut<AiClock>) {
    clock.advance(time.delta_secs() * TICKS, crate::flow::playing(&current));
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
    (platforms, mut pads, pov): (Query<(Entity, &super::platform::Platform)>, Query<(Entity, &mut super::platform::Teleport)>, Res<super::extro::Pov>),
) {
    let touched: Vec<sammy_core::id::Id> = touches.read().map(|t| id(t.0)).collect();
    // Wasp hits (EVENT_IMPACT on the lion, §5.1) are applied as they arrive.
    for h in wasp_hits.read() {
        if let Ok((_, mut e, ..)) = enemies.get_mut(h.target) {
            sammy_core::enemies::apply_impact(&mut e, &Impact { poison: h.poison, bounce_force: h.bounce_force, wasp_pan: h.wasp_pan, wasp_disp: h.wasp_disp });
        }
    }
    // Distances and line of sight are measured to `pov_Syn` (lion.wdl:948-950).
    let nodes: Vec<sammy_core::enemies::PathNode> = nodes.iter().map(|n| n.0.clone()).collect();
    let platforms: Vec<_> = platforms.iter().map(|(e, p)| (id(e), p.delta)).collect();
    let mut pad_list: Vec<_> = pads.iter().map(|(e, p)| (id(e), p.0.clone())).collect();
    let (sammy_entity, sammy_alive) = match player.single() {
        Ok((e, p)) => (Some(id(e)), p.state != PlayerState::Dead),
        Err(_) => (None, false),
    };
    let view = World {
        space: &world,
        sammy: pov.entity.map(|e| (e, pov.pos, pov.vel)),
        sammy_entity,
        sammy_alive,
        touched: &touched,
        nodes: &nodes,
        platforms: &platforms,
        ticks1: clock.ticks1,
    };
    let mut out = Out::default();
    for _ in 0..clock.ticks {
        for (me, mut e, mut anim, mut vis) in &mut enemies {
            let mut shared = Shared { g: &globals, rng: &mut rng.0, calls: &mut calls.0, game: &mut game.0, pads: &mut pad_list };
            let ModelAnimator { model, anim } = &mut *anim;
            let p = player.single_mut().ok().map(|(_, p)| p.into_inner());
            sammy_core::enemies::tick(&mut e, id(me), anim, &model.mdl, p.map(|p| &mut p.0), &view, &mut shared, &mut out);
            if e.blinking() {
                *vis = if e.hidden { Visibility::Hidden } else { Visibility::Inherited };
            }
        }
        sammy_core::enemies::end_tick(&mut calls);
    }
    for (pad_e, mut pad) in &mut pads {
        if let Some((_, p)) = pad_list.iter().find(|(i, _)| *i == id(pad_e)) {
            pad.busy = p.busy;
        }
    }
    for s in out.sounds {
        sfx.play(s);
    }
    for (at, n) in out.hit_stars {
        crate::effects::queue(&mut commands, move |ps| crate::effects::hit_stars(ps, at, n));
    }
    if out.warn {
        warnings.write(WarningAttack);
    }
    for p in out.call_rings {
        for (dz, delay, skin) in CALL_RINGS {
            commands.spawn((CallRing { delay, scale: 0.01, frame: 1.0, origin: [p[0], p[1], p[2] + dz], skin }, crate::flow::LevelScoped));
        }
    }
}

/// Smooth rendering between 16 Hz AI ticks.
pub fn place_enemies(clock: Res<AiClock>, mut q: Query<(&Enemy, &mut Transform)>) {
    let a = clock.acc.clamp(0.0, 1.0);
    for (e, mut tf) in &mut q {
        if let Some((pos, angles)) = e.pose(a) {
            tf.translation = coords::pos(pos);
            tf.rotation = coords::rotation(angles);
        }
    }
}

/// Wasp homing and fear tagging.
pub fn home_wasps(time: Res<Time>, mut wasps: Query<&mut Wasp>, mut enemies: Query<&mut Enemy>) {
    let t = time.delta_secs() * TICKS;
    for mut w in &mut wasps {
        let mut list: Vec<Mut<Enemy>> = enemies.iter_mut().collect();
        let mut refs: Vec<&mut sammy_core::enemies::Enemy> = list.iter_mut().map(|e| &mut e.0).collect();
        sammy_core::enemies::home_wasp(&mut w, &mut refs, t);
    }
}

/// Moves enemy boxes with their owners; dead enemies stop colliding.
pub fn sync_enemy_colliders(mut q: Query<(&Enemy, &mut BoxCollider)>) {
    for (e, mut c) in &mut q {
        if let Some((center, _, solid)) = e.collider() {
            c.center = center;
            c.solid = solid;
        }
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

/// `InitStartEnnemy` ([`sammy_core::enemies::StartTrigger`]).
#[derive(Component, Deref, DerefMut)]
pub struct StartTrigger(sammy_core::enemies::StartTrigger);

pub fn init_start_ennemy(obj: &Object, entity: &mut EntityCommands) {
    entity.insert(StartTrigger(sammy_core::enemies::StartTrigger::from_object(obj)));
}

pub fn run_start_triggers(clock: Res<AiClock>, mut triggers: Query<&mut StartTrigger>, mut enemies: Query<&mut Enemy>) {
    for _ in 0..clock.ticks {
        for mut tr in &mut triggers {
            let mut list: Vec<Mut<Enemy>> = enemies.iter_mut().collect();
            let mut refs: Vec<&mut sammy_core::enemies::Enemy> = list.iter_mut().map(|e| &mut e.0).collect();
            tr.tick(&mut refs);
        }
    }
}

/// Adds the collision box once the model is known.
pub fn add_enemy_colliders(mut commands: Commands, q: Query<(Entity, &Enemy), Without<BoxCollider>>) {
    for (ent, e) in &q {
        if let Some((center, half, _)) = e.collider() {
            commands.entity(ent).insert(BoxCollider(sammy_core::collision::BoxCollider { center, half, solid: true }));
        }
    }
}

/// Sammy's own collision box (scaled model bbox), so lions and wasps can touch him.
pub fn add_player_collider(mut commands: Commands, q: Query<(Entity, &Player, &ModelAnimator), Without<BoxCollider>>) {
    for (ent, p, anim) in &q {
        let Some((half, cz)) = sammy_core::enemies::player_box(&anim.model.mdl, super::player::SCALE) else { continue };
        commands.entity(ent).insert((BoxCollider(sammy_core::collision::BoxCollider { center: [p.pos[0], p.pos[1], p.pos[2] + cz], half, solid: true }), PlayerBoxOffset(cz)));
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
    clock.bump();
}

/// `RestartSammy` sets `AttackCall = 0` (player.wdl:894).
pub fn reset_calls(mut reset: MessageReader<crate::game::ResetScan>, mut calls: ResMut<FriendCalls>) {
    if reset.read().count() > 0 {
        calls.attack_call = 0.0;
    }
}
