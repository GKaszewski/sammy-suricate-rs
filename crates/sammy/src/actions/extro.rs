//! Extro level (`Extro.wdl`, docs/level-objects.md §2.14): the `pov_Syn` camera target,
//! id-25 scanners other than Sammy, and the scripted triggers `InitSetPovSyn`,
//! `InitSetAnimPlayer` and `InitSetFinExtro` (mm1.wdl:5395-5562).

use bevy::prelude::*;
use formats::wmb::Object;

use super::enemy::{AiClock, Enemy};
use super::player::{Player, State};
use crate::camera::{CameraParams, FollowState};

/// `IdPlayer` (Sammy's Skill1 in scans).
const ID_PLAYER: f32 = 3333.0;
/// `PlayerAnimSpeedLoopjambes` (mm1.wdl:52).
const ANIM_SPEED_LOOPJAMBES: f32 = 2.0;
/// `RangeScanDoor`: radius of the id-25 scans.
pub const RANGE_SCAN_25: f32 = 500.0;

/// `pov_Syn`: whom the camera follows and whom enemies measure distance and line of
/// sight to. Sammy unless a trigger (or `StartExtro`) retargets it to an enemy.
#[derive(Resource, Default)]
pub struct Pov {
    /// Enemy to follow; `None` = Sammy.
    pub target: Option<Entity>,
    /// Resolved this frame.
    pub entity: Option<Entity>,
    pub pos: [f32; 3],
    pub pan: f32,
    pub vel: [f32; 3],
}

/// Positions doing an id-25 scan this frame (Sammy, plus Flag5 lions and wasp enemies):
/// they trigger `SetCamera` zones and `InitTitle` credits.
#[derive(Resource, Default)]
pub struct Scanners(pub Vec<[f32; 3]>);

