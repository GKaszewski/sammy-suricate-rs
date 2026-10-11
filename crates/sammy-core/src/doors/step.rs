//! A door's per-tick behaviour: opening, holding, bells, closing and gems.

use super::*;

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
        self.keyless || crate::keys::fits(key, self.color, self.exact)
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
        // Sammy's per-tick door scan (id 25) drives `movePortail`; a door open for good
        // has no event any more (no recall, no flash).
        let scanned = self.scan_id == SAMMY_SCAN_ID && dist <= RANGE_SCAN_DOOR && !view.sammy_dead && self.state != DoorState::Open;
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
                    // Skill14 stays set: a gem still flying in lands and the key is spent.
                    out.push(DoorEvent::MorphDark);
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
