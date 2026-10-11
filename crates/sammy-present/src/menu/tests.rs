use super::*;

#[test]
fn focus_moves_toward_the_nearest_item_that_way() {
    let items = [(1, 10, [100.0, 100.0]), (2, 11, [200.0, 100.0]), (3, 12, [100.0, 300.0]), (4, 13, [400.0, 120.0])];
    assert_eq!(pad_focus(&items, Some(1), Some([1, 0]), false, [640.0, 480.0]), Some(2));
    assert_eq!(pad_focus(&items, Some(1), Some([0, -1]), false, [640.0, 480.0]), Some(3));
    assert_eq!(pad_focus(&items, Some(1), Some([-1, 0]), false, [640.0, 480.0]), Some(1));
    assert_eq!(pad_focus(&items, None, Some([1, 0]), false, [400.0, 200.0]), Some(2));
}
