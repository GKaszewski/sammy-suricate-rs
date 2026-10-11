//! Spawns a WMB level: lightmapped world geometry plus its entities
//! (MDL models, WMB prefabs, PCX sprites) loaded straight from the game files.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use formats::wmb::{entity_flags, Entity as WmbEntity, LevelMesh, Object, Wmb};

use crate::actions;
use crate::collision::Brush;
use crate::coords;
use crate::material::LevelMaterial;
use crate::model::{LoadedModel, ModelAnimator};
use crate::vfs::GameFiles;


/// Render layer for sky domes, drawn by a background camera before the world.
pub const SKY_LAYER: RenderLayers = RenderLayers::layer(1);

/// Render layer for `Near` entities (help signs), drawn last over a cleared depth buffer.
pub const NEAR_LAYER: RenderLayers = RenderLayers::layer(2);

/// Marker for every entity spawned from a WMB object record.
#[derive(Component)]
pub struct LevelObject(#[allow(dead_code)] pub Object);

/// A prefab WMB: its BSP (collision) and render parts.
pub type Prefab = (Arc<Wmb>, Vec<(Handle<Mesh>, Handle<StandardMaterial>)>);

/// Models, prefabs and sprites loaded from the game files, kept across levels so
/// gameplay systems can spawn and morph things at runtime.
#[derive(Resource, Default)]
pub struct GameAssets {
    pub models: HashMap<String, Option<Arc<LoadedModel>>>,
    pub prefabs: HashMap<String, Option<Prefab>>,
    sprites: HashMap<String, Option<(Handle<Mesh>, Handle<StandardMaterial>, u32)>>,
}

impl GameAssets {
    pub fn model(&self, file: &str) -> Option<Arc<LoadedModel>> {
        self.models.get(&file.to_lowercase()).cloned().flatten()
    }

    pub fn prefab(&self, file: &str) -> Option<Prefab> {
        self.prefabs.get(&file.to_lowercase()).cloned().flatten()
    }

    /// Sprite quad, material and frame count of a loaded PCX.
    pub fn sprite(&self, file: &str) -> Option<(Handle<Mesh>, Handle<StandardMaterial>, u32)> {
        self.sprites.get(&file.to_lowercase()).cloned().flatten()
    }
}

/// Files gameplay may need at runtime (morphs, projectiles, debris), loaded with every level.
const PRELOAD: &[&str] = &[
    "wasp1.mdl",
    "publogo.mdl",
    "surilogo.mdl",
    "plank1.mdl",
    "plank2.mdl",
    "plank3.mdl",
    "doorwk1.wmb",
    "doorwk2.wmb",
    "doorwk3.wmb",
    "doorwk4.wmb",
    "doordark.wmb",
    "platfor1.wmb",
    "circl100.mdl",
    "shadlio2.pcx",
    "ombblob2.pcx",
    // HUD and end-of-game screens (docs/hud-saves.md §1.6, §1.7, §3).
    "panwasp.mdl",
    "paneye.mdl",
    "lose.mdl",
    "win.mdl",
    "plus50+6.pcx",
    "plu100+2.pcx",
];

pub struct Spawner<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub files: &'a GameFiles,
    pub meshes: &'a mut Assets<Mesh>,
    pub images: &'a mut Assets<Image>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub level_materials: &'a mut Assets<LevelMaterial>,
    pub assets: &'a mut GameAssets,
}

