//! One AI tick of one enemy and the frame's inputs and outputs.

use super::*;

/// Context shared by one enemy's tick.
pub(super) struct Ctx<'a> {
    pub(super) g: &'a LionGlobals,
    pub(super) rng: &'a mut Rng,
    pub(super) sounds: &'a mut Vec<&'static str>,
    pub(super) space: &'a dyn Space,
    pub(super) sammy: Option<(Id, [f32; 3], [f32; 3])>,
    pub(super) ticks1: u32,
    pub(super) me: Id,
    pub(super) calls: &'a mut FriendCalls,
    pub(super) warn: &'a mut bool,
    /// Call rings to spawn at these positions.
    pub(super) call_rings: &'a mut Vec<[f32; 3]>,
    /// The head vertex in model space at the current animation frame.
    pub(super) head_local: Option<[f32; 3]>,
    pub(super) nodes: &'a [PathNode],
}

/// What the world looks like to the enemies this frame.
pub struct World<'a> {
    pub space: &'a dyn Space,
    /// `pov_Syn`: (entity, position, velocity) distances and sight are measured to.
    pub sammy: Option<(Id, [f32; 3], [f32; 3])>,
    /// Sammy himself and whether he can be hurt.
    pub sammy_entity: Option<Id>,
    pub sammy_alive: bool,
    /// Entities Sammy walked into this frame.
    pub touched: &'a [Id],
    pub nodes: &'a [PathNode],
    /// Platforms and their motion this frame.
    pub platforms: &'a [(Id, [f32; 3])],
    pub ticks1: u32,
}

/// Mutable state the enemies share.
pub struct Shared<'a> {
    pub g: &'a LionGlobals,
    pub rng: &'a mut Rng,
    pub calls: &'a mut FriendCalls,
    pub game: &'a mut GameState,
    /// Teleport pads (their busy flags change).
    pub pads: &'a mut [(Id, Teleport)],
}

/// What the enemies want shown.
#[derive(Default, Debug)]
pub struct Out {
    pub sounds: Vec<&'static str>,
    /// A lion started a chase (`Warning_attack`).
    pub warn: bool,
    pub call_rings: Vec<[f32; 3]>,
    /// Hit stars to burst: (position, count).
    pub hit_stars: Vec<([f32; 3], usize)>,
}

/// A wasp impact (EVENT_IMPACT on the lion).
#[derive(Clone, Copy, Debug)]
pub struct Impact {
    pub poison: f32,
    pub bounce_force: f32,
    pub wasp_pan: f32,
    pub wasp_disp: [f32; 3],
}

