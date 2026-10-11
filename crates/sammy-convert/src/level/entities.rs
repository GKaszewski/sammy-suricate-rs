//! Entity data: glTF rotations and JSON.

use super::*;

/// Acknex pan/tilt/roll (degrees) -> glTF quaternion `[x, y, z, w]`.
///
/// Native rotation = Rz(pan) * Ry(-tilt) * Rx(roll): pan turns counter-clockwise
/// seen from above, positive tilt pitches the nose (+X) up. The roll sign is
/// assumed (unverified). The quaternion's vector part is permuted like positions.
pub fn gltf_rotation([pan, tilt, roll]: [f32; 3]) -> [f32; 4] {
    let q = |axis: [f32; 3], deg: f32| {
        let h = deg.to_radians() * 0.5;
        let s = h.sin();
        [axis[0] * s, axis[1] * s, axis[2] * s, h.cos()]
    };
    let mul = |a: [f32; 4], b: [f32; 4]| {
        [
            a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
            a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
            a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
            a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
        ]
    };
    let n = mul(mul(q([0.0, 0.0, 1.0], pan), q([0.0, 1.0, 0.0], -tilt)), q([1.0, 0.0, 0.0], roll));
    let v = to_gltf([n[0], n[1], n[2]]);
    [v[0], v[1], v[2], n[3]]
}

pub(super) fn entity_to_json(e: &Entity) -> Value {
    match e {
        Entity::Position { origin, angles, name } => json!({
            "type": 1, "kind": "position", "name": name, "origin": origin, "angles": angles,
        }),
        Entity::Light { origin, color, range, flags } => json!({
            "type": 2, "kind": "light", "origin": origin, "color_percent": color, "range": range, "flags": flags,
        }),
        Entity::Object(o) => {
            let (names, unknown_bits) = wmb::entity_flag_names(o.flags);
            let mut v = json!({
                "type": o.kind,
                "kind": "entity",
                "name": o.name,
                "file": o.file,
                "action": o.action,
                "origin": o.origin,
                "angles": o.angles,
                "scale": o.scale,
                "skills": o.skills,
                "flags": o.flags,
                "flag_names": names,
                "flags_unknown_bits": unknown_bits,
                "ambient": o.ambient,
            });
            for (k, val) in [
                ("albedo", o.albedo.map(|x| json!(x))),
                ("path", o.path.map(|x| json!(x))),
                ("entity2", o.entity2.map(|x| json!(x))),
                ("material", o.material.as_ref().map(|x| json!(x))),
                ("string1", o.string1.as_ref().map(|x| json!(x))),
                ("string2", o.string2.as_ref().map(|x| json!(x))),
            ] {
                if let Some(val) = val {
                    v[k] = val;
                }
            }
            v
        }
        Entity::Info { azimuth, elevation, raw } => json!({
            "type": 5, "kind": "info", "sun_azimuth": azimuth, "sun_elevation": elevation, "raw_tail": hex(raw),
        }),
        Entity::Unknown { kind, raw } => json!({ "type": kind, "kind": "unknown", "raw": hex(raw) }),
    }
}

pub(super) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
