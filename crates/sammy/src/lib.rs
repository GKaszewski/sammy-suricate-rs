//! Sammy Suricate on Bevy: the frontend for [`sammy_core`] (rendering, audio, input,
//! windows, UI) and the feature plugins that drive the core rules each frame.
// Bevy systems take their parameters (queries, resources) as arguments.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod camera;
pub mod flow;
pub mod gameplay;
pub mod level;
pub mod platform;
pub mod render;
pub mod schedule;
pub mod ui;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::render::view::Tonemapping;

use camera::{CameraPlugin, FlyCamera, FollowState, MainCamera, SkyCamera};
use render::model::{LoadedModel, ModelAnimator, ModelPlugin};
use platform::vfs::GameFiles;

/// Default install location; override with the SAMMY_DIR environment variable.
pub const DEFAULT_GAME_DIR: &str = "/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate";

#[derive(Resource, Clone, Default)]
pub struct Args {
    /// `--model <file> [anim]` shows a single model.
    pub model: Option<(String, Option<String>)>,
    /// Level file name or slot number (default: slot 1).
    pub level: String,
}

/// The game directory: `SAMMY_DIR` or the default install.
pub fn game_dir() -> PathBuf {
    std::env::var_os("SAMMY_DIR").map(PathBuf::from).unwrap_or_else(|| DEFAULT_GAME_DIR.into())
}

/// Runs the game with a window (`sammy [slot | file.wmb | --model file [anim]]`).
pub fn run() -> anyhow::Result<()> {
    let dir = game_dir();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match argv.first().map(String::as_str) {
        Some("--model") => Args {
            model: Some((argv.get(1).cloned().unwrap_or("suricat1.mdl".into()), argv.get(2).cloned())),
            level: String::new(),
        },
        Some(level) => Args { model: None, level: level.to_owned() },
        None => Args { model: None, level: "menu".into() },
    };
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "Sammy Suricate".into(), ..default() }),
        ..default()
    }));
    add_game(&mut app, &dir, args)?;
    app.run();
    Ok(())
}

/// Every feature of the game. Any of them can be left out with
/// `SammyPlugins.build().disable::<…>()`; music, the intro and the debug tools are also
/// Cargo features.
pub struct SammyPlugins;

impl PluginGroup for SammyPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        // Registration order matters to ECS iteration order (and so to some random
        // rolls) until those systems iterate in a defined order: keep it.
        let g = bevy::app::PluginGroupBuilder::start::<Self>()
            .add(schedule::SchedulePlugin)
            .add(CameraPlugin)
            .add(ModelPlugin)
            .add(gameplay::GameplayPlugin)
            .add(render::material::LevelMaterialPlugin)
            .add(ui::hud::HudPlugin)
            .add(flow::FlowPlugin)
            .add(render::lights::LightsPlugin);
        #[cfg(feature = "debug-tools")]
        let g = g.add(platform::debug::DebugPlugin);
        let g = g
            .add(render::shadow::ShadowPlugin)
            .add(render::particles::ParticlePlugin)
            .add(ui::hud3d::Hud3dPlugin)
            .add(ui::text::UiTextPlugin)
            .add(ui::menu::MenuPlugin)
            .add(ui::options::OptionsPlugin);
        #[cfg(feature = "music")]
        let g = g.add(platform::music::MusicPlugin);
        
        g.add(ui::intro::IntroPlugin).add(platform::input::InputPlugin).add(platform::display::DisplayPlugin)
    }
}

/// Adds the game (all feature plugins, data and start-up) to an app that already has
/// Bevy's default plugins (windowed or headless).
pub fn add_game(app: &mut App, dir: &Path, args: Args) -> anyhow::Result<()> {
    let files = GameFiles::open(dir)?;
    // A test may supply its own save (and never touches the player's).
    if !app.world().contains_resource::<platform::save::SaveGame>() {
        app.insert_resource(platform::save::load(dir));
    }
    app.add_plugins(SammyPlugins)
        .insert_resource(files)
        .insert_resource(args)
        .add_systems(Startup, (platform::audio::load_sounds, setup))
        .add_systems(Update, (level::face_camera.in_set(schedule::GameSet::FaceCamera), level::animate_sprites));
    Ok(())
}

/// Fallback scene light: sun illuminance, its direction, and the ambient brightness.
const SUN_ILLUMINANCE: f32 = 8000.0;
const SUN_DIRECTION: Vec3 = Vec3::new(1.0, 2.0, 1.5);
const AMBIENT_BRIGHTNESS: f32 = 400.0;
/// Model viewer (`--model`): animation frame rate and the fly camera's start position.
const VIEWER_FPS: f32 = 16.0;
const VIEWER_CAM_POS: Vec3 = Vec3::new(0.0, 0.5, 4.0);

fn setup(
    mut commands: Commands,
    files: Res<GameFiles>,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pending: ResMut<flow::PendingLoad>,
) {
    commands.spawn((DirectionalLight { illuminance: SUN_ILLUMINANCE, ..default() }, Transform::from_translation(SUN_DIRECTION).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.insert_resource(GlobalAmbientLight { brightness: AMBIENT_BRIGHTNESS, ..default() });

    if let Some((name, anim)) = &args.model {
        let data = files.read(name).expect("model file");
        let model = Arc::new(LoadedModel::load(&data, &mut images, &mut materials).expect("parse model"));
        let frames = anim.as_deref().map(|a| model.animation(a)).unwrap_or_default();
        let mesh = meshes.add(model.build_mesh(frames.first().copied().unwrap_or(0)));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(model.material.clone()),
            Transform::default(),
            ModelAnimator::new(model.clone(), frames, VIEWER_FPS),
        ));
        commands.spawn((Camera3d::default(), Transform::from_translation(VIEWER_CAM_POS), FlyCamera::default()));
        commands.insert_resource(camera::CameraMode::Fly);
        return;
    }

    // A slot number (1-49) or a WMB file name.
    let file = match args.level.parse::<u32>() {
        Ok(slot) => flow::file_for(slot).map(str::to_owned).unwrap_or_else(|| panic!("no level in slot {slot}")),
        Err(_) if args.level == "menu" => ui::menu::MENU_FILE.to_owned(),
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