impl Spawner<'_, '_, '_> {
    /// Spawns the level and returns the player start (Acknex position + pan), if found.
    pub fn spawn_level(&mut self, name: &str) -> anyhow::Result<Option<(Vec3, f32)>> {
        let wmb = Arc::new(formats::wmb::parse(&self.files.read(name)?)?);
        self.commands.insert_resource(actions::player::LevelCollision(wmb.clone()));
        let root = self.commands.spawn((Name::new(name.to_owned()), Transform::default(), Visibility::default(), crate::flow::LevelRoot)).id();
        let level = wmb.build_mesh();
        let lightmap = level
            .lightmap
            .as_ref()
            .filter(|_| std::env::var_os("SAMMY_NOLM").is_none())
            .map(|lm| self.images.add(crate::image::to_bevy(lm, false, false)));
        let mut mats = HashMap::new();
        for (mesh, texture, lit) in self.batch_meshes(&wmb, &level, lightmap.is_some()) {
            let material = mats
                .entry((texture.clone(), lit))
                .or_insert_with(|| self.level_materials.add(LevelMaterial::new(texture, lit.then(|| lightmap.clone()).flatten())))
                .clone();
            self.commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), ChildOf(root)));
        }

        let player = wmb.entities.iter().find_map(|e| match e {
            WmbEntity::Object(o) if o.action.eq_ignore_ascii_case("initmarche") => Some(o),
            _ => None,
        });
        let start = player.map(|o| (coords::pos(o.origin), o.angles[0]));
        let pov_z = player.map_or(0.0, |o| o.origin[2]);
        for file in PRELOAD.iter().chain(crate::effects::PARTICLE_MAPS) {
            if file.ends_with(".mdl") {
                self.load_model(file);
            } else if file.ends_with(".pcx") {
                if !self.assets.sprites.contains_key(*file) {
                    let s = self.load_sprite(file);
                    self.assets.sprites.insert(file.to_string(), s);
                }
            } else {
                self.load_prefab(file);
            }
        }
        let mut gems = actions::pickups::GemSprites::default();
        for color in [1, 2, 3, 4, 5, 6, crate::game::MASTER_KEY] {
            let file = actions::pickups::gem_file(color).unwrap();
            if let Some((mesh, mat, frames)) = self.load_sprite(file) {
                // Runtime keys animate, so each gets its own material copy later.
                gems.0.insert(color, (mesh, mat, frames));
            }
        }
        self.commands.insert_resource(gems);
        for ent in &wmb.entities {
            let WmbEntity::Object(obj) = ent else { continue };
            self.spawn_object(obj.clone(), pov_z, root);
        }
        Ok(start)
    }

    /// Builds one Bevy mesh per texture batch: (mesh, texture, has lightmap UVs).
    fn batch_meshes(&mut self, wmb: &Wmb, level: &LevelMesh, use_lightmap: bool) -> Vec<(Handle<Mesh>, Option<Handle<Image>>, bool)> {
        let textures: Vec<Handle<Image>> =
            wmb.textures.iter().map(|t| self.images.add(crate::image::to_bevy(&t.image, true, true))).collect();
        let mut out = Vec::new();
        for batch in level.models.iter().flat_map(|m| &m.batches) {
            let tex_name = batch.texture.map_or("", |t| wmb.textures[t].name.as_str());
            if std::env::var_os("SAMMY_NODUMMY").is_some() && tex_name.to_lowercase().contains("dummy") {
                continue;
            }
            let lit = batch.lit && use_lightmap;
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, batch.positions.iter().map(|&p| coords::pos(p)).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, batch.normals.iter().map(|&n| coords::dir(n)).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, batch.uvs.clone())
                .with_inserted_indices(Indices::U32(batch.indices.clone()));
            if lit {
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, batch.lightmap_uvs.clone());
            }
            out.push((self.meshes.add(mesh), batch.texture.map(|t| textures[t].clone()), lit));
        }
        out
    }

    /// Prefab WMBs (doors, platforms...) have no lightmaps; Acknex lights them
    /// dynamically like models, so they use a lit StandardMaterial.
    fn prefab_meshes(&mut self, wmb: &Wmb) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
        let level = wmb.build_mesh();
        let mut mats: HashMap<Option<AssetId<Image>>, Handle<StandardMaterial>> = HashMap::new();
        self.batch_meshes(wmb, &level, false)
            .into_iter()
            .map(|(mesh, tex, _)| {
                let material = mats
                    .entry(tex.as_ref().map(|t| t.id()))
                    .or_insert_with(|| {
                        self.materials.add(StandardMaterial {
                            base_color_texture: tex,
                            perceptual_roughness: 1.0,
                            reflectance: 0.0,
                            ..default()
                        })
                    })
                    .clone();
                (mesh, material)
            })
            .collect()
    }

    pub fn load_model(&mut self, file: &str) -> Option<Arc<LoadedModel>> {
        let file = file.to_lowercase();
        if let Some(m) = self.assets.models.get(&file) {
            return m.clone();
        }
        let m = self.files.read(&file).ok().and_then(|d| LoadedModel::load(&d, self.images, self.materials).map(Arc::new).ok());
        self.assets.models.insert(file, m.clone());
        m
    }

    pub fn load_prefab(&mut self, file: &str) -> Option<Prefab> {
        let file = file.to_lowercase();
        if let Some(p) = self.assets.prefabs.get(&file) {
            return p.clone();
        }
        let p = self.files.read(&file).ok().and_then(|d| formats::wmb::parse(&d).ok()).map(|w| {
            let w = Arc::new(w);
            let meshes = self.prefab_meshes(&w);
            (w, meshes)
        });
        self.assets.prefabs.insert(file, p.clone());
        p
    }

    fn spawn_object(&mut self, mut obj: Object, pov_z: f32, parent: bevy::ecs::entity::Entity) {
        if obj.flags & entity_flags::INVISIBLE != 0 || obj.file.is_empty() {
            return;
        }
        // Debug aid: SAMMY_SKIP=action1,action2 hides objects with those actions.
        if std::env::var("SAMMY_SKIP").is_ok_and(|v| v.split(',').any(|a| a.eq_ignore_ascii_case(&obj.action))) {
            return;
        }
        // A4 drops a map-entity prefab onto its origin: the bounding-box bottom sits at
        // the entity z (menu titles only line up with their diamonds this way) [U].
        if obj.file.to_lowercase().ends_with(".wmb")
            && let Some((wmb, _)) = self.load_prefab(&obj.file)
        {
            let min_z = wmb.vertices.iter().map(|v| v[2]).fold(f32::MAX, f32::min);
            if min_z < 0.0 {
                obj.origin[2] -= min_z * obj.scale[2].abs();
            }
        }
        let mut entity = self.commands.spawn_empty();
        let init = actions::init(&mut obj, pov_z, &mut entity);
        let id = entity.id();
        if init.hidden {
            self.commands.entity(id).despawn();
            return;
        }
        if init.hidden_but_alive {
            // Logic-only entity: keep it (with its components) but render nothing.
            self.commands.entity(id).insert((Name::new(obj.name.clone()), LevelObject(obj.clone()), ChildOf(parent)));
            return;
        }
        let tf = Transform {
            translation: coords::pos(obj.origin),
            rotation: coords::rotation(obj.angles),
            scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs(),
        };
        let file = obj.file.to_lowercase();
        let base = (Name::new(obj.name.clone()), tf, Visibility::default(), LevelObject(obj.clone()), ChildOf(parent));
        if init.sky {
            self.commands.entity(id).insert(SKY_LAYER);
        }
        if file.ends_with(".mdl") {
            let Some(model) = self.load_model(&file) else {
                self.commands.entity(id).despawn();
                return warn!("missing model {file}");
            };
            let skin_n = init.skin.unwrap_or(1);
            let skin = if init.overlay { model.overlay_skin(skin_n, self.images, self.materials) } else { model.skin(skin_n) };
            let material = if init.sky { self.unlit(&skin) } else { skin };
            // Scenery without an action (or `InitMdl`) keeps a fixed frame; A4 frames are 1-based.
            let frame = init.frame.or(obj.action.is_empty().then_some(1.0));
            if let Some(f) = frame {
                let index = (f.floor().max(1.0) as usize - 1).min(model.mdl.frames.len().saturating_sub(1));
                let mesh = self.meshes.add(model.build_mesh(index));
                self.commands.entity(id).insert((base, Mesh3d(mesh), MeshMaterial3d(material)));
                if obj.flags & formats::wmb::entity_flags::SHADOW != 0 {
                    self.commands.entity(id).insert(crate::shadow::CastsShadow);
                }
                if init.solid_box && let Some(fr) = model.mdl.frames.get(index) {
                    let sc = obj.scale.map(f32::abs);
                    let center = std::array::from_fn(|i| obj.origin[i] + (fr.bbox_min[i] + fr.bbox_max[i]) * 0.5 * sc[i]);
                    let half = std::array::from_fn(|i| (fr.bbox_max[i] - fr.bbox_min[i]) * 0.5 * sc[i]);
                    self.commands.entity(id).insert(crate::collision::BoxCollider { center, half, solid: true });
                }
                return;
            }
            let (scene, pct) = init.anim.unwrap_or(("stand", 2.0));
            let mut animator = ModelAnimator::new(model.clone(), Vec::new(), 8.0);
            animator.play(scene, true, 0.0);
            animator.set_speed_pct(pct, actions::player::TICKS);
            let mesh = self.meshes.add(model.build_mesh(animator.frames.first().copied().unwrap_or(0)));
            self.commands.entity(id).insert((base, Mesh3d(mesh), MeshMaterial3d(material), animator));
        } else if file.ends_with(".wmb") {
            let Some((wmb, parts)) = self.load_prefab(&file) else {
                self.commands.entity(id).despawn();
                return warn!("missing prefab {file}");
            };
            // Map entities are solid unless passable (A4 collides with their BSP hulls).
            let solid = obj.flags & entity_flags::PASSABLE == 0 && !actions::is_passable(&obj.action);
            let brush = Brush { wmb, origin: obj.origin, pan: obj.angles[0], solid };
            self.commands.entity(id).insert((base, brush)).with_children(|c| {
                for (mesh, mat) in parts {
                    c.spawn((Mesh3d(mesh), MeshMaterial3d(mat)));
                }
            });
        } else if file.ends_with(".pcx") {
            let sprite = match self.assets.sprites.get(&file) {
                Some(s) => s.clone(),
                None => {
                    let s = self.load_sprite(&file);
                    self.assets.sprites.insert(file.clone(), s.clone());
                    s
                }
            };
            let Some((mesh, mat, frames)) = sprite else {
                self.commands.entity(id).despawn();
                return warn!("missing sprite {file}");
            };
            // Animated strips and fading signs get their own material.
            let (mat, anim) = if init.title {
                let mut m = self.materials.get(&mat).cloned().unwrap_or_default();
                m.alpha_mode = AlphaMode::Blend;
                (self.materials.add(m), None)
            } else if init.alpha.is_some_and(|a| a < 100.0) {
                let mut m = self.materials.get(&mat).cloned().unwrap_or_default();
                m.alpha_mode = AlphaMode::Blend;
                m.base_color.set_alpha(init.alpha.unwrap_or(100.0) / 100.0);
                (self.materials.add(m), None)
            } else if frames > 1 {
                let m = self.materials.get(&mat).cloned().unwrap_or_default();
                // Deterministic stand-in for WDL `random(frames)` start frame.
                let start = (id.index_u32().wrapping_mul(2654435761) >> 7) % frames;
                let fps = if obj.action.eq_ignore_ascii_case("initbonus3") { 48.0 } else { 0.0 };
                (self.materials.add(m), Some(SpriteAnim { frames, frame: start as f32, fps }))
            } else {
                (mat, None)
            };
            let mut e = self.commands.entity(id);
            if init.title {
                // `Visibility::Hidden` from init_title must survive `base`'s default visibility.
                e.insert((base, Mesh3d(mesh.clone()), MeshMaterial3d(mat), ScreenAligned, NEAR_LAYER));
                e.insert(Visibility::Hidden);
            } else if init.facing {
                e.insert((base, Mesh3d(mesh.clone()), MeshMaterial3d(mat), ScreenAligned));
            } else if init.oriented {
                e.insert((base, Mesh3d(mesh.clone()), MeshMaterial3d(mat)));
            } else {
                e.insert((base, Mesh3d(mesh.clone()), MeshMaterial3d(mat), Billboard));
            }
            if let Some(dz) = init.ground
                && let Some(top) = self.meshes.get(&mesh).and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?.iter().map(|p| p[1]).reduce(f32::max))
            {
                let half_h = top * coords::QUANTS_PER_UNIT * obj.scale[2].abs();
                e.insert(actions::decor::SpriteGround { origin: obj.origin, dz: half_h + dz });
            }
            if let Some(anim) = anim {
                e.insert(anim);
            }
        } else {
            self.commands.entity(id).despawn();
        }
    }

    fn unlit(&mut self, base: &Handle<StandardMaterial>) -> Handle<StandardMaterial> {
        let mut m = self.materials.get(base).cloned().unwrap_or_default();
        m.unlit = true;
        m.cull_mode = None;
        self.materials.add(m)
    }

    /// Sprites are upright quads sized in quants = pixels. `name+N.pcx` is an
    /// N-frame horizontal strip; only the first frame is shown for now.
    fn load_sprite(&mut self, file: &str) -> Option<(Handle<Mesh>, Handle<StandardMaterial>, u32)> {
        let pcx = formats::pcx::parse(&self.files.read(file).ok()?).ok()?;
        let frames: u32 = file
            .rsplit_once('+')
            .and_then(|(_, n)| n.trim_end_matches(".pcx").parse().ok())
            .unwrap_or(1)
            .max(1);
        let mut img = pcx.image.clone();
        // Acknex sprites are colour-keyed: palette index 0 (8-bit) or pure black (24-bit).
        if let Some(idx) = &pcx.indices {
            for (px, &i) in img.data.chunks_exact_mut(4).zip(idx) {
                if i == 0 {
                    px[3] = 0;
                }
            }
        } else {
            for px in img.data.chunks_exact_mut(4) {
                if crate::image::is_key(px) {
                    px[3] = 0;
                }
            }
        }
        let w = img.width as f32 / frames as f32 / coords::QUANTS_PER_UNIT;
        let h = img.height as f32 / coords::QUANTS_PER_UNIT;
        let u1 = 1.0 / frames as f32;
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[-w / 2.0, -h / 2.0, 0.0], [w / 2.0, -h / 2.0, 0.0], [w / 2.0, h / 2.0, 0.0], [-w / 2.0, h / 2.0, 0.0]],
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 1.0], [u1, 1.0], [u1, 0.0], [0.0, 0.0]])
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let material = self.materials.add(StandardMaterial {
            base_color_texture: Some(self.images.add(crate::image::to_bevy(&img, true, false))),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        Some((self.meshes.add(mesh), material, frames))
    }
}

