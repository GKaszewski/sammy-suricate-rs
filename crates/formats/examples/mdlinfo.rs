//! Prints an MDL's frame-0 bbox and scale: `cargo run --example mdlinfo <file>`.
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let m = formats::mdl::parse(&std::fs::read(&path).unwrap()).unwrap();
    let f = &m.frames[0];
    println!("{path}: bbox {:?}..{:?} frames {}", f.bbox_min, f.bbox_max, m.frames.len());
}
