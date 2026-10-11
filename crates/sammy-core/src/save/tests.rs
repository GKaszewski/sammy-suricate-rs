use super::*;

#[test]
fn win_unlocks_next_and_keeps_best() {
    let mut s = SaveGame::default();
    assert!(s.playable(1) && !s.playable(2) && s.playable(17));
    s.record_game(1, 300, None, true, Some(2));
    s.record_game(1, 100, Some([1.0, 2.0, 3.0]), false, None);
    assert!(s.playable(2));
    assert_eq!(s.best_scores[1], 300);
    assert_eq!(s.checkpoints[1], Some([1.0, 2.0, 3.0]));
    assert_eq!(s.last_level, 1);
    // Registered: 21 opens with 20.
    assert!(!s.playable(21));
    s.validate(20);
    assert!(s.playable(21));
}

#[test]
fn level_states_stay_numbers_in_json() {
    let s = SaveGame::default();
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"level_state\":[0,2,1,"));
    let back: SaveGame = serde_json::from_str(&json).unwrap();
    assert_eq!(back.state(1), LevelState::Unlocked);
    assert_eq!(back.state(2), LevelState::Locked);
    assert_eq!(back.state(13), LevelState::Absent);
}
