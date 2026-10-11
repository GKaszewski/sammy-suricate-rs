//! What to draw this frame: world items and HUD models.

use super::*;

impl Extras {

    /// This frame's world items.
    pub fn items(&self, sim: &Sim) -> Vec<Item> {
        let w = &sim.world;
        let mut out = Vec::new();
        for (id, e) in &w.enemies {
            if e.gone() {
                continue;
            }
            let x = w.bodies.get(id).map_or(e.pos, |b| b.pos);
            out.push(Item::Decal { file: SHADOW_FILE, pos: [x[0], x[1], e.floor() + SHADOW_LIFT], pan: e.pan + SHADOW_PAN_OFFSET, size: SHADOW_SIZE * SHADOW_SCALE * e.scale(), alpha: SHADOW_ALPHA });
            // Range rings while the Eye of Truth is active.
            if sim.game.eye_ticks > 0.0
                && let Some((skin, frame, radius)) = e.ring(sim.game.display_range)
            {
                let s = radius / RING_DIAMETER;
                out.push(Item::Model {
                    file: RING_FILE,
                    skin: skin.clamp(1, RING_SKINS),
                    // A4 frame numbers are 1-based.
                    frame: (frame - 1.0).max(0.0),
                    pos: [e.pos[0], e.pos[1], e.pos[2] + e.circle_z],
                    angles: [0.0; 3],
                    scale: [s, s, 1.0],
                    blend: Blend::Add,
                });
            }
        }
        for r in self.calls.iter().filter(|r| r.delay <= 0.0) {
            out.push(Item::Model {
                file: RING_FILE,
                skin: r.skin.clamp(1, RING_SKINS),
                frame: (r.frame - 1.0).max(0.0),
                pos: r.origin,
                angles: [0.0; 3],
                scale: [r.scale, r.scale, 1.0],
                blend: Blend::Add,
            });
        }
        for p in &self.popups {
            out.push(Item::Sprite { file: p.file, frame: p.frame as u32, pos: p.pos });
        }
        if let Some(l) = self.lose {
            // Facing the camera: turned half round about its up axis.
            out.push(Item::Title { file: LOSE_FILE, pos: l.pos, angles: [self.view.pan + 180.0, -self.view.tilt, 0.0], scale: LOSE_SCALE, light: LOSE_LIGHT });
        }
        out
    }

    /// The camera-space HUD models shown now (with the 2D HUD).
    pub fn hud_models(&self, sim: &Sim) -> Vec<HudModel> {
        let g = &sim.game;
        let mut out = Vec::new();
        if g.ammo > 0 {
            let k = g.ammo_type.clamp(1, WASP_SCALE.len() as u32);
            out.push(HudModel {
                file: WASP_FILE,
                skin: k,
                px: [WASP_SCREEN[0] - crate::hud::W, WASP_SCREEN[1]],
                depth: WASP_DEPTH,
                angles: [self.wasp_pan, 0.0, 0.0],
                scale: WASP_SCALE[k as usize - 1],
                blend: Blend::Alpha,
            });
        }
        if g.eye_ticks > 0.0 {
            out.push(HudModel { file: EYE_FILE, skin: 1, px: EYE_SCREEN, depth: EYE_DEPTH, angles: [0.0; 3], scale: 1.0, blend: Blend::Add });
        }
        out
    }

    /// The model files the extras use (to preload).
    pub const MODELS: [&'static str; 4] = [RING_FILE, LOSE_FILE, WASP_FILE, EYE_FILE];
}
