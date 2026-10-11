//! The order of a frame's game logic, explicit. Every feature plugin adds its systems to
//! one of these sets; the sets run in the order declared here. (The order is part of the
//! game's behaviour: the replay test locks it.)

use bevy::prelude::*;

/// Coarse stages of the `Update` schedule.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameSet {
    /// Level flow (phases, loading requests), the extro fade, flying popups, the sun.
    Flow,
    /// Model animation clocks advance, then scripted animations react.
    Animate,
    /// Camera resets and zones.
    Camera,
    /// The game rules ([`GameSet`] stages).
    Gameplay,
}

/// The gameplay stages, in order.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Engines and platforms move first (even before the animation clocks), so riders
    /// see this frame's positions.
    Platforms,
    /// Sammy's door scan; then the blob shadows follow last frame's movers.
    Doors,
    Player,
    /// Pickups, gems, exits, health pickups, deaths and teleport entry.
    Pickups,
    Teleports,
    Wasps,
    Enemies,
    Effects,
    /// Death/respawn sequences and the reset scan.
    Health,
    Titles,
}

pub struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        // Platforms move before the animation clocks and the camera; the other rules after.
        app.configure_sets(Update, (FrameSet::Flow, GameSet::Platforms, FrameSet::Animate, FrameSet::Camera, FrameSet::Gameplay).chain()).configure_sets(
            Update,
            (
                GameSet::Doors,
                GameSet::Player,
                GameSet::Pickups,
                GameSet::Teleports,
                GameSet::Wasps,
                GameSet::Enemies,
                GameSet::Effects,
                GameSet::Health,
                GameSet::Titles,
            )
                .chain()
                .in_set(FrameSet::Gameplay),
        );
    }
}
