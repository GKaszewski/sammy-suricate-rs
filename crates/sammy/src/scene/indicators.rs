//! Visuals drawn from the enemies' state: ground shadows (`InitOmbreLion`), the lion
//! range rings shown by the Eye of Truth (`DisplayEnnemyRange`, lion.wdl:1586-1653),
//! and the expanding rings of a friend call (`Circle_CallFriends`, lion.wdl:1015-1070).

use std::collections::HashMap;

use bevy::prelude::*;
use sammy_core::id::Id;
use sammy_core::sim::Event;

use super::assets::Loader;
use crate::game::{Game, GameEvent};
use crate::render::coords::{self, QUANTS_PER_UNIT};
use crate::render::model::ModelAnimator;

/// Lion shadow: `shadlio2.pcx` (48 quants square) at 1.2 × the lion's scale, 0.5 above
/// the floor, turned with the lion's pan + 90.
const SHADOW_FILE: &str = "shadlio2.pcx";
const SHADOW_SCALE: f32 = 1.2;
const SHADOW_SIZE: [f32; 2] = [48.0, 48.0];
const SHADOW_LIFT: f32 = 0.5;
const SHADOW_PAN_OFFSET: f32 = 90.0;
/// Shadows are half transparent (`transparent`).
const SHADOW_ALPHA: f32 = 0.5;
/// Range rings: `circl100.mdl` skins 1..=`RING_SKINS`, 1000 quants across at scale 1.
const RING_FILE: &str = "circl100.mdl";
const RING_SKINS: u32 = 3;
const RING_DIAMETER: f32 = 1000.0;
/// `Circle_CallFriends`: scale grows `CALL_GROW` per tick until `CALL_EASE_FROM`, then
/// eases (`CALL_EASE_RATE`) toward `CALL_EASE_TARGET`; the ring ends at scale
/// `CALL_END_SCALE` or frame `CALL_END_FRAME`; frames advance `CALL_FRAME_RATE` per tick.
const CALL_START_SCALE: f32 = 0.01;
const CALL_GROW: f32 = 0.2;
const CALL_EASE_FROM: f32 = 3.0;
const CALL_EASE_TARGET: f32 = 5.1;
const CALL_EASE_RATE: f32 = 0.5;
const CALL_FRAME_RATE: f32 = 4.0;
const CALL_END_FRAME: f32 = 100.0;
const CALL_END_SCALE: f32 = 5.0;

/// A ground shadow or range ring of an enemy.
#[derive(Component)]
pub struct EnemyMark;

/// A friend-call ring: delay (ticks), scale, frame, position, skin.
#[derive(Component)]
pub struct CallRing {
    delay: f32,
    scale: f32,
    frame: f32,
    origin: [f32; 3],
    skin: u32,
}

/// Additive, unlit ring materials per skin (1..=3).
fn ring_materials(loader: &mut Loader, cache: &mut Option<Vec<Handle<StandardMaterial>>>) -> Option<Vec<Handle<StandardMaterial>>> {
    if cache.is_none() {
        let model = loader.model(RING_FILE)?;
        *cache = Some(
            (1..=RING_SKINS)
                .map(|n| {
                    let mut m = loader.materials.get(&model.skin(n)).cloned().unwrap_or_default();
                    m.alpha_mode = AlphaMode::Add; // `flare`
                    m.unlit = true;
                    m.double_sided = true;
                    m.cull_mode = None;
                    loader.materials.add(m)
                })
                .collect(),
        );
    }
    cache.clone()
}

fn ring_bundle(loader: &mut Loader) -> Option<(Mesh3d, ModelAnimator)> {
    let model = loader.model(RING_FILE)?;
    let mut anim = ModelAnimator::new(model.clone(), (0..model.mdl.frames.len()).collect(), 0.0);
    anim.looping = false;
    Some((Mesh3d(loader.meshes.add(model.build_mesh(0))), anim))
}

