use super::*;
use crate::keys::MASTER_KEY;

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
    assert!(door(1, 0).accepts(KeyColor(1)));
    assert!(door(1, 0).accepts(MASTER_KEY));
    assert!(!door(1, 0).accepts(KeyColor(2)));
    assert!(!door(1, 0).accepts(KeyColor::NONE));
    // Orange door (1+2) opens with either primary or orange, not with magenta (1+4).
    assert!(door(3, 0).accepts(KeyColor(1)));
    assert!(door(3, 0).accepts(KeyColor(2)));
    assert!(door(3, 0).accepts(KeyColor(3)));
    assert!(!door(3, 0).accepts(KeyColor(5)));
    // Flag7: exact colour only.
    assert!(!door(3, flag(7)).accepts(KeyColor(1)));
    assert!(door(3, flag(7)).accepts(KeyColor(3)));
    // Flag6: no key needed. Flag2: not for Sammy at all.
    assert!(door(4, flag(6)).accepts(KeyColor::NONE));
    assert!(!door(4, flag(2)).accepts(MASTER_KEY));
    // Exit door needs the master key or a matching primary.
    assert!(door(EXIT_COLOR.get(), 0).accepts(MASTER_KEY));
    assert!(!door(EXIT_COLOR.get(), 0).accepts(KeyColor(6)));
}

#[test]
fn opens_rises_holds_and_closes() {
    let mut d = door(1, 0);
    let mut game = GameState { key: KeyColor(1), ..Default::default() };
    let mut origin = [0.0; 3];
    let mut out = Vec::new();
    let near = DoorView { sammy: [100.0, 0.0, 0.0], sammy_dead: false, blockers: &[] };
    d.step(&mut origin, &mut game, &near, Ticks(1.0), &mut out);
    assert_eq!(d.state, DoorState::Rising);
    assert!(out.contains(&DoorEvent::Sound("DoorOpen_son")) && out.contains(&DoorEvent::Flash { color: KeyColor(1) }));
    let away = DoorView { sammy: [5000.0, 0.0, 0.0], sammy_dead: false, blockers: &[] };
    for _ in 0..40 {
        d.step(&mut origin, &mut game, &away, Ticks(1.0), &mut out);
    }
    assert_eq!(d.state, DoorState::Down);
    assert_eq!(origin[2], 0.0);
    assert!(out.contains(&DoorEvent::Sound("DoorClose_son")));
}

/// Walking under a permanent door validates it while its gem is still flying: the
/// door turns dark and still wants the gem, so the key is spent (not returned).
#[test]
fn validated_door_keeps_the_flying_gem() {
    let mut d = door(1, flag(5));
    let mut game = GameState { key: KeyColor(1), ..Default::default() };
    let mut origin = [0.0; 3];
    let mut out = Vec::new();
    let near = DoorView { sammy: [200.0, 0.0, 0.0], sammy_dead: false, blockers: &[] };
    d.step(&mut origin, &mut game, &near, Ticks(1.0), &mut out);
    assert!(out.contains(&DoorEvent::LaunchGem { color: KeyColor(1) }) && d.gem_flying && game.key == KeyColor::NONE);
    let under = DoorView { sammy: [50.0, 0.0, 0.0], ..near };
    for _ in 0..20 {
        d.step(&mut origin, &mut game, &under, Ticks(1.0), &mut out);
    }
    assert!(out.contains(&DoorEvent::MorphDark));
    assert_eq!(d.state, DoorState::Open);
    assert!(d.gem_flying && d.validated);
    // Walking away no longer recalls it.
    let away = DoorView { sammy: [450.0, 0.0, 0.0], ..near };
    d.step(&mut origin, &mut game, &away, Ticks(1.0), &mut out);
    assert!(d.gem_flying);
}
