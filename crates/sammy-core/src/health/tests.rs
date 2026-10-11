use super::*;
use crate::keys::KeyColor;

fn run(kind: DeathKind, game: &mut GameState) -> Vec<DeathEvent> {
    let mut d = Dying::new(kind);
    let mut out = Vec::new();
    let victim = Victim { pos: [0.0; 3], pan: 0.0, vel: [0.0; 3] };
    for _ in 0..200 {
        d.step(game, None, victim, Ticks(1.0), &mut out);
    }
    out
}

#[test]
fn tile_death_respawns_at_start() {
    let mut game = GameState { lives: 2, key: KeyColor(4), start: Some(([1.0, 2.0, 3.0], 90.0)), hearts: 0.0, ..Default::default() };
    let out = run(DeathKind::Tile, &mut game);
    assert_eq!(out[0], DeathEvent::Started);
    assert!(matches!(out[1], DeathEvent::DropKey(Launch { color: KeyColor(4), .. })));
    assert!(out.contains(&DeathEvent::Respawn(Some(([1.0, 2.0, 3.0], 90.0)))));
    assert_eq!(out.last(), Some(&DeathEvent::Finished));
    assert_eq!((game.lives, game.key, game.hearts), (1, KeyColor::NONE, MAX_HEARTS));
}

#[test]
fn last_life_is_game_over() {
    let mut game = GameState { lives: 1, ..Default::default() };
    let out = run(DeathKind::Kaputt, &mut game);
    assert!(out.contains(&DeathEvent::Sound("Kaputt_son")) && out.contains(&DeathEvent::GameOver));
    assert!(!out.contains(&DeathEvent::Finished));
    assert!(game.lost);
}
