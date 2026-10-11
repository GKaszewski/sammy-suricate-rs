#[allow(unused_imports)]
use super::*;

pub fn snap_pickups(
    world: CollisionWorld,
    mut commands: Commands,
    mut q: Query<
        (Entity, &mut PickupPos, &mut Transform, Option<&crate::gameplay::indicators::ExtraHeight>, Option<&mut crate::gameplay::gems::GemGravity>, Has<HoverFromWed>),
        With<SnapToGround>,
    >,
) {
    if !world.ready() {
        return;
    }
    for (e, mut p, mut tf, extra, gravity, hover) in &mut q {
        let [x, y, z] = p.0;
        let tr = world.trace(Hull::Narrow, [x, y, z + SNAP_START], [x, y, z - 2000.0], None).trace;
        let floor = tr.end[2] - NARROW_FLOOR_OFFSET;
        if hover && let Some(mut g) = gravity {
            // Flag5: keep the WED height above the floor (+HeightGem) as the rest height.
            g.rest = z - floor + HEIGHT_GEM;
        }
        if tr.hit() && !tr.start_solid {
            p.0[2] = floor + HEIGHT_GEM + extra.map_or(0.0, |h| h.0);
            tf.translation = coords::pos(p.0);
        }
        commands.entity(e).remove::<SnapToGround>();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collect(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut sfx: Sfx,
    player: Query<&Player>,
    diamonds: Query<(Entity, &PickupPos, &Diamond), (Without<Shrinking>, Without<SnapToGround>)>,
    mut keys: Query<
        (Entity, &PickupPos, &KeyGem, Option<&mut LevelKey>, Option<&crate::gameplay::gems::GemLifetime>, Has<crate::gameplay::gems::Collectable>),
        (Without<Shrinking>, Without<SnapToGround>, Without<Collected>),
    >,
    sprites: Res<GemSprites>,
) {
    let Ok(p) = player.single() else { return };
    if p.state == crate::gameplay::player::State::Dead {
        return;
    }
    let near = |q: &[f32; 3]| in_scan(p.pos, p.scan_radius, *q);

    for (e, pos, d) in &diamonds {
        if near(&pos.0) {
            commands.entity(e).insert(Shrinking);
            let at = pos.0;
            crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::diamond_burst(ps, at));
            sfx.play("Diam_son");
            if d.has_popup() {
                sfx.play("clog3");
                crate::ui::hud3d::queue_popup(&mut commands, pos.0, d.points);
            }
            game.add_points(d.points, &mut sfx);
        }
    }

    // All key gems vanish once the goal is reached (gems.wdl:77).
    if game.goal_reached {
        for (e, _, _, _, _, collectable) in &keys {
            if collectable {
                commands.entity(e).insert(Shrinking);
            }
        }
        return;
    }
    // A gem flying into a door blocks pickups (`PlayerOldKey == 999`).
    if game.key_in_flight {
        return;
    }
    // Key_Hit: take the gem; mix primaries, otherwise throw the old key back out.
    for (e, pos, gem, level_key, life, collectable) in &mut keys {
        if !collectable || !near(&pos.0) {
            continue;
        }
        commands.entity(e).insert((Shrinking, crate::render::lights::flash(crate::render::lights::key_color(gem.color), 300.0, 15.0, 100.0)));
        if let Some(mut lk) = level_key {
            lk.home.get_or_insert(pos.0);
        }
        sfx.play("ColoredGem_son");
        let (at, color, held) = (pos.0, gem.color, game.key);
        crate::render::effects::queue(&mut commands, move |ps| crate::render::effects::gem_catch(ps, at, color));
        let (key, outcome) = take_key(game.key, gem.color);
        game.key = key;
        match outcome {
            KeyOutcome::Mixed => {
                sfx.play("GemMixing_son");
                // AnimGemMix1 end: each source primary bursts 70 stars (gems.wdl:596-605).
                let sammy = [p.pos[0], p.pos[1], p.pos[2] + 40.0];
                crate::render::effects::queue(&mut commands, move |ps| {
                    crate::render::effects::gem_mix(ps, sammy, held);
                    crate::render::effects::gem_mix(ps, sammy, color);
                });
            }
            KeyOutcome::Ejected(old) => crate::gameplay::gems::throw_back_key(&mut commands, &sprites, old, p, life.map_or(0.0, |l| l.ticks)),
            KeyOutcome::Taken => {}
        }
        break; // one gem per scan
    }
}

pub fn shrink(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, Has<Respawnable>), With<Shrinking>>,
) {
    let k = SHRINK_PER_TICK.powf(time.delta_secs() * TICKS);
    for (e, mut tf, respawnable) in &mut q {
        tf.scale *= k;
        if tf.scale.x < SHRINK_GONE {
            if respawnable {
                commands.entity(e).remove::<Shrinking>().insert((Collected, Visibility::Hidden));
            } else {
                commands.entity(e).despawn();
            }
        }
    }
}

/// Reset scan (id 1000) for keys: level keys come back home (or vanish for good
/// once Sammy holds the master key); runtime keys are removed.
pub fn reset_keys(
    mut reset: MessageReader<ResetScan>,
    mut commands: Commands,
    game: Res<GameState>,
    mut rng: ResMut<crate::gameplay::state::Rng>,
    mut keys: Query<(Entity, &mut PickupPos, &mut Transform, Option<&LevelKey>, Option<&mut crate::gameplay::gems::Surprise>), With<KeyGem>>,
) {
    if reset.read().count() == 0 {
        return;
    }
    for (e, mut pos, mut tf, level_key, surprise) in &mut keys {
        match level_key {
            Some(lk) if game.key != MASTER_KEY => {
                if let Some(home) = lk.home {
                    pos.0 = home;
                }
                tf.translation = coords::pos(pos.0);
                tf.scale = lk.scale;
                let mut ent = commands.entity(e);
                ent.remove::<(Collected, Shrinking)>().insert(Visibility::Inherited);
                // InitKey runs again: a surprise gem re-rolls (its colour keeps climbing).
                match surprise {
                    Some(mut s) => {
                        s.rearm(&mut rng.0);
                        ent.remove::<crate::gameplay::gems::Collectable>();
                    }
                    None => {
                        ent.insert(crate::gameplay::gems::Collectable);
                    }
                }
            }
            _ => {
                commands.entity(e).despawn();
            }
        }
    }
}
