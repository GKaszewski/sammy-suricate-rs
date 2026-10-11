//! A4 particles (`EMIT n, pos, function`, docs/particles.md): a CPU list of point
//! sprites, moved by their speed every tick and updated by their particle function.
//! All live particles that share a bitmap and blend mode are drawn as one camera-facing
//! quad batch, rebuilt every frame.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::camera::MainCamera;
use crate::coords;
use crate::level::GameAssets;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Particles>()
            .add_systems(PreUpdate, particle_clock)
            .add_systems(Update, (crate::effects::emit_trails, crate::effects::emit_sammy).run_if(|c: Res<crate::flow::CurrentLevel>| c.phase != crate::flow::Phase::Intro))
            .add_systems(PostUpdate, (update_particles, draw_particles).chain().before(TransformSystems::Propagate));
    }
}

/// Particle function, called once per tick after the move; returns false to remove it.
/// Particles it emits (nested `EMIT`) go to `spawn`.
pub type Behaviour = fn(&mut Particle, &mut ParticleCtx) -> bool;

pub struct ParticleCtx<'a> {
    pub rng: &'a mut ParticleRng,
    pub spawn: &'a mut Vec<Particle>,
}

/// A4 `MY_SIZE` to quad width in quants (docs/particles.md §1.5: size/100, uncalibrated).
pub const SIZE_SCALE: f32 = 0.01;

#[derive(Clone, Debug)]
pub struct Particle {
    /// Native position (quants).
    pub pos: [f32; 3],
    /// Quants per tick.
    pub vel: [f32; 3],
    /// A4 `MY_SIZE` (quad width = size × SIZE_SCALE quants).
    pub size: f32,
    /// RGB 0..255.
    pub color: [f32; 3],
    /// 0..100.
    pub alpha: f32,
    /// Bitmap (`my_map`); None = plain round dot in `color`.
    pub map: Option<&'static str>,
    /// `my_flare`: additive blending.
    pub flare: bool,
    /// Ticks since emission (`my_age`).
    pub age: f32,
    pub behaviour: Behaviour,
    /// Free per-particle state for behaviours.
    pub data: [f32; 4],
    /// Ticks before it appears (delayed bursts, `waitt n; EMIT`).
    pub delay: f32,
}

impl Particle {
    pub fn new(pos: [f32; 3], behaviour: Behaviour) -> Self {
        Self { pos, vel: [0.0; 3], size: 100.0, color: [255.0; 3], alpha: 100.0, map: None, flare: false, age: 0.0, behaviour, data: [0.0; 4], delay: 0.0 }
    }
}

/// Small xorshift for particle randomness (keeps the game `Rng` sequence untouched).
pub struct ParticleRng(u32);

impl ParticleRng {
    /// WDL `random(n)`: uniform in [0, n).
    pub fn random(&mut self, n: f32) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32 * n
    }
}

#[derive(Resource)]
pub struct Particles {
    pub list: Vec<Particle>,
    pub rng: ParticleRng,
    /// Fractional tick accumulator.
    acc: f32,
    /// Whole 16 Hz ticks elapsing this frame (emitters run their per-tick code this often).
    pub ticks: u32,
}

impl Default for Particles {
    fn default() -> Self {
        Self { list: Vec::new(), rng: ParticleRng(0x1234_5678), acc: 0.0, ticks: 0 }
    }
}

/// Hard cap so a runaway emitter can't stall the frame.
const MAX_PARTICLES: usize = 4000;

impl Particles {
    /// One 16 Hz tick: move, age and run every particle function.
    pub fn step(&mut self) {
        let mut spawn = Vec::new();
        let rng = &mut self.rng;
        self.list.retain_mut(|p| {
            if p.delay > 0.0 {
                p.delay -= 1.0;
                return true;
            }
            for i in 0..3 {
                p.pos[i] += p.vel[i];
            }
            p.age += 1.0;
            let mut ctx = ParticleCtx { rng, spawn: &mut spawn };
            (p.behaviour)(p, &mut ctx) && p.size > 0.0 && p.alpha > 0.0
        });
        let room = MAX_PARTICLES.saturating_sub(self.list.len());
        self.list.extend(spawn.into_iter().take(room));
    }

    /// `EMIT count, pos, init`: `init` sets up each new particle (the creation call of the
    /// particle function).
    pub fn emit(&mut self, count: usize, pos: [f32; 3], init: impl Fn(&mut Particle, &mut ParticleRng)) {
        self.emit_delayed(count, pos, 0.0, init);
    }

    /// `EMIT` after `delay` ticks.
    pub fn emit_delayed(&mut self, count: usize, pos: [f32; 3], delay: f32, init: impl Fn(&mut Particle, &mut ParticleRng)) {
        for _ in 0..count {
            if self.list.len() >= MAX_PARTICLES {
                return;
            }
            let mut p = Particle::new(pos, |_, _| true);
            init(&mut p, &mut self.rng);
            p.delay = delay;
            self.list.push(p);
        }
    }

