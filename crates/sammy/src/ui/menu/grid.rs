//! Wide screens see past the level grids' walls: the walls are extended.

use super::*;

/// Slab test of a world ray against a transformed box; returns the distance.
/// Port: the level-grid rooms are built only about 4:3 wide, so wide screens saw the
/// void beside them. Each room's back wall (the farthest face looking back at its camera
/// marker) is continued sideways and vertically by a quad just behind it, with the same
/// texture and texture alignment.
pub(crate) fn extend_grid_walls(
    mut commands: Commands,
    game: Res<Game>,
    mut events: MessageReader<GameEvent>,
    old: Query<Entity, With<GridWall>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !events.read().any(|e| matches!(e.0, Event::Loaded { .. })) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(level) = game.level.clone() else { return };
    let w = &*level;
    for q in sammy_present::menu::grid_walls(w, game.world.menu_markers.values()) {
        let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, q.corners.iter().map(|&p| coords::pos(p).to_array()).collect::<Vec<_>>());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![coords::pos([0.0, -1.0, 0.0]).normalize().to_array(); 4]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, q.uvs.to_vec());
        mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let image = images.add(crate::render::image::to_bevy(&w.textures[q.texture].image, true, true));
        let l = sammy_present::menu::WALL_LIGHT;
        let mat = materials.add(StandardMaterial { base_color_texture: Some(image), base_color: Color::srgb(l, l, l), unlit: true, cull_mode: None, ..default() });
        commands.spawn((GridWall, Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat), Transform::default()));
    }
}
