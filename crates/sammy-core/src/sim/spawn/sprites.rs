//! Sprites placed after the level is loaded.

use super::*;
use crate::space::Pos;
use crate::keys::KeyColor;


/// Sprites snapped onto the floor below their editor origin (once the level is there).
pub(in crate::sim) fn snap_sprites(sim: &mut Sim) {
    let Some(level) = sim.level.clone() else { return };
    let space = crate::sim::Space { level: Some(&level), brushes: &sim.world.brushes, boxes: &sim.world.boxes };
    let mut done = Vec::new();
    for (id, (origin, dz)) in &sim.world.sprite_ground {
        let [x, y, z] = *origin;
        let tr = crate::collision::Space::trace_static(&space, Hull::Point, [x, y, z], [x, y, z - GROUND_PROBE]).trace;
        done.push((*id, (tr.hit() && !tr.start_solid).then(|| [x, y, tr.end[2] + dz])));
    }
    for (id, pos) in done {
        sim.world.sprite_ground.remove(&id);
        if let (Some(p), Some(b)) = (pos, sim.world.bodies.get_mut(&id)) {
            b.pos = Pos(p);
        }
    }
}

/// A runtime gem sprite of `color` at `pos`.
pub(in crate::sim) fn gem_sprite(sim: &mut Sim, name: &str, color: KeyColor, pos: [f32; 3], scale: f32, fps: f32) -> Option<Id> {
    let file = crate::keys::gem_sprite(color)?;
    let frames = crate::sim::assets::sprite_frames(file);
    let id = sim.world.spawn_body(Body::new(name, pos).scaled(scale));
    sim.world.looks.insert(id, Look::new(Visual::Sprite { file: file.to_owned(), facing: SpriteFacing::Billboard }));
    sim.world.sprite_anims.insert(id, SpriteAnim { frames, frame: 0.0, fps });
    Some(id)
}
