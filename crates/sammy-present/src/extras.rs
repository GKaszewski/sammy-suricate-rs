//! What the game shows that is not a game entity, for any frontend: enemy ground
//! shadows and range rings (`InitOmbreLion`, `DisplayEnnemyRange`), friend-call rings
//! (`Circle_CallFriends`), the +50/+100 score popups (`FlyingPCX`), the lose title
//! (`InitTitleLoose`), the camera-space HUD wasp and Eye of Truth plate (docs/hud-saves.md
//! §1.6-1.7) and the sun angle of the engine model shadows. [`Extras`] keeps their state
//! and lists them each frame as plain items in native coordinates.

use sammy_core::flow::Phase;
use sammy_core::sim::{Event, Sim};
use sammy_core::time::TICKS_PER_SEC;

use crate::particles::Particles;

/// Lion shadow: `shadlio2.pcx` (48 quants square) at 1.2 × the lion's scale, 0.5 above
/// the floor, turned with the lion's pan + 90, half transparent.
const SHADOW_FILE: &str = "shadlio2.pcx";
const SHADOW_SCALE: f32 = 1.2;
const SHADOW_SIZE: f32 = 48.0;
const SHADOW_LIFT: f32 = 0.5;
const SHADOW_PAN_OFFSET: f32 = 90.0;
const SHADOW_ALPHA: f32 = 0.5;
/// Range rings: `circl100.mdl` skins 1..=`RING_SKINS`, 1000 quants across at scale 1.
pub const RING_FILE: &str = "circl100.mdl";
const RING_SKINS: u32 = 3;
const RING_DIAMETER: f32 = 1000.0;
/// `Circle_CallFriends`: scale grows `CALL_GROW` per tick until `CALL_EASE_FROM`, then
/// eases (`CALL_EASE_RATE`) toward `CALL_EASE_TARGET`; the frame runs `CALL_FRAME_RATE`
/// per tick; the ring ends at frame `CALL_END_FRAME` or scale `CALL_END_SCALE`.
const CALL_START_SCALE: f32 = 0.01;
const CALL_GROW: f32 = 0.2;
const CALL_EASE_FROM: f32 = 3.0;
const CALL_EASE_TARGET: f32 = 5.1;
const CALL_EASE_RATE: f32 = 0.5;
const CALL_FRAME_RATE: f32 = 4.0;
const CALL_END_FRAME: f32 = 100.0;
const CALL_END_SCALE: f32 = 5.0;
/// `Bonus3_Hit` popups: points → (sprite, life in ticks). The popup starts `POPUP_LIFT`
/// above the pickup, flies at `POPUP_SPEED` per tick toward the camera tilted up by
/// `POPUP_TILT_DEG` (`InvDirCamera`), and plays its strip at this many frames per tick.
const POPUPS: [(i32, &str, f32); 2] = [(50, "plus50+6.pcx", 32.0), (100, "plu100+2.pcx", 48.0)];
const POPUP_LIFT: f32 = 32.0;
const POPUP_SPEED: f32 = 40.0;
const POPUP_TILT_DEG: f32 = 20.0;
const POPUP_FRAMES_PER_TICK: f32 = 0.2;
/// The lose title (`InitTitleLoose`, mm1.wdl:5314-5393) starts `LOSE_START_DEPTH` ahead
/// moving `LOSE_START_SPEED` per tick, slows by `LOSE_DECEL` (floor `LOSE_MIN_SPEED`)
/// and stops past `LOSE_STOP_DEPTH`; it sits `LOSE_RAISE` above the view axis, turned
/// to face the camera, at scale `LOSE_SCALE`, lit amber, smoking.
const LOSE_FILE: &str = "lose.mdl";
const LOSE_START_DEPTH: f32 = 20.0;
const LOSE_START_SPEED: f32 = 20.0;
const LOSE_DECEL: f32 = 0.8;
const LOSE_MIN_SPEED: f32 = 2.0;
const LOSE_STOP_DEPTH: f32 = 128.0;
const LOSE_RAISE: f32 = 20.0;
const LOSE_SCALE: f32 = 0.32;
const LOSE_LIGHT: ([f32; 3], f32) = ([250.0, 200.0, 50.0], 500.0);
/// HUD wasp (`InitCurrentPanWasp`, panels.wdl:1411-1461): scale per wasp type, mode-6
/// screen pixel (camera space (100,-45,-35)), depth in quants, start pan (also its
/// pitch) and spin per tick.
const WASP_FILE: &str = "panwasp.mdl";
const WASP_SCALE: [f32; 7] = [0.3, 0.23, 0.2, 0.2, 0.25, 0.3, 0.2];
const WASP_SCREEN: [f32; 2] = [569.0, 434.0];
const WASP_DEPTH: f32 = 100.0;
const WASP_PAN_START: f32 = 90.0;
const WASP_SPIN: f32 = 2.0;
/// Eye of Truth plate: mode-6 screen pixel (camera space (200,-45,-72)), depth, additive.
const EYE_FILE: &str = "paneye.mdl";
const EYE_SCREEN: [f32; 2] = [445.0, 440.0];
const EYE_DEPTH: f32 = 200.0;
/// `sun_angle` (pan, tilt) of the model shadows: 135/85 in levels (mm1.wdl:618-620),
/// pan 90 in the extro (Extro.wdl:81); it circles at 10°/tick on the win/lose screen
/// (mm1.wdl:966).
const SUN: [f32; 2] = [135.0, 85.0];
const EXTRO_SUN_PAN: f32 = 90.0;
const SUN_SPIN_RATE: f32 = 10.0;