    pub fn random(&mut self, n: f32) -> f32 {
        self.rng.random(n)
    }
}

fn particle_clock(time: Res<Time>, mut ps: ResMut<Particles>) {
    ps.acc += time.delta_secs() * crate::actions::player::TICKS;
    ps.ticks = (ps.acc.floor() as u32).min(4);
    ps.acc = (ps.acc - ps.ticks as f32).min(1.0);
}

fn update_particles(mut particles: ResMut<Particles>) {
    for _ in 0..particles.ticks {
        particles.step();
    }
}

/// One draw batch per (bitmap, additive).
#[derive(Component)]
struct ParticleBatch;

#[derive(Default)]
struct Batches(HashMap<(Option<&'static str>, bool), (Entity, Handle<Mesh>)>);

#[allow(clippy::too_many_arguments)]
fn draw_particles(
    mut commands: Commands,
    particles: Res<Particles>,
    assets: Option<Res<GameAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
    mut batches: Local<Batches>,
    mut dot: Local<Option<Handle<Image>>>,
    alive: Query<(), With<ParticleBatch>>,
) {
    let Ok(cam) = cam.single() else { return };
    let (right, up) = (cam.right().as_vec3(), cam.up().as_vec3());
    // Batches despawned with the level are recreated on demand.
    batches.0.retain(|_, (e, _)| alive.contains(*e));

    let mut groups: HashMap<(Option<&'static str>, bool), Vec<&Particle>> = HashMap::new();
    for p in particles.list.iter().filter(|p| p.delay <= 0.0) {
        groups.entry((p.map, p.flare)).or_default().push(p);
    }
    // Empty the batches that have no particles this frame.
    for (key, (_, mesh)) in &batches.0 {
        if !groups.contains_key(key)
            && let Some(mut m) = meshes.get_mut(mesh)
        {
            m.insert_indices(Indices::U32(Vec::new()));
        }
    }
    for (key, list) in groups {
        let mut pos = Vec::with_capacity(list.len() * 4);
        let mut col = Vec::with_capacity(list.len() * 4);
        let mut uv = Vec::with_capacity(list.len() * 4);
        let mut idx = Vec::with_capacity(list.len() * 6);
        for p in list {
            let c = coords::pos(p.pos);
            let h = p.size * SIZE_SCALE * 0.5 / coords::QUANTS_PER_UNIT;
            let (r, u) = (right * h, up * h);
            let base = pos.len() as u32;
            for (corner, t) in [(-r - u, [0.0, 1.0]), (r - u, [1.0, 1.0]), (r + u, [1.0, 0.0]), (-r + u, [0.0, 0.0])] {
                pos.push((c + corner).to_array());
                uv.push(t);
                let a = (p.alpha / 100.0).clamp(0.0, 1.0);
                // Additive batches fade by darkening, blended ones by alpha.
                let k = if key.1 { a } else { 1.0 };
                col.push([p.color[0] / 255.0 * k, p.color[1] / 255.0 * k, p.color[2] / 255.0 * k, a]);
            }
            idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; col.len()])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_indices(Indices::U32(idx));
        if let Some((_, handle)) = batches.0.get(&key) {
            if let Some(mut m) = meshes.get_mut(handle) {
                *m = mesh;
            }
            continue;
        }
        let texture = match key.0 {
            Some(file) => assets.as_ref().and_then(|a| a.sprite(file)).and_then(|(_, m, _)| materials.get(&m).and_then(|m| m.base_color_texture.clone())),
            None => Some(dot.get_or_insert_with(|| images.add(dot_image())).clone()),
        };
        let material = materials.add(StandardMaterial {
            base_color_texture: texture,
            unlit: true,
            alpha_mode: if key.1 { AlphaMode::Add } else { AlphaMode::Blend },
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        let handle = meshes.add(mesh);
        let e = commands
            .spawn((ParticleBatch, Mesh3d(handle.clone()), MeshMaterial3d(material), NotShadowCaster, Transform::default(), bevy::camera::visibility::NoFrustumCulling, crate::flow::LevelScoped))
            .id();
        batches.0.insert(key, (e, handle));
    }
}

/// Soft round dot for bitmap-less particles.
fn dot_image() -> Image {
    const N: u32 = 16;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let d = ((x as f32 + 0.5 - N as f32 / 2.0).hypot(y as f32 + 0.5 - N as f32 / 2.0) / (N as f32 / 2.0)).min(1.0);
            let a = ((1.0 - d) * 2.0).min(1.0);
            data.extend([255, 255, 255, (a * 255.0) as u8]);
        }
    }
    crate::image::to_bevy(&formats::image::RgbaImage { width: N, height: N, data }, true, false)
}
