//! The 3D main menu's presentation (interfac.wdl; docs/menu.md §3), for any frontend:
//! pad focus over items on screen, the level tooltips, the version text and cursor, the
//! hover sparkles and the wide-screen back walls of the level grids. The menu's state
//! and choices are the game's ([`sammy_core::sim::MenuState`]).

use formats::wmb::Wmb;
use sammy_core::menu::LEVEL_ICON;
use sammy_core::save::{LevelState, SaveGame};
use sammy_core::sim::{MenuItem, MenuMarker};

use crate::text::WdlStrings;

/// `ChoixMenu` code 20: back to the root view.
pub const CHOICE_BACK: u32 = 20;
/// `Mouse_Range`: picking reach in quants.
pub const MOUSE_RANGE: f32 = 6000.0;
/// `Toc2` touch / `Ding1` click volumes.
pub const TOUCH_VOL: f32 = 25.0;
pub const CLICK_VOL: f32 = 50.0;
/// Version text, at this HUD pixel.
pub const VERSION_TEXT: &[u8] = b"Sammy Suricate V1.42";
pub const VERSION_POS: [f32; 2] = [2.0, 4.0];
/// Cursor sprite (`arrow.pcx`) size; its hot spot is the centre.
pub const CURSOR_FILE: &str = "arrow.pcx";
pub const CURSOR_SIZE: f32 = 16.0;
pub const CURSOR_HOTSPOT: f32 = 8.0;
/// Tooltip offsets from the pointer: title x (unlocked / locked), and the locked message.
const TOOLTIP_DX: f32 = 20.0;
const TOOLTIP_DX_LOCKED: f32 = 32.0;
const TOOLTIP_MSG_OFFSET: [f32; 2] = [-64.0, 26.0];
/// `particle_interface1`: sparkle orbit radius (quants), sideways wobble and per-tick
/// angle steps (degrees).
const SPARKLE_RADIUS: f32 = 64.0;
const SPARKLE_WOBBLE: f32 = 5.0;
const SPARKLE_STEP_A: f32 = 40.0;
const SPARKLE_STEP_B: f32 = 15.0;
/// Focus moves: minimum progress along the direction, and the weight of the sideways miss.
const FOCUS_MIN_ALONG: f32 = 1.0;
const FOCUS_CROSS_WEIGHT: f32 = 2.0;
/// Grid back walls: a face is one when its normal points this far along -Y; the search
/// window around the camera marker (depth ahead, half-width, half-height); faces within
/// `WALL_THICKNESS` of the farthest belong to it; the extension quad sits
/// `WALL_QUAD_OFFSET` behind and reaches `WALL_QUAD_SIDE` / `WALL_QUAD_VERT` out.
const WALL_NORMAL_Y: f32 = -0.99;
const WALL_DEPTH: f32 = 1500.0;
const WALL_HALF_X: f32 = 700.0;
const WALL_HALF_Z: f32 = 600.0;
const WALL_THICKNESS: f32 = 1.0;
const WALL_QUAD_OFFSET: f32 = 2.0;
const WALL_QUAD_SIDE: f32 = 8000.0;
const WALL_QUAD_VERT: f32 = 1200.0;
/// The extension walls are drawn unlit, dimmed to roughly the wall's baked light [I].
pub const WALL_LIGHT: f32 = 0.8;