/// How an item blends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    Alpha,
    /// A4 `flare`.
    Add,
}

/// Something to draw in the world (native coordinates; angles are A4 pan/tilt/roll).
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A model at one frame (0-based, fractional frames blend), unlit.
    Model { file: &'static str, skin: u32, frame: f32, pos: [f32; 3], angles: [f32; 3], scale: [f32; 3], blend: Blend },
    /// A model drawn over the level (near layer), lit by its own light.
    Title { file: &'static str, pos: [f32; 3], angles: [f32; 3], scale: f32, light: ([f32; 3], f32) },
    /// A flat square lying on the ground, `size` quants across, turned by `pan`.
    Decal { file: &'static str, pos: [f32; 3], pan: f32, size: f32, alpha: f32 },
    /// A camera-facing sprite strip at one frame.
    Sprite { file: &'static str, frame: u32, pos: [f32; 3] },
}

/// A model glued to the camera, next to the 2D HUD: placed at a mode-6 screen pixel
/// (`px`, x counted from the right edge when negative) and `depth` quants along the
/// view, turned by `angles` relative to the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct HudModel {
    pub file: &'static str,
    pub skin: u32,
    pub px: [f32; 2],
    pub depth: f32,
    pub angles: [f32; 3],
    pub scale: f32,
    pub blend: Blend,
}

/// The camera as the extras need it: position and A4 pan/tilt.
#[derive(Clone, Copy, Debug, Default)]
pub struct View {
    pub pos: [f32; 3],
    pub pan: f32,
    pub tilt: f32,
}

impl View {
    /// From a position looking along `forward`.
    pub fn looking(pos: [f32; 3], forward: [f32; 3]) -> Self {
        let h = forward[0].hypot(forward[1]);
        Self { pos, pan: forward[1].atan2(forward[0]).to_degrees(), tilt: forward[2].atan2(h).to_degrees() }
    }

    fn forward(&self) -> [f32; 3] {
        let (p, t) = (self.pan.to_radians(), self.tilt.to_radians());
        [t.cos() * p.cos(), t.cos() * p.sin(), t.sin()]
    }

    fn up(&self) -> [f32; 3] {
        let (p, t) = (self.pan.to_radians(), self.tilt.to_radians());
        [-t.sin() * p.cos(), -t.sin() * p.sin(), t.cos()]
    }
}

#[derive(Clone, Debug)]
struct CallRing {
    delay: f32,
    scale: f32,
    frame: f32,
    origin: [f32; 3],
    skin: u32,
}

#[derive(Clone, Debug)]
struct Popup {
    file: &'static str,
    pos: [f32; 3],
    dir: [f32; 3],
    life: f32,
    frame: f32,
    frames: u32,
}

#[derive(Clone, Copy, Debug)]
struct LoseTitle {
    depth: f32,
    speed: f32,
    pos: [f32; 3],
}

/// The extras' state.
#[derive(Clone, Debug)]
pub struct Extras {
    calls: Vec<CallRing>,
    popups: Vec<Popup>,
    lose: Option<LoseTitle>,
    wasp_pan: f32,
    /// `sun_angle` (pan, tilt) in degrees: the direction the sun is seen from.
    pub sun: [f32; 2],
    view: View,
}

impl Default for Extras {
    fn default() -> Self {
        Self { calls: Vec::new(), popups: Vec::new(), lose: None, wasp_pan: WASP_PAN_START, sun: SUN, view: View::default() }
    }
}

impl Extras {
    /// Takes a game event (call rings and popups start, a new level clears).
    pub fn event(&mut self, sim: &Sim, e: &Event) {
        match *e {
            Event::Loaded { .. } => {
                *self = Self { wasp_pan: self.wasp_pan, view: self.view, ..Self::default() };
                if sim.flow.slot == Some(sammy_core::levels::EXTRO_SLOT) {
                    self.sun[0] = EXTRO_SUN_PAN;
                }
            }
            Event::CallRings { at } => {
                for (dz, delay, skin) in sammy_core::enemies::CALL_RINGS {
                    self.calls.push(CallRing { delay, scale: CALL_START_SCALE, frame: 1.0, origin: [at[0], at[1], at[2] + dz], skin });
                }
            }
            Event::Popup { at, points } => {
                let Some(&(_, file, life)) = POPUPS.iter().find(|p| p.0 == points) else { return };
                let pos = [at[0], at[1], at[2] + POPUP_LIFT];
                // Pan toward the camera, tilted up (`InvDirCamera`).
                let cam = self.view.pos;
                let pan = (cam[1] - pos[1]).atan2(cam[0] - pos[0]);
                let tilt = POPUP_TILT_DEG.to_radians();
                let dir = [pan.cos() * tilt.cos(), pan.sin() * tilt.cos(), tilt.sin()];
                let frames = sammy_core::sim::assets::sprite_frames(file);
                self.popups.push(Popup { file, pos, dir, life, frame: 0.0, frames });
            }
            _ => {}
        }
    }

    /// Advances `secs` of game time seen from `view`; the lose title smokes into
    /// `particles` (once per particle tick).
    pub fn update(&mut self, sim: &Sim, view: View, secs: f32, particles: &mut Particles) {
        self.view = view;
        let t = secs * TICKS_PER_SEC;
        self.wasp_pan += WASP_SPIN * t;
        if matches!(sim.flow.phase, Phase::Won(_) | Phase::Lost(_)) {
            self.sun[0] += SUN_SPIN_RATE * t;
        }
        self.calls.retain_mut(|r| {
            if r.delay > 0.0 {
                r.delay -= t;
                return true;
            }
            if r.scale < CALL_EASE_FROM {
                r.scale += CALL_GROW * t;
            } else {
                r.scale += (CALL_EASE_TARGET - r.scale) * CALL_EASE_RATE * t;
            }
            r.frame += CALL_FRAME_RATE * t;
            r.frame < CALL_END_FRAME && r.scale < CALL_END_SCALE
        });
        self.popups.retain_mut(|p| {
            for (c, d) in p.pos.iter_mut().zip(p.dir) {
                *c += d * POPUP_SPEED * t;
            }
            p.life -= t;
            p.frame = (p.frame + POPUP_FRAMES_PER_TICK * t).rem_euclid(p.frames.max(1) as f32);
            p.life > 0.0
        });
        // The lose title: flies in, slows, stops; camera-relative.
        if matches!(sim.flow.phase, Phase::Lost(_)) {
            let l = self.lose.get_or_insert(LoseTitle { depth: LOSE_START_DEPTH, speed: LOSE_START_SPEED, pos: view.pos });
            if l.depth < LOSE_STOP_DEPTH {
                l.depth += l.speed * t;
                l.speed = (l.speed - LOSE_DECEL * t).max(LOSE_MIN_SPEED);
            }
            let (f, u) = (view.forward(), view.up());
            l.pos = [0, 1, 2].map(|i| view.pos[i] + f[i] * l.depth + u[i] * LOSE_RAISE);
            for _ in 0..particles.ticks {
                crate::effects::tile_smoke(particles, l.pos);
            }
        } else {
            self.lose = None;
        }
    }

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
