#[allow(unused_imports)]
use super::*;

pub fn init_player(obj: &mut Object, entity: &mut EntityCommands) {
    obj.scale = [SCALE; 3];
    let mut p = sammy_core::player::Player::from_object(obj);
    // Debug: SAMMY_START="x,y,z[,pan]" overrides the spawn point.
    if let Ok(v) = std::env::var("SAMMY_START") {
        let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        let (mut pos, mut pan) = (p.pos, p.pan);
        if n.len() >= 3 {
            pos = [n[0], n[1], n[2]];
        }
        if let Some(&a) = n.get(3) {
            pan = a;
        }
        p = sammy_core::player::Player::at(pos, pan, p.speed, p.scan_radius);
    }
    entity.insert(Player(p));
}
