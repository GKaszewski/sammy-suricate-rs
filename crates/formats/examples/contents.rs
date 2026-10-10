//! Prints hull-2 contents along a line: `cargo run --example contents <file> x0 y0 x1 y1 z`.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let w = formats::wmb::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let n: Vec<f32> = a[2..7].iter().map(|s| s.parse().unwrap()).collect();
    for i in 0..=16 {
        let t = i as f32 / 16.0;
        let p = [n[0] + (n[2] - n[0]) * t, n[1] + (n[3] - n[1]) * t, n[4]];
        println!("{p:?} hull2={} hull1={}", w.point_contents(0, 2, p), w.point_contents(0, 1, p));
    }
}
