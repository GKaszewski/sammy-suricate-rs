//! Doors (`InitDoor`/`movePortail`/`OuvrePortail`, mm1.wdl:2886-3283) and the
//! exit tile (`InitExit`/`TileExit_Hit`, mm1.wdl:3956-3975).

use bevy::prelude::*;
use formats::wmb::Object;

use super::player::{Player, TICKS};
use crate::audio::Sfx;
use crate::collision::Brush;
use crate::coords;
use crate::game::{GameState, MASTER_KEY};

/// `RangeScanDoor` (mm1.wdl:165): radius of Sammy's door scan (id 25).
const RANGE_SCAN_DOOR: f32 = 500.0;
/// Scan id of Sammy's per-tick door scan.
const DOOR_SCAN_ID: f32 = 25.0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum DoorState {
    Down,
    Rising,
    Up { hold: f32 },
    /// Hold over: wait until nobody is under the door, then lower.
    Clearing,
    Lowering,
    /// Stays open for good (one-shot doors, validated permanent doors).
    Open,
}

#[derive(Component)]
pub struct Door {
    z0: f32,
    /// `Skill13 = z0 + Skill2` (rise height, at least 300).
    top: f32,
    /// Skill4 / Skill5: up and down speed per tick (Skill5 = 0: never closes).
    up_speed: f32,
    down_speed: f32,
    /// Skill8: ticks held open (default 4).
    hold: f32,
    /// Skill3: horizontal open distance (default `RangeScanDoor - 150`).
    open_dist: f32,
    /// Skill6: colour bitmask (7 = exit door).
    color: u32,
    /// Skill7: scan id the door reacts to (25 = Sammy).
    scan_id: f32,
    flags: u32,
    state: DoorState,
    /// Ticks left of the "wrong key" flash.
    flash: f32,
    /// Skill2: rise height (for the closing clearance radius).
    rise: f32,
    /// Skill14: a gem is flying in (or a keyless Flag5 door is engaged).
    pub gem_flying: bool,
    /// Skill15: the gem arrived / Sammy walked under: the door stays open for good.
    pub validated: bool,
    /// Sammy came back into reach during the hold (`Skill9 = 4`).
    refreshed: bool,
    /// Bell/blink timer (Skill16).
    bell: f32,
    /// Closing clearance check: first pass uses Skill2 + 100, later passes 250.
    clear_pass: u8,
}

const fn flag(n: u32) -> u32 {
    1 << (n - 1)
}

pub fn init_door(obj: &Object, entity: &mut EntityCommands) {
    let s = &obj.skills;
    let rise = if s[1] <= 250.0 { 300.0 } else { s[1] };
    entity.insert(Door {
        z0: obj.origin[2],
        top: obj.origin[2] + rise,
        up_speed: s[3],
        down_speed: s[4],
        hold: if s[7] == 0.0 { 4.0 } else { s[7] },
        open_dist: if s[2] == 0.0 { RANGE_SCAN_DOOR - 150.0 } else { s[2] },
        color: s[5] as u32,
        scan_id: s[6],
        flags: obj.flags,
        state: DoorState::Down,
        flash: 0.0,
        rise,
        gem_flying: false,
        validated: false,
        refreshed: false,
        bell: 0.0,
        clear_pass: 0,
    });
}

impl Door {
    /// The open rule of `movePortail` for Sammy (`Flag2 == 0`).
    fn accepts(&self, key: u32) -> bool {
        if self.flags & flag(2) != 0 {
            return false;
        }
        if key == MASTER_KEY || self.flags & flag(6) != 0 {
            return true;
        }
        if self.flags & flag(7) != 0 {
            return key == self.color;
        }
        key & self.color != 0 && (key == self.color || matches!(key, 1 | 2 | 4))
    }

