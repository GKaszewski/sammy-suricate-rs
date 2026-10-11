mod actions;
mod audio;
mod camera;
mod collision;
mod coords;
mod debug;
mod flow;
mod game;
mod hud;
mod image;
mod level;
mod lights;
mod material;
mod model;
mod vfs;

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::render::view::Tonemapping;

use camera::{CameraPlugin, FlyCamera, FollowState, MainCamera, SkyCamera};
use model::{LoadedModel, ModelAnimator, ModelPlugin};
use vfs::GameFiles;

/// Default install location; override with the SAMMY_DIR environment variable.
const DEFAULT_GAME_DIR: &str = "/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate";

#[derive(Resource)]
struct Args {
    /// `--model <file> [anim]` shows a single model.
    model: Option<(String, Option<String>)>,
    /// Level file name or slot number (default: slot 1).
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
        None => Args { model: None, level: "1".into() },
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Sammy Suricate".into(), ..default() }),
            ..default()
        }))
        .add_plugins((CameraPlugin, ModelPlugin, actions::ActionsPlugin, material::LevelMaterialPlugin, hud::HudPlugin, flow::FlowPlugin, lights::LightsPlugin, debug::DebugPlugin))
        .insert_resource(files)
        .insert_resource(args)
        .add_systems(Startup, (audio::load_sounds, setup))
        .add_systems(Update, (level::face_camera, level::animate_sprites))
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
    mut pending: ResMut<flow::PendingLoad>,
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
            ModelAnimator::new(model.clone(), frames, 16.0),
        ));
        commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.5, 4.0), FlyCamera::default()));
        commands.insert_resource(camera::CameraMode::Fly);
        return;
    }

    // A slot number (1-49) or a WMB file name.
    let file = match args.level.parse::<u32>() {
        Ok(slot) => flow::file_for(slot).map(str::to_owned).unwrap_or_else(|| panic!("no level in slot {slot}")),
        Err(_) => args.level.clone(),
    };
    pending.0 = Some(file);
    let cam_tf = Transform::default();
    // Sky domes render first on their own layer; the world camera draws on top
    // with a fresh depth buffer, like Acknex's sky pass.
    commands.spawn((
        Camera3d::default(),
        Camera { order: 0, ..default() },
        Tonemapping::Linear,
        level::SKY_LAYER,
        cam_tf,
        SkyCamera,
    ));
    commands.spawn((
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::Linear,
        cam_tf,
        FlyCamera::default(),
        FollowState::default(),
        MainCamera,
    ));
    // `Near` entities (help signs) are drawn last over a fresh depth buffer.
    commands.spawn((
        Camera3d::default(),
        Camera { order: 2, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::Linear,
        level::NEAR_LAYER,
        IsDefaultUiCamera,
        cam_tf,
        SkyCamera,
    ));
}
