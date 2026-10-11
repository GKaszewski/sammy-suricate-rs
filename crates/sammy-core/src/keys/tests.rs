use super::*;

const K: fn(u32) -> KeyColor = KeyColor;

#[test]
fn key_mixing_follows_key_hit() {
    assert_eq!(take_key(K(0), K(4)), (K(4), KeyOutcome::Taken));
    assert_eq!(take_key(K(1), K(2)), (K(3), KeyOutcome::Mixed));
    assert_eq!(take_key(K(4), K(1)), (K(5), KeyOutcome::Mixed));
    assert_eq!(take_key(K(2), K(4)), (K(6), KeyOutcome::Mixed));
    // Same colour or a mixed key: the old one is ejected.
    assert_eq!(take_key(K(2), K(2)), (K(2), KeyOutcome::Ejected(K(2))));
    assert_eq!(take_key(K(3), K(4)), (K(4), KeyOutcome::Ejected(K(3))));
    assert_eq!(take_key(K(1), MASTER_KEY), (MASTER_KEY, KeyOutcome::Ejected(K(1))));
}

#[test]
fn locks() {
    assert!(RED.fits(RED | YELLOW, false));
    assert!(!(RED | BLUE).fits(RED | YELLOW, false));
    assert!(!RED.fits(RED | YELLOW, true));
    assert!(MASTER_KEY.fits(BLUE, true));
}
