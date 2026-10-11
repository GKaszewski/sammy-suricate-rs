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

pub fn run_doors(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    mut doors: Query<(Entity, &mut Door, &mut Brush, &mut Transform)>,
) {
    let Ok(p) = player.single() else { return };
    let t = time.delta_secs() * TICKS;
    for (ent, mut door, mut brush, mut tf) in &mut doors {
        let o = brush.origin;
        let horiz = ((o[0] - p.pos[0]).powi(2) + (o[1] - p.pos[1]).powi(2)).sqrt();
        let dist = (horiz.powi(2) + (o[2] - p.pos[2]).powi(2)).sqrt();
        let scanned = door.scan_id == DOOR_SCAN_ID && dist <= RANGE_SCAN_DOOR;
        let in_reach = scanned && horiz < door.open_dist - 4.0;
        let accepts = door.accepts(game.key);
        door.flash = (door.flash - t).max(0.0);

        door.state = match door.state {
            DoorState::Down if scanned && !accepts => {
                // Wrong or no key: flash (and blink the score for the exit door).
                if door.flash == 0.0 {
                    door.flash = 3.0;
                    if door.color == 7 && !game.goal_reached {
                        game.warn_score = 8.0;
                    }
                }
                DoorState::Down
            }
            DoorState::Down if in_reach && accepts => {
                sfx.play("DoorOpen_son");
                // DoorLight: the door glows in its colour (600 → 100 at -50/tick).
                if (1..=6).contains(&door.color) {
                    commands.entity(ent).insert(crate::lights::flash(crate::lights::key_color(door.color), 600.0, 50.0, 100.0));
                }
                if door.permanent() && game.key != MASTER_KEY {
                    // Permanent doors "catch" the gem: the key is spent (AnimGemKey).
                    game.key = 0;
                }
                DoorState::Rising
            }
            DoorState::Rising => {
                brush.origin[2] = (o[2] + door.up_speed * t).min(door.top);
                if brush.origin[2] >= door.top {
                    if door.permanent() || door.down_speed == 0.0 {
                        DoorState::Open
                    } else {
                        DoorState::Up { hold: door.hold }
                    }
                } else {
                    DoorState::Rising
                }
            }
            DoorState::Up { hold } => {
                // Sammy staying in range keeps it open (+4 ticks per scan).
                let hold = if in_reach && accepts { hold.max(4.0) } else { hold } - t;
                if hold > 0.0 {
                    DoorState::Up { hold }
                } else if horiz < 250.0 {
                    DoorState::Up { hold: 0.0 } // DoorScanEntities: never close on Sammy
                } else {
                    if door.hold > 4.0 {
                        sfx.play("Bell2");
                    }
                    sfx.play("DoorClose_son");
                    DoorState::Lowering
                }
            }
            DoorState::Lowering => {
                brush.origin[2] = (o[2] - door.down_speed * t).max(door.z0);
                if brush.origin[2] <= door.z0 { DoorState::Down } else { DoorState::Lowering }
            }
            s => s,
        };
        tf.translation = coords::pos(brush.origin);
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
