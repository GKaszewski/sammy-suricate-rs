//! Models, prefabs and sprites loaded for drawing.

use super::*;

/// A parsed MDL ready to draw: its vertex frames and skins.
pub struct ModelAsset {
    pub mdl: formats::mdl::Mdl,
    pub vb: formats::mdl::VertexBuffer,
    pub(super) skins: Vec<Texture>,
    /// Skins with the `overlay` colour key, built on demand.
    pub(super) overlay: std::cell::RefCell<HashMap<u32, Texture>>,
}

impl ModelAsset {
    /// Interleaved pos, normal, uv of frames `a`→`b` blended by `f`.
    pub fn vertices(&self, (a, b, f): (usize, usize, f32)) -> Vec<f32> {
        let last = self.mdl.frames.len() - 1;
        let (fa, fb) = (&self.mdl.frames[a.min(last)], &self.mdl.frames[b.min(last)]);
        let mut out = Vec::with_capacity(self.vb.xyz_index.len() * 8);
        for (i, &xi) in self.vb.xyz_index.iter().enumerate() {
            let xi = xi as usize;
            let lerp = |p: [f32; 3], q: [f32; 3]| [0, 1, 2].map(|k| p[k] + (q[k] - p[k]) * f);
            out.extend(lerp(fa.positions[xi], fb.positions[xi]));
            out.extend(lerp(fa.normals[xi], fb.normals[xi]));
            out.extend(self.vb.uvs[i]);
        }
        out
    }

    pub fn skin(&self, n: u32) -> Texture {
        self.skins[(n.max(1) as usize - 1).min(self.skins.len() - 1)]
    }
}

/// A sprite: its quad (one frame of the strip), texture and frame count.
pub struct SpriteAsset {
    pub mesh: Mesh,
    pub tex: Texture,
    pub frames: u32,
    pub(super) bounds: Bounds,
}

/// An upright `2w × 2h` quad facing -X, showing `u1` of its texture's width.
pub(super) fn quad(gfx: &Gfx, w: f32, h: f32, u1: f32) -> Mesh {
    #[rustfmt::skip]
    let v = [
        0.0, w, -h,  -1.0, 0.0, 0.0,  0.0, 1.0,
        0.0, -w, -h, -1.0, 0.0, 0.0,  u1, 1.0,
        0.0, -w, h,  -1.0, 0.0, 0.0,  u1, 0.0,
        0.0, w, h,   -1.0, 0.0, 0.0,  0.0, 0.0,
    ];
    Mesh::new(&gfx.gl, Layout::Model, &v, &[0, 1, 2, 0, 2, 3], false)
}

/// A skin with A4 `overlay`: palette index 0 (or black) is transparent.
pub(super) fn overlay_skin(gfx: &Gfx, asset: &ModelAsset, n: u32) -> Texture {
    if let Some(t) = asset.overlay.borrow().get(&n) {
        return *t;
    }
    let i = (n.max(1) as usize - 1).min(asset.mdl.skins.len().saturating_sub(1));
    let Some(skin) = asset.mdl.skins.get(i) else { return asset.skin(n) };
    let mut img = skin.image.clone();
    match &skin.indices {
        Some(idx) => img.data.as_chunks_mut::<4>().0.iter_mut().zip(idx).filter(|(_, i)| **i == 0).for_each(|(px, _)| px[3] = 0),
        None => img.data.as_chunks_mut::<4>().0.iter_mut().filter(|px| formats::image::is_color_key(&px[..])).for_each(|px| px[3] = 0),
    }
    let t = gfx.texture(&img, false, true);
    asset.overlay.borrow_mut().insert(n, t);
    t
}

/// Sprites are upright quads sized in quants = pixels, facing -X; `name+N.pcx` strips
/// show one frame at a time.
pub(super) fn load_sprite(gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<SpriteAsset>> {
    let pcx = formats::pcx::parse(&files.read(file).ok()?).ok()?;
    let frames = sammy_core::sim::assets::sprite_frames(file);
    let mut img = pcx.image.clone();
    // Colour key: palette index 0 (8-bit) or near-black (24-bit).
    match &pcx.indices {
        Some(idx) => img.data.as_chunks_mut::<4>().0.iter_mut().zip(idx).filter(|(_, i)| **i == 0).for_each(|(px, _)| px[3] = 0),
        None => img.data.as_chunks_mut::<4>().0.iter_mut().filter(|px| formats::image::is_color_key(&px[..])).for_each(|px| px[3] = 0),
    }
    let (w, h) = (img.width as f32 / frames as f32 / 2.0, img.height as f32 / 2.0);
    let mesh = quad(gfx, w, h, 1.0 / frames as f32);
    Some(Rc::new(SpriteAsset { mesh, tex: gfx.texture(&img, false, true), frames, bounds: (Vec3::new(-1.0, -w, -h), Vec3::new(1.0, w, h)) }))
}

impl Scene {
    pub fn model(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<ModelAsset>> {
        let key = file.to_lowercase();
        self.models
            .entry(key.clone())
            .or_insert_with(|| {
                let mdl = formats::mdl::parse(&files.read(&key).ok()?).ok()?;
                let vb = mdl.vertex_buffer();
                let mut skins: Vec<Texture> = mdl.skins.iter().map(|s| gfx.texture(&s.image, false, true)).collect();
                if skins.is_empty() {
                    skins.push(gfx.white);
                }
                Some(Rc::new(ModelAsset { mdl, vb, skins, overlay: Default::default() }))
            })
            .clone()
    }

    pub(super) fn prefab(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Parts> {
        let key = file.to_lowercase();
        self.prefabs
            .entry(key.clone())
            .or_insert_with(|| {
                let wmb = formats::wmb::parse(&files.read(&key).ok()?).ok()?;
                let level = wmb.build_mesh();
                let textures: Vec<Texture> = wmb.textures.iter().map(|t| gfx.texture(&t.image, true, true)).collect();
                let batches = level.models.iter().flat_map(|m| &m.batches);
                let points = batches.clone().flat_map(|b| b.positions.iter().map(|p| Vec3::from(*p)));
                let bounds = points.fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)));
                let parts = batches
                    .map(|b| {
                        let v: Vec<f32> = (0..b.positions.len()).flat_map(|i| b.positions[i].into_iter().chain(b.normals[i]).chain(b.uvs[i])).collect();
                        (Mesh::new(&gfx.gl, Layout::Model, &v, &b.indices, false), b.texture.map_or(gfx.white, |t| textures[t]))
                    })
                    .collect();
                Some(Rc::new(Prefab { parts, bounds }))
            })
            .clone()
    }

    pub fn sprite(&mut self, gfx: &Gfx, files: &GameFiles, file: &str) -> Option<Rc<SpriteAsset>> {
        let key = file.to_lowercase();
        self.sprites.entry(key.clone()).or_insert_with(|| load_sprite(gfx, files, &key)).clone()
    }

}
