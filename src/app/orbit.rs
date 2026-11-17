use super::*;
use crate::flight::angle;
use crate::world::{
    MapKind,
    network::MapKey,
    orbit::{Body, SurfaceMap, SurfaceWarp, smooth},
};
use glam::{Quat, Vec3, vec3};
use std::sync::Arc;

pub const ARM_ALTITUDE: f32 = 500.0;
const EXIT_ALTITUDE: f32 = 1200.0;
const ORBIT_ALTITUDE: f32 = 3000.0;
const DURATION: f32 = 8.0;

#[derive(Default)]
pub struct Orbit {
    pub boost: bool,
    pub transition: Option<Transition>,
    pub earth: Option<MapKey>,
    departure: Option<MapKind>,
    pub arrival_notice: f32,
    hold: f32,
    cooldown: f32,
}
pub struct Transition {
    pub ascent: bool,
    target: MapKey,
    destination: Arc<World>,
    elapsed: f32,
    duration: f32,
    start: Vec3,
    end: Vec3,
    velocity: Vec3,
    exit_velocity: Vec3,
    pitch: f32,
    bank: f32,
    sky: Quat,
}
impl Transition {
    fn position(&self, time: f32) -> Vec3 {
        let t = (time / self.duration).clamp(0.0, 1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        self.start * (2.0 * t3 - 3.0 * t2 + 1.0)
            + self.velocity * self.duration * (t3 - 2.0 * t2 + t)
            + self.end * (-2.0 * t3 + 3.0 * t2)
            + self.exit_velocity * self.duration * (t3 - t2)
    }
    fn velocity(&self, time: f32) -> Vec3 {
        let t = (time / self.duration).clamp(0.0, 1.0);
        let t2 = t * t;
        (self.start - self.end) * ((6.0 * t2 - 6.0 * t) / self.duration)
            + self.velocity * (3.0 * t2 - 4.0 * t + 1.0)
            + self.exit_velocity * (3.0 * t2 - 2.0 * t)
    }
}
impl App {
    fn orbit_enabled(&self) -> bool {
        self.mode == Mode::Free
            && !self.flight.demo
            && self.loader.pending.is_none()
            && self.travel.is_none()
    }
    pub(crate) fn orbit_body(&self) -> Body {
        let forward = self.flight.axes()[1];
        Body::ALL
            .into_iter()
            .min_by(|a, b| {
                let rank = |body: Body| {
                    let d = body.center() - self.flight.position;
                    d.length() * (1.0 + 3.0 * (1.0 - d.normalize_or_zero().dot(forward)))
                        - body.radius()
                };
                rank(*a).total_cmp(&rank(*b))
            })
            .unwrap()
    }
    fn orbit_key(&self, body: Body) -> Option<MapKey> {
        match body {
            Body::Moon => Some(MapKey {
                kind: MapKind::Moon,
                seed: MapKind::Moon.seed(),
            }),
            Body::Earth => self.orbit.earth,
        }
    }
    pub(super) fn orbit_controls(&mut self, controls: &mut Controls, dt: f32) {
        if !self.orbit_enabled() {
            return;
        }
        if let Some(transition) = &self.orbit.transition {
            controls.manual = true;
            controls.aerobatic = true;
            controls.turn = 0.0;
            controls.climb = if transition.ascent { 1.0 } else { -0.3 };
            controls.boost = false;
            controls.orbital = false;
            return;
        }
        if self.world.kind == MapKind::Space {
            let nearest = Body::ALL
                .into_iter()
                .map(|body| body.altitude(self.flight.position))
                .fold(f32::INFINITY, f32::min);
            if nearest < 1700.0 {
                let limit = ((nearest - 420.0) / 1200.0).clamp(0.05, 1.0);
                controls.throttle = Some(controls.throttle.unwrap_or(1.0).min(limit));
            }
        }
        let climbing = controls.boost && controls.climb > 0.35 && self.flight.axes()[1].z > 0.18;
        self.orbit.hold = if climbing { self.orbit.hold + dt } else { 0.0 };
        controls.orbital = controls.boost
            && (self.world.kind == MapKind::Space
                || (self.flight.position.z >= ARM_ALTITUDE && self.orbit.hold >= 0.6));
        self.orbit.boost = controls.orbital;
    }
    pub(super) fn prepare_orbit(&mut self) -> bool {
        if !self.orbit_enabled() {
            return false;
        }
        if let Some(transition) = &self.orbit.transition {
            if transition.ascent {
                self.network.request(transition.target);
            }
            return true;
        }
        if self.world.kind == MapKind::Space {
            if self.orbit.earth.is_none() {
                self.orbit.earth = Some(self.network.random_earth_except(self.orbit.departure));
            }
            let first = self.orbit_body();
            for body in [
                first,
                if first == Body::Moon {
                    Body::Earth
                } else {
                    Body::Moon
                },
            ] {
                let key = self.orbit_key(body).unwrap();
                if let Some(world) = self.network.get(key) {
                    let slot = &mut self.flight.planet_maps[body.index()];
                    if slot.as_ref().is_none_or(|map| map.key != key) {
                        *slot = Some(Arc::new(SurfaceMap::new(
                            world,
                            self.network.environment(key),
                        )));
                    }
                } else {
                    self.network.request(key);
                }
            }
            self.flight.orbit_quiet = Body::ALL
                .into_iter()
                .any(|body| body.altitude(self.flight.position) < 2200.0);
            true
        } else if self.flight.position.z >= ARM_ALTITUDE - 100.0 {
            self.network.request(MapKey {
                kind: MapKind::Space,
                seed: MapKind::Space.seed(),
            });
            true
        } else {
            false
        }
    }
    pub(super) fn step_orbit(&mut self, dt: f32) -> bool {
        self.orbit.cooldown = (self.orbit.cooldown - dt).max(0.0);
        self.orbit.arrival_notice = (self.orbit.arrival_notice - dt).max(0.0);
        if !self.orbit_enabled() {
            return false;
        }
        if let Some(mut transition) = self.orbit.transition.take() {
            transition.elapsed += dt;
            let progress = smooth(transition.elapsed / transition.duration);
            let position = transition.position(transition.elapsed);
            let velocity = transition.velocity(transition.elapsed);
            let target_pitch = if transition.ascent {
                std::f32::consts::FRAC_PI_2
            } else {
                -0.12
            };
            let pitch = transition.pitch + angle(target_pitch - transition.pitch) * progress;
            self.flight.cinematic_position(position, pitch, velocity);
            self.flight.bank = transition.bank * (1.0 - progress);
            self.flight.time += dt;
            self.flight.update_visuals(dt, &self.world);
            if !transition.ascent
                && let Some(adjustment) = &mut self.flight.camera_adjustment
            {
                adjustment.weight = 1.0 - progress;
            }
            if let Some(surface) = &mut self.flight.surface {
                surface.set_bend(if transition.ascent {
                    progress
                } else {
                    1.0 - progress
                });
                self.flight.sky_rotation = transition
                    .sky
                    .slerp(Quat::from_mat3(&surface.basis), progress);
            }
            self.flight.orbit_quiet = true;
            self.mission.elapsed += dt;
            if transition.elapsed >= transition.duration {
                if transition.ascent {
                    self.finish_ascent(transition.destination);
                }
                self.flight.orbital_scenery = None;
                self.flight.boost = 0.0;
                self.flight.orbital_thrust = 0.0;
                self.orbit.cooldown = 4.0;
                self.orbit.boost = false;
                self.flight.orbit_quiet = self.world.kind == MapKind::Space;
            } else {
                self.orbit.transition = Some(transition);
            }
            return true;
        }
        self.flight.orbit_quiet = self.world.kind == MapKind::Space
            && Body::ALL
                .into_iter()
                .any(|body| body.altitude(self.flight.position) < 2200.0);
        if self.world.kind == MapKind::Space {
            let mut bodies = Body::ALL;
            bodies.sort_by(|a, b| {
                a.altitude(self.flight.position)
                    .total_cmp(&b.altitude(self.flight.position))
            });
            for body in bodies {
                let delta = body.center() - self.flight.position;
                if self.orbit.cooldown <= 0.0
                    && body.altitude(self.flight.position) < 1050.0
                    && delta.normalize_or_zero().dot(self.flight.axes()[1]) > 0.1
                    && let Some(key) = self.orbit_key(body)
                {
                    if self.network.get(key).is_some() {
                        self.begin_descent(body, key);
                        return true;
                    }
                    self.network.request(key);
                }
                let radius = body.radius() + 420.0;
                if delta.length() < radius {
                    let normal = (-delta).try_normalize().unwrap_or(Vec3::Z);
                    self.flight
                        .translate(body.center() + normal * radius - self.flight.position);
                    self.flight.speed *= 0.5;
                }
            }
        } else if self.orbit.cooldown <= 0.0
            && self.orbit.boost
            && self.flight.position.z >= EXIT_ALTITUDE
        {
            let target = MapKey {
                kind: MapKind::Space,
                seed: MapKind::Space.seed(),
            };
            if self.network.get(target).is_some() {
                self.flight.orbital_scenery = self.network.get(target).cloned();
                let body = Body::of(self.world.kind);
                self.flight.surface = Some(
                    self.flight
                        .surface
                        .unwrap_or_else(|| SurfaceWarp::ground(&self.world, self.flight.position))
                        .at_surface_position(self.flight.position),
                );
                let mut map = SurfaceMap::new(
                    &self.world,
                    crate::world::network::Environment {
                        weather: self.flight.weather,
                        theme: self.theme,
                    },
                );
                map.origin = self.flight.surface.unwrap().origin;
                self.flight.planet_maps[body.index()] = Some(Arc::new(map));
                if body == Body::Earth {
                    self.orbit.earth = Some(MapKey::of(&self.world));
                    self.orbit.departure = Some(self.world.kind);
                }
                let end = (self.flight.position + self.flight.velocity() * 3.0)
                    .with_z(ORBIT_ALTITUDE.max(self.flight.position.z + 300.0));
                self.begin_transition(true, target, end);
                return true;
            }
        }
        false
    }
    fn begin_transition(&mut self, ascent: bool, target: MapKey, end: Vec3) {
        self.auto = false;
        self.navigation = navigation::Navigation::default();
        self.flight.orbit_quiet = true;
        let velocity = self.flight.velocity();
        let duration = if ascent {
            DURATION
        } else {
            DURATION
                .min((self.flight.position.z - end.z).abs() / (-velocity.z).max(1.0) * 2.2)
                .max(4.0)
        };
        let pitch = if ascent {
            std::f32::consts::FRAC_PI_2
        } else {
            -0.12_f32
        };
        let exit_velocity = vec3(
            self.flight.yaw.sin() * pitch.cos(),
            -self.flight.yaw.cos() * pitch.cos(),
            pitch.sin(),
        ) * self.flight.vehicle.speed();
        self.orbit.transition = Some(Transition {
            ascent,
            target,
            destination: if ascent {
                self.network.get(target).unwrap().clone()
            } else {
                self.world.clone()
            },
            elapsed: 0.0,
            duration,
            start: self.flight.position,
            end,
            velocity,
            exit_velocity,
            pitch: self.flight.pitch,
            bank: self.flight.bank,
            sky: self.flight.sky_rotation,
        });
    }
    fn begin_descent(&mut self, body: Body, key: MapKey) {
        let destination = self.network.take(key).unwrap();
        let slot = &mut self.flight.planet_maps[body.index()];
        if slot.as_ref().is_none_or(|map| map.key != key) {
            *slot = Some(Arc::new(SurfaceMap::new(
                &destination,
                self.network.environment(key),
            )));
        }
        let map = slot.as_ref().unwrap().clone();
        let normal = (self.flight.position - body.center()).normalize();
        let anchor = map.uv(normal);
        let surface = SurfaceWarp::new(body, map.origin, anchor, 1.0);
        let rotation = surface.basis.transpose();
        let adjustment = self.flight.camera_adjustment;
        self.flight.enter_region(&destination);
        self.flight.camera_adjustment = adjustment;
        self.flight.transform_frame(
            rotation,
            anchor.extend(0.0) - rotation * surface.space_origin(),
        );
        self.flight.surface = Some(surface);
        self.flight.sky_rotation = Quat::from_mat3(&surface.basis);
        let environment = map.environment;
        self.flight.weather = environment.weather;
        self.theme = environment.theme;
        let previous = std::mem::replace(&mut self.world, destination);
        self.flight.orbital_scenery = Some(previous.clone());
        self.network.remember(previous);
        let end = self.flight.position
            + vec3(
                self.flight.yaw.sin() * 480.0,
                -self.flight.yaw.cos() * 480.0,
                0.0,
            );
        let altitude = (self.world.airspace.floor(&self.world, end) + 130.0)
            .max(self.world.ceiling as f32 + 80.0);
        self.network
            .park_portals(&self.world, end.with_z(altitude), self.flight.axes()[1]);
        self.begin_transition(false, key, end.with_z(altitude));
    }
    fn finish_ascent(&mut self, destination: Arc<World>) {
        self.network.take(MapKey::of(&destination));
        let surface = self.flight.surface.unwrap();
        let rotation = surface.basis;
        self.flight.space = true;
        for wing in &mut self.flight.wingmen {
            wing.space = true;
        }
        self.flight.transform_frame(
            rotation,
            surface.space_origin() - rotation * surface.anchor.extend(0.0),
        );
        if surface.body == Body::Earth {
            self.orbit.earth = Some(self.network.random_earth_except(self.orbit.departure));
        }
        self.orbit.arrival_notice = 5.0;
        self.sounds.push(crate::audio::Sound::Phase);
        self.flight.surface = None;
        self.flight.orbital_scenery = None;
        self.flight.sky_rotation = Quat::IDENTITY;
        self.flight.weather = crate::content::weather::Weather::Clear;
        self.theme = 0;
        let previous = std::mem::replace(&mut self.world, destination);
        self.network.remember(previous);
        self.network
            .park_portals(&self.world, self.flight.position, self.flight.axes()[1]);
    }
}
pub fn thin_air(altitude: f32) -> f32 {
    smooth((altitude - 500.0) / 700.0)
}

#[cfg(test)]
mod tests;
