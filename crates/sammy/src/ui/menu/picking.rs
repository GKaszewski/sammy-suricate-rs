//! Picking menu items: the mouse ray against their boxes, and the pad focus.

use super::*;
use sammy_present::menu::{CHOICE_BACK, CLICK_VOL, MOUSE_RANGE, TOUCH_VOL};

/// Ray-box test: rays this parallel to a slab are treated as parallel, and boxes are
/// at least this thick (half extent).
const RAY_PARALLEL_EPS: f32 = 1e-8;
const MIN_HALF_EXTENT: f32 = 0.05;

/// The menu items shown (code, locked, icon) by their mirrored entities.
type Items<'w, 's> = Query<'w, 's, (&'static Mirrored, &'static Visibility, &'static GlobalTransform, Option<&'static Aabb>, Option<&'static Children>)>;

/// Ray from the cursor against the items' mesh boxes (`Mouse_Range`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn pick(
    mut game: ResMut<Game>,
    mut state: ResMut<MenuState>,
    mouse: Res<ButtonInput<MouseButton>>,
    controls: Res<crate::platform::input::Controls>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    items: Items,
    parts: Query<(&GlobalTransform, &Aabb)>,
    mut sfx: Sfx,
    mut choices: MessageWriter<MenuChoice>,
    mut particles: ResMut<crate::render::particles::Particles>,
) {
    let (Ok(win), Ok((camera, cam_tf))) = (windows.single(), cam.single()) else { return };
    let ray = if state.modal { None } else { win.cursor_position().and_then(|c| camera.viewport_to_world(cam_tf, c).ok()) };
    let menu = &game.world.menu_items;
    let mut best: Option<(Id, f32)> = None;
    if let Some(ray) = ray {
        for (m, vis, gt, aabb, children) in &items {
            if menu.get(&m.0).is_none_or(|i| i.code == 0) || *vis == Visibility::Hidden {
                continue;
            }
            let own = aabb.map(|a| (*gt, *a));
            let kids = children.into_iter().flatten().filter_map(|c| parts.get(*c).ok().map(|(g, a)| (*g, *a)));
            for (g, a) in own.into_iter().chain(kids) {
                if let Some(d) = ray_obb(ray, &g, &a)
                    && d * coords::QUANTS_PER_UNIT < MOUSE_RANGE
                    && best.is_none_or(|(_, b)| d < b)
                {
                    best = Some((m.0, d));
                }
            }
        }
    }
    let mut now = best.map(|(id, _)| id);
    // A pad press only activates an item that was already focused.
    let before = game.menu.hovered;
    if controls.pad && !state.modal {
        now = pad_focus(&controls, before, camera, cam_tf, &items, &game);
        // B backs out of a level grid.
        if controls.back && game.menu.view != 0 {
            choices.write(MenuChoice(CHOICE_BACK));
        }
    }
    if now != before {
        // Touch: `Toc2` at 25 (the game lights the item).
        if now.is_some() {
            sfx.play_vol("Toc2", TOUCH_VOL);
        }
        state.sparkle = [0.0; 2];
        game.menu.hovered = now;
    }
    let hovered = game.menu.hovered.and_then(|h| game.world.menu_items.get(&h).map(|i| (h, i.clone())));
    // Sparkles orbit a hovered icon (`particle_interface1`, 64 quants out).
    if let Some((h, item)) = &hovered
        && item.is_icon()
        && !item.locked
        && let Some((_, _, gt, ..)) = items.iter().find(|(m, ..)| m.0 == *h)
    {
        for _ in 0..particles.ticks {
            let off = sammy_present::menu::sparkle(&mut state.sparkle);
            let c = coords::native(gt.translation());
            sammy_present::effects::menu_sparkle(&mut particles, [c[0] + off[0], c[1] + off[1], c[2] + off[2]]);
        }
    }
    if (mouse.just_pressed(MouseButton::Left) || (controls.pad && controls.accept && before == game.menu.hovered))
        && !state.modal
        && let Some((_, item)) = hovered
        && !item.locked
    {
        sfx.play_vol("Ding1", CLICK_VOL);
        state.clicked_once = true;
        choices.write(MenuChoice(item.code));
    }
}

/// Gamepad focus over the menu items on screen: the D-pad / stick moves to the nearest
/// item with another code in that direction; the first press picks the one nearest the
/// screen centre.
#[allow(clippy::type_complexity)]
pub(crate) fn pad_focus(
    controls: &crate::platform::input::Controls,
    current: Option<Id>,
    camera: &Camera,
    cam_tf: &GlobalTransform,
    items: &Items,
    game: &Game,
) -> Option<Id> {
    let size = camera.logical_viewport_size()?;
    let on_screen: Vec<(Id, u32, [f32; 2])> = items
        .iter()
        .filter(|(_, vis, ..)| **vis != Visibility::Hidden)
        .filter_map(|(m, _, gt, ..)| {
            let code = game.world.menu_items.get(&m.0).map(|i| i.code).filter(|c| *c != 0)?;
            let p = camera.world_to_viewport(cam_tf, gt.translation()).ok()?;
            (p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y).then_some((m.0, code, p.to_array()))
        })
        .collect();
    sammy_present::menu::pad_focus(&on_screen, current, controls.nav, controls.accept, size.to_array())
}

pub(crate) fn ray_obb(ray: Ray3d, gt: &GlobalTransform, aabb: &Aabb) -> Option<f32> {
    let inv = gt.affine().inverse();
    let o = inv.transform_point3(ray.origin);
    let d = inv.transform_vector3(ray.direction.as_vec3());
    let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents).max(Vec3::splat(MIN_HALF_EXTENT)));
    let (mut t0, mut t1) = (f32::MIN, f32::MAX);
    for i in 0..3 {
        if d[i].abs() < RAY_PARALLEL_EPS {
            if (o[i] - c[i]).abs() > h[i] {
                return None;
            }
            continue;
        }
        let (a, b) = ((c[i] - h[i] - o[i]) / d[i], (c[i] + h[i] - o[i]) / d[i]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    if t1 < t0.max(0.0) {
        return None;
    }
    // Back to world distance.
    Some((gt.transform_point(o + d * t0.max(0.0)) - ray.origin).length())
}
