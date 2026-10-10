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

fn screenshot(mut commands: Commands, mut job: ResMut<ScreenshotJob>, mut exit: MessageWriter<AppExit>) {
    job.frame += 1;
    if job.frame == 60 {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(job.path.clone()));
    }
    if job.frame == 90 {
        exit.write(AppExit::Success);
    }
}