/// One AI tick of one enemy (the body of the original per-entity loop).
#[allow(clippy::too_many_arguments)]
pub fn tick(e: &mut Enemy, me: Id, anim: &mut Anim, model: &Mdl, player: Option<&mut Player>, w: &World, s: &mut Shared, out: &mut Out) {
    if !e.initialised {
        init_runtime(e, model, w.space, me, s.rng);
    }
    e.prev_pos = e.pos;
    e.prev_pan = e.pan;
    // Teleport pads with Flag2 (mm1.wdl:4236-4285): fade out −2/tick, jump, fade in
    // +1.5/tick, forced idle and intangible throughout.
    if let Some((pad, dest, fading_in)) = e.teleport {
        e.etat = 0;
        if !fading_in {
            e.alpha -= 2.0;
            if e.alpha <= 0.0 {
                e.alpha = 0.0;
                e.pos = dest;
                e.prev_pos = dest;
                e.teleport = Some((pad, dest, true));
            }
        } else {
            e.alpha += 1.5;
            if e.alpha >= 100.0 {
                e.alpha = 100.0;
                e.teleport = None;
                if let Some((_, p)) = s.pads.iter_mut().find(|(id, _)| *id == pad) {
                    p.busy = false;
                }
            }
        }
        return;
    }
    let grounded = e.vspeed == 0.0 && e.pos[2] <= e.floor + e.hover + 41.0;
    if grounded && e.alive() && let Some(fe) = e.floor_entity {
        if let Some((_, pad)) = s.pads.iter_mut().find(|(id, _)| *id == fe)
            && pad.lions
            && !pad.busy
        {
            pad.busy = true;
            e.teleport = Some((fe, pad.dest, false));
            return;
        }
        // Moving platforms carry the lion (`LionForce` = platform motion).
        if let Some((_, d)) = w.platforms.iter().find(|(id, _)| *id == fe) {
            for (p, d) in e.pos.iter_mut().zip(d) {
                *p += d;
            }
        }
    }
    // Contacts with Sammy (either mover): the swipe rules (§4.1).
    let touching = w.sammy_entity.is_some_and(|sm| w.touched.contains(&me) || e_touches(w.space, e, me, sm));
    if touching && w.sammy_alive && e.class == LION
        && let Some(p) = player
    {
        let n = contact_sammy(e, p, s.game, s.g, &mut out.sounds, s.rng);
        if n > 0 {
            out.hit_stars.push((e.pos, n));
        }
    }
    let head_local = (e.head_vertex != 0)
        .then(|| anim.current_frame().and_then(|f| model.frames.get(f)).and_then(|f| f.positions.get(e.head_vertex)).copied())
        .flatten();
    let mut ctx = Ctx {
        g: s.g,
        rng: s.rng,
        sounds: &mut out.sounds,
        space: w.space,
        sammy: w.sammy,
        ticks1: w.ticks1,
        me,
        calls: s.calls,
        warn: &mut out.warn,
        call_rings: &mut out.call_rings,
        head_local,
        nodes: w.nodes,
    };
    enemy_behavior(e, &mut ctx);
    anim_lion(e, &mut ctx);
    sync_anim(e, anim);
}

/// The wasp hits of a frame, applied before the AI ticks.
pub fn apply_impact(e: &mut Enemy, h: &Impact) {
    wasp_hit(e, h);
}

/// Lion moving into Sammy is detected by a tiny probe along its last velocity.
pub(super) fn e_touches(space: &dyn Space, e: &Enemy, me: Id, sammy: Id) -> bool {
    let v = e.vel;
    if v[0].abs() + v[1].abs() < 0.01 {
        return false;
    }
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    let end = [e.pos[0] + v[0] / len * 4.0, e.pos[1] + v[1] / len * 4.0, e.pos[2]];
    space.trace(HULL, e.pos, end, Some(me)).entity == Some(sammy)
}

pub(super) fn init_runtime(e: &mut Enemy, model: &Mdl, space: &dyn Space, me: Id, rng: &mut Rng) {
    e.initialised = true;
    e.circle_z = 8.0 + rng.random(12.0);
    // `Sonar my,2000`, then hover at 28×scale (15×scale for scorpions) (lion.wdl:151-173).
    let tr = space.trace(Hull::Point, e.pos, [e.pos[0], e.pos[1], e.pos[2] - 2000.0], Some(me)).trace;
    if tr.hit() {
        e.floor = tr.end[2];
    }
    let h = if e.flying_init { e.pos[2] - e.floor } else if e.spec == 3 { 15.0 * e.scale } else { 28.0 * e.scale };
    e.pos[2] = e.floor + h;
    e.prev_pos = e.pos;
    e.hover = h;
    // Collision box from the model's first frame.
    if let Some(f) = model.frames.first() {
        let (mn, mx) = (f.bbox_min, f.bbox_max);
        let hx = (mx[0] - mn[0]).max(mx[1] - mn[1]) * 0.5 * e.scale;
        e.half = [hx * 0.7, hx * 0.7, (mx[2] - mn[2]) * 0.5 * e.scale];
        e.center_z = (mx[2] + mn[2]) * 0.5 * e.scale;
    }
}
