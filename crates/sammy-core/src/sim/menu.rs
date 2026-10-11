//! The 3D main menu (`menu_greenpepper.WMB`, interfac.wdl; docs/menu.md §3): title
//! models and level icons that bob and spin, the level grids' camera markers, the menu
//! camera and what a choice does. Picking (mouse rays, pad focus) is the frontend's.

use formats::wmb::Object;

use super::world::World;
use super::{Dt, Event, Sim};
use crate::angle::Angles;
use crate::camera::CameraPose;
use crate::flow::Phase;
use crate::id::Id;
use crate::intro::MENU_ORIGIN;
use crate::level::Presentation;
use crate::menu::{Bob, Choice, LEVEL_ICON, Look, MenuAction};
use crate::percent::Percent;
use crate::space::Pos;
use crate::space::Quants;

/// Camera `arc` in the menu (degrees).
pub const MENU_ARC: f32 = 70.0;
/// Icons grow ×1.2 while hovered.
const HOVER_SCALE: f32 = 1.2;
/// Hover light colours (orange; darker for locked icons) and range.
const LIGHT_COLOR: [f32; 3] = [255.0, 180.0, 50.0];
const LIGHT_COLOR_LOCKED: [f32; 3] = [255.0, 100.0, 20.0];
const LIGHT_RANGE: Quants = Quants(300.0);

/// `InitMainMenu` / `InitMainTitle` entity (interfac.wdl:23-199, :950-1014).
#[derive(Clone, Debug)]
pub struct MenuItem {
    /// Skill1 = `ChoixMenu` code (0 decor, 1 Play, 2-4 worlds, 5 Options, 6 Quit,
    /// 20 Back, 50 Bonus, 100+N level icon); 0 once hidden.
    pub code: u32,
    bob: Bob,
    /// A sprite (screen-parallel) rather than a model.
    pub sprite: bool,
    /// Own opacity (Flag3 = 80 %).
    alpha: f32,
    scale: [f32; 3],
    styled: bool,
    /// Icon of a locked / absent level (`LevelNotReady`).
    pub locked: bool,
    /// Absent level: drawn darkened.
    pub dark: bool,
}

impl MenuItem {
    /// A level icon (hover grows it, tooltips name the level).
    pub fn is_icon(&self) -> bool {
        self.code > LEVEL_ICON
    }
}

/// `Pos*Levels` camera marker of a level grid (interfac.wdl:903-923).
#[derive(Clone, Copy, Debug)]
pub struct MenuMarker {
    pub code: u32,
    pub pos: [f32; 3],
    pub pan: f32,
    pub tilt: f32,
}

/// What the menu shows.
#[derive(Clone, Debug, Default)]
pub struct MenuState {
    /// 0 = root view, else the code of the level grid shown (2, 3, 4, 50).
    pub view: u32,
    /// The item under the cursor / pad focus (set by the frontend).
    pub hovered: Option<Id>,
    /// Debug: camera pose (x, y, z, pan, tilt) instead of the view's.
    pub debug_camera: Option<([f32; 3], f32, f32)>,
    /// The item carrying the hover light.
    lit: Option<Id>,
}

pub(super) fn init_item(w: &mut World, id: Id, obj: &Object, title: bool) -> Presentation {
    let sprite = obj.file.to_lowercase().ends_with(".pcx");
    w.menu_items.insert(
        id,
        MenuItem {
            code: obj.skill(1) as u32,
            bob: Bob::from_object(obj, title),
            sprite,
            alpha: crate::menu::base_alpha(obj),
            scale: obj.scale.map(f32::abs),
            styled: false,
            locked: false,
            dark: false,
        },
    );
    // `facing = 1`: sprites are screen-parallel.
    Presentation { facing: sprite, ..Default::default() }
}

pub(super) fn init_marker(w: &mut World, id: Id, obj: &Object, code: u32) -> Presentation {
    w.menu_markers.insert(id, MenuMarker { code, pos: obj.origin, pan: obj.angles[0], tilt: obj.angles[1] });
    Presentation { hidden_but_alive: true, ..Default::default() }
}

impl Sim {
    /// The menu is interactive (the menu scene without the intro running).
    pub fn in_menu(&self) -> bool {
        self.flow.phase == Phase::Menu && !self.intro.active
    }

    /// A click on a menu item (`ChoixMenu`).
    pub fn menu_choice(&mut self, code: u32) {
        match crate::menu::act(Choice::from_code(code), &mut self.save) {
            MenuAction::StartLevel(file) => {
                self.emit(Event::Saved);
                self.request_load(file);
            }
            MenuAction::ShowView(v) => self.menu.view = v.code(),
            MenuAction::OpenOptions => self.emit(Event::OpenOptions),
            MenuAction::Quit => self.emit(Event::Quit),
            MenuAction::Nothing => {}
        }
    }
}

/// Item looks from `LevelsInfo` (interfac.wdl:94-175), hover bob and spin
/// (`InitMainMenu :74-198`, `InitMainTitle :977-1014`).
pub(super) fn step(sim: &mut Sim, dt: Dt) {
    if !sim.in_menu() {
        return;
    }
    let hovered = sim.menu.hovered;
    let w = &mut sim.world;
    // Touch: an orange light on the hovered item (darker for locked icons).
    if sim.menu.lit != hovered {
        if let Some(old) = sim.menu.lit.take() {
            w.lights.remove(&old);
        }
        if let Some(h) = hovered
            && let Some(item) = w.menu_items.get(&h)
        {
            let color = if item.locked { LIGHT_COLOR_LOCKED } else { LIGHT_COLOR };
            w.lights.insert(h, super::Light { color, range: LIGHT_RANGE, fade: None });
        }
        sim.menu.lit = hovered;
    }
    for (id, item) in w.menu_items.iter_mut() {
        let Some(b) = w.bodies.get_mut(id) else { continue };
        if !item.styled {
            item.styled = true;
            match crate::menu::look(Choice::from_code(item.code), item.alpha, &sim.save) {
                Look::Hidden => {
                    b.hidden = true;
                    item.code = 0;
                }
                Look::Shown { alpha } => b.alpha = Percent(alpha),
                Look::Locked { alpha, dark } => {
                    item.locked = true;
                    item.dark = dark;
                    b.alpha = Percent(alpha);
                }
            }
        }
        if item.bob.moves() && item.code != 0 {
            b.pos = Pos(item.bob.step(dt.t));
            if !item.sprite {
                b.angles = Angles::from_array(item.bob.angles);
            }
        }
        let k = if hovered == Some(*id) && item.is_icon() && !item.locked { HOVER_SCALE } else { 1.0 };
        b.scale = item.scale.map(|s| s * k);
    }
}

/// Root view: `CameraPosOrigine`; level grids: hard cut to the marker (arc 70).
pub(super) fn place_camera(sim: &mut Sim) {
    if !sim.in_menu() {
        return;
    }
    let params = &mut sim.camera.params;
    params.arc = MENU_ARC;
    let view = sim.menu.view;
    let (mut pos, mut pan, mut tilt) = match sim.world.menu_markers.values().find(|m| m.code == view && view != 0) {
        Some(m) => (m.pos, m.pan, m.tilt),
        None => params.origin.unwrap_or(MENU_ORIGIN),
    };
    if let Some(c) = sim.menu.debug_camera {
        (pos, pan, tilt) = c;
    }
    sim.camera.pose = Some(CameraPose::Angles { pos, angles: Angles::from_array([pan, tilt, 0.0]) });
}
