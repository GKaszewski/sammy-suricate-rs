//! Packing face lightmaps into one atlas.

use alloc::{vec, vec::Vec};

use crate::image::RgbaImage;

/// Simple shelf packer; each rect gets a 1-luxel border to avoid bleeding.
pub(super) struct Atlas {
    pub(super) width: u32,
    pub(super) height: u32,
    /// Top-left of the inner (unpadded) rect per input.
    pub(super) slots: Vec<Option<[u32; 2]>>,
}

impl Atlas {
    pub(super) fn pack(sizes: &[Option<(u32, u32)>]) -> Option<Atlas> {
        let area: u64 = sizes.iter().flatten().map(|&(w, h)| ((w + 2) * (h + 2)) as u64).sum();
        if area == 0 {
            return None;
        }
        let max_w = sizes.iter().flatten().map(|s| s.0 + 2).max().unwrap_or(1);
        let width = (libm::sqrt(area as f64 * 1.15) as u32).next_power_of_two().max(max_w.next_power_of_two()).max(64);
        let mut order: Vec<usize> = (0..sizes.len()).filter(|&i| sizes[i].is_some()).collect();
        order.sort_by_key(|&i| core::cmp::Reverse((sizes[i].unwrap().1, sizes[i].unwrap().0)));
        let mut slots = vec![None; sizes.len()];
        let (mut x, mut y, mut shelf_h) = (0u32, 0u32, 0u32);
        for i in order {
            let (w, h) = sizes[i].unwrap();
            let (pw, ph) = (w + 2, h + 2);
            if x + pw > width {
                x = 0;
                y += shelf_h;
                shelf_h = 0;
            }
            slots[i] = Some([x + 1, y + 1]);
            x += pw;
            shelf_h = shelf_h.max(ph);
        }
        let height = (y + shelf_h).next_multiple_of(4);
        Some(Atlas { width, height, slots })
    }
}

/// Copies `src` to (x, y) and replicates its edge pixels into the 1-pixel border.
pub(super) fn blit_padded(dst: &mut RgbaImage, src: &RgbaImage, x: u32, y: u32) {
    let (sw, sh) = (src.width as i64, src.height as i64);
    for dy in -1..=sh {
        for dx in -1..=sw {
            let sx = dx.clamp(0, sw - 1) as usize;
            let sy = dy.clamp(0, sh - 1) as usize;
            let tx = (x as i64 + dx) as usize;
            let ty = (y as i64 + dy) as usize;
            let s = (sy * src.width as usize + sx) * 4;
            let d = (ty * dst.width as usize + tx) * 4;
            dst.data[d..d + 4].copy_from_slice(&src.data[s..s + 4]);
        }
    }
}
