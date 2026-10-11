//! `SAMMY_SCREENSHOT=<path>`: save screenshots at the times (seconds) listed in
//! `SAMMY_SHOT_SECS` (default "2.5,4"), log player/game state, then exit.
//! Used for automated visual checks.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(path) = std::env::var("SAMMY_SCREENSHOT") {
            let times: Vec<f32> = std::env::var("SAMMY_SHOT_SECS")
                .unwrap_or_else(|_| "2.5,4".into())
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            app.insert_resource(ScreenshotJob { path, times, next: 0, log_timer: 0.0 }).add_systems(Update, screenshot);
        }
    }
}

#[derive(Resource)]
struct ScreenshotJob {
    path: String,
    times: Vec<f32>,
    next: usize,
    log_timer: f32,
}

fn screenshot(
    mut commands: Commands,
    time: Res<Time>,
    mut job: ResMut<ScreenshotJob>,
    mut exit: MessageWriter<AppExit>,
    player: Query<&crate::actions::player::Player>,
    game: Res<crate::game::GameState>,
    current: Res<crate::flow::CurrentLevel>,
    enemies: Query<&crate::actions::enemy::Enemy>,
    calls: Res<crate::actions::enemy::FriendCalls>,
) {
    let now = time.elapsed_secs();
    job.log_timer -= time.delta_secs();
    if job.log_timer <= 0.0 {
        job.log_timer = 1.0;
        if let Ok(p) = player.single() {
            info!(
                "t={now:.1} {} {:?} player {:?} state {:?} on {:?} | score {}/{} key {} lives {} hearts {:.0} start {:?} won {}",
                current.file, current.phase, p.pos.map(|v| v.round()), p.state, p.ground_entity, game.score, game.goal, game.key, game.lives, game.hearts, game.start.map(|(s, _)| s.map(|v| v.round())), game.won
            );
            info!("   calls {:?}", *calls);
            for e in &enemies {
                let d = (0..3).map(|i| (e.pos[i] - p.pos[i]).powi(2)).sum::<f32>().sqrt();
                info!("   enemy {} dist {:.0}", e.debug_line(), d);
            }
        }
    }
    if let Some(&t) = job.times.get(job.next)
        && now >= t
    {
        let p = job.path.replace(".png", &format!("_{t}.png"));
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(p));
        job.next += 1;
    }
    if job.next >= job.times.len() && now >= job.times.last().copied().unwrap_or(0.0) + 0.5 {
        exit.write(AppExit::Success);
    }
}