impl Scanners {
    /// Nearest scanner distance to `p`.
    pub fn nearest(&self, p: [f32; 3]) -> Option<([f32; 3], f32)> {
        self.0
            .iter()
            .map(|s| (*s, (0..3).map(|i| (s[i] - p[i]).powi(2)).sum::<f32>().sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

pub fn update_pov(mut pov: ResMut<Pov>, mut scanners: ResMut<Scanners>, player: Query<(Entity, &Player)>, enemies: Query<(Entity, &Enemy)>) {
    scanners.0.clear();
    let sammy = player.single().ok();
    if let Some((_, p)) = sammy {
        scanners.0.push(p.pos);
    }
    scanners.0.extend(enemies.iter().filter(|(_, e)| e.scans_25()).map(|(_, e)| e.pos));
    if let Some((ent, e)) = pov.target.and_then(|t| enemies.get(t).ok()).filter(|(_, e)| !e.gone()) {
        (pov.entity, pov.pos, pov.pan, pov.vel) = (Some(ent), e.pos, e.pan, e.vel);
    } else if let Some((ent, p)) = sammy {
        (pov.entity, pov.pos, pov.pan, pov.vel) = (Some(ent), p.pos, p.pan, p.vel);
    }
}

/// `StartExtro` (Extro.wdl:155-159): the camera opens on this wasp.
pub fn start_extro(entity: &mut EntityCommands) {
    let id = entity.id();
    entity.commands().queue(move |w: &mut World| w.resource_mut::<Pov>().target = Some(id));
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    PovSyn,
    AnimPlayer,
    FinExtro,
}

/// One of the extro's invisible sphere-scan triggers.
#[derive(Component)]
pub struct ExtroTrigger {
    kind: Kind,
    pos: [f32; 3],
    s: [f32; 8],
    timer: f32,
}

pub fn init_trigger(action: &str, obj: &Object, entity: &mut EntityCommands) {
    let kind = match action {
        "initsetpovsyn" => Kind::PovSyn,
        "initsetanimplayer" => Kind::AnimPlayer,
        _ => Kind::FinExtro,
    };
    let mut s = [0.0; 8];
    s.copy_from_slice(&obj.skills[..8]);
    entity.insert(ExtroTrigger { kind, pos: obj.origin, s, timer: 0.0 });
}

/// Sammy is in a scripted animation (`PlayerEtat` 2/3 forced by `SetAnimPlayer`):
/// no input, no death. `salut` waves once, then loops "loopjambes".
#[derive(Component)]
pub struct ScriptedAnim {
    salut: bool,
}

/// Fade to black after `SetFinExtro`, in fog percent; at 100 the extro ends.
#[derive(Resource)]
pub struct FinExtro(pub f32);

#[derive(Component)]
pub struct FadePanel;

#[allow(clippy::too_many_arguments)]
pub fn run_triggers(
    clock: Res<AiClock>,
    mut commands: Commands,
    params: Res<CameraParams>,
    mut pov: ResMut<Pov>,
    mut triggers: Query<(Entity, &mut ExtroTrigger)>,
    mut player: Query<(Entity, &mut Player, &mut crate::model::ModelAnimator)>,
    enemies: Query<(Entity, &Enemy)>,
    mut follow: Query<&mut FollowState>,
) {
    for _ in 0..clock.ticks {
        for (me, mut tr) in &mut triggers {
            tr.timer -= 1.0;
            if tr.timer > 0.0 {
                continue;
            }
            // Scan periods: SetPovSyn / SetAnimPlayer every 2 ticks, SetFinExtro every 4.
            tr.timer = if tr.kind == Kind::FinExtro { 4.0 } else { 2.0 };
            let (p, s) = (tr.pos, tr.s);
            let within = |q: [f32; 3]| (0..3).map(|i| (q[i] - p[i]).powi(2)).sum::<f32>().sqrt() <= s[1];
            // First matching entity in the sphere (Sammy is IdPlayer 3333).
            let hit: Option<(Option<Entity>, [f32; 3])> = if s[0] == ID_PLAYER {
                player.single().ok().filter(|(_, pl, _)| within(pl.pos)).map(|(_, pl, _)| (None, pl.pos))
            } else {
                enemies
                    .iter()
                    .find(|(_, e)| e.class() as f32 == s[0] && !e.gone() && (s[2] == 0.0 || e.spec_id() as f32 == s[2]) && within(e.pos))
                    .map(|(ent, e)| (Some(ent), e.pos))
            };
            let Some((target, at)) = hit else { continue };
            match tr.kind {
                Kind::PovSyn => {
                    // Hard cut: dist behind at the world-fixed back angle, height above.
                    pov.target = target;
                    let a = params.back.to_radians();
                    let cam = [at[0] + a.cos() * params.dist, at[1] + a.sin() * params.dist, at[2] + params.height];
                    for mut f in &mut follow {
                        f.snap_to(cam);
                    }
                }
                Kind::AnimPlayer => {
                    if let Ok((pe, mut pl, mut anim)) = player.single_mut() {
                        let salut = s[2] == 12.0;
                        pl.state = State::Scripted;
                        anim.play(if salut { "salut" } else { "loopjambes" }, !salut, s[5]);
                        anim.set_speed_pct(s[4], super::player::TICKS);
                        commands.entity(pe).insert(ScriptedAnim { salut });
                    }
                }
                Kind::FinExtro => {
                    commands.insert_resource(FinExtro(1.0));
                    commands.spawn((
                        FadePanel,
                        crate::flow::LevelScoped,
                        Node { width: percent(100), height: percent(100), position_type: PositionType::Absolute, ..default() },
                        BackgroundColor(Color::BLACK.with_alpha(0.0)),
                        GlobalZIndex(100),
                    ));
                }
            }
            // All shipped triggers are one-shot (Skill4 = 0).
            commands.entity(me).despawn();
        }
    }
}

/// "salut" plays once, then "loopjambes" loops at its default speed (anims.wdl:485-496).
pub fn scripted_anims(mut q: Query<(&mut ScriptedAnim, &mut crate::model::ModelAnimator)>) {
    for (mut s, mut anim) in &mut q {
        if s.salut && anim.finished() {
            s.salut = false;
            anim.play("loopjambes", true, 0.0);
            anim.set_speed_pct(ANIM_SPEED_LOOPJAMBES, super::player::TICKS);
        }
    }
}

/// `SetFinExtro`: fog +6/tick to 100, then black and back to the start (no menu yet).
pub fn fade_out(
    time: Res<Time>,
    mut commands: Commands,
    fin: Option<ResMut<FinExtro>>,
    current: Res<crate::flow::CurrentLevel>,
    mut pending: ResMut<crate::flow::PendingLoad>,
    mut panel: Query<&mut BackgroundColor, With<FadePanel>>,
) {
    let Some(mut fin) = fin else { return };
    fin.0 = (fin.0 + 6.0 * time.delta_secs() * super::player::TICKS).min(100.0);
    for mut bg in &mut panel {
        bg.0 = Color::BLACK.with_alpha(fin.0 / 100.0);
    }
    if fin.0 >= 100.0 {
        commands.remove_resource::<FinExtro>();
        let next = current.slot.map_or(1, crate::flow::next_slot);
        pending.0 = crate::flow::file_for(next).map(str::to_owned);
    }
}
