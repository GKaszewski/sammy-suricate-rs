//! Help signs: `InitTitle` / `Title_Hit` (Intro2.wdl:492-760). A sprite that fades
//! in when Sammy comes within range, stays while he is near, lingers, fades out,
//! and can be dismissed with Enter. One-shot. Drawn on top of the world (`Near`).

use bevy::prelude::*;
use formats::wmb::Object;

use super::player::{Player, TICKS};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    Waiting,
    FadeIn,
    Near,
    Linger(f32),
    FadeOut,
    Done,
}

#[derive(Component)]
pub struct Title {
    /// Trigger centre (Skill16-18: origin after the -100 drop).
    center: [f32; 3],
    /// Trigger distance (Skill21 = frac(Skill1)×1000, default 300).
    range: f32,
    /// Target alpha in percent (Skill8).
    alpha_max: f32,
    /// Fade-in ticks, lifetime after leaving, fade-out ticks (Skill5/6/7).
    fade_in: f32,
    life: f32,
    fade_out: f32,
    alpha: f32,
    phase: Phase,
}

pub fn init_title(obj: &mut Object, entity: &mut EntityCommands) {
    obj.origin[2] -= 100.0;
    let s = &obj.skills;
    let range = ((s[0].fract() * 1000.0) + 0.5).floor();
    let or1 = |v: f32| if v == 0.0 { 1.0 } else { v };
    entity.insert((
        Title {
            center: obj.origin,
            range: if range == 0.0 { 300.0 } else { range },
            alpha_max: if s[7] == 0.0 { 100.0 } else { s[7] },
            fade_in: or1(s[4]),
            life: or1(s[5]),
            fade_out: or1(s[6]),
            alpha: 0.0,
            phase: Phase::Waiting,
        },
        Visibility::Hidden,
    ));
}

pub fn update_titles(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    player: Query<&Player>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&mut Title, &mut Visibility, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    for (mut title, mut vis, mat) in &mut q {
        let d = (0..3).map(|i| (title.center[i] - p.pos[i]).powi(2)).sum::<f32>().sqrt();
        let near = d <= title.range;
        let enter = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);
        title.phase = match title.phase {
            Phase::Waiting if near => Phase::FadeIn,
            Phase::FadeIn | Phase::Near | Phase::Linger(_) if enter => Phase::FadeOut,
            Phase::FadeIn => {
                title.alpha += title.alpha_max / title.fade_in * t;
                if title.alpha >= title.alpha_max {
                    title.alpha = title.alpha_max;
                    Phase::Near
                } else {
                    Phase::FadeIn
                }
            }
            Phase::Near if !near => Phase::Linger(title.life),
            Phase::Linger(left) if left - t <= 0.0 => Phase::FadeOut,
            Phase::Linger(left) => Phase::Linger(left - t),
            Phase::FadeOut => {
                title.alpha -= title.alpha_max / title.fade_out * t;
                if title.alpha <= 0.0 { Phase::Done } else { Phase::FadeOut }
            }
            other => other,
        };
        let shown = !matches!(title.phase, Phase::Waiting | Phase::Done);
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if shown && let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color.set_alpha((title.alpha / 100.0).clamp(0.0, 1.0));
        }
    }
}
