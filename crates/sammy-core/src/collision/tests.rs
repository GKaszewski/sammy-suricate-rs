use super::segment_box;

#[test]
fn segment_hits_box_face() {
    let (f, n) = segment_box([-10.0, 0.0, 0.0], [10.0, 0.0, 0.0], [-1.0; 3], [1.0; 3]).unwrap();
    assert!((f - 0.45).abs() < 0.01, "{f}");
    assert_eq!(n, [-1.0, 0.0, 0.0]);
    let (f, n) = segment_box([0.0, 0.0, 10.0], [0.0, 0.0, -10.0], [-1.0; 3], [1.0; 3]).unwrap();
    assert!((f - 0.45).abs() < 0.01, "{f}");
    assert_eq!(n, [0.0, 0.0, 1.0]);
    assert!(segment_box([-10.0, 5.0, 0.0], [10.0, 5.0, 0.0], [-1.0; 3], [1.0; 3]).is_none());
    assert!(segment_box([0.0; 3], [10.0, 0.0, 0.0], [-1.0; 3], [1.0; 3]).is_none(), "inside start escapes");
}
