mod actions;
mod camera;
mod coords;
mod debug;
mod image;
mod level;
mod material;
mod model;
mod vfs;

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::render::view::Tonemapping;

use camera::{FlyCamera, FlyCameraPlugin};
use model::{LoadedModel, ModelAnimator, ModelPlugin};
use vfs::GameFiles;

/// Default install location; override with the SAMMY_DIR environment variable.
const DEFAULT_GAME_DIR: &str = "/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate";

#[derive(Resource)]
struct Args {
    /// `--model <file> [anim]` shows a single model.
    model: Option<(String, Option<String>)>,
    /// Level file name (default 10001.wmb).
    level: String,
}

fn main() -> anyhow::Result<()> {
    let dir = std::env::var_os("SAMMY_DIR").map(PathBuf::from).unwrap_or_else(|| DEFAULT_GAME_DIR.into());
    let files = GameFiles::open(&dir)?;
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match argv.first().map(String::as_str) {
        Some("--model") => Args {
            model: Some((argv.get(1).cloned().unwrap_or("suricat1.mdl".into()), argv.get(2).cloned())),
            level: String::new(),
        },
        Some(level) => Args { model: None, level: level.to_owned() },
        None => Args { model: None, level: "10001.wmb".into() },
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Sammy Suricate".into(), ..default() }),
            ..default()
        }))
        .add_plugins((FlyCameraPlugin, ModelPlugin, actions::ActionsPlugin, material::LevelMaterialPlugin, debug::DebugPlugin))
        .insert_resource(files)
        .insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, level::face_camera)
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
    mut level_materials: ResMut<Assets<material::LevelMaterial>>,
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
        return;
    }

    let mut spawner = level::Spawner {
        commands: &mut commands,
        files: &files,
        meshes: &mut meshes,
        images: &mut images,
        materials: &mut materials,
        level_materials: &mut level_materials,
    };
    let start = spawner.spawn_level(&args.level).unwrap_or_else(|e| panic!("loading {}: {e}", args.level));
    // Start the camera behind and above the player start, looking the way Sammy faces.
    let (pos, pan) = start.unwrap_or((Vec3::ZERO, 0.0));
    let yaw = pan.to_radians();
    let back = Quat::from_rotation_y(yaw) * Vec3::Z;
    let cam_tf = Transform::from_translation(pos + back * 6.0 + Vec3::Y * 3.0);
    // Sky domes render first on their own layer; the world camera draws on top
    // with a fresh depth buffer, like Acknex's sky pass.
    commands.spawn((
        Camera3d::default(),
        Camera { order: 0, ..default() },
        Tonemapping::Linear,
        level::SKY_LAYER,
        cam_tf,
        FlyCamera { yaw, pitch: -0.35 },
    ));
    commands.spawn((
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::Linear,
        cam_tf,
        FlyCamera { yaw, pitch: -0.35 },
        camera::MainCamera,
    ));
}
