use super::*;

#[test]
fn key_mixing_follows_key_hit() {
    assert_eq!(take_key(0, 4), (4, KeyOutcome::Taken));
    assert_eq!(take_key(1, 2), (3, KeyOutcome::Mixed));
    assert_eq!(take_key(4, 1), (5, KeyOutcome::Mixed));
    assert_eq!(take_key(2, 4), (6, KeyOutcome::Mixed));
    // Same colour or a mixed key: the old one is ejected.
    assert_eq!(take_key(2, 2), (2, KeyOutcome::Ejected(2)));
    assert_eq!(take_key(3, 4), (4, KeyOutcome::Ejected(3)));
    assert_eq!(take_key(1, MASTER_KEY), (MASTER_KEY, KeyOutcome::Ejected(1)));
}

#[test]
fn locks() {
    assert!(fits(RED, RED | YELLOW, false));
    assert!(!fits(RED | BLUE, RED | YELLOW, false));
    assert!(!fits(RED, RED | YELLOW, true));
    assert!(fits(MASTER_KEY, BLUE, true));
}
