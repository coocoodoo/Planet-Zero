use glam::{Vec2, Vec3, vec2, vec3};
use std::collections::VecDeque;
use std::f32::consts::TAU;

use crate::world::World;

pub const MODEL_SCALE: f32 = 0.5;
pub const VIEW_SCALE: f32 = 0.82;
pub const MIN_CLEARANCE: f32 = 36.0;

pub fn angle(value: f32) -> f32 {
    value - TAU * (value / TAU).round()
}

pub fn ease(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

#[derive(Default)]
pub struct Controls {
    pub bank: Option<f32>,
    pub turn: f32,
    pub climb: f32,
    pub boost: bool,
    pub manual: bool,
}

#[derive(Clone)]
pub struct Flight {
    pub wingmen: Vec<Flight>,
    pub wing_count: usize,
    pub demo: bool,
    pub hud: bool,
    pub vehicle: crate::vehicle::Vehicle,
    pub weather: crate::weather::Weather,
    pub demo_time: f32,
    pub position: Vec3,
    pub yaw: f32,
    pub bank: f32,
    pub pitch: f32,
    pub rate: f32,
    pub boost: f32,
    pub speed: f32,
    pub time: f32,
    pub spin: f32,
    pub view: usize,
    pub weights: [f32; 3],
    pub follow: Vec3,
    pub camera_yaw: f32,
    pub trails: VecDeque<[Vec3; 2]>,
    pub spread: f32,
    pub exploded: bool,
    roll_time: f32,
    trail_time: f32,
}

impl Flight {
    pub fn new(world: &World) -> Self {
        let position = vec3(
            world.spawn.0,
            world.spawn.1,
            110.0_f32.max(world.height(world.spawn.0, world.spawn.1) + 70.0),
        );
        let mut flight = Self {
            wingmen: Vec::new(),
            wing_count: 2,
            demo: false,
            hud: true,
            vehicle: crate::vehicle::Vehicle::Falcon,
            weather: crate::weather::Weather::Clear,
            demo_time: 0.0,
            position,
            yaw: 0.55,
            bank: 0.0,
            pitch: 0.0,
            rate: 0.0,
            boost: 0.0,
            speed: 92.0,
            time: 0.0,
            spin: 0.0,
            view: 0,
            weights: [1.0, 0.0, 0.0],
            follow: position,
            camera_yaw: -0.33,
            trails: VecDeque::with_capacity(160),
            spread: 1.0,
            exploded: false,
            roll_time: -1.0,
            trail_time: 0.0,
        };
        for side in [-1.0, 1.0] {
            let mut wing = flight.clone();
            wing.wingmen.clear();
            wing.wing_count = 0;
            wing.position = flight.local(vec3(side * 45.0, 116.0, 3.0));
            flight.wingmen.push(wing);
        }
        flight
    }

    pub fn change_world(&mut self, world: &World) {
        let weather = self.weather;
        let hud = self.hud;
        let vehicle = self.vehicle;
        let (view, weights, demo, wing_count) =
            (self.view, self.weights, self.demo, self.wing_count);
        *self = Self::new(world);
        self.view = view;
        self.weights = weights;
        self.demo = demo;
        self.wing_count = wing_count;
        self.weather = weather;
        self.hud = hud;
        self.vehicle = vehicle;
    }

    pub fn set_demo(&mut self, enabled: bool) {
        self.demo = enabled;
        self.demo_time = 0.0;
        if enabled {
            self.view = 1;
        }
    }

    pub fn demonstration(&self) -> &'static str {
        [
            "CLIMB",
            "BARREL ROLLS",
            "FIGURE EIGHT",
            "DIVE / PULL UP",
            "FORMATION SPLIT",
            "SPIRAL CLIMB",
            "FAST PASS",
        ][(self.demo_time / 8.0) as usize % 7]
    }

    fn demo_controls(&mut self, dt: f32) -> Controls {
        self.demo_time += dt;
        let stage = (self.demo_time / 8.0) as usize % 7;
        let phase = self.demo_time % 8.0;
        if phase < dt || self.demo_time <= dt {
            self.view = if stage == 2 || stage == 4 { 0 } else { 1 };
        }
        let wave = (phase * std::f32::consts::PI / 8.0).sin();
        let (turn, altitude, boost) = match stage {
            0 => (0.2, 220.0, true),
            1 => {
                if [1.0, 4.5].iter().any(|&t| phase >= t && phase - dt < t) {
                    self.roll();
                }
                (0.15, 220.0, phase < 5.5)
            }
            2 => ((phase * TAU / 8.0).sin() * 0.95, 180.0, false),
            3 => (wave * 0.4, 220.0 - 120.0 * wave * wave, phase > 4.0),
            4 => (-0.15, 175.0, false),
            5 => (0.85, 140.0 + phase * 14.0, true),
            _ => (0.22, 95.0, true),
        };
        Controls {
            turn,
            climb: ((altitude - self.position.z) * 1.4 / (self.speed * 0.55)).clamp(-1.0, 1.0),
            boost,
            manual: true,
            bank: None,
        }
    }

    pub fn roll(&mut self) {
        if self.roll_time < 0.0 {
            self.roll_time = 0.0;
        }
    }

    pub fn axes(&self) -> [Vec3; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sr, cr) = (self.bank + self.spin).sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let right = vec3(cy * cr, sy * cr, -sr);
        let up = vec3(cy * sr, sy * sr, cr);
        let forward = vec3(sy, -cy, 0.0);
        [right, forward * cp + up * sp, up * cp - forward * sp]
    }

    pub fn local(&self, point: Vec3) -> Vec3 {
        let [right, forward, up] = self.axes();
        let point = point * self.vehicle.shape();
        self.position + (right * point.x + forward * point.y + up * point.z) * MODEL_SCALE
    }

    pub fn step(&mut self, dt: f32, controls: &Controls, world: &World) {
        let scripted = if self.demo {
            Some(self.demo_controls(dt))
        } else {
            None
        };
        let controls = scripted.as_ref().unwrap_or(controls);
        self.time += dt;
        let boost_goal = f32::from(controls.boost);
        self.boost += (boost_goal - self.boost) * ease(4.0, dt);
        self.speed = self.vehicle.speed() * (1.0 + self.boost * 0.8);
        let turn = if controls.manual {
            controls.turn
        } else {
            0.62 * (self.time * 0.31 + 1.3).sin() + 0.38 * (self.time * 0.73 + 0.4).sin()
        };
        self.rate += (turn.clamp(-1.0, 1.0) * self.vehicle.turn() - self.rate) * ease(2.6, dt);
        self.yaw += self.rate * dt;
        self.bank += (controls
            .bank
            .unwrap_or(self.rate / self.vehicle.turn() * 52.0_f32.to_radians())
            - self.bank)
            * ease(5.0, dt);
        self.position.x += self.yaw.sin() * self.speed * dt;
        self.position.y -= self.yaw.cos() * self.speed * dt;
        let mut climb = if controls.manual {
            controls.climb * self.speed * 0.55
        } else {
            (110.0 - self.position.z) * 0.6
        };
        let forward = vec3(self.yaw.sin(), -self.yaw.cos(), 0.0);
        let right = vec3(forward.y, -forward.x, 0.0);
        let floor = clearance_floor(world, self.position);
        let mut ahead_floor = floor;
        for seconds in [0.25, 0.6, 1.0] {
            for side in [-1.0, 0.0, 1.0] {
                let probe = self.position + forward * self.speed * seconds + right * side * 12.0;
                ahead_floor = ahead_floor.max(world.height(probe.x, probe.y) + MIN_CLEARANCE);
            }
        }
        // 前方地形触发爬升，下降速度随剩余离地空间收敛
        climb = climb.max((ahead_floor + 12.0 - self.position.z) * 1.8);
        climb = climb.clamp(-self.speed * 0.55, self.speed * 0.8);
        let old_altitude = self.position.z;
        self.position.z = (old_altitude + climb * dt).clamp(floor, 300.0);
        let actual_climb = (self.position.z - old_altitude) / dt.max(0.001);
        self.pitch += ((actual_climb / self.speed).clamp(-0.5, 0.6) - self.pitch) * ease(4.0, dt);
        if self.roll_time >= 0.0 {
            self.roll_time += dt / 1.0;
            let t = self.roll_time.min(1.0);
            self.spin = t * t * (3.0 - 2.0 * t) * TAU;
            if self.roll_time >= 1.0 {
                self.roll_time = -1.0;
                self.spin = 0.0;
            }
        }
        self.follow += (self.position - self.follow) * ease(4.5, dt);
        self.camera_yaw += angle(-self.yaw * 0.6 - self.camera_yaw) * ease(1.5, dt);
        for i in 0..3 {
            self.weights[i] += (f32::from(i == self.view) - self.weights[i]) * ease(3.2, dt);
        }
        self.spread += ((if self.exploded { 5.0 } else { 1.0 }) - self.spread) * ease(5.0, dt);
        for (i, wing) in self.wingmen.iter_mut().enumerate() {
            let side = if i == 0 { -1.0 } else { 1.0 };
            let (sy, cy) = self.yaw.sin_cos();
            let split = if self.demo && (self.demo_time / 8.0) as usize % 7 == 4 {
                (std::f32::consts::PI * (self.demo_time % 8.0) / 8.0)
                    .sin()
                    .powi(2)
            } else {
                0.0
            };
            let goal = self.position
                + vec3(cy, sy, 0.0) * side * (27.0 + split * 55.0)
                + vec3(sy, -cy, 0.0) * 70.0
                + vec3(0.0, 0.0, 2.0 + (self.time * 1.3 + i as f32 * 2.0).sin());
            wing.position += (goal - wing.position) * ease(4.5, dt);
            wing.position.z = wing.position.z.max(clearance_floor(world, wing.position));
            wing.yaw += angle(self.yaw - wing.yaw) * ease(4.0, dt);
            wing.bank += (self.bank - side * split * 0.7 - wing.bank) * ease(3.2, dt);
            wing.pitch += (self.pitch - wing.pitch) * ease(3.0, dt);
            wing.weather = self.weather;
            wing.vehicle = self.vehicle;
            wing.time = self.time + i as f32 * 0.17;
            wing.boost = self.boost;
            wing.speed = self.speed;
            wing.rate = self.rate;
            wing.spin = self.spin;
            wing.trail_time += dt;
            if wing.trail_time >= 0.025 {
                wing.trail_time %= 0.025;
                wing.trails.push_front([
                    wing.local(vec3(-15.5, -7.7, -1.0)),
                    wing.local(vec3(15.5, -7.7, -1.0)),
                ]);
                wing.trails.truncate(100);
            }
        }
        self.trail_time += dt;
        if self.trail_time >= 0.025 {
            self.trail_time %= 0.025;
            self.trails.push_front([
                self.local(vec3(-15.5, -7.7, -1.0)),
                self.local(vec3(15.5, -7.7, -1.0)),
            ]);
            self.trails.truncate(160);
        }
    }
}

