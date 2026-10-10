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
) {
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
