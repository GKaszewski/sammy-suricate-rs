//! `SAMMY_SCREENSHOT=<path>`: save a screenshot after a short warm-up, then exit.
//! Used for automated visual checks.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(path) = std::env::var("SAMMY_SCREENSHOT") {
            app.insert_resource(ScreenshotJob { path, frame: 0 }).add_systems(Update, screenshot);
        }
    }
}

#[derive(Resource)]
struct ScreenshotJob {
    path: String,
    frame: u32,
}

fn screenshot(
    mut commands: Commands,
    mut job: ResMut<ScreenshotJob>,
    mut exit: MessageWriter<AppExit>,
    cam: Query<&GlobalTransform, With<crate::camera::MainCamera>>,
    objs: Query<(&GlobalTransform, &crate::level::LevelObject)>,
    player: Query<&crate::actions::player::Player>,
    game: Res<crate::game::GameState>,
) {
    if job.frame % 50 == 0
        && let Ok(p) = player.single()
    {
        info!("f{} player pos {:?} pan {:.0} state {:?} grounded {} on {:?}; game {:?}", job.frame, p.pos, p.pan, p.state, p.grounded, p.ground_entity, *game);
    }
    job.frame += 1;
    if [150, 250].contains(&job.frame) {
        let p = job.path.replace(".png", &format!("_{}.png", job.frame));
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(p));
    }
    if job.frame == 250 {
        if let Ok(c) = cam.single() {
            info!("camera at {} fwd {}", c.translation(), c.forward());
            for (tf, o) in &objs {
                let d = tf.translation().distance(c.translation());
                if d < 4.0 {
                    info!("near camera ({d:.2}): {} {} {} scale {:?}", o.0.name, o.0.file, o.0.action, tf.scale());
                }
            }
        }
    }
    if job.frame == 280 {
        exit.write(AppExit::Success);
    }
}