fn clearance_floor(world: &World, position: Vec3) -> f32 {
    let mut height = 0.0_f32;
    for y in [-8.0, 0.0, 8.0] {
        for x in [-8.0, 0.0, 8.0] {
            height = height.max(world.height(position.x + x, position.y + y));
        }
    }
    height + MIN_CLEARANCE
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
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
        let focal = w * 0.6 * VIEW_SCALE;
        let target = flight.position + vec3(flight.yaw.sin() * 64.0, -flight.yaw.cos() * 64.0, 1.8);
        let position = flight.follow.lerp(target, cockpit);
        let q = chase / (2.6 * 46.0 * 0.6 * (1.0 - 0.16 * flight.boost)) + cockpit / 64.0;
        let scale = (chase * zoom.ln() + cockpit * ((w * 0.6) / 64.0).ln()).exp() * VIEW_SCALE;
        let slope = (45.0 * top + 12.0 * chase).to_radians().tan() - flight.pitch * 0.5 * chase;
        Self {
            position,
            yaw: flight.camera_yaw + angle(-flight.yaw - flight.camera_yaw) * (chase + cockpit),
            q,
            scale,
            slope: slope.max((2.0 - position.z) * q),
            center: vec2(
                w * 0.5,
                h * (0.5 * top + 0.64 * chase + 0.43 * cockpit)
                    + cockpit * focal * flight.pitch.tan(),
            ),
        }
    }

    pub fn project(&self, point: Vec3) -> Option<(Vec2, f32, f32)> {
        // q 为零时使用等比例投影，增大 q 后远处的物体逐渐缩小
        let delta = point - self.position;
        let (s, c) = self.yaw.sin_cos();
        let depth = -(delta.x * s + delta.y * c);
        let denominator = 1.0 + self.q * depth;
        if denominator < 0.12 {
            return None;
        }
        let scale = self.scale / denominator;
        Some((
            self.center + vec2(delta.x * c - delta.y * s, -(delta.z + self.slope * depth)) * scale,
            depth,
            scale,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CLOUD_SIZE, SIZE};

    fn terrain() -> World {
        World {
            heights: vec![0; SIZE * SIZE],
            materials: vec![0; SIZE * SIZE],
            clouds: vec![0; CLOUD_SIZE * CLOUD_SIZE],
            spawn: (256.0, 350.0),
            kind: crate::world::MapKind::Islands,
            seed: 7,
        }
    }

    #[test]
    fn descent_keeps_the_formation_above_terrain() {
        let mut world = terrain();
        world.heights[..220 * SIZE].fill(40);
        let mut flight = Flight::new(&world);
        flight.position.z = 60.0;
        flight.yaw = 0.0;
        let controls = Controls {
            climb: -1.0,
            boost: true,
            manual: true,
            ..Default::default()
        };
        for _ in 0..240 {
            let before = flight.position.z;
            flight.step(1.0 / 60.0, &controls, &world);
            assert!(
                flight.position.z - world.height(flight.position.x, flight.position.y)
                    >= MIN_CLEARANCE
            );
            assert!((flight.position.z - before).abs() < 3.0);
            for wing in &flight.wingmen {
                assert!(
                    wing.position.z - world.height(wing.position.x, wing.position.y)
                        >= MIN_CLEARANCE
                );
            }
        }
    }

    #[test]
    fn demonstration_completes_a_safe_cycle_and_releases_control() {
        let world = terrain();
        let mut flight = Flight::new(&world);
        flight.set_demo(true);
        let mut seen = std::collections::HashSet::new();
        let mut rolled = false;
        let mut min_altitude = f32::INFINITY;
        let mut max_altitude = 0.0_f32;
        for _ in 0..3600 {
            flight.step(1.0 / 60.0, &Controls::default(), &world);
            seen.insert(flight.demonstration());
            rolled |= flight.spin > 3.0;
            min_altitude = min_altitude.min(flight.position.z);
            max_altitude = max_altitude.max(flight.position.z);
            assert!(flight.position.is_finite() && flight.position.z >= MIN_CLEARANCE);
        }
        assert_eq!(seen.len(), 7);
        assert!(rolled && max_altitude - min_altitude > 100.0);
        flight.set_demo(false);
        let controls = Controls {
            turn: -1.0,
            manual: true,
            ..Default::default()
        };
        for _ in 0..180 {
            flight.step(1.0 / 60.0, &controls, &world);
        }
        assert!(flight.rate < -1.0 && !flight.demo);
    }

    #[test]
    fn ground_protection_levels_out_the_aircraft() {
        let world = terrain();
        let mut flight = Flight::new(&world);
        let controls = Controls {
            climb: -1.0,
            manual: true,
            ..Default::default()
        };
        for _ in 0..600 {
            flight.step(1.0 / 60.0, &controls, &world);
        }
        assert!(flight.position.z >= MIN_CLEARANCE);
        assert!(flight.pitch.abs() < 0.01);
    }
}
