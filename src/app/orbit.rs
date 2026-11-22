use super::*;
use crate::world::{
    MapKind,
    network::MapKey,
    orbit::{Body, SurfaceMap, SurfaceWarp},
};
use glam::{Quat, Vec3, vec3};
use std::sync::Arc;

pub const ARM_ALTITUDE: f32 = 1000.0 / 3.0;

#[derive(Default)]
pub struct Orbit {
    pub boost: bool,
    pub transition: Option<Arc<World>>,
    pub descending: bool,
    pub earth: Option<MapKey>,
    departure: Option<MapKind>,
    pub arrival_notice: f32,
    hold: f32,
    cooldown: f32,
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
        self.flight.orbit_flight = self.orbit_enabled();
        if !self.flight.orbit_flight {
            return;
        }
        if self.world.kind == MapKind::Space {
            let limit = Body::ALL
                .into_iter()
                .map(|body| {
                    ((body.altitude(self.flight.position) - body.orbit_altitude()) / body.radius()
                        + 0.5)
                        .clamp(0.05, 1.0)
                })
                .fold(1.0_f32, f32::min);
            controls.throttle = Some(controls.throttle.unwrap_or(1.0).min(limit));
        }
        let climbing = controls.climb > 0.35 && self.flight.axes()[1].z > 0.18;
        self.orbit.hold = if climbing { self.orbit.hold + dt } else { 0.0 };
        controls.orbital = if self.world.kind == MapKind::Space {
            controls.boost
        } else {
            self.flight.position.z >= ARM_ALTITUDE && self.orbit.hold >= 0.6
        };
        if controls.orbital && !self.orbit.boost && self.world.kind != MapKind::Space {
            self.sounds.push(crate::audio::Sound::OrbitIgnite);
        }
        self.orbit.boost = controls.orbital;
    }
    pub(super) fn prepare_orbit(&mut self) {
        if !self.orbit_enabled() {
            return;
        }
        if self.orbit.transition.is_some() {
            return;
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
                if body.altitude(self.flight.position) > 6000.0 {
                    self.flight.planet_maps[body.index()] = None;
                    continue;
                }
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
                .any(|body| body.altitude(self.flight.position) < body.orbit_altitude() + 500.0);
        } else if self.flight.position.z >= ARM_ALTITUDE - 100.0 {
            self.network.request(MapKey {
                kind: MapKind::Space,
                seed: MapKind::Space.seed(),
            });
        }
    }
    /// 太空地图中把飞机挡在星体外侧，所有模式都生效
    pub(super) fn keep_clear_of_bodies(&mut self) {
        if self.world.kind != MapKind::Space {
            return;
        }
        for body in Body::ALL {
            let delta = body.center() - self.flight.position;
            let radius = body.radius() + 420.0;
            if delta.length() < radius {
                let normal = (-delta).try_normalize().unwrap_or(Vec3::Z);
                self.flight
                    .translate(body.center() + normal * radius - self.flight.position);
                self.flight.speed *= 0.5;
            }
        }
    }
    pub(super) fn step_orbit(&mut self, dt: f32) -> bool {
        self.orbit.cooldown = (self.orbit.cooldown - dt).max(0.0);
        self.orbit.arrival_notice = (self.orbit.arrival_notice - dt).max(0.0);
        if !self.orbit_enabled() {
            return false;
        }
        if let Some(space) = self.orbit.transition.take() {
            let altitude = self.flight.position.z;
            let body = Body::of(self.world.kind);
            let bend = body.curvature(altitude);
            if let Some(mut surface) = self.flight.surface {
                let latitude = self.flight.position.y - surface.origin.y;
                let pole = std::f32::consts::FRAC_PI_2 * crate::world::orbit::UV_SCALE;
                // 越过极点后经度移半圈，纬度和切向坐标反向
                if latitude.abs() > pole {
                    let rotation = glam::Mat3::from_rotation_z(std::f32::consts::PI);
                    let offset = vec3(
                        self.flight.position.x * 2.0 + pole * 2.0,
                        surface.origin.y * 2.0 + latitude.signum() * pole * 2.0,
                        0.0,
                    );
                    self.flight.transform_frame(rotation, offset, false);
                    if let Some(target) = &mut self.navigation.target
                        && target.portal.is_none()
                    {
                        target.position = rotation * target.position + offset;
                    }
                }
                // 曲面锚点跟随飞机，局部高度沿地表法线测量
                surface = SurfaceWarp::new(
                    surface.body,
                    surface.origin,
                    self.flight.position.truncate(),
                    bend,
                );
                self.flight.surface = Some(surface);
                self.flight.sky_rotation =
                    Quat::IDENTITY.slerp(Quat::from_mat3(&surface.basis), bend);
            }
            self.flight.orbit_quiet = true;
            self.mission.elapsed += dt;
            if altitude >= body.orbit_altitude() {
                self.finish_ascent(space);
                self.orbit.cooldown = 0.4;
            } else if altitude <= body.surface_altitude() {
                self.orbit.descending = false;
                self.sounds.push(crate::audio::Sound::Landing);
                self.flight.orbital_scenery = None;
                self.flight.orbit_quiet = false;
                self.orbit.cooldown = 0.4;
            } else {
                self.orbit.transition = Some(space);
            }
            return true;
        }
        self.flight.orbit_quiet = self.world.kind == MapKind::Space
            && Body::ALL
                .into_iter()
                .any(|body| body.altitude(self.flight.position) < body.orbit_altitude() + 500.0);
        if self.world.kind == MapKind::Space {
            let mut bodies = Body::ALL;
            bodies.sort_by(|a, b| {
                a.altitude(self.flight.position)
                    .total_cmp(&b.altitude(self.flight.position))
            });
            for body in bodies {
                let delta = body.center() - self.flight.position;
                if self.orbit.cooldown <= 0.0
                    && body.altitude(self.flight.position) < body.orbit_altitude()
                    && delta.normalize_or_zero().dot(self.flight.axes()[1]) > 0.1
                    && let Some(key) = self.orbit_key(body)
                {
                    if self.network.get(key).is_some() {
                        self.begin_descent(body, key);
                        return true;
                    }
                    self.network.request(key);
                }
            }
            self.keep_clear_of_bodies();
        } else if self.orbit.cooldown <= 0.0
            && self.orbit.boost
            && self.flight.position.z >= Body::of(self.world.kind).surface_altitude()
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
                self.auto = false;
                self.orbit.descending = false;
                self.orbit.transition = self.network.get(target).cloned();
                self.sounds.push(crate::audio::Sound::Breakthrough);
                return true;
            }
        }
        false
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
        let offset = anchor.extend(0.0) - rotation * surface.space_origin();
        let camera_before = crate::flight::Camera::new(&self.flight, 320, 170);
        let roll_before = self.flight.screen_roll() * self.flight.weights[2];
        self.flight.transform_pose(rotation, offset, false);
        self.flight.enter_region(&destination);
        self.flight.surface = Some(surface);
        self.flight
            .preserve_camera(camera_before, rotation, offset, roll_before);
        self.flight.sky_rotation = Quat::from_mat3(&surface.basis);
        let environment = map.environment;
        self.flight.weather = environment.weather;
        let previous = std::mem::replace(&mut self.world, destination);
        self.flight.orbital_scenery = Some(previous.clone());
        self.network.remember(previous.clone());
        let mut end = self.flight.position
            + vec3(
                self.flight.yaw.sin() * 480.0,
                -self.flight.yaw.cos() * 480.0,
                0.0,
            );
        let altitude = (self.world.airspace.floor(&self.world, end) + 130.0)
            .max(self.world.ceiling as f32 + 80.0);
        let approach = (self.flight.position.z - altitude).max(0.0) / 0.35;
        end += vec3(self.flight.yaw.sin(), -self.flight.yaw.cos(), 0.0) * approach;
        self.network
            .park_portals(&self.world, end.with_z(altitude), self.flight.axes()[1]);
        let automatic = self.navigation.active;
        self.navigation = navigation::Navigation::default();
        if automatic {
            self.navigation.target = Some(navigation::Waypoint {
                map: key,
                position: end.with_z(altitude - 20.0),
                body: None,
                portal: None,
            });
            self.navigation.active = true;
        }
        self.auto = false;
        self.orbit.descending = true;
        self.orbit.transition = Some(previous);
        self.sounds.push(crate::audio::Sound::Reentry);
    }
    fn finish_ascent(&mut self, destination: Arc<World>) {
        self.orbit.descending = false;
        self.network.take(MapKey::of(&destination));
        let surface = self.flight.surface.unwrap();
        let rotation = surface.basis;
        self.flight.transform_frame(
            rotation,
            surface.space_origin() - rotation * surface.anchor.extend(0.0),
            true,
        );
        if surface.body == Body::Earth {
            self.orbit.earth = Some(self.network.random_earth_except(self.orbit.departure));
        }
        self.orbit.arrival_notice = 5.0;
        self.sounds.push(crate::audio::Sound::OrbitArrival);
        self.flight.surface = None;
        self.flight.orbital_scenery = None;
        self.flight.sky_rotation = Quat::IDENTITY;
        self.flight.weather = crate::content::weather::WeatherState::new(
            crate::content::weather::Weather::Clear,
            self.world.kind,
            self.world.seed,
        );
        let previous = std::mem::replace(&mut self.world, destination);
        self.network.remember(previous);
        self.network
            .park_portals(&self.world, self.flight.position, self.flight.axes()[1]);
    }
}
#[cfg(test)]
mod tests;
