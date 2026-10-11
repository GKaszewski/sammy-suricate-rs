//! Per-level camera settings and their runtime state.

use super::*;

/// Per-level follow-camera settings (`InitCamera1`/`InitCamera2`, mm1.wdl:2719-2851).
#[derive(Clone, Copy, Debug)]
pub struct CameraParams {
    /// `HautCamPerso`: height above Sammy.
    pub height: f32,
    /// `PersoDistCam`: distance behind.
    pub dist: f32,
    /// `PersoAngArriere`: angle behind Sammy (180 = directly behind).
    pub back: f32,
    /// `Camera_PlusTilt`: look-at point above Sammy.
    pub plus_tilt: f32,
    /// Type-3 horizontal / vertical frictions.
    pub fric_h: f32,
    pub fric_v: f32,
    /// Level-start camera origin and pan/tilt (`CameraPosOrigine`, `InitGame2` Skill2-6).
    pub origin: Option<([f32; 3], f32, f32)>,
    /// `CameraTypeStart` from `InitCamera1` flags (2 = `HandleCamera`, 3 = follow camera).
    pub start_type: u8,
    /// `CameraCanMoveInZ` (InitCamera1 Flag1).
    pub can_move_z: bool,
    /// Type-2 frictions (`Camera1Friction`/`Camera1VertFriction`, InitCamera2 Skill1/Skill4).
    pub fric1_h: f32,
    pub fric1_v: f32,
    /// `Camera3FrictionStart` (InitCamera3 Skill8): level-start sweep speed (both axes).
    pub start_fric: f32,
    /// Values restored on respawn (`HautCamPersoOrg`, `PersoDistCamOrg`, `Camera_PlusTiltOrg`).
    pub org: (f32, f32, f32),
    /// `camera.arc`: horizontal field of view in degrees (`CameraArcVoulu`, default 60).
    pub arc: f32,
    /// Emergency-camera zoom limits (`InitCamera4` Skill1; distance floor 50).
    pub zoom_min_height: f32,
    pub zoom_min_dist: f32,
}

impl Default for CameraParams {
    fn default() -> Self {
        // InitLvl.wdl defaults.
        Self {
            height: 800.0,
            dist: 400.0,
            back: 180.0,
            plus_tilt: 200.0,
            fric_h: 4.0,
            fric_v: 0.01,
            zoom_min_height: 105.0,
            zoom_min_dist: 50.0,
            arc: 60.0,
            origin: Some(([832.0, -316.0, 1550.0], 90.0, -65.0)),
            start_type: 2,
            can_move_z: false,
            fric1_h: 1.0,
            fric1_v: 0.1,
            start_fric: 0.5,
            org: (400.0, 800.0, 200.0),
        }
    }
}

impl CameraParams {
    pub fn apply_init_camera1(&mut self, obj: &Object) {
        let s = &obj.skills;
        let or = |v: f32, d: f32| if v == 0.0 { d } else { v };
        self.height = or(s[0], self.height);
        self.dist = or(s[3], self.dist);
        self.back = or(s[6], self.back);
        // Skill8 is "always applied, even 0" (docs/gameplay.md §6.1).
        self.plus_tilt = s[7];
        self.org = (self.dist, self.height, self.plus_tilt);
        // Flags tested in order Flag2, Flag3, Flag4, each overriding; none = type 2.
        let f = |n: u32| obj.flags & (1 << (n - 1)) != 0;
        self.can_move_z = f(1);
        self.start_type = 2;
        if f(2) {
            self.start_type = 3;
        }
        if f(3) {
            self.start_type = 2; // type 1 is unused by shipped data; treated as type 2
        }
        if f(4) {
            self.start_type = 2;
        }
    }

    /// `InitCamera3` Skill8: start-sweep friction (0 → `Camera3FrictionOrg`).
    pub fn apply_init_camera3(&mut self, obj: &Object) {
        if obj.skills[7] != 0.0 {
            self.start_fric = obj.skills[7];
        }
    }

    pub fn apply_init_game2(&mut self, obj: &Object) {
        let s = &obj.skills;
        let (mut o, mut pan, mut tilt) = self.origin.unwrap_or(([832.0, -316.0, 1550.0], 90.0, -65.0));
        // Each skill overrides the default only when non-zero (mm1.wdl:1835-1846).
        for i in 0..3 {
            if s[1 + i] != 0.0 {
                o[i] = s[1 + i];
            }
        }
        if s[4] != 0.0 {
            pan = s[4];
        }
        if s[5] != 0.0 {
            tilt = if s[5] == 90.0 { -90.0 } else { s[5] }; // "erreur de Wed !"
        }
        self.origin = Some((o, pan, tilt));
    }

    pub fn apply_init_camera2(&mut self, obj: &Object) {
        let s = &obj.skills;
        if s[0] != 0.0 {
            self.fric1_h = s[0];
        }
        if s[3] != 0.0 {
            self.fric1_v = s[3];
        }
        if s[2] != 0.0 {
            self.fric_h = s[2];
        }
        if s[5] != 0.0 {
            self.fric_v = s[5];
        }
        // Original bug: tests Skill7 but assigns Skill6 (mm1.wdl:2818-2821).
        if s[6] != 0.0 {
            self.arc = s[5];
        }
    }
}

/// Camera globals that change during play (`CameraType`, `CameraBouge`, zone tween).
#[derive(Clone, Debug, Default)]
pub struct CameraRuntime {
    /// Current camera type (2 or 3); 0 = not yet chosen this level.
    pub cam_type: u8,
    /// A zone transition is running (`CameraBouge == 1`).
    pub bouge: bool,
    pub(super) tween: Option<ZoneTween>,
    /// Ticks left of the 0.25 friction cap after a zone transition.
    pub(super) fric_cap: f32,
    pub(super) acc: f32,
}

#[derive(Clone, Debug)]
pub(super) struct ZoneTween {
    pub(super) zone: Id,
    pub(super) steps: f32,
    pub(super) k: f32,
    pub(super) d: [f32; 4],
    pub(super) target: [f32; 4],
}
