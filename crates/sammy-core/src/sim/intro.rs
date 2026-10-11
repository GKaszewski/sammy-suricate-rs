//! Boot intro (`IntroSammy`, Intro2.wdl:1-229; docs/menu.md §2): inside the menu scene a
//! scripted Sammy walks the path-node loop with the follow camera behind him while the
//! A5, publisher and Suricate logos fade in front of the camera. Node key 999 or a skip
//! key ends it: the camera glides to the menu pose.

use super::{Dt, Sim};
use crate::angle::Angles;
use crate::camera::CameraPose;
use crate::collision::{Hull, Space as _};
use crate::flow::Phase;
use crate::input::Controls;
use crate::intro::{END_KEY, FRICTION, FRICTION_AT, GLIDE_FOG_CLEAR, Logo, MENU_ORIGIN, PLUS_TILT};
use crate::player::BLOB_HAUT;
use crate::space::Pos;
use crate::time::Ticks;
use crate::time::TICKS_PER_SEC;

/// Intro fog is a percentage; the fog starts fully black.
const FULL_PERCENT: f32 = 100.0;
/// `Sonar`: the floor trace starts this far above and ends this far below the feet.
const SONAR_UP: f32 = 200.0;
const SONAR_DOWN: f32 = 2000.0;
/// Walk cycle: `WALK_ANIM` × walk speed percent per tick.
const WALK_ANIM: f32 = 0.5;

/// The intro's state.
#[derive(Clone, Debug, Default)]
pub struct IntroState {
    /// The intro owns the camera and the menu is not interactive yet.
    pub active: bool,
    /// `MoveMenu`: the camera glides back to the menu pose.
    pub gliding: bool,
    acc: Ticks,
    ticks: f32,
    /// `Camera.fog` (0..100, black).
    pub fog: f32,
    pub logo: Logo,
    /// The boot load happened (only a menu at boot plays the intro).
    played: bool,
    loads: u32,
}

impl IntroState {
    /// The follow camera drives the view (not while gliding).
    pub fn following(&self) -> bool {
        self.active && !self.gliding
    }
}

/// Starts the intro on the boot load of the menu.
pub(super) fn start(sim: &mut Sim) {
    let intro = &mut sim.intro;
    if sim.loads == intro.loads {
        return;
    }
    intro.loads = sim.loads;
    intro.active = false;
    intro.gliding = false;
    // Only the boot runs the intro; later menu loads leave Sammy standing (docs §2.6).
    let boot = !intro.played;
    intro.played = true;
    if !boot || sim.flow.phase != Phase::Menu || sim.debug.no_intro {
        return;
    }
    *intro = IntroState { active: true, fog: FULL_PERCENT, played: true, loads: sim.loads, ..Default::default() };
    let mut pan = 0.0;
    for w in sim.world.walkers.values_mut() {
        w.walking = true; // `EtapesIntro = 1`
        pan = w.pan.get();
    }
    // `Camera_PlusTilt = 50`, arc 70, start at `CameraPosOrigine`.
    let c = &mut sim.camera;
    c.params.plus_tilt = PLUS_TILT;
    c.params.arc = super::menu::MENU_ARC;
    c.follow.start_from(MENU_ORIGIN.0, pan, &c.params);
}

pub(super) fn tick(sim: &mut Sim, controls: &Controls, dt: Dt) {
    let skip = controls.confirm || controls.options || controls.accept || controls.back;
    let intro = &mut sim.intro;
    if intro.active && !intro.gliding && skip {
        intro.gliding = true;
    }
    let t = dt.t;
    let nodes: Vec<crate::intro::PathNode> = sim.world.intro_nodes.values().cloned().collect();
    for w in sim.world.walkers.values_mut() {
        if !w.walking || t <= Ticks::ZERO {
            continue;
        }
        // Sub-step long frames so the node logic keeps its 1-tick resolution.
        let n = t.get().ceil().max(1.0);
        for _ in 0..n as u32 {
            w.step(&nodes, t / n);
        }
        if intro.active && !intro.gliding && w.key == END_KEY {
            intro.gliding = true;
        }
    }
    intro.acc += t;
    while intro.acc >= Ticks(1.0) {
        intro.acc -= Ticks(1.0);
        if !intro.active {
            continue;
        }
        intro.ticks += 1.0;
        // After the first 8 (+4) ticks: `Camera3Friction = 0.1`, vertical 0.5.
        if intro.ticks == FRICTION_AT {
            (sim.camera.params.fric_h, sim.camera.params.fric_v) = FRICTION;
        }
        crate::intro::logo_tick(&mut intro.logo, &mut intro.fog);
        if intro.gliding {
            intro.fog = (intro.fog - GLIDE_FOG_CLEAR).max(0.0);
        }
    }
    // `Sonar`: keep the feet on the floor; walk cycle at 0.5 × speed % per tick.
    let space = super::Space::new(&sim.level, &sim.world);
    let mut placed = Vec::new();
    for (id, w) in &sim.world.walkers {
        let [x, y, z] = w.pos;
        let mut pos = w.pos;
        if sim.level.is_some() {
            let tr = space.trace_static(Hull::Point, [x, y, z + SONAR_UP], [x, y, z - SONAR_DOWN]).trace;
            if tr.hit() && !tr.start_solid {
                pos[2] = tr.end[2] + BLOB_HAUT;
            }
        }
        placed.push((*id, pos, w.pan, if w.walking { WALK_ANIM * w.speed() } else { 0.0 }));
    }
    for (id, pos, pan, speed) in placed {
        if let Some(w) = sim.world.walkers.get_mut(&id) {
            w.pos = pos;
        }
        if let Some(b) = sim.world.bodies.get_mut(&id) {
            b.pos = Pos(pos);
            b.angles = Angles::from_array([pan.get(), 0.0, 0.0]);
        }
        if let Some(a) = sim.world.animators.get_mut(&id) {
            a.anim.set_speed_pct(speed, TICKS_PER_SEC);
        }
    }
}

/// `MoveMenu` (interfac.wdl:625-653): position and angles ease 0.2 per tick to the menu
/// pose; done within 2 quants in X and Z.
pub(super) fn glide(sim: &mut Sim, dt: Dt) {
    if !sim.intro.gliding {
        return;
    }
    let Some(pose) = sim.camera.pose else { return };
    let (mut pos, mut pan, mut tilt) = super::camera::pan_tilt(&pose);
    if crate::intro::glide(&mut pos, &mut pan, &mut tilt, MENU_ORIGIN, dt.t) {
        let intro = &mut sim.intro;
        intro.active = false;
        intro.gliding = false;
        intro.fog = 0.0;
    }
    sim.camera.pose = Some(CameraPose::Angles { pos, angles: Angles::from_array([pan, tilt, 0.0]) });
}

impl IntroState {
    /// What the intro shows in front of the camera: the logo (0 none, 1 A5 sprite,
    /// 2 publisher, 3 Suricate), its distance ahead and opacity, and the black fog
    /// (percent).
    pub fn overlay(&self) -> (u8, f32, f32, f32) {
        if self.active { (self.logo.shown, self.logo.dist, self.logo.alpha, self.fog) } else { (0, 0.0, 0.0, 0.0) }
    }
}
