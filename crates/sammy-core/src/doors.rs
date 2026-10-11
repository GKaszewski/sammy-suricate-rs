//! Doors (`InitDoor` / `movePortail` / `OuvrePortail`, mm1.wdl:2886-3283) and the
//! exit tile (`InitExit` / `TileExit_Hit`, mm1.wdl:3956-3975).

use formats::wmb::Object;

use crate::game::{GameState, MASTER_KEY};

/// `RangeScanDoor` (mm1.wdl:165): radius of Sammy's per-tick door scan.
pub const RANGE_SCAN_DOOR: f32 = 500.0;
/// Scan id of Sammy's door scan; a door with another Skill7 never reacts to him.
pub const SAMMY_SCAN_ID: f32 = 25.0;
/// Colour of the exit door (red + yellow + blue).
pub const EXIT_COLOR: u32 = 7;
/// Rise heights up to this are replaced by [`MIN_RISE`].
const RISE_DEFAULT_BELOW: f32 = 250.0;
const MIN_RISE: f32 = 300.0;
/// Skill8 = 0: ticks held open.
const DEFAULT_HOLD: f32 = 4.0;
/// Skill3 = 0: open within this horizontal distance.
const DEFAULT_OPEN_DIST: f32 = RANGE_SCAN_DOOR - 150.0;
/// Hold extension while Sammy or a gem keeps the door busy (`Skill9 = 4`).
const HOLD_REFRESH: f32 = 4.0;
/// Close enough to walk under: validates a door a gem is flying into.
const UNDER_DOOR: f32 = 128.0;
/// Recall margin: Sammy beyond `open_dist - 4` recalls a flying gem.
const RECALL_MARGIN: f32 = 4.0;
/// Ticks of the "wrong key" flash.
const WRONG_KEY_FLASH: f32 = 3.0;
/// Ticks the score panel blinks when the exit door is refused (`WarnScoreGoal`).
const WARN_SCORE_TICKS: f32 = 8.0;
/// Holds longer than this ring bells: a blink every [`BELL_PERIOD`], `Bell1` in the
/// last [`BELL_WINDOW`] ticks (but not the very last [`BELL_QUIET`]).
const BELL_WINDOW: f32 = 48.0;
const BELL_PERIOD: f32 = 12.0;
const BELL_QUIET: f32 = 4.0;
/// Closing clearance (`DoorScanEntities`): first pass rise + 100, later passes 250.
const CLEAR_FIRST_EXTRA: f32 = 100.0;
const CLEAR_LATER: f32 = 250.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorState {
    Down,
    Rising,
    Up { hold: f32 },
    /// Hold over: wait until nobody is under the door, then lower.
    Clearing,
    Lowering,
    /// Stays open for good (one-shot doors, validated permanent doors).
    Open,
}

#[derive(Clone, Debug)]
pub struct Door {
    z0: f32,
    /// `Skill13 = z0 + rise`.
    top: f32,
    /// Skill4 / Skill5: up and down speed per tick (Skill5 = 0: never closes).
    up_speed: f32,
    pub down_speed: f32,
    /// Skill8: ticks held open.
    hold: f32,
    /// Skill3: horizontal open distance.
    open_dist: f32,
    /// Skill6: colour bitmask ([`EXIT_COLOR`] = exit door).
    pub color: u32,
    /// Skill7: scan id the door reacts to.
    scan_id: f32,
    /// Flag2: never for Sammy; Flag5: permanent (a key gem opens it for good);
    /// Flag6: no key needed; Flag7: exact colour only.
    no_sammy: bool,
    permanent: bool,
    keyless: bool,
    exact: bool,
    pub state: DoorState,
    /// Ticks left of the "wrong key" flash.
    flash: f32,
    /// Skill2: rise height (at least [`MIN_RISE`]).
    rise: f32,
    /// Skill14: a gem is flying in (or a keyless Flag5 door is engaged).
    pub gem_flying: bool,
    /// Skill15: the gem arrived / Sammy walked under: the door stays open for good.
    pub validated: bool,
    /// Sammy came back into reach during the hold.
    refreshed: bool,
    /// Bell/blink timer (Skill16).
    bell: f32,
    /// Closing clearance check pass (first uses the rise, later ones 250).
    clear_pass: u8,
}

/// What the frontend shows for a door step.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorEvent {
    Sound(&'static str),
    /// `CreateGemKeyAnim`: the held key (colour) flies from Sammy into the door.
    LaunchGem { color: u32 },
    /// A coloured door opens: flash of its colour.
    Flash { color: u32 },
    /// A validated permanent door turns into `doordark.wmb`.
    MorphDark,
}

