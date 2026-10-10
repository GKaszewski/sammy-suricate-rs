//! Decoded RGBA8 image shared by all texture-producing parsers.

#[derive(Clone, Debug)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// Row-major, top-to-bottom, 4 bytes per pixel.
    pub data: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, data: vec![0; (width * height * 4) as usize] }
    }

    /// Converts packed RGB565 pixels to RGBA.
    pub fn from_rgb565(width: u32, height: u32, px: impl IntoIterator<Item = u16>) -> Self {
        let data = px
            .into_iter()
            .flat_map(|p| {
                let r = ((p >> 11) & 0x1f) as u8;
                let g = ((p >> 5) & 0x3f) as u8;
                let b = (p & 0x1f) as u8;
                [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255]
            })
            .collect();
        Self { width, height, data }
    }

    /// Expands palettized pixels using a 256-entry RGB palette.
    pub fn from_indexed(width: u32, height: u32, px: &[u8], palette: &[[u8; 3]; 256]) -> Self {
        let data = px
            .iter()
            .flat_map(|&i| {
                let [r, g, b] = palette[i as usize];
                [r, g, b, 255]
            })
            .collect();
        Self { width, height, data }
    }
}
