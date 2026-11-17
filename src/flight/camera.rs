use super::*;
use glam::{Vec2, vec2};

#[derive(Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub q: f32,
    pub scale: f32,
    pub slope: f32,
    pub center: Vec2,
}
impl Camera {
    pub fn new(flight: &Flight, width: usize, height: usize) -> Self {
        let [top, chase, cockpit] = flight.weights;
        let w = width as f32;
        let h = height as f32;
        let zoom = (0.19 * w / (34.0 * 0.6)).max(1.0);
        let q = chase / (2.6 * 46.0 * 0.6 * (1.0 - 0.16 * flight.boost)) + cockpit / 64.0;
        let mut camera = Self {
            position: flight.follow,
            yaw: 0.0,
            pitch: 0.0,
            forward: Vec3::ZERO,
            right: Vec3::ZERO,
            up: Vec3::ZERO,
            q,
            scale: (chase * zoom.ln() + cockpit * ((w * 0.6) / 64.0).ln()).exp() * VIEW_SCALE,
            slope: (45.0 * top + 12.0 * chase).to_radians().tan(),
            center: vec2(w * 0.5, h * (0.5 * top + 0.64 * chase + 0.43 * cockpit)),
        };
        camera.orient(
            flight.camera_yaw + angle(-flight.yaw - flight.camera_yaw) * (chase + cockpit),
            flight.pitch * (chase + cockpit),
        );
        let target = flight.position + camera.forward * 64.0 + camera.up * 1.8;
        camera.position = flight.follow.lerp(target, cockpit);
        camera
    }
    pub fn orient(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let forward = vec3(-sy, -cy, 0.0);
        self.right = vec3(cy, -sy, 0.0);
        self.forward = forward * cp + Vec3::Z * sp;
        self.up = Vec3::Z * cp - forward * sp;
    }
    pub fn project(&self, point: Vec3) -> Option<(Vec2, f32, f32)> {
        let delta = point - self.position;
        let depth = delta.dot(self.forward);
        let denominator = 1.0 + self.q * depth;
        if denominator < 0.12 {
            return None;
        }
        let scale = self.scale / denominator;
        Some((
            self.center
                + vec2(
                    delta.dot(self.right),
                    -(delta.dot(self.up) + self.slope * depth),
                ) * scale,
            depth,
            scale,
        ))
    }
    pub fn ray(&self, p: Vec2) -> (Vec3, Vec3) {
        let offset = (p - self.center) / self.scale;
        if self.q > 0.00001 {
            (
                self.position - self.forward / self.q + self.up * (self.slope / self.q),
                self.forward
                    + self.right * (offset.x * self.q)
                    + self.up * (-offset.y * self.q - self.slope),
            )
        } else {
            (
                self.position + self.right * offset.x - self.up * offset.y,
                self.forward - self.up * self.slope,
            )
        }
    }
}
