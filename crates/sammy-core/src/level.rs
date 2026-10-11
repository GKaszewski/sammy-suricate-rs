//! Level objects: how each WMB entity's action presents it.

/// How a level object is presented, as decided by its action's initialisation.
#[derive(Clone, Debug, Default)]
pub struct Presentation {
    /// Render unlit, behind the world (sky domes).
    pub sky: bool,
    /// Script sets `my.invisible` immediately (config/marker entities).
    pub hidden: bool,
    /// Help sign: own fading material, drawn on top, screen-aligned.
    pub title: bool,
    /// Invisible logic entity that still runs (platform engines).
    pub hidden_but_alive: bool,
    /// Model skin (A4 1-based `my.skin`).
    pub skin: Option<u32>,
    /// Starting animation scene and speed (% of the scene per tick).
    pub anim: Option<(&'static str, f32)>,
    /// Sprites: snap to the floor below at `floor + half height + offset` (`InitArbre`, `OnGround`).
    pub ground: Option<f32>,
    /// Sprites: non-zero angles are drawn oriented (fixed in the world) instead of camera-facing.
    pub oriented: bool,
    /// Sprites: `facing` (screen-parallel).
    pub facing: bool,
    /// Opacity in percent (`ent_alphaset`), below 100 = blended.
    pub alpha: Option<f32>,
    /// Models: fixed frame (A4 1-based, fractions floored), no animation.
    pub frame: Option<f32>,
    /// Models: `overlay` colour key (palette index 0 / black is transparent).
    pub overlay: bool,
    /// Models: `flare` (drawn additively).
    pub flare: bool,
    /// Models: solid bounding box collider.
    pub solid_box: bool,
}

/// Map entities the player walks through (pickups, triggers).
pub fn is_passable(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    ["initrestart", "initbonusvie", "initeyeoftruth", "initsmallheart", "inithive"].contains(&a.as_str())
}

/// Config and marker entities: their actions only read skills into globals
/// and hide the entity (mm1.wdl InitGame*, InitCamera*, ...).
pub fn is_marker(action: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "initgame", "initcamera", "initextraballes", "initnextraballes", "initpublic", "initinvisible",
        "initset", "initprotection", "initlionskills", "initrangemdl",
    ];
    // The extro's `InitSetPovSyn` / `InitSetAnimPlayer` / `InitSetFinExtro` are live triggers.
    let extro_trigger = ["initsetpovsyn", "initsetanimplayer", "initsetfinextro"].contains(&action);
    PREFIXES.iter().any(|p| action.starts_with(p)) && !extro_trigger
}

