use super::*;

#[test]
fn table() {
    assert_eq!(file_for(10), Some("10012.wmb"));
    assert_eq!(slot_for("EXTRO.WMB"), Some(EXTRO_SLOT));
    assert_eq!(next_slot(9), 10);
    assert_eq!(next_slot(12), 17);
    assert_eq!(next_slot(44), EXTRO_SLOT);
}
