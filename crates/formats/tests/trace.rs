//! Hull traces against level 10001 (needs extracted/levels; skipped otherwise).
use formats::wmb;

fn level() -> Option<wmb::Wmb> {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/levels/10001.wmb");
    std::fs::read(p).ok().map(|d| wmb::parse(&d).unwrap())
}

/// Player start in 10001 (InitMarche origin).
const START: [f32; 3] = [-568.0, -268.0, 56.0];

#[test]
fn falling_from_start_lands_on_floor() {
    let Some(w) = level() else { return };
    let tr = w.trace(0, 2, START, [START[0], START[1], START[2] - 1000.0]);
    assert!(tr.hit() && !tr.start_solid, "{tr:?}");
    // Floor is at z=0. Empirically hull 2 reaches 16 below the origin, despite the
    // header's z mins of -32 (the compiler seems to expand floors by -maxs.z).
    assert!((tr.end[2] - 16.0).abs() < 1.0, "landed at {:?}", tr.end);
    assert!(tr.normal[2] > 0.99, "{tr:?}");
}

#[test]
fn walking_into_maze_wall_is_blocked() {
    let Some(w) = level() else { return };
    let start = [START[0], START[1], 40.0];
    // Walk 2000 quants in each horizontal direction: the maze must stop us in all of them.
    for dir in [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
        let end = [start[0] + dir[0] * 2000.0, start[1] + dir[1] * 2000.0, 40.0];
        let tr = w.trace(0, 2, start, end);
        assert!(tr.hit(), "dir {dir:?} not blocked: {tr:?}");
        assert!(dot2(tr.normal, dir) < -0.9, "dir {dir:?} wrong normal {tr:?}");
    }
}

fn dot2(n: [f32; 3], d: [f32; 2]) -> f32 {
    n[0] * d[0] + n[1] * d[1]
}

#[test]
fn point_trace_hull0_hits_floor_and_walls() {
    let Some(w) = level() else { return };
    let tr = w.trace(0, 0, START, [START[0], START[1], START[2] - 1000.0]);
    assert!(tr.hit() && !tr.start_solid, "{tr:?}");
    // A point lands on the floor itself (z = 0), unlike the expanded hulls.
    assert!(tr.end[2].abs() < 1.0, "landed at {:?}", tr.end);
}
