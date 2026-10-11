//! The frame's camera, culling, and entity orientation.

use super::*;

/// The camera of a frame.
pub struct Camera {
    pub view_proj: Mat4,
    pub pos: Vec3,
    /// World axes of the view: forward, left, up.
    pub axes: Mat3,
    /// Vertical field of view (radians) and aspect ratio.
    pub fov: f32,
    pub aspect: f32,
    /// Frustum planes (normal inward, `n·p + d >= 0` inside).
    pub(super) planes: [Vec4; 6],
}

impl Camera {
    pub fn from_pose(pose: &CameraPose, arc: f32, aspect: f32) -> Self {
        let (pos, target) = match *pose {
            CameraPose::LookAt { pos, target } => (Vec3::from(pos), Vec3::from(target)),
            CameraPose::Angles { pos, angles } => {
                let (p, t) = (angles.pan.radians(), angles.tilt.radians());
                let pos = Vec3::from(pos);
                (pos, pos + Vec3::new(t.cos() * p.cos(), t.cos() * p.sin(), t.sin()))
            }
        };
        let forward = (target - pos).normalize_or(Vec3::X);
        let left = Vec3::Z.cross(forward).normalize_or(Vec3::Y);
        let up = forward.cross(left);
        // `arc` is horizontal on the original 4:3 screen (Hor+ beyond it).
        let fov = 2.0 * ((arc.to_radians() / 2.0).tan() / aspect.min(ASPECT_43)).atan();
        let view = Mat4::look_at_rh(pos, target, Vec3::Z);
        let proj = Mat4::perspective_rh_gl(fov, aspect, NEAR, FAR);
        let view_proj = proj * view;
        // Gribb-Hartmann: each plane is the w row plus or minus another row.
        let rows = [0, 1, 2, 3].map(|i| view_proj.row(i));
        let planes = [rows[3] + rows[0], rows[3] - rows[0], rows[3] + rows[1], rows[3] - rows[1], rows[3] + rows[2], rows[3] - rows[2]].map(|p| p / p.truncate().length());
        Self { view_proj, pos, axes: Mat3::from_cols(forward, left, up), fov, aspect, planes }
    }

    /// Whether a sphere is at least partly in view.
    pub fn sees(&self, (c, r): Sphere) -> bool {
        self.planes.iter().all(|p| p.truncate().dot(c) + p.w >= -r)
    }
}

/// A4 euler angles (pan about Z, tilt nose-up, roll) as a rotation in native axes.
pub fn rotation(angles: impl Into<sammy_core::angle::Angles>) -> Quat {
    let [pan, tilt, roll] = angles.into().to_array();
    Quat::from_rotation_z(pan.to_radians()) * Quat::from_rotation_y(-tilt.to_radians()) * Quat::from_rotation_x(roll.to_radians())
}

pub(super) fn transform(b: &Body, rot: Quat) -> Mat4 {
    Mat4::from_scale_rotation_translation(Vec3::from(b.scale), rot, Vec3::from(b.pos.get()))
}

/// How a sprite is turned this frame.
pub(super) fn sprite_rotation(b: &Body, facing: SpriteFacing, cam: &Camera) -> Quat {
    match facing {
        // Turn about the vertical axis to face the camera (A4 sprites).
        SpriteFacing::Billboard => {
            let d = Vec3::from(b.pos.get()) - cam.pos;
            Quat::from_rotation_z(d.y.atan2(d.x))
        }
        SpriteFacing::Screen => Quat::from_mat3(&cam.axes),
        // The quad faces -X; an oriented sprite faces along its angles.
        SpriteFacing::Oriented => rotation(b.angles) * Quat::from_rotation_z(std::f32::consts::PI),
    }
}
