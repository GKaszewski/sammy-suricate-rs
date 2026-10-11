//! The level's world geometry: lightmapped texture batches of the loaded WMB, rebuilt
//! whenever the game loads a level.

use std::collections::HashMap;

use bevy::prelude::*;

use super::assets::batch_meshes;
use crate::game::{Game, GameEvent};
use crate::render::material::LevelMaterial;
use sammy_core::sim::Event;

/// Root of the level geometry.
#[derive(Component)]
pub struct LevelGeometry;

pub(super) fn rebuild(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    game: Res<Game>,
    old: Query<Entity, With<LevelGeometry>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut level_materials: ResMut<Assets<LevelMaterial>>,
) {
    let Some(file) = events.read().filter_map(|e| if let GameEvent(Event::Loaded { file }) = e { Some(file.clone()) } else { None }).last() else { return };
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(wmb) = game.level.clone() else { return };
    let root = commands.spawn((Name::new(file.clone()), Transform::default(), Visibility::default(), LevelGeometry)).id();
    let level = wmb.build_mesh();
    let lightmap = level
        .lightmap
        .as_ref()
        .filter(|_| std::env::var_os("SAMMY_NOLM").is_none())
        .map(|lm| images.add(crate::render::image::to_bevy(lm, false, false)));
    // Port: the menu's level-grid rooms are boxed in `black` walls that frame the 4:3
    // view; they are dropped so the extended back walls show on wide screens.
    let skip: &[&str] = if file.eq_ignore_ascii_case(sammy_core::levels::MENU_FILE) { &["black"] } else { &[] };
    let mut mats = HashMap::new();
    for (mesh, texture, lit) in batch_meshes(&wmb, &level, lightmap.is_some(), skip, &mut meshes, &mut images) {
        let material = mats
            .entry((texture.clone(), lit))
            .or_insert_with(|| level_materials.add(LevelMaterial::new(texture, lit.then(|| lightmap.clone()).flatten())))
            .clone();
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), ChildOf(root)));
    }
}
