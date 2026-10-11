//! Extro level as Bevy systems: the `pov_Syn` camera target, id-25 scanners and the
//! scripted triggers, from [`sammy_core::extro`].

use bevy::prelude::*;
use formats::wmb::Object;
use sammy_core::extro::{Body, TriggerKind, Wanted};

use super::enemies::Enemy;
use super::player::{Player, State};
use crate::camera::{CameraParams, FollowState};
use crate::gameplay::collision::{entity, id};

pub use sammy_core::extro::RANGE_SCAN_25;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

/// The extro's camera target, scanners and scripted triggers.
pub struct ExtroPlugin;

impl Plugin for ExtroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Pov>()
            .init_resource::<Scanners>()
            .init_resource::<TriggerClock>()
            .add_systems(PreUpdate, (update_pov, trigger_clock))
            .add_systems(Update, run_triggers.in_set(GameSet::ExtroTriggers))
            .add_systems(Update, scripted_anims.in_set(GameSet::ScriptedAnims))
            .add_systems(Update, fade_out.in_set(GameSet::Flow));
    }
}


/// `pov_Syn` ([`sammy_core::extro::Pov`]): whom the camera follows and whom enemies
/// measure distance and line of sight to.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Pov(pub sammy_core::extro::Pov);

/// This frame's id-25 scanners ([`sammy_core::extro::Scanners`]).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Scanners(pub sammy_core::extro::Scanners);

pub fn update_pov(
    mut pov: ResMut<Pov>,
    mut scanners: ResMut<Scanners>,
    player: Query<(Entity, &Player)>,
    enemies: Query<(Entity, &Enemy)>,
    (intro, walker): (Res<crate::ui::intro::Intro>, Query<(Entity, &crate::ui::intro::Walker)>),
) {
    scanners.0.0.clear();
    let sammy: Option<Body> = player.single().ok().map(|(e, p)| (id(e), p.pos, p.pan, p.vel));
    if let Some((_, pos, ..)) = sammy {
        scanners.0.0.push(pos);
    }
    // The intro's scripted Sammy scans id 25 (camera zones, hint title) while it runs.
    let walker: Option<Body> = walker.single().ok().map(|(e, w)| (id(e), w.pos, w.pan, w.vel));
    if let (Some((_, pos, ..)), true) = (walker, intro.active) {
        scanners.0.0.push(pos);
    }
    scanners.0.0.extend(enemies.iter().filter(|(_, e)| e.scans_25()).map(|(_, e)| e.pos));
    let target: Option<Body> = pov.target.and_then(|t| enemies.get(entity(t)).ok()).filter(|(_, e)| !e.gone()).map(|(e, en)| (id(e), en.pos, en.pan, en.vel));
    pov.resolve(sammy, walker, target);
}

/// `StartExtro`: after a short wait the camera target becomes this wasp, overriding any
/// trigger that fired on the first ticks.
#[derive(Component)]
pub struct StartExtroPov(f32);

pub fn start_extro(entity: &mut EntityCommands) {
    entity.insert(StartExtroPov(sammy_core::extro::START_POV_DELAY));
}

/// The triggers' own 16 Hz clock: their scan loops are not gated by `PartieEncours`, so
/// they already run during the level-start sweep (mm1.wdl:5411-5416).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TriggerClock(sammy_core::time::TickClock);

pub fn trigger_clock(time: Res<Time>, mut clock: ResMut<TriggerClock>) {
    clock.advance(time.delta_secs() * super::player::TICKS);
}

/// One of the extro's triggers ([`sammy_core::extro::Trigger`]).
#[derive(Component, Deref, DerefMut)]
pub struct ExtroTrigger(sammy_core::extro::Trigger);

pub fn init_trigger(action: &str, obj: &Object, entity: &mut EntityCommands) {
    let kind = match action {
        "initsetpovsyn" => TriggerKind::PovSyn,
        "initsetanimplayer" => TriggerKind::AnimPlayer,
        _ => TriggerKind::FinExtro,
    };
    entity.insert(ExtroTrigger(sammy_core::extro::Trigger::from_object(kind, obj)));
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
    clock: Res<TriggerClock>,
    mut commands: Commands,
    params: Res<CameraParams>,
    mut pov: ResMut<Pov>,
    mut triggers: Query<(Entity, &mut ExtroTrigger)>,
    mut player: Query<(Entity, &mut Player, &mut crate::render::model::ModelAnimator)>,
    enemies: Query<(Entity, &Enemy)>,
    mut follow: Query<&mut FollowState>,
    mut start: Query<(Entity, &mut StartExtroPov)>,
) {
    for _ in 0..clock.ticks {
        for (e, mut s) in &mut start {
            s.0 -= 1.0;
            if s.0 <= 0.0 {
                pov.target = Some(id(e));
                commands.entity(e).remove::<StartExtroPov>();
            }
        }
        for (me, mut tr) in &mut triggers {
            if !tr.scans() {
                continue;
            }
            // First matching entity in the sphere.
            let hit: Option<(Option<Entity>, [f32; 3])> = match tr.wanted() {
                Wanted::Sammy => player.single().ok().filter(|(_, pl, _)| tr.within(pl.pos)).map(|(_, pl, _)| (None, pl.pos)),
                Wanted::Enemy { class, spec } => enemies
                    .iter()
                    .find(|(_, e)| e.class() as f32 == class && !e.gone() && spec.is_none_or(|s| e.spec_id() as f32 == s) && tr.within(e.pos))
                    .map(|(ent, e)| (Some(ent), e.pos)),
            };
            let Some((target, at)) = hit else { continue };
            match tr.kind {
                TriggerKind::PovSyn => {
                    pov.target = target.map(id);
                    let cam = sammy_core::extro::cut_camera(at, params.back, params.dist, params.height);
                    for mut f in &mut follow {
                        f.snap_to(cam);
                    }
                }
                TriggerKind::AnimPlayer => {
                    if let Ok((pe, mut pl, mut anim)) = player.single_mut() {
                        let scene = tr.scene();
                        pl.state = State::Scripted;
                        scene.start(&mut anim.anim);
                        commands.entity(pe).insert(ScriptedAnim { salut: scene.salut });
                    }
                }
                TriggerKind::FinExtro => {
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
            tr.done = true;
            commands.entity(me).despawn();
        }
    }
}

pub fn scripted_anims(mut q: Query<(&mut ScriptedAnim, &mut crate::render::model::ModelAnimator)>) {
    for (mut s, mut anim) in &mut q {
        s.salut = sammy_core::extro::scripted_anim(s.salut, &mut anim.anim);
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
    mut save: ResMut<crate::platform::save::SaveGame>,
) {
    let Some(mut fin) = fin else { return };
    fin.0 = (fin.0 + sammy_core::extro::FADE_SPEED * time.delta_secs() * super::player::TICKS).min(100.0);
    for mut bg in &mut panel {
        bg.0 = Color::BLACK.with_alpha(fin.0 / 100.0);
    }
    if fin.0 >= 100.0 {
        commands.remove_resource::<FinExtro>();
        // `Branch MainMenu` (Extro.wdl:152); Play then resumes level 20 (Extro.wdl:114).
        let _ = &current;
        save.last_level = sammy_core::extro::RESUME_LEVEL;
        crate::platform::save::write(&save);
        pending.0 = Some(crate::ui::menu::MENU_FILE.to_owned());
    }
}
