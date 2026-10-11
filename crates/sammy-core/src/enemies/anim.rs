//! The lion's animation steps and their mapping onto the model animator.

use super::*;

/// `AnimLion` (§3): advances the animation step and handles end-of-animation transitions.
pub(super) fn anim_lion(e: &mut Enemy, ctx: &mut Ctx<'_>) {
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
            e.hidden = toggles % 2 != 0;
            if e.dying >= 33.0 {
                e.hidden = true;
                e.class = 0; // inert
            }
        }
        10 => {
            if e.spec == 3 {
                ctx.sounds.push("ScorpionAttack_son");
            } else {
                ctx.sounds.push(["Hit1", "Hit2", "Hit3"][(ctx.rng.random(3.0) as usize).min(2)]);
            }
            let r = e.run_speed();
            e.set_state(2, r);
        }
        11 => {
            ctx.sounds.push(["LionHit1_son", "LionHit2_son", "tigre_son"][(ctx.rng.random(3.0) as usize).min(2)]);
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
pub(super) fn sync_anim(e: &Enemy, anim: &mut Anim) {
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
