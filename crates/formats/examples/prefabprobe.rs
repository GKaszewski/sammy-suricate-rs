//! Probes hull contents around a prefab's origin: `cargo run --example prefabprobe <file>`.
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let w = formats::wmb::parse(&std::fs::read(&path).unwrap()).unwrap();
    let m = &w.models[0];
    println!("{path}: bounds {:?}..{:?}", m.mins, m.maxs);
    for p in [[0.0, 0.0, 0.0], [0.0, 0.0, 300.0], [500.0, 0.0, 0.0], [0.0, 500.0, 50.0], [-500.0, -500.0, -500.0]] {
        println!("  {p:?}: hull2={} hull1={}", w.point_contents(0, 2, p), w.point_contents(0, 1, p));
    }
    let tr = w.trace(0, 2, [-400.0, 0.0, 40.0], [400.0, 0.0, 40.0]);
    println!("  trace x -400..400 @z40: {tr:?}");
    let tr = w.trace(0, 2, [0.0, -400.0, 40.0], [0.0, 400.0, 40.0]);
    println!("  trace y -400..400 @z40: {tr:?}");
}