    fn permanent(&self) -> bool {
        self.flags & flag(5) != 0
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_doors(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    sprites: Res<super::pickups::GemSprites>,
    assets: Res<crate::level::GameAssets>,
    player: Query<&Player>,
    enemies: Query<&super::enemy::Enemy>,
    mut doors: Query<(Entity, &mut Door, &mut Brush, &mut Transform, Option<&Children>)>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    for (ent, mut door, mut brush, mut tf, children) in &mut doors {
        let o = brush.origin;
        let horiz = ((o[0] - p.pos[0]).powi(2) + (o[1] - p.pos[1]).powi(2)).sqrt();
        let dist = (horiz.powi(2) + (o[2] - p.pos[2]).powi(2)).sqrt();
        // Sammy's per-tick door scan (id 25, radius 500) drives `movePortail`.
        let scanned = door.scan_id == DOOR_SCAN_ID && dist <= RANGE_SCAN_DOOR && p.state != super::player::State::Dead;
        let accepts = door.accepts(game.key);
        door.flash = (door.flash - t).max(0.0);

        // OuvrePortail, reached with the right key (or while a gem is flying).
        if scanned && (accepts || door.gem_flying) {
            if horiz > door.open_dist - 4.0 {
                if door.down_speed != 0.0 {
                    door.gem_flying = false; // recall the flying gem
                }
            } else if horiz < 128.0 && door.gem_flying {
                door.validated = true; // walking under the door validates it
            } else if matches!(door.state, DoorState::Up { .. }) {
                door.refreshed = true;
            } else if accepts && matches!(door.state, DoorState::Down) {
                if door.permanent() && !door.gem_flying {
                    // CreateGemKeyAnim: the held gem flies into the door (none for key 0).
                    door.gem_flying = true;
                    door.validated = false;
                    if game.key != 0 {
                        super::gems::spawn_door_gem(&mut commands, &sprites, ent, game.key, p.pos);
                        game.key_in_flight = true;
                        if !game.goal_reached {
                            game.key = 0;
                        }
                    }
                }
                sfx.play("DoorOpen_son");
                if (1..=6).contains(&door.color) {
                    commands.entity(ent).insert(crate::lights::flash(crate::lights::key_color(door.color), 600.0, 50.0, 100.0));
                }
                door.state = DoorState::Rising;
            }
        } else if scanned && !accepts && door.flash == 0.0 && matches!(door.state, DoorState::Down) {
            // Wrong or no key: flash (and blink the score for the exit door).
            door.flash = 3.0;
            if door.color == 7 && !game.goal_reached {
                game.warn_score = 8.0;
            }
        }

        door.state = match door.state {
            DoorState::Rising => {
                brush.origin[2] = (o[2] + door.up_speed * t).min(door.top);
                if brush.origin[2] < door.top {
                    DoorState::Rising
                } else if door.down_speed == 0.0 && !door.permanent() {
                    DoorState::Open // burn door
                } else {
                    door.refreshed = false;
                    door.bell = 0.0;
                    DoorState::Up { hold: door.hold }
                }
            }
            DoorState::Up { hold } => {
                if door.permanent() && door.validated {
                    morph(&mut commands, ent, &mut brush, children, &assets, "doordark.wmb");
                    door.gem_flying = false;
                    DoorState::Open
                } else {
                    let hold = hold - t;
                    // Bells: long holds blink every 12 ticks, Bell1 in the last 48 ticks.
                    if door.hold > 48.0 {
                        door.bell -= t;
                        if door.bell <= 0.0 {
                            door.bell = 12.0;
                            if hold < 48.0 && hold > 4.0 {
                                sfx.play("Bell1");
                            }
                        }
                    }
                    if hold > 0.0 {
                        DoorState::Up { hold }
                    } else if door.refreshed || door.gem_flying {
                        door.refreshed = false;
                        DoorState::Up { hold: 4.0 }
                    } else {
                        if door.hold > 4.0 {
                            sfx.play("Bell2");
                        }
                        door.clear_pass = 0;
                        DoorState::Clearing
                    }
                }
            }
            DoorState::Clearing => {
                if door.down_speed == 0.0 {
                    DoorState::Up { hold: 4.0 } // never lowers (Skill5 = 0)
                } else {
                    // DoorScanEntities: Sammy or a lion within range keeps the door up.
                    let r = if door.clear_pass == 0 { door.rise + 100.0 } else { 250.0 };
                    let d3 = |q: [f32; 3]| ((q[0] - o[0]).powi(2) + (q[1] - o[1]).powi(2) + (q[2] - o[2]).powi(2)).sqrt();
                    let blocked = d3(p.pos) < r || enemies.iter().any(|e| e.alive() && d3(e.pos) < r);
                    door.clear_pass = 1;
                    if blocked {
                        DoorState::Clearing
                    } else {
                        sfx.play("DoorClose_son");
                        DoorState::Lowering
                    }
                }
            }
            DoorState::Lowering => {
                brush.origin[2] = (o[2] - door.down_speed * t).max(door.z0);
                if brush.origin[2] <= door.z0 {
                    door.gem_flying = false;
                    DoorState::Down
                } else {
                    DoorState::Lowering
                }
            }
            s => s,
        };
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
    mut gems: Query<(Entity, &mut super::gems::DoorGem, &mut Transform)>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    for (e, mut g, mut tf) in &mut gems {
        let Ok((mut door, brush)) = doors.get_mut(g.door) else {
            commands.entity(e).despawn();
            continue;
        };
        let target = (brush.origin, door.gem_flying, door.validated);
        match super::gems::fly_door_gem(&mut g, target, p.pos, t) {
            Some(true) => {
                sfx.play("broken4");
                let (at, color) = (super::gems::door_gem_pos(&g), g.color);
                crate::effects::queue(&mut commands, move |ps| crate::effects::gem_explode(ps, at, color));
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
            None => tf.translation = coords::pos(super::gems::door_gem_pos(&g)),
        }
    }
}

/// Exit tile: Sammy standing on it wins the level (Win_son, then `partieGagnee` 8 ticks later).
#[derive(Component)]
pub struct ExitTile {
    countdown: Option<f32>,
}

pub fn init_exit(entity: &mut EntityCommands) {
    entity.insert(ExitTile { countdown: None });
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
        match exit.countdown {
            None if p.grounded && p.ground_entity == Some(e) && !game.won => {
                sfx.play("Win_son");
                exit.countdown = Some(8.0);
            }
            Some(left) if left - t <= 0.0 => {
                game.won = true;
                exit.countdown = Some(f32::INFINITY);
            }
            Some(left) => exit.countdown = Some(left - t),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(color: u32, flags: u32) -> Door {
        Door {
            z0: 0.0,
            top: 300.0,
            up_speed: 40.0,
            down_speed: 80.0,
            hold: 4.0,
            open_dist: 350.0,
            color,
            scan_id: 25.0,
            flags,
            state: DoorState::Down,
            flash: 0.0,
            rise: 300.0,
            gem_flying: false,
            validated: false,
            refreshed: false,
            bell: 0.0,
            clear_pass: 0,
        }
    }

    #[test]
    fn open_rule_matches_move_portail() {
        // Red door: red, master key; not yellow or none.
        assert!(door(1, 0).accepts(1));
        assert!(door(1, 0).accepts(MASTER_KEY));
        assert!(!door(1, 0).accepts(2));
        assert!(!door(1, 0).accepts(0));
        // Orange door (1+2) opens with either primary or orange, not with magenta (1+4).
        assert!(door(3, 0).accepts(1));
        assert!(door(3, 0).accepts(2));
        assert!(door(3, 0).accepts(3));
        assert!(!door(3, 0).accepts(5));
        // Flag7: exact colour only.
        assert!(!door(3, flag(7)).accepts(1));
        assert!(door(3, flag(7)).accepts(3));
        // Flag6: no key needed. Flag2: not for Sammy at all.
        assert!(door(4, flag(6)).accepts(0));
        assert!(!door(4, flag(2)).accepts(MASTER_KEY));
        // Exit door (7) needs the master key or a matching primary.
        assert!(door(7, 0).accepts(MASTER_KEY));
    }
}
