use super::*;

#[test]
fn fates() {
    assert_eq!(Fate::from_skill(0.0), Fate::Decorative);
    assert_eq!(Fate::from_skill(64.0), Fate::Lifetime(64.0));
    assert_eq!(Fate::from_skill(1000.0), Fate::Stays);
    assert_eq!(Fate::from_skill(1001.0), Fate::Respawns);
}

#[test]
fn door_gem_reaches_a_waiting_door() {
    let mut g = DoorGem::new(2, [0.0; 3]);
    let target = DoorGemTarget { door_pos: [200.0, 0.0, 0.0], wanted: true, validated: false };
    let arrived = (0..100).find_map(|_| g.fly(target, [0.0; 3], 1.0));
    assert_eq!(arrived, Some(true));
}
