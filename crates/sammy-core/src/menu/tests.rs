use super::*;

#[test]
fn codes() {
    assert_eq!(Choice::from_code(1), Choice::Play);
    assert_eq!(Choice::from_code(3), Choice::Grid(View::Jump));
    assert_eq!(Choice::from_code(105), Choice::Level(LevelSlot(5)));
    assert_eq!(Choice::from_code(0), Choice::Decor);
}

#[test]
fn locked_icons_are_ghosts() {
    let save = SaveGame::default();
    assert_eq!(look(Choice::Level(LevelSlot(1)), 100.0, &save), Look::Shown { alpha: 100.0 });
    assert_eq!(look(Choice::Level(LevelSlot(2)), 100.0, &save), Look::Locked { alpha: 50.0, dark: false });
    assert_eq!(look(Choice::Play, 100.0, &save), Look::Hidden);
}
