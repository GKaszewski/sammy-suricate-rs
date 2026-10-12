//! Keeping the scene in step with the game: the level geometry and entity views.

use super::*;

/// Level chunk edge (quants): batches are split so a dynamic light only re-shades the
/// chunks it reaches and off-screen chunks are skipped.
pub(super) const CHUNK: f32 = 1024.0;

/// Splits a batch's triangles by the chunk of their centre: each chunk's indices and
/// bounding sphere.
pub(super) fn chunks(positions: &[[f32; 3]], indices: &[u32]) -> Vec<(Vec<u32>, Sphere)> {
    let mut cells: std::collections::BTreeMap<[i32; 3], Vec<u32>> = Default::default();
    for tri in indices.as_chunks::<3>().0.iter() {
        let c = tri.iter().map(|&i| Vec3::from(positions[i as usize])).sum::<Vec3>() / 3.0;
        cells.entry((c / CHUNK).floor().as_ivec3().to_array()).or_default().extend_from_slice(tri);
    }
    cells
        .into_values()
        .map(|idx| {
            let (lo, hi) = idx.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), &i| (lo.min(Vec3::from(positions[i as usize])), hi.max(Vec3::from(positions[i as usize]))));
            (idx, ((lo + hi) / 2.0, (hi - lo).length() / 2.0))
        })
        .collect()
}

impl Scene {
    /// Rebuilds the level geometry after a load, and the entity views. Animated models
    /// in view of `cam` (all without one) get this frame's pose.
    pub fn sync(&mut self, gfx: &Gfx, files: &GameFiles, sim: &Sim, cam: Option<&Camera>) {
        if self.loaded != sim.loads {
            self.loaded = sim.loads;
            self.build_level(gfx, sim);
        }
        let w = &sim.world;
        self.views.retain(|id, (_, v)| {
            let keep = w.looks.contains_key(id);
            if !keep && let View::Model { mesh, .. } = v {
                mesh.delete(&gfx.gl);
            }
            keep
        });
        for (id, look) in &w.looks {
            if self.views.get(id).is_some_and(|(rev, _)| *rev == look.rev) {
                continue;
            }
            let view = self.view(gfx, files, sim, *id, look);
            if let Some((_, View::Model { mesh, .. })) = self.views.insert(*id, (look.rev, view)) {
                mesh.delete(&gfx.gl);
            }
        }
        // Animated models on screen: blend this frame's vertex frames.
        let mut animated = 0;
        for (id, (_, v)) in self.views.iter_mut() {
            let visible = w.bodies.get(id).is_some_and(|b| !b.hidden && cam.is_none_or(|c| c.sees(sphere(b, v))));
            if let View::Model { asset, mesh, shown, .. } = v
                && let Some(blend) = w.animators.get(id).and_then(|a| a.anim.sample())
                && *shown != Some(blend)
                && visible
            {
                *shown = Some(blend);
                mesh.update(&gfx.gl, &asset.vertices(blend));
                animated += 1;
            }
        }
        self.stats.set(Stats { animated, ..Default::default() });
    }

    /// Where an entity is on screen: its bounds projected to window pixels
    /// (`[x0, y0, x1, y1]`, y down) and its distance, if it is in front of the camera.
    pub fn screen_rect(&self, sim: &Sim, id: Id, cam: &Camera, size: [f32; 2]) -> Option<([f32; 4], f32)> {
        let b = sim.world.bodies.get(&id)?;
        let (_, view) = self.views.get(&id)?;
        let (bounds, rot) = match view {
            View::Model { asset, .. } => {
                let f = asset.mdl.frames.first()?;
                ((Vec3::from(f.bbox_min), Vec3::from(f.bbox_max)), rotation(b.angles))
            }
            View::Prefab(p) => (p.bounds, rotation(b.angles)),
            View::Sprite { asset, facing } => (asset.bounds, sprite_rotation(b, *facing, cam)),
            View::Missing => return None,
        };
        let m = cam.view_proj * transform(b, rot);
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for i in 0..8 {
            let c = Vec3::new(if i & 1 == 0 { bounds.0.x } else { bounds.1.x }, if i & 2 == 0 { bounds.0.y } else { bounds.1.y }, if i & 4 == 0 { bounds.0.z } else { bounds.1.z });
            let p = m * c.extend(1.0);
            if p.w <= 0.0 {
                return None;
            }
            let (x, y) = ((p.x / p.w * 0.5 + 0.5) * size[0], (0.5 - p.y / p.w * 0.5) * size[1]);
            lo = [lo[0].min(x), lo[1].min(y)];
            hi = [hi[0].max(x), hi[1].max(y)];
        }
        Some(([lo[0], lo[1], hi[0], hi[1]], (Vec3::from(b.pos.get()) - cam.pos).length()))
    }

