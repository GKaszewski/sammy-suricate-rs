//! Health, lives and hazards:
//! - `InitTileDeath`/`TileDeath_Hit` → `TeleportFor1Life` (mm1.wdl:4023, player.wdl:699)
//! - `InitLava`/`Lava_Hit` (mm1.wdl:4869), `Player_Kaputt` (player.wdl:772)
//! - `RestartSammy` respawn + reset scan (player.wdl:884)
//! - `InitSmallHeart` (mm1.wdl:4721), `InitBonusVie` (mm1.wdl:2633), `InitRestart` checkpoints (mm1.wdl:3771)

use bevy::prelude::*;
use formats::wmb::Object;

use super::pickups::{Collected, GemSprites, PickupPos, Respawnable, Shrinking, SnapToGround};
use super::player::{Player, State, TICKS};
use crate::audio::Sfx;
use crate::camera::FollowState;
use crate::coords;
use crate::game::{GameState, ResetScan, MAX_HEARTS, MAX_LIVES};
use crate::model::ModelAnimator;

#[derive(Component)]
pub struct DeathTile;

#[derive(Component)]
pub struct LavaTile;

#[derive(Component)]
pub struct SmallHeart {
    amount: f32,
    scale: Vec3,
    /// Health was full: hidden for this many ticks, then grows back.
    away: Option<f32>,
}

#[derive(Component)]
pub struct BigHeart {
    lives: i32,
}

/// Boost pickup (`InitBoost`, mm1.wdl:4315-4464).
#[derive(Component)]
pub struct BoostPickup {
    amount: f32,
    scale: Vec3,
    /// Boost was full: hidden for this many ticks, then grows back.
    away: Option<f32>,
}

pub fn init_boost(obj: &Object, entity: &mut EntityCommands) {
    let amount = if obj.skills[1] == 0.0 { 1.0 } else { obj.skills[1] };
    entity.insert((BoostPickup { amount, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs(), away: None }, PickupPos(obj.origin)));
}

