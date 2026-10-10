mod camera;
mod coords;
mod debug;
mod image;
mod model;
mod vfs;

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;

use camera::{FlyCamera, FlyCameraPlugin};
use model::{LoadedModel, ModelAnimator, ModelPlugin};
use vfs::GameFiles;

/// Default install location; override with the SAMMY_DIR environment variable.
const DEFAULT_GAME_DIR: &str = "/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate";

#[derive(Resource)]
struct Args {
    /// `--model <file> [anim]` shows a single model; otherwise a level file name.
    model: Option<(String, Option<String>)>,
}

fn main() -> anyhow::Result<()> {
    let dir = std::env::var_os("SAMMY_DIR").map(PathBuf::from).unwrap_or_else(|| DEFAULT_GAME_DIR.into());
    let files = GameFiles::open(&dir)?;
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match argv.first().map(String::as_str) {
        Some("--model") => Args { model: Some((argv.get(1).cloned().unwrap_or("suricat1.mdl".into()), argv.get(2).cloned())) },
        _ => Args { model: Some(("suricat1.mdl".into(), Some("walkA".into()))) },
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Sammy Suricate".into(), ..default() }),
            ..default()
        }))
        .add_plugins((FlyCameraPlugin, ModelPlugin, debug::DebugPlugin))
        .insert_resource(files)
        .insert_resource(args)
        .add_systems(Startup, setup)
        .run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    files: Res<GameFiles>,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((DirectionalLight { illuminance: 8000.0, ..default() }, Transform::from_xyz(1.0, 2.0, 1.5).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.insert_resource(GlobalAmbientLight { brightness: 400.0, ..default() });

    if let Some((name, anim)) = &args.model {
        let data = files.read(name).expect("model file");
        let model = Arc::new(LoadedModel::load(&data, &mut images, &mut materials).expect("parse model"));
        let frames = anim.as_deref().map(|a| model.animation(a)).unwrap_or_default();
        let mesh = meshes.add(model.build_mesh(frames.first().copied().unwrap_or(0)));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(model.material.clone()),
            Transform::default(),
            ModelAnimator { model: model.clone(), frames, fps: 16.0, time: 0.0 },
        ));
        commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.5, 4.0), FlyCamera::default()));
    }
}