    pub(super) fn view(&mut self, gfx: &Gfx, files: &GameFiles, sim: &Sim, id: Id, look: &Look) -> View {
        match &look.visual {
            Visual::Model { file, skin, overlay, frame } => {
                let Some(asset) = self.model(gfx, files, file) else { return View::Missing };
                let first = frame.or_else(|| sim.world.animators.get(&id).and_then(|a| a.anim.sample()).map(|(a, _, _)| a)).unwrap_or(0);
                let mesh = Mesh::new(&gfx.gl, Layout::Model, &asset.vertices((first, first, 0.0)), &asset.vb.indices, frame.is_none());
                let tex = if *overlay { overlay_skin(gfx, &asset, *skin) } else { asset.skin(*skin) };
                View::Model { asset, mesh, tex, shown: frame.map(|f| (f, f, 0.0)) }
            }
            Visual::Prefab { file } => self.prefab(gfx, files, file).map_or(View::Missing, View::Prefab),
            Visual::Sprite { file, facing } => self.sprite(gfx, files, file).map_or(View::Missing, |asset| View::Sprite { asset, facing: *facing }),
        }
    }

    pub(super) fn build_level(&mut self, gfx: &Gfx, sim: &Sim) {
        if let Some(old) = self.level.take() {
            for (m, ..) in &old.batches {
                m.delete(&gfx.gl);
            }
            for (m, _) in &old.walls {
                m.delete(&gfx.gl);
            }
            for t in old.textures.into_iter().chain(old.lightmap) {
                gfx.delete_texture(t);
            }
        }
        let Some(wmb) = sim.level.clone() else { return };
        let mesh = wmb.build_mesh();
        let lightmap = mesh.lightmap.as_ref().map(|lm| gfx.lightmap(lm));
        let textures: Vec<Texture> = wmb.textures.iter().map(|t| gfx.texture(&t.image, true, true)).collect();
        let menu = sim.flow.file.eq_ignore_ascii_case(sammy_core::levels::MENU_FILE);
        let mut batches = Vec::new();
        for b in mesh.models.iter().flat_map(|m| &m.batches) {
            let name = b.texture.map_or("", |t| wmb.textures[t].name.as_str());
            if menu && MENU_SKIP.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                continue;
            }
            // Batches without lightmap UVs draw with flat light.
            let lit = b.lit && lightmap.is_some();
            let tex = b.texture.map_or(gfx.white, |t| textures[t]);
            for (indices, bounds) in chunks(&b.positions, &b.indices) {
                // Each chunk gets its own vertices, numbered from 0.
                let mut map = HashMap::new();
                let mut v: Vec<f32> = Vec::new();
                let local: Vec<u32> = indices
                    .iter()
                    .map(|&i| {
                        *map.entry(i).or_insert_with(|| {
                            let i = i as usize;
                            v.extend(b.positions[i].into_iter().chain(b.uvs[i]).chain(if lit { b.lightmap_uvs[i] } else { [0.0, 0.0] }));
                            (v.len() / Layout::World.floats() - 1) as u32
                        })
                    })
                    .collect();
                batches.push((Mesh::new(&gfx.gl, Layout::World, &v, &local, false), tex, lit, bounds));
            }
        }
        let walls = if menu {
            sammy_present::menu::grid_walls(&wmb, sim.world.menu_markers.values())
                .into_iter()
                .map(|q| {
                    let v: Vec<f32> = (0..4).flat_map(|i| q.corners[i].into_iter().chain([0.0, -1.0, 0.0]).chain(q.uvs[i])).collect();
                    (Mesh::new(&gfx.gl, Layout::Model, &v, &[0, 1, 2, 0, 2, 3], false), textures[q.texture])
                })
                .collect()
        } else {
            Vec::new()
        };
        self.level = Some(Level { batches, walls, lightmap, textures });
    }

}
