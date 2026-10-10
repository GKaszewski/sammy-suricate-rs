//! Minimal HUD: score/goal (top right, blinks on `WarnScoreGoal`), held key colour,
//! lives, and the level-complete banner. The original PCX panels come later.

use bevy::prelude::*;

use crate::actions::player::TICKS;
use crate::game::{GameState, MASTER_KEY};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(Update, update);
    }
}

#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct KeyText;
#[derive(Component)]
struct Banner;

fn setup(mut commands: Commands) {
    let font = TextFont { font_size: FontSize::Px(28.0), ..default() };
    commands.spawn((
        Text::new(""),
        font.clone(),
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: px(12), right: px(16), padding: UiRect::axes(px(10), px(4)), ..default() },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ScoreText,
        TextShadow::default(),
    ));
    commands.spawn((
        Text::new(""),
        font.clone(),
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: px(12), left: px(16), padding: UiRect::axes(px(10), px(4)), ..default() },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        KeyText,
        TextShadow::default(),
    ));
    commands.spawn((
        Text::new(""),
        TextFont { font_size: FontSize::Px(56.0), ..default() },
        TextColor(Color::srgb(1.0, 0.85, 0.2)),
        Node { position_type: PositionType::Absolute, top: percent(40), width: percent(100), justify_content: JustifyContent::Center, ..default() },
        TextLayout::justify(Justify::Center),
        Banner,
        TextShadow::default(),
    ));
}

fn key_name(key: u32) -> (&'static str, Color) {
    match key {
        1 => ("red", Color::srgb(1.0, 0.2, 0.2)),
        2 => ("yellow", Color::srgb(1.0, 0.95, 0.2)),
        3 => ("orange", Color::srgb(1.0, 0.6, 0.1)),
        4 => ("blue", Color::srgb(0.3, 0.5, 1.0)),
        5 => ("magenta", Color::srgb(1.0, 0.3, 1.0)),
        6 => ("green", Color::srgb(0.3, 1.0, 0.3)),
        MASTER_KEY => ("master", Color::WHITE),
        _ => ("none", Color::srgb(0.6, 0.6, 0.6)),
    }
}

fn update(
    time: Res<Time>,
    mut game: ResMut<GameState>,
    mut score: Query<(&mut Text, &mut TextColor), (With<ScoreText>, Without<KeyText>, Without<Banner>)>,
    mut key: Query<(&mut Text, &mut TextColor), (With<KeyText>, Without<ScoreText>, Without<Banner>)>,
    mut banner: Query<&mut Text, (With<Banner>, Without<ScoreText>, Without<KeyText>)>,
) {
    game.warn_score = (game.warn_score - time.delta_secs() * TICKS).max(0.0);
    if let Ok((mut t, mut c)) = score.single_mut() {
        t.0 = format!("{} / {}", game.score, game.goal);
        let blink = game.warn_score > 0.0 && (game.warn_score as i32 / 4) % 2 == 0;
        c.0 = if blink {
            Color::srgb(1.0, 0.2, 0.2)
        } else if game.goal_reached {
            Color::srgb(0.3, 1.0, 0.3)
        } else {
            Color::WHITE
        };
    }
    if let Ok((mut t, mut c)) = key.single_mut() {
        let (name, color) = key_name(game.key);
        t.0 = format!("key: {name}   lives: {}", game.lives);
        c.0 = color;
    }
    if let Ok(mut t) = banner.single_mut() {
        t.0 = if game.won { "Level complete!".into() } else { String::new() };
    }
}