/// The world as a door sees it.
pub struct DoorView<'a> {
    pub sammy: [f32; 3],
    pub sammy_dead: bool,
    /// Living enemies (they keep a door from closing on them).
    pub blockers: &'a [[f32; 3]],
}

/// The prefab a validated permanent door becomes.
pub const DARK_DOOR: &str = "doordark.wmb";

impl Door {
    pub fn from_object(obj: &Object) -> Self {
        let rise = if obj.skill(2) <= RISE_DEFAULT_BELOW { MIN_RISE } else { obj.skill(2) };
        Self {
            z0: obj.origin[2],
            top: obj.origin[2] + rise,
            up_speed: obj.skill(4),
            down_speed: obj.skill(5),
            hold: if obj.skill(8) == 0.0 { DEFAULT_HOLD } else { obj.skill(8) },
            open_dist: if obj.skill(3) == 0.0 { DEFAULT_OPEN_DIST } else { obj.skill(3) },
            color: obj.skill(6) as u32,
            scan_id: obj.skill(7),
            no_sammy: obj.flag(2),
            permanent: obj.flag(5),
            keyless: obj.flag(6),
            exact: obj.flag(7),
            state: DoorState::Down,
            flash: 0.0,
            rise,
            gem_flying: false,
            validated: false,
            refreshed: false,
            bell: 0.0,
            clear_pass: 0,
        }
    }

    /// The open rule of `movePortail` for Sammy.
    pub fn accepts(&self, key: u32) -> bool {
        if self.no_sammy {
            return false;
        }
        if key == MASTER_KEY || self.keyless {
            return true;
        }
        if self.exact {
            return key == self.color;
        }
        // A primary key (red 1, yellow 2, blue 4) opens every door containing it.
        key & self.color != 0 && (key == self.color || matches!(key, 1 | 2 | 4))
    }

    pub fn permanent(&self) -> bool {
        self.permanent
    }

    /// One frame (`t` ticks); `origin` is the door brush's position, moved here.
    pub fn step(&mut self, origin: &mut [f32; 3], game: &mut GameState, view: &DoorView, t: f32, out: &mut Vec<DoorEvent>) {
        let o = *origin;
        let p = view.sammy;
        let horiz = ((o[0] - p[0]).powi(2) + (o[1] - p[1]).powi(2)).sqrt();
        let dist = (horiz.powi(2) + (o[2] - p[2]).powi(2)).sqrt();
        // Sammy's per-tick door scan (id 25) drives `movePortail`.
        let scanned = self.scan_id == SAMMY_SCAN_ID && dist <= RANGE_SCAN_DOOR && !view.sammy_dead;
        let accepts = self.accepts(game.key);
        self.flash = (self.flash - t).max(0.0);

        // OuvrePortail, reached with the right key (or while a gem is flying).
        if scanned && (accepts || self.gem_flying) {
            if horiz > self.open_dist - RECALL_MARGIN {
                if self.down_speed != 0.0 {
                    self.gem_flying = false; // recall the flying gem
                }
            } else if horiz < UNDER_DOOR && self.gem_flying {
                self.validated = true; // walking under the door validates it
            } else if matches!(self.state, DoorState::Up { .. }) {
                self.refreshed = true;
            } else if accepts && matches!(self.state, DoorState::Down) {
                if self.permanent && !self.gem_flying {
                    // CreateGemKeyAnim: the held gem flies into the door (none for key 0).
                    self.gem_flying = true;
                    self.validated = false;
                    if game.key != 0 {
                        out.push(DoorEvent::LaunchGem { color: game.key });
                        game.key_in_flight = true;
                        if !game.goal_reached {
                            game.key = 0;
                        }
                    }
                }
                out.push(DoorEvent::Sound("DoorOpen_son"));
                if (1..=6).contains(&self.color) {
                    out.push(DoorEvent::Flash { color: self.color });
                }
                self.state = DoorState::Rising;
            }
        } else if scanned && !accepts && self.flash == 0.0 && matches!(self.state, DoorState::Down) {
            // Wrong or no key: flash (and blink the score for the exit door).
            self.flash = WRONG_KEY_FLASH;
            if self.color == EXIT_COLOR && !game.goal_reached {
                game.warn_score = WARN_SCORE_TICKS;
            }
        }

        self.state = match self.state {
            DoorState::Rising => {
                origin[2] = (o[2] + self.up_speed * t).min(self.top);
                if origin[2] < self.top {
                    DoorState::Rising
                } else if self.down_speed == 0.0 && !self.permanent {
                    DoorState::Open // burn door
                } else {
                    self.refreshed = false;
                    self.bell = 0.0;
                    DoorState::Up { hold: self.hold }
                }
            }
            DoorState::Up { hold } => {
                if self.permanent && self.validated {
                    out.push(DoorEvent::MorphDark);
                    self.gem_flying = false;
                    DoorState::Open
                } else {
                    let hold = hold - t;
                    if self.hold > BELL_WINDOW {
                        self.bell -= t;
                        if self.bell <= 0.0 {
                            self.bell = BELL_PERIOD;
                            if hold < BELL_WINDOW && hold > BELL_QUIET {
                                out.push(DoorEvent::Sound("Bell1"));
                            }
                        }
                    }
                    if hold > 0.0 {
                        DoorState::Up { hold }
                    } else if self.refreshed || self.gem_flying {
                        self.refreshed = false;
                        DoorState::Up { hold: HOLD_REFRESH }
                    } else {
                        if self.hold > BELL_QUIET {
                            out.push(DoorEvent::Sound("Bell2"));
                        }
                        self.clear_pass = 0;
                        DoorState::Clearing
                    }
                }
            }
            DoorState::Clearing => {
                if self.down_speed == 0.0 {
                    DoorState::Up { hold: HOLD_REFRESH } // never lowers (Skill5 = 0)
                } else {
                    // DoorScanEntities: Sammy or an enemy within range keeps the door up.
                    let r = if self.clear_pass == 0 { self.rise + CLEAR_FIRST_EXTRA } else { CLEAR_LATER };
                    let d3 = |q: [f32; 3]| ((q[0] - o[0]).powi(2) + (q[1] - o[1]).powi(2) + (q[2] - o[2]).powi(2)).sqrt();
                    let blocked = d3(p) < r || view.blockers.iter().any(|&e| d3(e) < r);
                    self.clear_pass = 1;
                    if blocked {
                        DoorState::Clearing
                    } else {
                        out.push(DoorEvent::Sound("DoorClose_son"));
                        DoorState::Lowering
                    }
                }
            }
            DoorState::Lowering => {
                origin[2] = (o[2] - self.down_speed * t).max(self.z0);
                if origin[2] <= self.z0 {
                    self.gem_flying = false;
                    DoorState::Down
                } else {
                    DoorState::Lowering
                }
            }
            s => s,
        };
    }
}

