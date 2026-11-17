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
        let zoom = (0.19_f32 * 320.0 / (34.0 * 0.6)).max(1.0);
        let q = chase / (2.6 * 46.0 * 0.6 * (1.0 - 0.16 * flight.boost)) + cockpit / 64.0;
        let mut camera = Self {
            position: flight.follow,
            yaw: 0.0,
            pitch: 0.0,
            forward: Vec3::ZERO,
            right: Vec3::ZERO,
            up: Vec3::ZERO,
            q,
            scale: (chase * zoom.ln() + cockpit * 3.0_f32.ln()).exp() * VIEW_SCALE * w / 320.0,
            slope: (45.0 * top + 12.0 * chase).to_radians().tan(),
            center: vec2(w * 0.5, h * (0.5 * top + 0.64 * chase + 0.43 * cockpit)),
        };
        camera.orient(
            flight.camera_yaw + angle(-flight.yaw - flight.camera_yaw) * (chase + cockpit),
            flight.pitch * (chase + cockpit),
        );
        if flight.space || flight.space_attitude.is_some() {
            let [right, forward, up] = flight.axes();
            let base = glam::Quat::from_mat3(&glam::Mat3::from_cols(
                camera.right,
                -camera.forward,
                camera.up,
            ));
            let target = glam::Quat::from_mat3(&glam::Mat3::from_cols(right, -forward, up));
            let q = base.slerp(target, chase + cockpit);
            camera.right = q * Vec3::X;
            camera.forward = q * -Vec3::Y;
            camera.up = q * Vec3::Z;
            camera.yaw = (-camera.forward.x).atan2(-camera.forward.y);
            camera.pitch = camera.forward.z.clamp(-1.0, 1.0).asin();
            camera.slope = (45.0 * top).to_radians().tan();
        } else {
            camera.slope = (45.0 * top + 12.0 * chase * (1.0 - flight.orbital_thrust))
                .to_radians()
                .tan();
        }
        let target = flight.position + camera.forward * 64.0 + camera.up * 1.8;
        camera.position = flight.follow.lerp(target, cockpit);
        if flight.fixed_weight > 0.0001 {
            let delta = flight.position - flight.director_anchor;
            let yaw = (-delta.x).atan2(-delta.y);
            let pitch = delta.z.atan2(delta.truncate().length());
            let t = flight.fixed_weight;
            let forward = delta.normalize_or_zero();
            let position = flight.director_anchor + forward * 180.0;
            camera.position = camera.position.lerp(position, t);
            camera.orient(
                camera.yaw + angle(yaw - camera.yaw) * t,
                camera.pitch + angle(pitch - camera.pitch) * t,
            );
            camera.q += (1.0 / 180.0 - camera.q) * t;
            camera.slope *= 1.0 - t;
            camera.scale += (2.6 * w / 320.0 - camera.scale) * t;
            camera.center = camera.center.lerp(vec2(w * 0.5, h * 0.5), t);
        }
        if let Some(adjustment) = flight.camera_adjustment {
            let rotation = glam::Quat::IDENTITY.slerp(adjustment.rotation, adjustment.weight);
            camera.position += adjustment.offset * adjustment.weight;
            camera.slope += adjustment.slope * adjustment.weight;
            camera.right = rotation * camera.right;
            camera.up = rotation * camera.up;
            camera.forward = rotation * camera.forward;
            camera.yaw = (-camera.forward.x).atan2(-camera.forward.y);
            camera.pitch = camera.forward.z.clamp(-1.0, 1.0).asin();
        }
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
    pub fn rotate_yaw(&mut self, rotation: f32) {
        let q = glam::Quat::from_rotation_z(rotation);
        self.right = q * self.right;
        self.forward = q * self.forward;
        self.up = q * self.up;
        self.yaw -= rotation;
    }
    pub fn portal_frame(
        &self,
        portal: crate::world::portal::Portal,
    ) -> crate::world::portal::Portal {
        let (eye, ray) = self.ray(self.center);
        portal.facing(if self.q > 0.00001 {
            eye
        } else {
            portal.center - ray * 100.0
        })
    }

    pub fn enters_portal(
        &self,
        portal: crate::world::portal::Portal,
        from: Vec3,
        to: Vec3,
    ) -> bool {
        let movement = to - from;
        if movement.length_squared() < 0.000001 || (from - portal.center).dot(movement) >= 0.0 {
            return false;
        }
        let t = (-(from - portal.center).dot(movement) / movement.length_squared()).clamp(0.0, 1.0);
        let nearest = from.lerp(to, t);
        if nearest.distance_squared(portal.center) < (portal.radius * 0.7).powi(2) {
            return true;
        }
        if nearest.distance_squared(portal.center) > (portal.radius * 1.6).powi(2) {
            return false;
        }
        let Some((center, _, scale)) = self.project(portal.center) else {
            return false;
        };
        let Some((point, _, _)) = self.project(nearest) else {
            return false;
        };
        point.distance_squared(center) < (portal.radius * scale * 0.82).powi(2)
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
