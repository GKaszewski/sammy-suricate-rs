use super::*;

/// Every effect spawns something and dies out within its documented lifetime.
#[test]
fn every_effect_spawns_and_expires() {
    let o = [0.0; 3];
    let effects: Vec<(&str, Box<dyn Fn(&mut Particles)>)> = vec![
        ("gem_trail", Box::new(move |ps| gem_trail(ps, o, 3))),
        ("gem_trail_master", Box::new(move |ps| gem_trail(ps, o, sammy_core::keys::MASTER_KEY))),
        ("gem_explode", Box::new(move |ps| gem_explode(ps, o, 4))),
        ("gem_catch", Box::new(move |ps| gem_catch(ps, o, 1))),
        ("gem_mix", Box::new(move |ps| gem_mix(ps, o, 2))),
        ("diamond", Box::new(move |ps| diamond_burst(ps, o))),
        ("dust", Box::new(move |ps| dust(ps, o, 3000.0, 10.0))),
        ("hit", Box::new(move |ps| hit_stars(ps, o, 10))),
        ("speedy", Box::new(move |ps| speedy(ps, o))),
        ("boost", Box::new(move |ps| boost_sparkle(ps, o))),
        ("teleport", Box::new(move |ps| teleport_sparkle(ps, o, false, 3))),
        ("teleport2", Box::new(move |ps| teleport_sparkle(ps, o, true, 3))),
        ("restart", Box::new(move |ps| restart_sparkle(ps, o))),
        ("smoke", Box::new(move |ps| tile_smoke(ps, o))),
        ("wasp_explode", Box::new(move |ps| wasp_explode(ps, o, 3, 2.0))),
        ("hive_burst", Box::new(move |ps| hive_burst(ps, o))),
        ("hive_trail", Box::new(move |ps| hive_wasp_trail(ps, o, 1))),
        ("get_life", Box::new(move |ps| get_life(ps, o, 2.0))),
        ("small_heart", Box::new(move |ps| small_heart(ps, o))),
        ("take_eye", Box::new(move |ps| take_eye(ps, o))),
    ];
    for (name, f) in effects {
        let mut ps = Particles::default();
        f(&mut ps);
        assert!(!ps.list.is_empty(), "{name} spawned nothing");
        for _ in 0..80 {
            ps.step();
        }
        assert!(ps.list.is_empty(), "{name} still has {} particles after 80 ticks", ps.list.len());
    }
    // Probabilistic emitters fire over enough ticks.
    let mut ps = Particles::default();
    for _ in 0..64 {
        wasp_trail(&mut ps, o, 1, 1.0);
        eye_idle(&mut ps, o);
        win_sparkle(&mut ps, o);
    }
    assert!(ps.list.len() > 40);
}
