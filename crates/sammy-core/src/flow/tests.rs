use super::*;

#[test]
fn wins_route_to_next_menu_or_extro() {
    let mut save = SaveGame::default();
    assert_eq!(after_win(1, &save), "10002.wmb");
    assert_eq!(after_win(12, &save), MENU_FILE);
    save.validate(28);
    save.validate(44);
    assert_eq!(after_win(44, &save), "extro.wmb");
}

#[test]
fn a_win_is_recorded_once() {
    let mut save = SaveGame::default();
    let mut f = LevelFlow { slot: Some(1), file: "10001.wmb".into(), phase: Phase::Playing, intro_ticks: 0.0 };
    let game = GameState { won: true, score: 500, ..Default::default() };
    assert!(f.step(FlowInput { t: Ticks(1.0), ..Default::default() }, &game, &mut save).saved);
    assert_eq!(f.phase, Phase::Won(0.0));
    assert!(save.playable(2));
    let step = f.step(FlowInput { t: Ticks(20.0), ..Default::default() }, &game, &mut save);
    assert!(!step.saved && step.load.is_none());
    let step = f.step(FlowInput { t: Ticks(1.0), any: true, ..Default::default() }, &game, &mut save);
    assert_eq!(step.load.as_deref(), Some("10002.wmb"));
}
