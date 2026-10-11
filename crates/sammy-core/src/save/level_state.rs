//! A level's progress in the save.

use super::*;

/// `LevelsInfo[n]`; stored as its WDL number (0 / 1 / 2).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(into = "u8", try_from = "u8")]
pub enum LevelState {
    /// No such level (or its file is missing).
    #[default]
    Absent,
    /// Present, not reached yet.
    Locked,
    /// Reached or won: playable.
    Unlocked,
}

impl LevelState {
    /// From a `LevelsExist` value of an original save.
    pub fn from_wdl(v: f32) -> Self {
        if v >= 2.0 {
            Self::Unlocked
        } else if v != 0.0 {
            Self::Locked
        } else {
            Self::Absent
        }
    }
}

impl From<LevelState> for u8 {
    fn from(s: LevelState) -> u8 {
        s as u8
    }
}

impl TryFrom<u8> for LevelState {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Absent),
            1 => Ok(Self::Locked),
            2 => Ok(Self::Unlocked),
            _ => Err(format!("bad level state {v}")),
        }
    }
}
