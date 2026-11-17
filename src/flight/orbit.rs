use super::*;
use glam::{Mat3, Quat};

#[derive(Clone, Copy)]
pub struct CameraAdjustment {
    pub rotation: Quat,
    pub offset: Vec3,
    pub weight: f32,
    pub slope: f32,
}
impl Flight {
    pub fn velocity(&self) -> Vec3 {
        if self.space_attitude.is_some() {
            return self.axes()[1] * self.speed;
        }
        vec3(
            self.yaw.sin() * self.pitch.cos() * self.speed,
            -self.yaw.cos() * self.pitch.cos() * self.speed,
            self.vertical_speed,
        )
    }
    pub fn transform_frame(&mut self, rotation: Mat3, offset: Vec3) {
        let camera = Camera::new(self, 320, 170);
        let roll_before = self.screen_roll() * self.weights[2];
        self.transform_pose(rotation, offset);
        self.preserve_camera(camera, rotation, offset, roll_before);
    }
    pub fn settle_surface_attitude(&mut self) {
        if self.space_attitude.is_none() {
            return;
        }
        let camera = Camera::new(self, 320, 170);
        let roll_before = self.screen_roll() * self.weights[2];
        self.space_attitude = None;
        for wing in &mut self.wingmen {
            wing.space_attitude = None;
        }
        self.preserve_camera(camera, Mat3::IDENTITY, Vec3::ZERO, roll_before);
    }
    fn preserve_camera(&mut self, camera: Camera, rotation: Mat3, offset: Vec3, roll_before: f32) {
        self.camera_adjustment = None;
        let target = Camera::new(self, 320, 170);
        let roll = Quat::from_axis_angle(
            rotation * camera.forward,
            self.screen_roll() * self.weights[2] - roll_before,
        );
        let desired = Mat3::from_cols(
            roll * (rotation * camera.right),
            roll * (rotation * camera.up),
            -(rotation * camera.forward),
        );
        let baseline = Mat3::from_cols(target.right, target.up, -target.forward);
        self.camera_adjustment = Some(CameraAdjustment {
            rotation: Quat::from_mat3(&(desired * baseline.transpose())).normalize(),
            offset: rotation * camera.position + offset - target.position,
            weight: 1.0,
            slope: camera.slope - target.slope,
        });
    }
    fn transform_pose(&mut self, rotation: Mat3, offset: Vec3) {
        let [right, forward, _] = self.axes().map(|v| rotation * v);
        let velocity = rotation
            * vec3(
                self.yaw.sin() * self.speed * self.pitch.cos(),
                -self.yaw.cos() * self.speed * self.pitch.cos(),
                self.vertical_speed,
            );
        self.position = rotation * self.position + offset;
        self.follow = rotation * self.follow + offset;
        self.director_anchor = rotation * self.director_anchor + offset;
        self.yaw = forward.x.atan2(-forward.y);
        self.pitch = forward.z.clamp(-1.0, 1.0).asin();
        let base_right = vec3(self.yaw.cos(), self.yaw.sin(), 0.0);
        let base_up = forward.cross(base_right);
        self.bank = (-right.dot(base_up)).atan2(right.dot(base_right));
        self.space_attitude = if self.space {
            Some(
                Quat::from_mat3(&Mat3::from_cols(right, -forward, forward.cross(right)))
                    .normalize(),
            )
        } else {
            None
        };
        self.spin = 0.0;
        self.camera_yaw = -self.yaw;
        self.vertical_speed = velocity.z;
        self.rate = 0.0;
        self.pitch_rate = 0.0;
        self.loop_remaining = 0.0;
        self.roll_time = -1.0;
        self.portal_transit = None;
        self.portal_exit = None;
        self.portal_cooldown = 5.0;
        for trail in &mut self.trails {
            for point in trail {
                *point = rotation * *point + offset;
            }
        }
        for wing in &mut self.wingmen {
            wing.transform_pose(rotation, offset);
        }
    }
    pub fn cinematic_position(&mut self, position: Vec3, pitch: f32, velocity: Vec3) {
        self.position = position;
        self.pitch = pitch;
        self.pitch_rate = 0.0;
        self.vertical_speed = velocity.z;
        self.speed = velocity.length();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec2;
    #[test]
    fn frame_rotation_preserves_screen_projection_and_formation() {
        let world = World::generate(crate::world::MapKind::Space, 7);
        for view in 0..3 {
            let mut f = Flight::new(&world);
            f.view = view;
            f.weights = [0.0; 3];
            f.weights[view] = 1.0;
            f.yaw = 0.7;
            f.pitch = -0.4;
            f.bank = 0.27;
            let point = f.position + f.axes()[1] * 240.0 + f.axes()[0] * 20.0;
            let projection = |flight: &Flight, point: Vec3| {
                let p = Camera::new(flight, 320, 170).project(point).unwrap().0;
                let pivot = vec2(160.0, 170.0 * 0.43);
                let (s, c) = (-flight.screen_roll() * flight.weights[2]).sin_cos();
                let d = p - pivot;
                vec2(c * d.x - s * d.y, s * d.x + c * d.y) + pivot
            };
            let before = projection(&f, point);
            let axes = f.axes();
            let wing = f.wingmen[0].position - f.position;
            let rotation = Mat3::from_quat(Quat::from_rotation_x(1.2) * Quat::from_rotation_z(0.9));
            let offset = vec3(2000.0, 800.0, 600.0);
            f.transform_frame(rotation, offset);
            assert!(
                before.distance(projection(&f, rotation * point + offset)) < 0.003,
                "view {view}"
            );
            for (old, new) in axes.into_iter().zip(f.axes()) {
                assert!((rotation * old - new).length() < 0.001);
            }
            assert!((f.wingmen[0].position - f.position - rotation * wing).length() < 0.001);
            let point = rotation * point + offset;
            let before = projection(&f, point);
            f.space = false;
            f.settle_surface_attitude();
            assert!(
                before.distance(projection(&f, point)) < 0.003,
                "surface view {view}"
            );
            assert!(f.space_attitude.is_none());
        }
    }
}
