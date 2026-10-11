//! Sammy Suricate on Bevy: the frontend for [`sammy_core`]. The game runs as one
//! [`sammy_core::sim::Sim`] ticked each frame ([`game`]); the plugins here show it
//! (scene, cameras, HUD, menus, effects) and connect it to the machine (input, audio,
//! windows, files, saves).
// Bevy systems take their parameters (queries, resources) as arguments.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod camera;
pub mod game;
pub mod platform;
pub mod render;
pub mod scene;
pub mod ui;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::render::view::Tonemapping;

use camera::{CameraPlugin, FlyCamera, MainCamera, SkyCamera};
use platform::vfs::GameFiles;
use render::model::{LoadedModel, ModelAnimator, ModelPlugin};

#[derive(Resource, Clone, Default)]
pub struct Args {
    /// `--model <file> [anim]` shows a single model.
    pub model: Option<(String, Option<String>)>,
    /// Level file name or slot number (default: slot 1).
    pub level: String,
}

/// Runs the game with a window:
/// `sammy [--data <game folder>] [slot | file.wmb | menu | --model file [anim]]`.
pub fn run() -> anyhow::Result<()> {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let data = argv.iter().position(|a| a == "--data").map(|i| {
        let dir = argv.get(i + 1).cloned().unwrap_or_default();
        argv.drain(i..(i + 2).min(argv.len()));
        std::path::PathBuf::from(dir)
    });
    let args = match argv.first().map(String::as_str) {
        Some("--model") => Args {
            model: Some((argv.get(1).cloned().unwrap_or("suricat1.mdl".into()), argv.get(2).cloned())),
            level: String::new(),
        },
        Some(level) => Args { model: None, level: level.to_owned() },
        None => Args { model: None, level: "menu".into() },
    };
    let storage = sammy_platform::Storage::user();
    let files = sammy_platform::game_files(data, &storage)?;
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "Sammy Suricate".into(), ..default() }),
        ..default()
    }));
    add_game(&mut app, files, storage, args);
    app.run();
    Ok(())
}

/// Every feature of the game. Any of them can be left out with
/// `SammyPlugins.build().disable::<…>()`; music, the intro and the debug tools are also
/// Cargo features.
pub struct SammyPlugins;

impl PluginGroup for SammyPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let g = bevy::app::PluginGroupBuilder::start::<Self>()
            .add(platform::input::InputPlugin)
            .add(platform::display::DisplayPlugin)
            .add(game::GamePlugin)
            .add(scene::ScenePlugin)
            .add(CameraPlugin)
            .add(ModelPlugin)
            .add(render::material::LevelMaterialPlugin)
            .add(render::lights::LightsPlugin)
            .add(render::shadow::ShadowPlugin)
            .add(render::particles::ParticlePlugin)
            .add(ui::text::UiTextPlugin)
            .add(ui::hud::HudPlugin)
            .add(ui::menu::MenuPlugin)
            .add(ui::options::OptionsPlugin);
        #[cfg(feature = "intro")]
        let g = g.add(ui::intro::IntroPlugin);
        #[cfg(feature = "music")]
        let g = g.add(platform::music::MusicPlugin);
        #[cfg(feature = "debug-tools")]
        let g = g.add(platform::debug::DebugPlugin);
        g
    }
}

/// Adds the game (all feature plugins, data and start-up) to an app that already has
/// Bevy's default plugins (windowed or headless).
pub fn add_game(app: &mut App, files: sammy_platform::GameFiles, storage: sammy_platform::Storage, args: Args) {
    // A test may supply its own game (and never touches the player's save).
    if !app.world().contains_resource::<game::Game>() {
        app.insert_resource(game::new_game(platform::save::load(&storage, &files)));
    }
    app.add_plugins(SammyPlugins)
        .insert_resource(GameFiles(files))
        .insert_resource(platform::storage::Storage(storage))
        .insert_resource(args)
        .add_systems(Startup, (platform::audio::load_sounds, setup));
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
    mut game: ResMut<game::Game>,
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
        Ok(slot) => sammy_core::levels::file_for(sammy_core::levels::LevelSlot(slot)).map(str::to_owned).unwrap_or_else(|| panic!("no level in slot {slot}")),
        Err(_) if args.level == "menu" => sammy_core::levels::MENU_FILE.to_owned(),
        Err(_) => args.level.clone(),
    };
    game.request_load(file);
    let cam_tf = Transform::default();
    // Sky domes render first on their own layer; the world camera draws on top
    // with a fresh depth buffer, like Acknex's sky pass.
    commands.spawn((
        Camera3d::default(),
        Camera { order: 0, ..default() },
        Tonemapping::Linear,
        scene::SKY_LAYER,
        cam_tf,
        SkyCamera,
    ));
    commands.spawn((
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::Linear,
        cam_tf,
        FlyCamera::default(),
        MainCamera,
    ));
    // `Near` entities (help signs) are drawn last over a fresh depth buffer.
    commands.spawn((
        Camera3d::default(),
        Camera { order: 2, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::Linear,
        scene::NEAR_LAYER,
        IsDefaultUiCamera,
        cam_tf,
        SkyCamera,
    ));
}