/// The item `dir` (x right, y up) moves to from `at`: the nearest in that direction, the
/// sideways miss counting double (screen coordinates, y down). `None` if there is none.
pub fn nearest_in_direction<K: Copy>(at: [f32; 2], dir: [i32; 2], candidates: impl IntoIterator<Item = (K, [f32; 2])>) -> Option<K> {
    let d = [dir[0] as f32, -dir[1] as f32];
    candidates
        .into_iter()
        .filter_map(|(k, p)| {
            let v = [p[0] - at[0], p[1] - at[1]];
            let along = v[0] * d[0] + v[1] * d[1];
            let miss = [v[0] - d[0] * along, v[1] - d[1] * along];
            (along > FOCUS_MIN_ALONG).then(|| (k, along + FOCUS_CROSS_WEIGHT * miss[0].hypot(miss[1])))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(k, _)| k)
}

/// Gamepad focus over the menu items on screen (`(key, code, screen position)`): a move
/// goes to the nearest item with another code that way; the first press picks the one
/// nearest the screen centre.
pub fn pad_focus<K: Copy + PartialEq>(items: &[(K, u32, [f32; 2])], current: Option<K>, nav: Option<[i32; 2]>, accept: bool, screen: [f32; 2]) -> Option<K> {
    let Some(&(_, code, at)) = current.and_then(|c| items.iter().find(|i| i.0 == c)) else {
        if nav.is_some() || accept {
            let centre = [screen[0] / 2.0, screen[1] / 2.0];
            let dist = |p: [f32; 2]| (p[0] - centre[0]).hypot(p[1] - centre[1]);
            return items.iter().min_by(|a, b| dist(a.2).total_cmp(&dist(b.2))).map(|i| i.0);
        }
        return None;
    };
    let Some(d) = nav else { return current };
    nearest_in_direction(at, d, items.iter().filter(|i| i.1 != code).map(|i| (i.0, i.2))).or(current)
}

/// A level icon's tooltip: its name, and for a locked level why (`[x, y]` in HUD pixels).
pub struct Tooltip {
    pub title: Vec<u8>,
    pub title_at: [f32; 2],
    pub message: Option<(Vec<u8>, [f32; 2])>,
}

/// The tooltip of `item` with the pointer at `at` (HUD pixels), if it is a level icon.
pub fn tooltip(item: &MenuItem, save: &SaveGame, strings: &WdlStrings, at: [f32; 2]) -> Option<Tooltip> {
    if !item.is_icon() {
        return None;
    }
    let n = item.code - LEVEL_ICON;
    let title = strings.get(&format!("level{n}_Str")).to_vec();
    let dx = if item.locked { TOOLTIP_DX_LOCKED } else { TOOLTIP_DX };
    let message = item.locked.then(|| {
        let msg = if save.state(n) == LevelState::Locked { "mustbewin_Str" } else { "LevelNothere_Str" };
        (strings.get(msg).to_vec(), [at[0] + TOOLTIP_MSG_OFFSET[0], at[1] + TOOLTIP_MSG_OFFSET[1]])
    });
    Some(Tooltip { title, title_at: [at[0] + dx, at[1]], message })
}

/// Sparkles orbiting a hovered icon (`particle_interface1`): the offset from the icon
/// for the orbit angles, which then advance one tick.
pub fn sparkle(angles: &mut [f32; 2]) -> [f32; 3] {
    let (a, b) = (angles[0].to_radians(), angles[1].to_radians());
    let off = [
        SPARKLE_RADIUS * a.cos() * b.cos() - SPARKLE_WOBBLE * a.sin(),
        SPARKLE_RADIUS * a.sin() * b.cos() + SPARKLE_WOBBLE * a.cos(),
        SPARKLE_RADIUS * b.sin(),
    ];
    angles[0] += SPARKLE_STEP_A;
    angles[1] += SPARKLE_STEP_B;
    off
}

/// A wide-screen extension of a level grid's back wall: corners (native, CCW from
/// bottom-left) and texture coordinates, textured with the wall's `texture`.
#[derive(Clone, Debug)]
pub struct WallQuad {
    pub texture: usize,
    pub corners: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
}

/// Port: the level-grid rooms are built only about 4:3 wide, so wide screens saw the
/// void beside them. Each room's back wall (the farthest face looking back at its camera
/// marker) is continued sideways and vertically by a quad just behind it, with the same
/// texture and texture alignment.
pub fn grid_walls<'a>(w: &Wmb, markers: impl IntoIterator<Item = &'a MenuMarker>) -> Vec<WallQuad> {
    let mut out = Vec::new();
    for m in markers {
        let [mx, my, mz] = m.pos;
        let faces: Vec<(usize, [f32; 3], [f32; 3])> = (0..w.faces.len())
            .filter(|&f| w.face_visible(f) && w.face_normal(f)[1] < WALL_NORMAL_Y)
            .map(|f| {
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for v in w.face_vertices(f) {
                    for i in 0..3 {
                        lo[i] = lo[i].min(v[i]);
                        hi[i] = hi[i].max(v[i]);
                    }
                }
                (f, lo, hi)
            })
            .filter(|(_, lo, hi)| lo[1] > my && lo[1] < my + WALL_DEPTH && hi[0] > mx - WALL_HALF_X && lo[0] < mx + WALL_HALF_X && hi[2] > mz - WALL_HALF_Z && lo[2] < mz + WALL_HALF_Z)
            .collect();
        let Some(wall_y) = faces.iter().map(|f| f.1[1]).reduce(f32::max) else { continue };
        let wall: Vec<_> = faces.iter().filter(|f| (f.1[1] - wall_y).abs() < WALL_THICKNESS).collect();
        let Some(&&(face, ..)) = wall.first() else { continue };
        let (Some(tex), ti) = (w.face_texture(face), &w.texinfos[w.faces[face].texinfo as usize]) else { continue };
        let (lo_x, hi_x) = wall.iter().fold((f32::MAX, f32::MIN), |a, f| (a.0.min(f.1[0]), a.1.max(f.2[0])));
        let (lo_z, hi_z) = wall.iter().fold((f32::MAX, f32::MIN), |a, f| (a.0.min(f.1[2]), a.1.max(f.2[2])));
        let t = &w.textures[tex];
        let (tw, th) = (t.width as f32, t.height as f32);
        let y = wall_y + WALL_QUAD_OFFSET;
        // Vertically only a little: the rooms lie under the menu desert.
        let corners = [
            [lo_x - WALL_QUAD_SIDE, y, lo_z - WALL_QUAD_VERT],
            [hi_x + WALL_QUAD_SIDE, y, lo_z - WALL_QUAD_VERT],
            [hi_x + WALL_QUAD_SIDE, y, hi_z + WALL_QUAD_VERT],
            [lo_x - WALL_QUAD_SIDE, y, hi_z + WALL_QUAD_VERT],
        ];
        let uvs = corners.map(|p| {
            let st = ti.st(p);
            [st[0] / tw, st[1] / th]
        });
        out.push(WallQuad { texture: tex, corners, uvs });
    }
    out
}

#[cfg(test)]
mod tests;
