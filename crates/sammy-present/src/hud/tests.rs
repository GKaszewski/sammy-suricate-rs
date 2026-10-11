use super::*;

#[test]
fn digits_are_right_aligned_without_leading_zeros() {
    let game = GameState { score: 42, goal: 540, ..Default::default() };
    let q = Hud::default().quads(&game, [W, H], |_| [10.0, 20.0]);
    let score: Vec<&Quad> = q.iter().filter(|q| q.image == "boink24.pcx").collect();
    assert_eq!(score.len(), 2);
    assert_eq!(score[0].src.unwrap()[0], 40.0); // "4"
    assert_eq!(score[1].dst[0], 519.0 + 4.0 * 10.0);
}

#[test]
fn ui_scale_is_whole_above_640x480() {
    assert_eq!(ui_scale(960.0, 544.0, 0), 1.0);
    assert_eq!(ui_scale(1920.0, 1080.0, 0), 2.0);
    assert_eq!(ui_scale(1920.0, 1080.0, 1), 1.0);
    assert_eq!(ui_scale(320.0, 240.0, 0), 0.5);
}