/// The exit tile: standing on it plays `Win_son`, and the level is won 8 ticks later.
#[derive(Clone, Debug, Default)]
pub struct ExitTile {
    countdown: Option<f32>,
}

/// Ticks between stepping on the exit and `partieGagnee`.
const EXIT_DELAY: f32 = 8.0;

impl ExitTile {
    /// One frame; returns the sound to play, if any.
    pub fn step(&mut self, standing_on: bool, game: &mut GameState, t: f32) -> Option<&'static str> {
        match self.countdown {
            None if standing_on && !game.won => {
                self.countdown = Some(EXIT_DELAY);
                return Some("Win_son");
            }
            Some(left) if left - t <= 0.0 => {
                game.won = true;
                self.countdown = Some(f32::INFINITY);
            }
            Some(left) => self.countdown = Some(left - t),
            None => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(color: u32, flags: u32) -> Door {
        let obj = Object { skills: vec![0.0, 0.0, 0.0, 40.0, 80.0, color as f32, SAMMY_SCAN_ID, 0.0], flags, ..Default::default() };
        Door::from_object(&obj)
    }

    const fn flag(n: u32) -> u32 {
        1 << (n - 1)
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
        // Exit door needs the master key or a matching primary.
        assert!(door(EXIT_COLOR, 0).accepts(MASTER_KEY));
        assert!(!door(EXIT_COLOR, 0).accepts(6));
    }

    #[test]
    fn opens_rises_holds_and_closes() {
        let mut d = door(1, 0);
        let mut game = GameState { key: 1, ..Default::default() };
        let mut origin = [0.0; 3];
        let mut out = Vec::new();
        let near = DoorView { sammy: [100.0, 0.0, 0.0], sammy_dead: false, blockers: &[] };
        d.step(&mut origin, &mut game, &near, 1.0, &mut out);
        assert_eq!(d.state, DoorState::Rising);
        assert!(out.contains(&DoorEvent::Sound("DoorOpen_son")) && out.contains(&DoorEvent::Flash { color: 1 }));
        let away = DoorView { sammy: [5000.0, 0.0, 0.0], sammy_dead: false, blockers: &[] };
        for _ in 0..40 {
            d.step(&mut origin, &mut game, &away, 1.0, &mut out);
        }
        assert_eq!(d.state, DoorState::Down);
        assert_eq!(origin[2], 0.0);
        assert!(out.contains(&DoorEvent::Sound("DoorClose_son")));
    }
}