/// Flat shadow decals under the enemies.
pub(super) fn ground_shadows(mut commands: Commands, game: Res<Game>, mut loader: Loader, mut shadows: Local<HashMap<Id, Entity>>, mut q: Query<(&mut Transform, &mut Visibility), With<EnemyMark>>) {
    let w = &game.world;
    shadows.retain(|id, e| {
        let keep = w.enemies.contains_key(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    for (id, enemy) in &w.enemies {
        let Some(&e) = shadows.get(id) else {
            let Some(sprite) = loader.sprite(SHADOW_FILE) else { return };
            let mut m = loader.materials.get(&sprite.material).cloned().unwrap_or_default();
            m.alpha_mode = AlphaMode::Blend;
            m.base_color = Color::srgba(1.0, 1.0, 1.0, SHADOW_ALPHA);
            m.depth_bias = 1.0;
            let k = SHADOW_SCALE * enemy.scale() * 0.5 / QUANTS_PER_UNIT;
            let mesh = loader.meshes.add(Plane3d::new(Vec3::Y, Vec2::new(SHADOW_SIZE[0] * k, SHADOW_SIZE[1] * k)));
            let e = commands.spawn((EnemyMark, Mesh3d(mesh), MeshMaterial3d(loader.materials.add(m)), Transform::default(), Visibility::Hidden)).id();
            shadows.insert(*id, e);
            continue;
        };
        let Ok((mut tf, mut vis)) = q.get_mut(e) else { continue };
        let x = w.bodies.get(id).map_or(enemy.pos, |b| b.pos);
        tf.translation = coords::pos([x[0], x[1], enemy.floor() + SHADOW_LIFT]);
        tf.rotation = Quat::from_rotation_y((enemy.pan + SHADOW_PAN_OFFSET).to_radians());
        *vis = if enemy.gone() { Visibility::Hidden } else { Visibility::Inherited };
    }
}

/// Lion range rings while the Eye of Truth is active.
pub(super) fn range_rings(
    mut commands: Commands,
    game: Res<Game>,
    mut loader: Loader,
    mut cache: Local<Option<Vec<Handle<StandardMaterial>>>>,
    mut rings: Local<HashMap<Id, Entity>>,
    mut q: Query<(&mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>, &mut ModelAnimator)>,
) {
    let w = &game.world;
    rings.retain(|id, e| {
        let keep = w.enemies.contains_key(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    let Some(mats) = ring_materials(&mut loader, &mut cache) else { return };
    for (id, enemy) in &w.enemies {
        let Some(&e) = rings.get(id) else {
            let Some((mesh, anim)) = ring_bundle(&mut loader) else { return };
            let e = commands.spawn((mesh, anim, MeshMaterial3d(mats[0].clone()), Transform::default(), Visibility::Hidden)).id();
            rings.insert(*id, e);
            continue;
        };
        let Ok((mut tf, mut vis, mut mat, mut anim)) = q.get_mut(e) else { continue };
        let shown = (game.game.eye_ticks > 0.0 && !enemy.gone()).then(|| enemy.ring(game.game.display_range)).flatten();
        let Some((skin, frame, radius)) = shown else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Inherited;
        mat.0 = mats[(skin.clamp(1, RING_SKINS) - 1) as usize].clone();
        anim.time = (frame - 1.0).max(0.0); // A4 frame numbers are 1-based
        let s = radius / RING_DIAMETER;
        tf.translation = coords::pos([enemy.pos[0], enemy.pos[1], enemy.pos[2] + enemy.circle_z]);
        tf.scale = Vec3::new(s, 1.0, s);
    }
}

/// Friend calls: three rings each grow +0.2/tick to scale 3, then ease to 5; frame
/// +4/tick to 100, then gone.
pub(super) fn call_rings(
    time: Res<Time>,
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    mut loader: Loader,
    mut cache: Local<Option<Vec<Handle<StandardMaterial>>>>,
    mut rings: Query<(Entity, &mut CallRing, &mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>, &mut ModelAnimator)>,
) {
    let Some(mats) = ring_materials(&mut loader, &mut cache) else { return };
    for ev in events.read() {
        let GameEvent(Event::CallRings { at }) = ev else { continue };
        for (dz, delay, skin) in sammy_core::enemies::CALL_RINGS {
            let Some((mesh, anim)) = ring_bundle(&mut loader) else { return };
            commands.spawn((
                CallRing { delay, scale: CALL_START_SCALE, frame: 1.0, origin: [at[0], at[1], at[2] + dz], skin },
                mesh,
                anim,
                MeshMaterial3d(mats[0].clone()),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
    let t = time.delta_secs() * sammy_core::time::TICKS_PER_SEC;
    for (e, mut r, mut tf, mut vis, mut mat, mut anim) in &mut rings {
        if r.delay > 0.0 {
            r.delay -= t;
            continue;
        }
        *vis = Visibility::Inherited;
        mat.0 = mats[(r.skin.clamp(1, RING_SKINS) - 1) as usize].clone();
        if r.scale < CALL_EASE_FROM {
            r.scale += CALL_GROW * t;
        } else {
            r.scale += (CALL_EASE_TARGET - r.scale) * CALL_EASE_RATE * t;
        }
        r.frame += CALL_FRAME_RATE * t;
        anim.time = (r.frame - 1.0).clamp(0.0, (anim.frames.len() - 1) as f32);
        tf.translation = coords::pos(r.origin);
        tf.scale = Vec3::new(r.scale, 1.0, r.scale);
        if r.frame >= CALL_END_FRAME || r.scale >= CALL_END_SCALE {
            commands.entity(e).despawn();
        }
    }
}
