//! Per-level lion tuning and the friend-call state.

use super::*;

/// Per-level tuning globals (`InitLevelglobal` values, §1.5), overridable by
/// `InitLionSkills1/2` and `InitGame6/7`.
#[derive(Clone, Copy, Debug)]
pub struct LionGlobals {
    pub rotation_all: f32,
    pub rotation_near: f32,
    pub rotation_far: f32,
    pub delta_dest_pan: f32,
    pub love_patrol: f32,
    pub lion_force: f32,
    pub near_target_path: f32,
    pub far_target_path: f32,
    pub intensite_bounce: f32,
    pub amorti_bounce: f32,
    pub limit_time_without_patrol: f32,
    pub random_attack: f32,
    pub hargne: f32,
    pub lachete: f32,
    pub frightened: f32,
    pub dist_muzzle: f32,
    pub gravite: f32,
    pub acc_gravite: f32,
}

impl Default for LionGlobals {
    fn default() -> Self {
        Self {
            rotation_all: 0.2,
            rotation_near: 0.02,
            rotation_far: 0.2,
            delta_dest_pan: 5.0,
            love_patrol: 0.05,
            lion_force: 2.0,
            near_target_path: 100.0,
            far_target_path: 150.0,
            intensite_bounce: 15.0,
            amorti_bounce: 10.0,
            limit_time_without_patrol: 64.0,
            random_attack: 50.0,
            hargne: 2.0,
            lachete: 0.5,
            frightened: 0.05,
            dist_muzzle: 32.0,
            gravite: -4.0,
            acc_gravite: 3.0,
        }
    }
}

impl LionGlobals {
    /// `InitLionSkills1` (mm1.wdl:1928-1962).
    pub fn apply_lion_skills1(&mut self, obj: &Object) {
        let s = &obj.skills;
        let set = |v: &mut f32, x: f32| {
            if x != 0.0 {
                *v = x;
            }
        };
        set(&mut self.rotation_near, s[0]);
        set(&mut self.rotation_far, s[1]);
        set(&mut self.rotation_all, s[2]);
        set(&mut self.delta_dest_pan, s[3]);
        set(&mut self.love_patrol, s[4]);
        set(&mut self.lion_force, s[5]);
        set(&mut self.near_target_path, s[6]);
        set(&mut self.far_target_path, s[7]);
    }

    /// `InitLionSkills2`.
    pub fn apply_lion_skills2(&mut self, obj: &Object) {
        let s = &obj.skills;
        let set = |v: &mut f32, x: f32| {
            if x != 0.0 {
                *v = x;
            }
        };
        set(&mut self.intensite_bounce, s[0]);
        set(&mut self.amorti_bounce, s[1]);
        set(&mut self.limit_time_without_patrol, s[2]);
        set(&mut self.random_attack, s[3]);
    }

    /// `InitGame6` (mm1.wdl:1866-1889): Skill3-6.
    pub fn apply_game6(&mut self, obj: &Object) {
        let s = &obj.skills;
        if s[2] != 0.0 {
            self.hargne = s[2];
        }
        if s[3] != 0.0 {
            self.lachete = s[3];
        }
        if s[4] != 0.0 {
            self.frightened = s[4];
        }
        if s[5] != 0.0 {
            self.dist_muzzle = if s[5] == 999.0 { 0.0 } else { s[5] };
        }
    }

    /// `InitGame7` Skill5: enemy gravity.
    pub fn apply_game7(&mut self, obj: &Object) {
        let s = &obj.skills;
        if s[4] != 0.0 {
            self.gravite = s[4];
        }
    }
}

/// Friend calls (`CallFriendsToHelp`, lion.wdl:998-1070; §4.5). These globals are
/// not reset per level in the original (`AnswerFriends` never is).
#[derive(Clone, Debug)]
pub struct FriendCalls {
    /// `AttackCall`: ticks left of an active call.
    pub attack_call: f32,
    /// `FriendCall`: caller's `spec + weight`.
    pub(super) friend_call: f32,
    /// `AnswerFriends`: 0 locks friend calls for the rest of the game.
    pub(super) answer: i32,
    pub tempo_call: f32,
    pub range_call: f32,
}

impl FriendCalls {
    /// `InitGame6`: Skill7 = TempoCall; Skill8 ≠ 0 sets RangeCall to *Skill7* (original bug).
    pub fn apply_game6(&mut self, obj: &Object) {
        let s = &obj.skills;
        if s[6] != 0.0 {
            self.tempo_call = s[6];
        }
        if s[7] != 0.0 {
            self.range_call = s[6];
        }
    }
}

impl Default for FriendCalls {
    fn default() -> Self {
        Self { attack_call: 0.0, friend_call: 0.0, answer: 1, tempo_call: 128.0, range_call: 10000.0 }
    }
}

/// After each AI tick: `AttackCall -= time` while a call is running.
pub fn end_tick(calls: &mut FriendCalls) {
    calls.attack_call = (calls.attack_call - 1.0).max(0.0);
}

/// Three expanding rings per friend call: z offset, delay (ticks) and skin.
pub const CALL_RINGS: [(f32, f32, u32); 3] = [(50.0, 0.0, 1), (85.0, 8.0, 3), (150.0, 26.0, 1)];
