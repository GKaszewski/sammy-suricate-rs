//! Debug screenshots: `SAMMY_SCREENSHOT=out.png` saves `out_<t>.png` at the times (seconds)
//! listed in `SAMMY_SHOT_SECS` (default "2.5,4"), then the game quits. Used for
//! automated visual checks (also on the Vita, where the path is e.g. `ux0:data/...`).

use glow::HasContext;

pub struct Shots {
    path: Option<String>,
    times: Vec<f32>,
    next: usize,
}

impl Shots {
    pub fn from_env() -> Self {
        let path = std::env::var("SAMMY_SCREENSHOT").ok();
        let times = std::env::var("SAMMY_SHOT_SECS").unwrap_or_else(|_| "2.5,4".into()).split(',').filter_map(|s| s.trim().parse().ok()).collect();
        Self { path, times, next: 0 }
    }

    /// A shot is due at `now` seconds.
    pub fn due(&self, now: f32) -> bool {
        self.path.is_some() && self.times.get(self.next).is_some_and(|t| now >= *t)
    }

    pub fn done(&self) -> bool {
        self.next >= self.times.len()
    }

    /// Saves the current back buffer.
    pub fn save(&mut self, gl: &glow::Context, w: u32, h: u32) {
        let (Some(path), Some(t)) = (&self.path, self.times.get(self.next)) else { return };
        let out = path.replace(".png", &format!("_{t}.png"));
        self.next += 1;
        let mut px = vec![0u8; (w * h * 4) as usize];
        unsafe { gl.read_pixels(0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut px))) };
        // GL rows are bottom-up.
        let row = (w * 4) as usize;
        let flipped: Vec<u8> = px.chunks_exact(row).rev().flatten().copied().collect();
        match std::fs::File::create(&out).map_err(|e| e.to_string()).and_then(|f| {
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().and_then(|mut wr| wr.write_image_data(&flipped)).map_err(|e| e.to_string())
        }) {
            Ok(()) => log::info!("saved {out}"),
            Err(e) => log::warn!("{out}: {e}"),
        }
    }
}