/// Plays a horizontal sprite strip (`name+N.pcx`) by shifting the material's UVs.
#[derive(Component)]
pub struct SpriteAnim {
    pub frames: u32,
    pub frame: f32,
    /// Frames per second (WDL: frames per tick x 16).
    pub fps: f32,
}

pub fn animate_sprites(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&mut SpriteAnim, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (mut anim, mat) in &mut q {
        anim.frame = (anim.frame + anim.fps * time.delta_secs()).rem_euclid(anim.frames as f32);
        if let Some(mut m) = materials.get_mut(&mat.0) {
            let offset = anim.frame.floor() / anim.frames as f32;
            if m.uv_transform.translation.x != offset {
                m.uv_transform = bevy::math::Affine2::from_translation(Vec2::new(offset, 0.0));
            }
        }
    }
}

/// Camera-facing sprite (rotates around the vertical axis only, like Acknex sprites).
#[derive(Component)]
pub struct Billboard;

/// Sprite parallel to the screen (WDL `Facing` + pan/tilt copied from the camera).
#[derive(Component)]
pub struct ScreenAligned;

pub fn face_camera(
    cam: Query<&Transform, (With<crate::camera::MainCamera>, Without<Billboard>, Without<ScreenAligned>)>,
    mut q: ParamSet<(Query<&mut Transform, With<Billboard>>, Query<&mut Transform, With<ScreenAligned>>)>,
) {
    let Ok(cam) = cam.single() else { return };
    for mut tf in &mut q.p0() {
        let d = cam.translation - tf.translation;
        tf.rotation = Quat::from_rotation_y(d.x.atan2(d.z));
    }
    for mut tf in &mut q.p1() {
        tf.rotation = cam.rotation;
    }
}
