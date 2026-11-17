use super::*;
use crate::content::weather::Weather;
use crate::world::MapKind;
use crate::world::celestial::{MOON_RADIUS, moon_center};
use crate::world::network::MapKey;
use glam::{Vec3, vec3};

pub const ARM_ALTITUDE: f32 = 600.0;
const EXIT_ALTITUDE: f32 = 1200.0;
const FADE_OUT: f32 = 1.25;
const HOLD: f32 = 0.3;
const FADE_IN: f32 = 1.8;

#[derive(Default)]
pub struct Orbit {
    pub boost: bool,
    pub transition: Option<Transition>,
    cooldown: f32,
}

pub struct Transition {
    pub target: MapKey,
    elapsed: f32,
    arrived: bool,
}
impl Transition {
    pub fn opacity(&self) -> f32 {
        let t = if self.elapsed < FADE_OUT {
            self.elapsed / FADE_OUT
        } else {
            1.0 - (self.elapsed - FADE_OUT - HOLD) / FADE_IN
        }
        .clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

impl App {
    fn orbit_enabled(&self) -> bool {
        self.mode == Mode::Free
            && !self.flight.demo
            && self.loader.pending.is_none()
            && self.travel.is_none()
    }
    pub(crate) fn orbit_ready(&self) -> bool {
        self.orbit_enabled()
            && self.orbit.cooldown <= 0.0
            && (self.world.kind == MapKind::Space
                || (self.flight.position.z >= ARM_ALTITUDE && self.flight.axes()[1].z > 0.18))
    }
    pub(super) fn toggle_orbit(&mut self) {
        if self.orbit.transition.is_some() {
            return;
        }
        if self.orbit.boost {
            self.orbit.boost = false;
        } else if self.orbit_ready() {
            self.orbit.boost = true;
            self.auto = false;
            self.navigation.active = false;
            self.sounds.push(crate::audio::Sound::Confirm);
        }
    }
    pub(super) fn orbit_controls(&self, controls: &mut Controls) {
        if !self.orbit_enabled() || !self.orbit.boost {
            return;
        }
        controls.orbital = true;
        controls.manual = true;
        controls.aerobatic = true;
        if self.world.kind != MapKind::Space {
            controls.climb = 1.0;
        }
    }
    pub(super) fn orbit_preload(&self) -> Option<MapKey> {
        if !self.orbit_enabled() {
            return None;
        }
        if let Some(transition) = &self.orbit.transition {
            return (!transition.arrived).then_some(transition.target);
        }
        let kind = if self.world.kind == MapKind::Space {
            MapKind::Moon
        } else if self.flight.position.z >= ARM_ALTITUDE - 100.0 || self.orbit.boost {
            MapKind::Space
        } else {
            return None;
        };
        Some(MapKey {
            kind,
            seed: kind.seed(),
        })
    }
    pub(super) fn step_orbit(&mut self, dt: f32) -> bool {
        self.orbit.cooldown = (self.orbit.cooldown - dt).max(0.0);
        if !self.orbit_enabled() {
            return false;
        }
        if self.world.kind == MapKind::Space {
            let center = moon_center(self.flight.position);
            let delta = self.flight.position - center;
            let minimum = MOON_RADIUS + 100.0;
            if delta.length() < minimum {
                let position = center + delta.try_normalize().unwrap_or(Vec3::Z) * minimum;
                self.flight.translate(position - self.flight.position);
            }
        }
        if let Some(mut transition) = self.orbit.transition.take() {
            transition.elapsed += dt;
            if transition.arrived
                && self
                    .rendered_map
                    .is_some_and(|key| key != transition.target)
            {
                transition.elapsed = transition.elapsed.min(FADE_OUT + HOLD);
            }
            if !transition.arrived && transition.elapsed >= FADE_OUT {
                if self.orbit_arrive(transition.target) {
                    transition.arrived = true;
                    self.orbit.boost = false;
                    self.orbit.cooldown = 6.0;
                } else {
                    transition.elapsed = FADE_OUT;
                    self.network.request(transition.target);
                }
            }
            if transition.elapsed < FADE_OUT + HOLD + FADE_IN {
                self.orbit.transition = Some(transition);
            }
            return true;
        }
        let Some(target) = self.orbit_preload() else {
            return false;
        };
        self.network.request(target);
        if self.orbit.cooldown > 0.0 || self.network.get(target).is_none() {
            return false;
        }
        let crossing = if self.world.kind == MapKind::Space {
            let delta = moon_center(self.flight.position) - self.flight.position;
            delta.length() < MOON_RADIUS + 320.0
                && delta.normalize_or_zero().dot(self.flight.axes()[1]) > 0.55
        } else {
            self.orbit.boost && self.flight.position.z >= EXIT_ALTITUDE
        };
        if crossing {
            self.orbit.transition = Some(Transition {
                target,
                elapsed: 0.0,
                arrived: false,
            });
        }
        crossing
    }
    pub(super) fn orbit_arrive(&mut self, target: MapKey) -> bool {
        let Some(destination) = self.network.take(target) else {
            return false;
        };
        let position = if target.kind == MapKind::Space {
            let direction = vec3(self.flight.yaw.sin(), -self.flight.yaw.cos(), 0.0);
            (moon_center(Vec3::ZERO) - direction * 1500.0).with_z(320.0)
        } else {
            vec3(destination.spawn.0, destination.spawn.1, 300.0)
        };
        self.flight.enter_orbit_region(&destination, position);
        self.flight.weather = Weather::Clear;
        self.theme = 0;
        self.navigation = navigation::Navigation::default();
        let previous = std::mem::replace(&mut self.world, destination);
        self.network.remember(previous);
        self.sounds.push(crate::audio::Sound::Warp);
        true
    }
}

pub fn thin_air(altitude: f32) -> f32 {
    let t = ((altitude - 500.0) / 700.0).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
