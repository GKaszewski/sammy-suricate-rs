fn main() {
    // On the Vita, SDL2 (linked through pkg-config) uses vitaGL, which is C++ and needs
    // a few libraries its .pc file does not list; they go last on the link line.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("vita") {
        for l in ["SceShaccCgExt", "taihen_stub", "SceShaccCg_stub", "stdc++", "m"] {
            println!("cargo:rustc-link-arg=-l{l}");
        }
    }
}
