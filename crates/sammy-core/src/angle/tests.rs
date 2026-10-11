#[test]
fn wraps() {
    assert_eq!(super::ang(190.0), -170.0);
    assert_eq!(super::ang(-190.0), 170.0);
    assert_eq!(super::ang(45.0), 45.0);
}
