use super::*;

/// Real-time cost of each song at a few qualities (needs the game files and a
/// SoundFont): `cargo test --release -p sammy-platform --features music -- --ignored --nocapture`.
#[test]
#[ignore]
fn render_cost() {
    let storage = Storage::user();
    let Some(dir) = crate::Settings::load(&storage).game_dir else { return eprintln!("skip: no game dir") };
    let Ok(files) = GameFiles::open(std::path::Path::new(&dir)) else { return };
    let Some((name, data)) = find_soundfont(&storage, &files) else { return eprintln!("skip: no SoundFont") };
    let font = parse_soundfont(&data).unwrap();
    eprintln!("{name}");
    for q in [Quality::default(), Quality { rate: 22050, polyphony: 32, effects: false }, Quality { rate: 22050, polyphony: 24, effects: false }] {
        let secs = 20.0;
        let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
        let mut worst: f64 = 0.0;
        let mut total = 0.0;
        for file in FILES {
            let mut song = Song::new(&font, &files, file, true, q).unwrap();
            let t = std::time::Instant::now();
            for _ in 0..(secs * q.rate as f32 / BLOCK as f32) as usize {
                song.render(&mut l, &mut r);
            }
            let cost = t.elapsed().as_secs_f64() / secs as f64;
            worst = worst.max(cost);
            total += cost;
        }
        eprintln!("{q:?}: mean {:.2}% worst {:.2}% of real time", total / FILES.len() as f64 * 100.0, worst * 100.0);
    }
}
