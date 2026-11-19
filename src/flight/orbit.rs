use super::*;
use glam::{Mat3, Quat};

#[derive(Clone, Copy)]
pub struct CameraAdjustment {
    pub rotation: Quat,
    pub offset: Vec3,
    pub weight: f32,
    pub slope: f32,
    pub scale: f32,
    pub q: f32,
}
impl Flight {
    pub fn velocity(&self) -> Vec3 {
        if self.space {
            return self.axes()[1] * self.speed + self.frame_drift;
        }
        (self.axes()[1] * self.speed).with_z(self.vertical_speed) + self.frame_drift
    }
    pub fn transform_frame(&mut self, rotation: Mat3, offset: Vec3, space: bool) {
        let camera = Camera::new(self, 320, 170);
        let roll_before = self.screen_roll() * self.weights[2];
        self.transform_pose(rotation, offset, space);
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
    pub(crate) fn preserve_camera(
        &mut self,
        camera: Camera,
        rotation: Mat3,
        offset: Vec3,
        roll_before: f32,
    ) {
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
            scale: camera.scale,
            q: camera.q,
        });
    }
    pub(crate) fn transform_pose(&mut self, rotation: Mat3, offset: Vec3, space: bool) {
        let free_attitude = self.space || self.space_attitude.is_some();
        let [right, forward, _] = self.axes().map(|v| rotation * v);
        let velocity = rotation * self.velocity();
        self.space = space;
        self.position = rotation * self.position + offset;
        self.follow = rotation * self.follow + offset;
        self.director_anchor = rotation * self.director_anchor + offset;
        self.yaw = forward.x.atan2(-forward.y);
        self.pitch = forward.z.clamp(-1.0, 1.0).asin();
        self.view_altitude = self.position.z;
        let base_right = vec3(self.yaw.cos(), self.yaw.sin(), 0.0);
        let base_up = forward.cross(base_right);
        self.bank = angle((-right.dot(base_up)).atan2(right.dot(base_right)) - self.spin);
        self.space_attitude = if self.space || free_attitude {
            Some(
                Quat::from_mat3(&Mat3::from_cols(right, -forward, forward.cross(right)))
                    .normalize()
                    * Quat::from_rotation_y(-self.spin),
            )
        } else {
            None
        };
        self.camera_yaw = -self.yaw;
        self.vertical_speed = velocity.z;
        self.frame_drift = Vec3::ZERO;
        self.frame_drift = velocity - self.velocity();
        self.portal_transit = None;
        self.portal_exit = None;
        self.portal_cooldown = 5.0;
        for trail in &mut self.trails {
            for point in trail {
                *point = rotation * *point + offset;
            }
        }
        for wing in &mut self.wingmen {
            wing.transform_pose(rotation, offset, space);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec2;
    #[test]
    fn camera_adjustment_preserves_projection_at_every_resolution() {
        let world = World::generate(crate::world::MapKind::Space, 7);
        let mut flight = Flight::new(&world);
        for view in 0..3 {
            flight.weights = [0.0; 3];
            flight.weights[view] = 1.0;
            flight.transform_frame(Mat3::from_rotation_z(0.7), Vec3::ZERO, true);
            for weight in [1.0, 0.5, 0.0] {
                flight.camera_adjustment.as_mut().unwrap().weight = weight;
                let native = Camera::new(&flight, 320, 170);
                let point = native.position + native.forward * 200.0 + native.right * 20.0;
                let expected = native.project(point).unwrap().0;
                for scale in [2, 4, 8] {
                    let camera = Camera::new(&flight, 320 * scale, 170 * scale);
                    let actual = camera.project(point).unwrap().0 / scale as f32;
                    assert!(actual.distance(expected) < 0.001);
                }
            }
        }
    }
    #[test]
    fn frame_rotation_preserves_screen_projection_and_formation() {
        let world = World::generate(crate::world::MapKind::Space, 7);
        for (view, look) in
            (0..3).flat_map(|v| [glam::Vec2::ZERO, vec2(0.2, -0.3)].map(|look| (v, look)))
        {
            let mut f = Flight::new(&world);
            f.view = view;
            f.camera_look = look;
            f.spin = 0.4;
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
            f.transform_frame(rotation, offset, true);
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
                "surface view {view}, look {look:?}, before {before:?}, after {:?}",
                projection(&f, point)
            );
            assert!(f.space_attitude.is_none());
        }
    }
}
