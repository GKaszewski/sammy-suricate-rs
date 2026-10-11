//! One frame of the game: the original main loop's steps in order.

use super::*;

impl Sim {
    /// One frame: `secs` of game time (0 while paused) and this frame's input.
    pub fn tick(&mut self, secs: f32, controls: &Controls, files: &dyn Files) {
        let dt = Dt { t: secs * TICKS_PER_SEC, secs };
        if let Some(file) = self.pending_load.take() {
            self.load(&file, files);
        }
        let mut controls = controls.clone();
        if self.debug.autowalk {
            controls.forward = 1.0;
        }
        if self.playing()
            && let Some(left) = self.debug.give_up_after.as_mut()
        {
            *left -= secs;
            if *left <= 0.0 {
                self.debug.give_up_after = None;
                self.give_up();
            }
        }
        let reset = std::mem::take(&mut self.reset_scan);
        self.touched.clear();
        self.wasp_hits.clear();
        self.warn = false;

        // Before the main loop: who the camera follows, the scanners, the scenes.
        extro::update_pov(self);
        self.trigger_clock.advance(dt.t);
        intro::start(self);
        intro::tick(self, &controls, dt);
        menu::step(self, dt);

        // The main loop, in the order of the original port's frame.
        flow::run(self, &controls, dt);
        extro::fade_out(self, dt);
        spawn::snap_sprites(self);
        platforms::step(self, dt);
        self.animate(dt);
        extro::scripted_anims(self);
        camera::reset_on_respawn(self, reset);
        camera::zones(self, dt);
        doors::run(self, dt);
        player::step(self, &controls, dt);
        platforms::enter_teleports(self);
        health::detect_deaths(self, dt);
        health::collect(self, dt);
        doors::run_exits(self, dt);
        pickups::shrink(self, dt);
        doors::run_door_gems(self, dt);
        gems::surprises(self, dt);
        gems::gravity(self, dt);
        gems::lifetimes(self, dt);
        gems::fly(self, dt);
        pickups::collect(self);
        pickups::snap(self);
        platforms::run_teleports(self, dt);
        wasps::collect_ammo(self);
        wasps::fire(self, &controls, dt);
        wasps::spin_pickups(self, dt);
        wasps::escapees(self, dt);
        enemies::home_wasps(self, dt);
        wasps::fly(self, dt);
        wasps::corpses(self, dt);
        wasps::weak_doors(self);
        wasps::planks(self, dt);
        wasps::hives(self, dt);
        gems::boulders(self, dt);
        wasps::hive_wasps(self, dt);
        wasps::expire_pickups(self, dt);
        enemies::player_collider(self);
        enemies::ai(self, dt);
        extro::run_triggers(self);
        self.ai.bump();
        enemies::colliders(self);
        enemies::start_triggers(self);
        enemies::place(self);
        enemies::reset_calls(self, reset);
        health::warning_light(self, dt);
        health::run_eye(self, dt);
        health::spin(self, dt);
        health::animate_hearts(self, dt);
        health::reset_checkpoints(self, reset);
        pickups::reset_keys(self, reset);
        health::run_death(self, dt);
        extro::titles(self, &controls, dt);
        self.fade_lights(dt);
        // `WarnScoreGoal`: the score panel's blink runs out.
        self.game.warn_score = (self.game.warn_score - dt.t).max(0.0);

        // After the frame: the camera, then what hangs on it.
        camera::follow(self, dt);
        intro::glide(self, dt);
        menu::place_camera(self);
        camera::sky(self, dt);
    }

    /// Model and sprite animation clocks.
    fn animate(&mut self, dt: Dt) {
        for a in self.world.animators.values_mut() {
            a.anim.advance(dt.secs);
        }
        for a in self.world.sprite_anims.values_mut() {
            a.frame = (a.frame + a.fps * dt.secs).rem_euclid(a.frames as f32);
        }
    }

    fn fade_lights(&mut self, dt: Dt) {
        for l in self.world.lights.values_mut() {
            if let Some((rate, until)) = l.fade {
                l.range -= rate * dt.t;
                if l.range <= until {
                    l.range = 0.0;
                    l.fade = None;
                }
            }
        }
    }
}
