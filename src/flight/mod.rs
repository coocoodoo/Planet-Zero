use glam::{Vec3, vec3};
mod camera;
mod safety;
#[cfg(test)]
mod tests;
pub use camera::Camera;
use safety::{clearance_floor, flight_floor};
use std::collections::VecDeque;
use std::f32::consts::TAU;

use crate::world::World;

pub const MODEL_SCALE: f32 = 0.5;
pub const VIEW_SCALE: f32 = 0.82;
pub const MIN_CLEARANCE: f32 = 18.0;

pub fn angle(value: f32) -> f32 {
    value - TAU * (value / TAU).round()
}

pub fn ease(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

#[derive(Default)]
pub struct Controls {
    pub bank: Option<f32>,
    pub aerobatic: bool,
    pub turn: f32,
    pub climb: f32,
    pub boost: bool,
    pub manual: bool,
}

#[derive(Clone)]
pub struct Flight {
    pub wingmen: Vec<Flight>,
    pub space: bool,
    pub warp_flash: f32,
    portal_cooldown: f32,
    pub wing_count: usize,
    pub demo: bool,
    pub hud: bool,
    pub vehicle: crate::content::vehicles::Vehicle,
    pub weather: crate::content::weather::Weather,
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
    roll_direction: f32,
    pitch_rate: f32,
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
            space: world.kind == crate::world::MapKind::Space,
            warp_flash: 0.0,
            portal_cooldown: 0.0,
            wing_count: 2,
            demo: false,
            hud: true,
            vehicle: crate::content::vehicles::Vehicle::Falcon,
            weather: crate::content::weather::Weather::Clear,
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
            roll_direction: 1.0,
            pitch_rate: 0.0,
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

    pub fn traverse_portal(&mut self, world: &World, previous: Vec3) -> bool {
        if world.portals.len() != 2 || self.portal_cooldown > 0.0 {
            return false;
        }
        for (i, entry) in world.portals.iter().enumerate() {
            if entry.crossed(previous, self.position) {
                let entry = entry.facing(previous);
                let exit = world.portals[1 - i];
                let rotation = entry.rotation_to(exit);
                self.position = entry.transform(exit, self.position);
                let distance = (self.position - exit.center).dot(exit.normal());
                let offset = exit.normal() * (exit.radius * 0.8 + 8.0 - distance).max(0.0);
                self.position += offset;
                self.follow = entry.transform(exit, self.follow) + offset;
                self.yaw += rotation;
                self.camera_yaw -= rotation;
                self.trails.clear();
                for wing in &mut self.wingmen {
                    wing.position = entry.transform(exit, wing.position) + offset;
                    wing.follow = entry.transform(exit, wing.follow) + offset;
                    wing.yaw += rotation;
                    wing.camera_yaw -= rotation;
                    wing.trails.clear();
                }
                self.portal_cooldown = 0.8;
                self.warp_flash = 0.45;
                return true;
            }
        }
        false
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
            self.view = 1;
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
            aerobatic: false,
        }
    }

    pub fn roll(&mut self) {
        self.roll_toward(1.0);
    }
    pub fn roll_toward(&mut self, direction: f32) {
        if self.roll_time < 0.0 {
            self.roll_time = 0.0;
            self.roll_direction = direction.signum();
        }
    }

    pub fn axes(&self) -> [Vec3; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sr, cr) = (self.bank + self.spin).sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let forward = vec3(sy * cp, -cy * cp, sp);
        let right = vec3(cy, sy, 0.0);
        let up = vec3(-sy * sp, cy * sp, cp);
        [right * cr - up * sr, forward, right * sr + up * cr]
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
        self.portal_cooldown = (self.portal_cooldown - dt).max(0.0);
        self.warp_flash = (self.warp_flash - dt).max(0.0);
        if world.kind.vacuum() {
            self.weather = crate::content::weather::Weather::Clear;
        }
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
        let manual_attitude = controls.manual && controls.aerobatic;
        if manual_attitude {
            let input = controls.climb.clamp(-1.0, 1.0);
            let neutral = (self.pitch / std::f32::consts::PI).round() * std::f32::consts::PI;
            let target_rate = if input.abs() > 0.04 {
                input * 1.25
            } else {
                angle(neutral - self.pitch) * 2.5
            };
            self.pitch_rate += (target_rate - self.pitch_rate) * ease(5.0, dt);
            self.pitch = angle(self.pitch + self.pitch_rate * dt);
        }
        let forward = self.axes()[1];
        let horizontal = if manual_attitude {
            forward
        } else {
            vec3(self.yaw.sin(), -self.yaw.cos(), 0.0)
        };
        if manual_attitude && controls.climb.abs() > 0.35 {
            let lift = safety::loop_lift(
                world,
                self.position,
                self.yaw,
                self.pitch,
                controls.climb * 1.25,
                self.speed,
            );
            self.position.z += lift * ease(3.5, dt);
        }
        let proposed = self.position + horizontal * self.speed * dt;
        let (floor, ahead_climb) = flight_floor(world, self.position, horizontal, self.speed);
        let mut climb = if manual_attitude {
            forward.z * self.speed
        } else if controls.manual {
            controls.climb * self.speed * 0.55
        } else {
            (110.0 - self.position.z) * 0.6
        };
        climb = climb.max(ahead_climb).clamp(-self.speed, self.speed);
        let old_altitude = self.position.z;
        self.position.x = proposed.x;
        self.position.y = proposed.y;
        self.position.z = (old_altitude + climb * dt)
            .clamp(floor.max(clearance_floor(world, self.position)), 900.0);
        let actual_climb = (self.position.z - old_altitude) / dt.max(0.001);
        if !manual_attitude {
            self.pitch += angle((actual_climb / self.speed).clamp(-0.7, 0.9).asin() - self.pitch)
                * ease(4.0, dt);
        }
        if self.roll_time >= 0.0 {
            self.roll_time += dt / 1.0;
            let t = self.roll_time.min(1.0);
            self.spin = self.roll_direction * t * t * (3.0 - 2.0 * t) * TAU;
            if self.roll_time >= 1.0 {
                self.roll_time = -1.0;
                self.spin = 0.0;
            }
        }
        self.follow += (self.position - self.follow) * ease(4.5, dt);
        self.camera_yaw += angle(-self.yaw * 0.6 - self.camera_yaw) * ease(1.5, dt);
        let camera_top = if self.demo && matches!((self.demo_time / 8.0) as usize % 7, 2 | 4) {
            (std::f32::consts::PI * (self.demo_time % 8.0) / 8.0)
                .sin()
                .powi(2)
                * 0.9
        } else {
            0.0
        };
        let target_weights = if self.demo {
            [camera_top, 1.0 - camera_top, 0.0]
        } else {
            std::array::from_fn(|i| f32::from(i == self.view))
        };
        for (weight, target) in self.weights.iter_mut().zip(target_weights) {
            *weight += (target - *weight) * ease(if self.demo { 2.0 } else { 3.2 }, dt);
        }
        self.spread += ((if self.exploded { 5.0 } else { 1.0 }) - self.spread) * ease(5.0, dt);
        let [formation_right, formation_forward, formation_up] = self.axes();
        for (i, wing) in self.wingmen.iter_mut().enumerate() {
            let side = if i == 0 { -1.0 } else { 1.0 };

            let split = if self.demo && (self.demo_time / 8.0) as usize % 7 == 4 {
                (std::f32::consts::PI * (self.demo_time % 8.0) / 8.0)
                    .sin()
                    .powi(2)
            } else {
                0.0
            };
            let goal = self.position
                + formation_right * side * (27.0 + split * 55.0)
                + formation_forward * 70.0
                + formation_up * (2.0 + (self.time * 1.3 + i as f32 * 2.0).sin());
            wing.position += (goal - wing.position) * ease(4.5, dt);
            wing.position.z = wing.position.z.max(clearance_floor(world, wing.position));
            wing.yaw += angle(self.yaw - wing.yaw) * ease(4.0, dt);
            wing.bank += (self.bank - side * split * 0.7 - wing.bank) * ease(3.2, dt);
            wing.pitch += angle(self.pitch - wing.pitch) * ease(3.0, dt);
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
