#[allow(unused_imports)]
use super::*;

#[allow(clippy::too_many_arguments)]
pub fn run_doors(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    sprites: Res<crate::gameplay::pickups::GemSprites>,
    assets: Res<crate::level::GameAssets>,
    player: Query<&Player>,
    enemies: Query<&crate::gameplay::enemies::Enemy>,
    mut doors: Query<(Entity, &mut Door, &mut Brush, &mut Transform, Option<&Children>)>,
    mut events: Local<Vec<DoorEvent>>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    let blockers: Vec<[f32; 3]> = enemies.iter().filter(|e| e.alive()).map(|e| e.pos).collect();
    let view = DoorView { sammy: p.pos, sammy_dead: p.state == crate::gameplay::player::State::Dead, blockers: &blockers };
    for (ent, mut door, mut brush, mut tf, children) in &mut doors {
        events.clear();
        door.step(&mut brush.origin, &mut game, &view, t, &mut events);
        for ev in events.drain(..) {
            match ev {
                DoorEvent::Sound(name) => sfx.play(name),
                DoorEvent::LaunchGem { color } => crate::gameplay::gems::spawn_door_gem(&mut commands, &sprites, ent, color, p.pos),
                DoorEvent::Flash { color } => {
                    commands.entity(ent).insert(crate::render::lights::flash(crate::render::lights::key_color(color), 600.0, 50.0, 100.0));
                }
                DoorEvent::MorphDark => morph(&mut commands, ent, &mut brush, children, &assets, sammy_core::doors::DARK_DOOR),
            }
        }
        tf.translation = coords::pos(brush.origin);
    }
}

/// `morph`: swap a map entity's prefab (model and collision hull).
pub fn morph(
    commands: &mut Commands,
    e: Entity,
    brush: &mut Brush,
    children: Option<&Children>,
    assets: &crate::level::GameAssets,
    file: &str,
) {
    let Some((wmb, parts)) = assets.prefab(file) else { return };
    brush.wmb = wmb;
    for c in children.into_iter().flatten() {
        commands.entity(*c).despawn();
    }
    commands.entity(e).with_children(|c| {
        for (mesh, mat) in parts {
            c.spawn((Mesh3d(mesh), MeshMaterial3d(mat)));
        }
    });
}

/// Moves gems flying into permanent doors; arrival validates the door, a recall
/// returns the key to Sammy.
pub fn run_door_gems(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut doors: Query<(&mut Door, &Brush)>,
    mut gems: Query<(Entity, &mut crate::gameplay::gems::DoorGem, &mut Transform)>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    for (e, mut g, mut tf) in &mut gems {
        let Ok((mut door, brush)) = doors.get_mut(g.door) else {
            commands.entity(e).despawn();
            continue;
        };
        let target = sammy_core::gems::DoorGemTarget { door_pos: brush.origin, wanted: door.gem_flying, validated: door.validated };
        match g.fly(target, p.pos, t) {
            Some(true) => {
                sfx.play("broken4");
                let (at, color) = (g.pos, g.color);
                crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::gem_explode(ps, at, color));
                door.validated = true;
                game.key_in_flight = false;
                commands.entity(e).despawn();
            }
            Some(false) => {
                // Back with Sammy: the key is returned. The original also overwrote a master
                // key won during the flight (gems.wdl:1200), leaving only a colour the exit
                // refuses: a soft-lock, so the port keeps the master key.
                if game.key != MASTER_KEY {
                    game.key = g.color;
                }
                game.key_in_flight = false;
                commands.entity(e).despawn();
            }
            None => tf.translation = coords::pos(g.pos),
        }
    }
}

pub fn run_exits(
    time: Res<Time>,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut exits: Query<(Entity, &mut ExitTile)>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    for (e, mut exit) in &mut exits {
        let on = p.grounded && p.ground() == Some(e);
        if let Some(sound) = exit.step(on, &mut game, t) {
            sfx.play(sound);
        }
    }
}
