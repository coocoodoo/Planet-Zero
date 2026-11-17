use glam::{Vec3, vec3};
mod camera;
mod director;
mod orbit;
mod portals;
pub use orbit::CameraAdjustment;
mod safety;
pub use portals::PortalTransit;
#[cfg(test)]
mod tests;
pub use camera::Camera;
use safety::{clearance_floor, flight_floor};
use std::collections::VecDeque;
use std::f32::consts::TAU;

use crate::world::World;

pub const MODEL_SCALE: f32 = 0.5;
pub const VIEW_SCALE: f32 = 0.82;
pub const MIN_CLEARANCE: f32 = 28.0;
const LOOP_RATE: f32 = 1.25;
const NORMAL_PITCH: f32 = 0.55;

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
    pub orbital: bool,
    pub manual: bool,
}

#[derive(Clone)]
pub struct Flight {
    pub orbit_quiet: bool,
    pub surface: Option<crate::world::orbit::SurfaceWarp>,
    pub planet_maps: [Option<std::sync::Arc<crate::world::orbit::SurfaceMap>>; 2],
    pub sky_rotation: glam::Quat,
    pub camera_adjustment: Option<CameraAdjustment>,
    pub wingmen: Vec<Flight>,
    pub space: bool,
    pub warp_flash: f32,
    pub portal_transit: Option<PortalTransit>,
    portal_cooldown: f32,
    portal_exit: Option<crate::world::portal::Portal>,
    portal_exit_pending: bool,
    pub wing_count: usize,
    pub portal_wait: f32,
    rejoining: bool,
    camera_offset: f32,
    pub demo: bool,
    pub hud: bool,
    pub faceted: bool,
    pub vehicle: crate::content::vehicles::Vehicle,
    pub weather: crate::content::weather::Weather,
    pub demo_time: f32,
    pub demo_target: Option<crate::world::portal::Portal>,
    pub fixed_weight: f32,
    pub director_anchor: Vec3,
    pub director_shot: usize,
    pub position: Vec3,
    pub yaw: f32,
    pub bank: f32,
    pub pitch: f32,
    pub rate: f32,
    pub boost: f32,
    pub orbital_thrust: f32,
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
    vertical_speed: f32,
    loop_remaining: f32,
    loop_direction: f32,
    aerobatic_ground: f32,
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
            orbit_quiet: false,
            surface: None,
            planet_maps: [None, None],
            sky_rotation: glam::Quat::IDENTITY,
            camera_adjustment: None,
            wingmen: Vec::new(),
            space: world.kind == crate::world::MapKind::Space,
            warp_flash: 0.0,
            portal_transit: None,
            portal_cooldown: 0.0,
            portal_exit: None,
            portal_exit_pending: false,
            wing_count: 2,
            portal_wait: 0.0,
            rejoining: false,
            camera_offset: 0.0,
            demo: false,
            hud: true,
            faceted: true,
            vehicle: crate::content::vehicles::Vehicle::Falcon,
            weather: crate::content::weather::Weather::Clear,
            demo_time: 0.0,
            demo_target: None,
            fixed_weight: 0.0,
            director_anchor: position,
            director_shot: 1,
            position,
            yaw: 0.55,
            bank: 0.0,
            pitch: 0.0,
            rate: 0.0,
            boost: 0.0,
            orbital_thrust: 0.0,
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
            vertical_speed: 0.0,
            loop_remaining: 0.0,
            loop_direction: 1.0,
            aerobatic_ground: world
                .ceiling
                .max(world.obstacles.iter().copied().max().unwrap_or(0))
                as f32,
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
        let faceted = self.faceted;
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
        self.faceted = faceted;
        self.vehicle = vehicle;
    }

    pub fn enter_region(&mut self, world: &World) {
        self.orbit_quiet = false;
        self.surface = None;
        self.sky_rotation = glam::Quat::IDENTITY;
        self.camera_adjustment = None;
        self.space = world.kind == crate::world::MapKind::Space;
        for wing in &mut self.wingmen {
            wing.enter_region(world);
        }
        self.aerobatic_ground = world
            .ceiling
            .max(world.obstacles.iter().copied().max().unwrap_or(0))
            as f32;
    }
    pub fn translate(&mut self, offset: Vec3) {
        self.position += offset;
        self.follow += offset;
        self.director_anchor += offset;
        for trail in &mut self.trails {
            for point in trail {
                *point += offset;
            }
        }
        for wing in &mut self.wingmen {
            wing.translate(offset);
        }
    }
    pub fn loop_altitude(&self) -> f32 {
        self.aerobatic_ground + self.vehicle.speed() * 1.8 * 2.0 / LOOP_RATE + 64.0
    }
    pub fn can_loop(&self) -> bool {
        self.space || self.position.z >= self.loop_altitude()
    }
    pub fn loop_toward(&mut self, direction: f32) -> bool {
        if !self.can_loop() || self.loop_remaining > 0.0 || direction == 0.0 {
            return false;
        }
        self.loop_direction = direction.signum();
        self.loop_remaining = TAU;
        true
    }
    pub fn can_warp(&self) -> bool {
        self.portal_cooldown <= 0.0
    }
    pub fn set_demo(&mut self, enabled: bool) {
        self.demo = enabled;
        self.demo_time = 0.0;
        self.demo_target = None;
        if !enabled {
            self.fixed_weight = 0.0;
        }
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
        if let Some(portal) = self.demo_target {
            let forward = -portal.normal();
            let along = (portal.center - self.position).dot(forward);
            let target = if along < 120.0 {
                portal.center + forward * 250.0
            } else {
                portal.center
            };
            let delta = target - self.position;
            return Controls {
                turn: (angle(delta.x.atan2(-delta.y) - self.yaw) * 1.5 / self.vehicle.turn())
                    .clamp(-0.65, 0.65),
                climb: ((portal.center.z - self.position.z) * 1.5 / (self.speed * 0.55))
                    .clamp(-1.0, 1.0),
                manual: true,
                ..Controls::default()
            };
        }
        let stage = (self.demo_time / 8.0) as usize % 7;
        let phase = self.demo_time % 8.0;
        if phase < dt || self.demo_time <= dt {
            self.view = 1;
        }
        let wave = (phase * std::f32::consts::PI / 8.0).sin();
        let (turn, altitude, boost) = match stage {
            0 => (0.2, (self.aerobatic_ground + 150.0).max(220.0), true),
            1 => {
                if [1.0, 4.5].iter().any(|&t| phase >= t && phase - dt < t) {
                    self.roll();
                }
                (
                    0.15,
                    (self.aerobatic_ground + 150.0).max(220.0),
                    phase < 5.5,
                )
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
            orbital: false,
        }
    }

    pub fn roll(&mut self) -> bool {
        self.roll_toward(1.0)
    }
    pub fn roll_toward(&mut self, direction: f32) -> bool {
        if self.roll_time >= 0.0 || (!self.space && self.position.z < self.aerobatic_ground + 80.0)
        {
            return false;
        }
        self.roll_time = 0.0;
        self.roll_direction = direction.signum();
        true
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
        if let Some(adjustment) = &mut self.camera_adjustment {
            adjustment.weight *= (-dt * 0.8).exp();
            if adjustment.weight < 0.0001 {
                self.camera_adjustment = None;
            }
        }
        self.portal_cooldown = (self.portal_cooldown - dt).max(0.0);
        if let Some(transit) = &mut self.portal_transit {
            transit.elapsed += dt;
            if transit.elapsed > 2.0
                && self.position.distance(transit.entry.center) > transit.entry.radius * 4.0
            {
                self.portal_transit = None;
            }
        }
        self.warp_flash = (self.warp_flash - dt).max(0.0);
        if world.kind.vacuum() {
            self.weather = crate::content::weather::Weather::Clear;
        }
        let boost_goal = f32::from(controls.boost);
        self.boost += (boost_goal - self.boost) * ease(4.0, dt);
        self.orbital_thrust += (f32::from(controls.orbital) - self.orbital_thrust) * ease(1.5, dt);
        self.speed = self.vehicle.speed() * (1.0 + self.boost * 0.8 + self.orbital_thrust * 3.0);
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
            let target_rate = if self.loop_remaining > 0.0 {
                self.loop_direction * LOOP_RATE
            } else {
                let neutral = (self.pitch / std::f32::consts::PI).round() * std::f32::consts::PI;
                let pitch_limit = NORMAL_PITCH + self.orbital_thrust * 0.6;
                angle(neutral + input * pitch_limit - self.pitch) * 2.0
            };
            self.pitch_rate += (target_rate - self.pitch_rate) * ease(5.0, dt);
            let mut rotation = self.pitch_rate * dt;
            if self.loop_remaining > 0.0 {
                rotation = self.loop_direction * rotation.abs().min(self.loop_remaining);
                self.loop_remaining = (self.loop_remaining - rotation.abs()).max(0.0);
                if self.loop_remaining == 0.0 {
                    self.pitch_rate = 0.0;
                }
            }
            self.pitch = angle(self.pitch + rotation);
        } else {
            self.loop_remaining = 0.0;
        }
        let forward = self.axes()[1];
        let horizontal = if manual_attitude {
            forward
        } else {
            vec3(self.yaw.sin(), -self.yaw.cos(), 0.0)
        };
        let proposed = self.position + horizontal * self.speed * dt;
        let (_, ahead_climb) =
            flight_floor(world, self.position, horizontal, self.speed, self.rate);
        let mut climb = if manual_attitude {
            forward.z * self.speed
        } else if controls.manual {
            controls.climb * self.speed * 0.55
        } else {
            (110.0 - self.position.z) * 0.6
        };
        climb = climb.max(ahead_climb).clamp(-self.speed, self.speed);
        if self.loop_remaining <= 0.0 {
            let limit = self.speed * (0.4 + self.orbital_thrust * 0.55);
            climb = climb.clamp(-limit, limit);
        }
        let acceleration = if self.loop_remaining > 0.0 || self.orbital_thrust > 0.0001 {
            90.0
        } else {
            24.0
        };
        self.vertical_speed +=
            (climb - self.vertical_speed).clamp(-acceleration * dt, acceleration * dt);
        let old_altitude = self.position.z;
        let target = vec3(
            proposed.x,
            proposed.y,
            old_altitude + self.vertical_speed * dt,
        );
        self.position = if self.surface.is_some_and(|surface| surface.bend > 0.0) {
            target
        } else {
            safety::advance(world, self.position, target)
        };
        let actual_climb = (self.position.z - old_altitude) / dt.max(0.001);
        if (actual_climb - self.vertical_speed).abs() > 0.01 {
            self.vertical_speed = actual_climb;
        }
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
        self.update_visuals(dt, world);
    }
    pub(crate) fn update_visuals(&mut self, dt: f32, world: &World) {
        self.follow += (self.position - self.follow) * ease(4.5, dt);
        self.camera_yaw +=
            angle(-self.yaw * 0.6 + self.camera_offset - self.camera_yaw) * ease(1.5, dt);
        if self.demo {
            self.director_camera(dt);
        }
        let target_weights = std::array::from_fn::<_, 3, _>(|i| f32::from(i == self.view));
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
            if wing.portal_wait > 0.0 {
                wing.portal_wait = (wing.portal_wait - dt).max(0.0);
                continue;
            }
            if wing.rejoining {
                let delta = goal - wing.position;
                wing.position += delta.clamp_length_max(self.speed * 1.7 * dt);
                wing.rejoining = delta.length() > 12.0;
            } else {
                wing.position += (goal - wing.position) * ease(4.5, dt);
            }
            if self.surface.is_none_or(|surface| surface.bend <= 0.0) {
                wing.position.z = wing.position.z.max(clearance_floor(world, wing.position));
            }
            wing.yaw += angle(self.yaw - wing.yaw) * ease(4.0, dt);
            wing.bank += (self.bank - side * split * 0.7 - wing.bank) * ease(3.2, dt);
            wing.pitch += angle(self.pitch - wing.pitch) * ease(3.0, dt);
            wing.weather = self.weather;
            wing.space = self.space;
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
        self.update_portal_visibility();
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
