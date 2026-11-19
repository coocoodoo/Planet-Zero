use super::*;
use glam::{Mat3, Quat};

impl Flight {
    pub fn attitude(&self) -> Quat {
        self.space_attitude.unwrap_or_else(|| {
            let [right, forward, up] = self.axes();
            Quat::from_mat3(&Mat3::from_cols(right, -forward, up)).normalize()
        })
    }
    pub fn screen_roll(&self) -> f32 {
        if self.space || self.space_attitude.is_some() {
            0.0
        } else {
            self.bank + self.spin
        }
    }
    pub(super) fn sync_space_angles(&mut self) {
        let Some(q) = self.space_attitude else { return };
        let right = q * Vec3::X;
        let forward = q * -Vec3::Y;
        self.pitch = forward.z.clamp(-1.0, 1.0).asin();
        if forward.truncate().length_squared() > 1e-8 {
            self.yaw = forward.x.atan2(-forward.y);
        }
        let base_right = vec3(self.yaw.cos(), self.yaw.sin(), 0.0);
        let base_up = forward.cross(base_right);
        self.bank = (-right.dot(base_up)).atan2(right.dot(base_right));
    }
    pub(super) fn steer_attitude(
        &mut self,
        dt: f32,
        controls: &Controls,
        freedom: f32,
        pitch_delta: f32,
        bank_delta: f32,
    ) {
        let mut q = self.attitude();
        if let Some(direction) = controls.direction {
            let target = direction.normalize_or_zero();
            let forward = q * -Vec3::Y;
            let angle = forward.dot(target).clamp(-1.0, 1.0).acos();
            if angle > 0.0001 {
                let rotation = Quat::from_rotation_arc(forward, target);
                q = Quat::IDENTITY.slerp(rotation, (1.4 * dt / angle).min(1.0)) * q;
            }
            self.rate = 0.0;
            self.pitch_rate = 0.0;
        } else {
            self.rate +=
                (controls.turn.clamp(-1.0, 1.0) * self.vehicle.turn() - self.rate) * ease(5.0, dt);
            let loop_active = self.loop_remaining > 0.0;
            let limit = NORMAL_PITCH + freedom * (std::f32::consts::FRAC_PI_2 - NORMAL_PITCH);
            let input = controls.climb.clamp(-1.0, 1.0);
            let leveling = angle(input * limit - self.pitch) * 2.0 * (1.0 - freedom);
            let goal = if controls.heading_only {
                0.0
            } else if loop_active {
                self.loop_direction * LOOP_RATE
            } else if controls.pitch_target.is_some() {
                leveling
            } else {
                leveling + input * 1.15 * freedom
            };
            if freedom == 1.0 && goal == 0.0 {
                self.pitch_rate = 0.0;
            } else {
                self.pitch_rate += (goal - self.pitch_rate) * ease(5.0, dt);
            }
            let mut pitch = self.pitch_rate * dt;
            if loop_active {
                pitch = self.loop_direction * pitch.abs().min(self.loop_remaining);
                self.loop_remaining = (self.loop_remaining - pitch.abs()).max(0.0);
            } else if !controls.heading_only {
                pitch += pitch_delta * freedom;
            }
            if self.space {
                q = q * Quat::from_rotation_z(self.rate * dt) * Quat::from_rotation_x(-pitch);
            } else {
                // 地表俯仰沿当地水平轴调整，到达垂直方向后保持
                q = Quat::from_rotation_z(self.rate * dt) * q;
                let yaw = self.yaw + self.rate * dt;
                let right = vec3(yaw.cos(), yaw.sin(), 0.0);
                if !loop_active {
                    let allowed = limit.max(self.pitch.abs() - dt * 0.8);
                    pitch = (self.pitch + pitch).clamp(-allowed, allowed) - self.pitch;
                }
                q = Quat::from_axis_angle(right, -pitch) * q;
            }
            q *= Quat::from_rotation_y(bank_delta * freedom);
            if freedom < 1.0 && !loop_active && !controls.heading_only {
                self.space_attitude = Some(q.normalize());
                self.sync_space_angles();
                let bank = controls
                    .bank
                    .unwrap_or(self.rate / self.vehicle.turn() * 52.0_f32.to_radians());
                q *= Quat::from_rotation_y(
                    angle(bank - self.bank) * ease(5.0 * (1.0 - freedom), dt),
                );
            }
        }
        self.space_attitude = Some(q.normalize());
        self.sync_space_angles();
    }
    pub(super) fn step_space(&mut self, dt: f32, controls: &Controls) {
        let base = self.vehicle.speed() * (1.0 + self.boost * 0.8 + self.orbital_thrust * 3.0);
        // 悬停模式油门为 0，速度归零并保持位置
        let throttle = controls.throttle.unwrap_or(1.0).clamp(0.0, 1.0);
        // 油门归零时快速制动
        let rate = if throttle > 0.0 { 4.0 } else { 10.0 };
        self.speed += (base * throttle - self.speed) * ease(rate, dt);
        let velocity = self.axes()[1] * self.speed + self.frame_drift;
        self.vertical_speed = velocity.z;
        self.position += velocity * dt;
    }
    pub fn orbital_altitude(&self) -> f32 {
        if self.space {
            crate::world::orbit::Body::ALL
                .into_iter()
                .map(|body| body.altitude(self.position))
                .fold(f32::INFINITY, f32::min)
        } else {
            self.position.z
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_rotation_crosses_both_poles_and_keeps_the_camera_aligned() {
        let world = World::generate(crate::world::MapKind::Space, 1999);
        let mut f = Flight::new(&world);
        f.yaw = 0.0;
        f.position.z = 3000.0;
        f.follow = f.position;
        let mut high = -1.0_f32;
        let mut low = 1.0_f32;
        let mut total = 0.0;
        for _ in 0..420 {
            let before = f.axes()[1];
            f.step(
                1.0 / 60.0,
                &Controls {
                    manual: true,
                    climb: 1.0,
                    ..Default::default()
                },
                &world,
            );
            let forward = f.axes()[1];
            high = high.max(forward.z);
            low = low.min(forward.z);
            total += before.dot(forward).clamp(-1.0, 1.0).acos();
            for view in [1, 2] {
                f.weights = [0.0; 3];
                f.weights[view] = 1.0;
                let camera = Camera::new(&f, 320, 170);
                assert!(camera.ray(camera.center).1.normalize().dot(forward) > 0.999);
                assert!(camera.up.dot(camera.forward).abs() < 0.001);
            }
        }
        assert!(high > 0.9999 && low < -0.9999);
        assert!(total > TAU);
        for _ in 0..90 {
            f.step(
                1.0 / 60.0,
                &Controls {
                    manual: true,
                    ..Default::default()
                },
                &world,
            );
        }
        let q = f.attitude();
        for _ in 0..90 {
            f.step(
                1.0 / 60.0,
                &Controls {
                    manual: true,
                    ..Default::default()
                },
                &world,
            );
        }
        assert!(q.dot(f.attitude()).abs() > 0.9999);
    }
    #[test]
    fn space_translation_uses_forward_velocity_without_altitude_clamping() {
        let world = World::generate(crate::world::MapKind::Space, 1999);
        let mut f = Flight::new(&world);
        f.space_attitude = Some(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        f.position.z = 25000.0;
        let before = f.position;
        f.step(
            0.1,
            &Controls {
                manual: true,
                ..Default::default()
            },
            &world,
        );
        assert!((f.position - before).truncate().length() < 0.001);
        assert!((f.position.z - before.z - f.speed * 0.1).abs() < 0.01);
        f.space_attitude = Some(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
        let before = f.position;
        f.step(
            0.1,
            &Controls {
                manual: true,
                ..Default::default()
            },
            &world,
        );
        assert!((f.position.z - before.z + f.speed * 0.1).abs() < 0.01);
    }
}

#[cfg(test)]
mod bank_tests {
    use super::*;
    #[test]
    fn bank_input_rolls_the_spacecraft() {
        let world = World::generate(crate::world::MapKind::Space, 1999);
        let mut f = Flight::new(&world);
        f.space_attitude = Some(Quat::IDENTITY);
        f.position.z = 3000.0;
        f.follow = f.position;
        f.step(
            1.0 / 60.0,
            &Controls {
                manual: true,
                bank: Some(0.0),
                ..Default::default()
            },
            &world,
        );
        let controls = Controls {
            manual: true,
            aerobatic: true,
            bank: Some(65.0_f32.to_radians()),
            ..Default::default()
        };
        for _ in 0..40 {
            f.step(1.0 / 60.0, &controls, &world);
        }
        assert!(f.bank > 1.0, "bank {}", f.bank);
        assert!(f.axes()[1].z.abs() < 0.01);
    }
}
