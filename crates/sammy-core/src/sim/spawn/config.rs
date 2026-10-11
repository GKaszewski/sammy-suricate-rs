//! Config entities: level objects that only set per-level tuning.

use formats::wmb::Object;

use super::Sim;

impl Sim {
    /// `InitGame*` / `InitCamera*` / `InitLionSkills*`: per-level globals.
    pub(super) fn apply_config(&mut self, action: &str, obj: &Object) {
        match action {
            "initgame" => self.game.apply_init_game(obj),
            "initgame2" => {
                self.game.apply_init_game2(obj);
                self.camera.params.apply_init_game2(obj);
            }
            "initcamera1" => self.camera.params.apply_init_camera1(obj),
            "initgame3" => self.jump.apply_init_game3(obj),
            "initlionskills1" => self.lions.apply_lion_skills1(obj),
            "initlionskills2" => self.lions.apply_lion_skills2(obj),
            "initgame5" => self.tuning.apply_init_game5(obj),
            "initgame4" => self.tuning.apply_init_game4(obj),
            "initgame6" => {
                self.lions.apply_game6(obj);
                self.calls.apply_game6(obj);
            }
            "initgame7" => {
                self.lions.apply_game7(obj);
                self.game.apply_init_game7(obj);
                if obj.skills[0] != 0.0 {
                    self.jump.bounce_on_enemy = obj.skills[0];
                }
            }
            "initcamera3" => self.camera.params.apply_init_camera3(obj),
            "initcamera2" => self.camera.params.apply_init_camera2(obj),
            _ => {}
        }
    }
}
