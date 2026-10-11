//! Diamonds (`InitBonus3` / `Bonus3_Hit`, mm1.wdl:1200-1291) and colour key gems
//! (`InitKey` / `Key_Hit`, gems.wdl:40-350), picked up by Sammy's scan (id 1, radius
//! `BlobZoneScan`).

use formats::wmb::Object;
use crate::keys::KeyColor;
use crate::points::Points;
use crate::space::Quants;

/// A picked-up item shrinks ×0.8 per tick (`ScaleDownGem`) ...
pub const SHRINK_PER_TICK: f32 = 0.8;
/// ... and is gone below this scale.
pub const SHRINK_GONE: f32 = 0.05;
/// Diamonds worth these points also fly a "+50" / "+100" popup.
pub const POPUP_POINTS: [Points; 2] = [Points(50), Points(100)];
/// Ground snapping traces start this far above an item and use the narrow hull, whose
/// floor contact is 16 quants above the real floor.
pub const SNAP_START: f32 = 8.0;
pub const NARROW_FLOOR_OFFSET: f32 = 16.0;

/// A diamond.
#[derive(Clone, Copy, Debug)]
pub struct Diamond {
    /// Skill2.
    pub points: Points,
    /// Snap to the floor + `HeightGem` (Flag6 keeps the WED height).
    pub snap: bool,
}

impl Diamond {
    pub fn from_object(obj: &Object) -> Self {
        Self { points: Points(obj.skill(2) as i32), snap: !obj.flag(6) }
    }

    pub fn has_popup(&self) -> bool {
        POPUP_POINTS.contains(&self.points)
    }
}

/// A key gem placed in the level.
#[derive(Clone, Copy, Debug)]
pub struct KeySpec {
    /// Skill6.
    pub color: KeyColor,
    /// Skill5: surprise gem (colour climbs by up to this much).
    pub surprise: Option<f32>,
    /// Flag7: falls to its rest height and rides platforms.
    pub gravity: bool,
    /// Flag5 (with Flag7): rests at its WED height above the floor instead of 25.
    pub hover_from_wed: bool,
    /// Snap to the floor (Flag6 keeps the WED height).
    pub snap: bool,
}

impl KeySpec {
    pub fn from_object(obj: &Object) -> Self {
        Self {
            color: KeyColor(obj.skill(6) as u32),
            surprise: (obj.skill(5) != 0.0).then_some(obj.skill(5)),
            gravity: obj.flag(7),
            hover_from_wed: obj.flag(7) && obj.flag(5),
            snap: !obj.flag(6),
        }
    }
}

/// Sammy's pickup scan: within `radius` of his origin.
pub fn in_scan(sammy: [f32; 3], radius: Quants, item: [f32; 3]) -> bool {
    let radius = radius.get();
    (0..3).map(|i| (item[i] - sammy[i]).powi(2)).sum::<f32>().sqrt() <= radius
}
