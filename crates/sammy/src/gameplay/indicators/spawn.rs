#[allow(unused_imports)]
use super::*;

#[allow(clippy::type_complexity)]
pub fn spawn_shadows(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    enemies: Query<(Entity, &Enemy), Without<HasShadow>>,
    player: Query<Entity, (With<Player>, Without<HasShadow>)>,
) {
    let mut spawn = |owner: Entity, file: &str, sx: f32, sy: f32| {
        let Some((mat, size)) = shadow_material(&assets, &mut materials, file) else { return };
        let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(size[0] * sx * 0.5, size[1] * sy * 0.5) / QUANTS_PER_UNIT));
        commands.spawn((GroundShadow { owner }, Mesh3d(mesh), MeshMaterial3d(mat), Transform::default(), crate::flow::LevelScoped));
        commands.entity(owner).insert(HasShadow);
    };
    for (e, enemy) in &enemies {
        // Lion shadow: shadlio2.pcx at 1.2 × the lion's scale.
        spawn(e, "shadlio2.pcx", 1.2 * enemy.scale(), 1.2 * enemy.scale());
    }
    // Sammy's blob sprite (ombblob2.pcx) is only shown below 40 fps (player.wdl:197-207);
    // he casts the engine model shadow instead (crate::render::shadow).
    for e in &player {
        commands.entity(e).insert((HasShadow, crate::render::shadow::CastsShadow));
    }
}

pub fn init_eye(obj: &Object, entity: &mut EntityCommands) {
    let eye = sammy_core::eye::EyePickup::from_object(obj);
    let snap = eye.snap;
    entity.insert((EyePickup { eye, scale: Vec3::new(obj.scale[1], obj.scale[2], obj.scale[0]).abs() }, PickupPos(obj.origin)));
    if snap {
        entity.insert((SnapToGround, ExtraHeight(sammy_core::eye::EXTRA_HEIGHT)));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_rings(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: Local<Option<Vec<Handle<StandardMaterial>>>>,
    enemies: Query<Entity, (With<Enemy>, Without<HasRing>)>,
    calls: Query<Entity, (With<CallRing>, Without<Mesh3d>)>,
) {
    let Some(mats) = ring_materials(&assets, &mut materials, &mut cache) else { return };
    commands.insert_resource(RingMaterials(mats.clone()));
    for owner in &enemies {
        let Some((mesh, anim)) = ring_bundle(&assets, &mut meshes) else { return };
        commands.spawn((RangeRing { owner }, mesh, anim, MeshMaterial3d(mats[0].clone()), Transform::default(), Visibility::Hidden, crate::flow::LevelScoped));
        commands.entity(owner).insert(HasRing);
    }
    for e in &calls {
        let Some((mesh, anim)) = ring_bundle(&assets, &mut meshes) else { return };
        commands.entity(e).insert((mesh, anim, MeshMaterial3d(mats[0].clone()), Transform::default(), Visibility::Hidden));
    }
}
