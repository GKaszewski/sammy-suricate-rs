#[allow(unused_imports)]
use super::*;

/// A wasp hit something solid (sent to targets such as lions).
#[derive(Message, Clone, Copy)]
pub struct WaspHit {
    pub target: Entity,
    pub poison: f32,
    pub bounce_force: f32,
    pub wasp_pan: f32,
    pub wasp_disp: [f32; 3],
}
