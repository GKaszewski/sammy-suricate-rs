use super::*;

#[test]
fn table() {
    assert_eq!(file_for(LevelSlot(10)), Some("10012.wmb"));
    assert_eq!(slot_for("EXTRO.WMB"), Some(EXTRO_SLOT));
    assert_eq!(next_slot(LevelSlot(9)), LevelSlot(10));
    assert_eq!(next_slot(LevelSlot(12)), LevelSlot(17));
    assert_eq!(next_slot(LevelSlot(44)), EXTRO_SLOT);
}