#[derive(Component)]
pub struct Checkpoint {
    home: [f32; 3],
    /// Skill8: Sammy's pan after respawning here.
    pan: f32,
    /// Skill2-4 spin speeds (Flag3).
    spin: Option<[f32; 3]>,
    angles: [f32; 3],
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DeathKind {
    /// Deadly tile: lose a life, fade out, respawn (no death animation).
    Tile,
    /// Out of health: death animation, lose a life, blink, respawn.
    Kaputt,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Dying,
    FadeOut,
    Blink,
    FadeIn,
    GameOver,
}

/// Running death/respawn sequence on Sammy.
#[derive(Component)]
pub struct Dying {
    kind: DeathKind,
    stage: Stage,
    ticks: f32,
    /// Sammy's opacity in percent (`my.alpha`).
    alpha: f32,
    sound_played: bool,
}

pub fn init_tile_death(entity: &mut EntityCommands) {
    entity.insert(DeathTile);
}

pub fn init_lava(entity: &mut EntityCommands) {
    entity.insert(LavaTile);
}

pub fn init_small_heart(obj: &Object, entity: &mut EntityCommands) {
    let amount = if obj.skills[1] == 0.0 { 12.0 } else { obj.skills[1] };
    let scale = Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs();
    entity.insert((SmallHeart { amount, scale, away: None }, PickupPos(obj.origin)));
    if obj.flags & (1 << 5) == 0 {
        entity.insert(SnapToGround);
    }
}

pub fn init_bonus_vie(obj: &Object, entity: &mut EntityCommands) {
    entity.insert((BigHeart { lives: obj.skills[1] as i32 }, PickupPos(obj.origin)));
}

pub fn init_restart(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    entity.insert((
        Checkpoint {
            home: obj.origin,
            pan: s[7],
            spin: (obj.flags & (1 << 2) != 0).then_some([s[1], s[2], s[3]]),
            angles: obj.angles,
        },
        PickupPos(obj.origin),
        Respawnable,
    ));
}

/// Records the level spawn as the first respawn point.
pub fn init_start(pos: [f32; 3], pan: f32, entity: &mut EntityCommands) {
    entity.commands().queue(move |w: &mut World| w.resource_mut::<GameState>().start = Some((pos, pan)));
}

/// Hazards under Sammy's feet and running out of health start a death.
pub fn detect_deaths(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    player: Query<(Entity, &Player), Without<Dying>>,
    tiles: Query<(Has<DeathTile>, Has<LavaTile>)>,
) {
    let Ok((e, p)) = player.single() else { return };
    // `Player_Kaputt` returns at once while `PlayerEtat == 3` (extro scripted Sammy).
    if matches!(p.state, State::Dead | State::Scripted) {
        return;
    }
    let (death, lava) = p.ground_entity.filter(|_| p.grounded && p.vz == 0.0).and_then(|g| tiles.get(g).ok()).unwrap_or_default();
    if lava {
        game.hearts -= time.delta_secs() * TICKS; // 1 heart per tick
    }
    let kind = if death {
        DeathKind::Tile
    } else if game.hearts <= 0.0 {
        DeathKind::Kaputt
    } else {
        return;
    };
    game.hearts = game.hearts.max(0.0);
    commands.entity(e).insert(Dying { kind, stage: Stage::Dying, ticks: 0.0, alpha: 100.0, sound_played: false });
}

pub fn set_alpha(materials: &mut Assets<StandardMaterial>, mat: &MeshMaterial3d<StandardMaterial>, alpha: f32) {
    if let Some(mut m) = materials.get_mut(&mat.0) {
        let a = (alpha / 100.0).clamp(0.0, 1.0);
        m.alpha_mode = if a < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque };
        m.base_color.set_alpha(a);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_death(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    sprites: Res<GemSprites>,
    jump: Res<super::player::JumpParams>,
    mut reset: MessageWriter<ResetScan>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut follow: Query<&mut FollowState>,
    mut player: Query<(Entity, &mut Player, &mut Dying, &mut Visibility, &MeshMaterial3d<StandardMaterial>, Option<&mut ModelAnimator>)>,
) {
    let Ok((e, mut p, mut d, mut vis, mat, mut anim)) = player.single_mut() else { return };
    let t = time.delta_secs() * TICKS;
    d.ticks += t;
    match d.stage {
        Stage::Dying => {
            if d.ticks == t {
                // ResetSammy: input off; the held key is dropped unless the goal is reached.
                p.state = State::Dead;
                if game.key != 0 && !game.goal_reached {
                    // EjectNewKey: a decorative gem pops out (fate 0, never collectable).
                    let key = game.key;
                    game.key = 0;
                    let pos = [p.pos[0], p.pos[1], p.pos[2] + 30.0];
                    let hs = p.vel[0].abs() + p.vel[1].abs();
                    super::gems::launch_gem(&mut commands, &sprites, &jump, key, pos, p.pan, hs, 40.0, 0.0);
                }
                if d.kind == DeathKind::Kaputt
                    && let Some(a) = anim.as_deref_mut()
                {
                    a.play("kaputt", false, 0.0);
                    a.set_speed_pct(3.0, TICKS);
                }
            }
            let done = match d.kind {
                DeathKind::Tile => true,
                DeathKind::Kaputt => {
                    // Camera_PlusTilt -= Camera_PlusTilt*0.75*time: the camera looks down at Sammy.
                    for mut f in follow.iter_mut() {
                        let k = f.tilt_scale.unwrap_or(1.0);
                        f.tilt_scale = Some((k - k * 0.75 * t).max(0.0));
                    }
                    if d.ticks >= 8.0 && !d.sound_played {
                        sfx.play("Kaputt_son");
                        d.sound_played = true;
                    }
                    d.ticks >= 64.0 || anim.as_deref().is_some_and(|a| a.finished())
                }
            };
            if done {
                game.lives -= 1;
                d.ticks = 0.0;
                d.stage = if game.lives <= 0 {
                    game.lost = true;
                    for mut f in follow.iter_mut() {
                        f.tilt_scale = Some(0.0);
                        f.orbit = true;
                    }
                    if let Some(a) = anim.as_deref_mut() {
                        a.play("finkaputt", true, 0.0);
                        a.set_speed_pct(2.0, TICKS);
                    }
                    Stage::GameOver
                } else if d.kind == DeathKind::Tile {
                    Stage::FadeOut
                } else {
                    Stage::Blink
                };
            }
        }
        Stage::FadeOut => {
            d.alpha = (d.alpha - 2.0 * t).max(0.0);
            set_alpha(&mut materials, mat, d.alpha);
            if d.alpha <= 0.0 {
                d.stage = respawn(&mut p, &mut d, &mut game, &mut reset, &mut follow);
            }
        }
        Stage::Blink => {
            // ~46 ticks of invisible/visible flicker, faster towards the end.
            let period = if d.ticks < 16.0 { 4.0 } else if d.ticks < 30.0 { 3.0 } else { 2.0 };
            *vis = if (d.ticks / period) as i32 % 2 == 0 { Visibility::Hidden } else { Visibility::Inherited };
            if d.ticks >= 46.0 {
                *vis = Visibility::Inherited;
                d.alpha = 0.0;
                d.stage = respawn(&mut p, &mut d, &mut game, &mut reset, &mut follow);
            }
        }
        Stage::FadeIn => {
            d.alpha = (d.alpha + 10.0 * t).min(100.0);
            set_alpha(&mut materials, mat, d.alpha);
            if d.alpha >= 100.0 {
                p.state = State::Stand;
                commands.entity(e).remove::<Dying>();
            }
        }
        Stage::GameOver => {}
    }
}

/// `RestartSammy`: reset scan, full health, back to the respawn point, fade in.
fn respawn(
    p: &mut Player,
    d: &mut Dying,
    game: &mut GameState,
    reset: &mut MessageWriter<ResetScan>,
    follow: &mut Query<&mut FollowState>,
) -> Stage {
    reset.write(ResetScan);
    game.hearts = MAX_HEARTS;
    if let Some((pos, pan)) = game.start {
        p.teleport(pos, pan);
        for mut f in follow.iter_mut() {
            f.snap_to([pos[0] + 32.0, pos[1] + 32.0, pos[2] + 150.0]);
        }
    }
    d.ticks = 0.0;
    Stage::FadeIn
}

#[allow(clippy::type_complexity)]
pub fn collect_health(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut hearts: Query<(Entity, &PickupPos, &mut SmallHeart), (Without<Shrinking>, Without<SnapToGround>)>,
    big: Query<(Entity, &PickupPos, &BigHeart), Without<Shrinking>>,
    checkpoints: Query<(Entity, &PickupPos, &Checkpoint), (Without<Shrinking>, Without<Collected>)>,
    mut boosts: Query<(Entity, &PickupPos, &mut BoostPickup, &mut Transform, &mut Visibility), Without<Shrinking>>,
) {
    let Ok(p) = player.single() else { return };
    if p.state == State::Dead {
        return;
    }
    let near = |q: &[f32; 3]| (0..3).map(|i| (q[i] - p.pos[i]).powi(2)).sum::<f32>().sqrt() <= p.scan_radius;

    for (e, pos, mut heart) in &mut hearts {
        if heart.away.is_some() || !near(&pos.0) {
            continue;
        }
        if game.hearts >= MAX_HEARTS {
            heart.away = Some(48.0); // full: duck away and come back
            continue;
        }
        sfx.play("Gagne1");
        sfx.play("Chimes2");
        game.hearts = (game.hearts + heart.amount).min(MAX_HEARTS);
        commands.entity(e).insert(Shrinking);
    }
    for (e, pos, b) in &big {
        if game.lives < MAX_LIVES && near(&pos.0) {
            game.lives = (game.lives + b.lives).min(MAX_LIVES);
            sfx.play("Getlife_son");
            commands.entity(e).insert((Shrinking, crate::lights::flash([50.0, 250.0, 10.0], 300.0, 40.0, 64.0)));
        }
    }
    for (e, pos, mut b, mut tf, mut vis) in &mut boosts {
        if let Some(left) = b.away {
            // Ducked away because boost was full: shrink, hide 64 ticks, grow back.
            let t = time.delta_secs() * TICKS;
            let left = left - t;
            if left > 64.0 {
                tf.scale *= 0.9f32.powf(t);
            } else {
                *vis = Visibility::Hidden;
            }
            if left <= 0.0 {
                *vis = Visibility::Inherited;
                tf.scale = b.scale;
                b.away = None;
            } else {
                b.away = Some(left);
            }
            continue;
        }
        if !near(&pos.0) {
            continue;
        }
        if game.boost <= 0.0 {
            b.away = Some(64.0 + 40.0);
            continue;
        }
        game.boost = (game.boost - b.amount).max(0.0);
        sfx.play("chimes2");
        commands.entity(e).insert((Shrinking, crate::lights::flash([150.0, 50.0, 100.0], 200.0, 10.0, 64.0)));
    }
    for (e, pos, c) in &checkpoints {
        if near(&pos.0) {
            sfx.play("Restart_Son");
            game.start = Some(([c.home[0], c.home[1], c.home[2] + 150.0], c.pan));
            commands.entity(e).insert(Shrinking);
        }
    }
}

/// Small hearts that ducked away (health was full) shrink, wait and grow back.
pub fn animate_hearts(time: Res<Time>, mut q: Query<(&mut SmallHeart, &mut Transform, &mut Visibility), Without<Shrinking>>) {
    let t = time.delta_secs() * TICKS;
    for (mut h, mut tf, mut vis) in &mut q {
        let Some(left) = h.away else { continue };
        let left = left - t;
        if left > 0.0 {
            tf.scale *= 0.8f32.powf(t);
            if tf.scale.x < 0.1 {
                *vis = Visibility::Hidden;
            }
            h.away = Some(left);
        } else {
            *vis = Visibility::Inherited;
            tf.scale = (tf.scale.max(Vec3::splat(0.1)) * 1.5f32.powf(t)).min(h.scale);
            if tf.scale.x >= h.scale.x {
                h.away = None;
            }
        }
    }
}

/// Big hearts spin 10°/tick; checkpoints spin when Flag3 is set.
pub fn spin_pickups(
    time: Res<Time>,
    mut hearts: Query<(&mut Transform, &PickupPos), (With<BigHeart>, Without<Checkpoint>)>,
    mut checkpoints: Query<(&mut Transform, &mut Checkpoint), Without<BigHeart>>,
) {
    let t = time.delta_secs() * TICKS;
    for (mut tf, _) in &mut hearts {
        tf.rotate_y((10.0 * t).to_radians());
    }
    for (mut tf, mut c) in &mut checkpoints {
        if let Some(spin) = c.spin {
            for i in 0..3 {
                c.angles[i] += spin[i] * t;
            }
            tf.rotation = coords::rotation(c.angles);
        }
    }
}

/// Reset scan for checkpoints (`RestartWaitForRestart`): taken ones come back.
pub fn reset_checkpoints(
    mut reset: MessageReader<ResetScan>,
    mut commands: Commands,
    mut q: Query<(Entity, &Checkpoint, &mut Transform), With<Collected>>,
) {
    if reset.read().count() == 0 {
        return;
    }
    for (e, c, mut tf) in &mut q {
        tf.translation = coords::pos(c.home);
        tf.scale = Vec3::ONE;
        commands.entity(e).remove::<Collected>().insert(Visibility::Inherited);
    }
}
