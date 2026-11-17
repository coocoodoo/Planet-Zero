use super::*;
use crate::app::chart::offset;
use crate::world::network::MapKey;
use glam::Vec3;

#[derive(Clone, Copy)]
pub struct Waypoint {
    pub map: MapKey,
    pub position: Vec3,
    pub portal: Option<u32>,
}
#[derive(Default)]
pub struct Navigation {
    pub target: Option<Waypoint>,
    pub active: bool,
    pub arrived: bool,
}
impl App {
    pub(super) fn mark_chart(&mut self) {
        self.chart.prepare(&self.world, self.flight.position);
        let center = self.chart.center;
        let portal = self.network.routes(&self.world).into_iter().find(|r| {
            offset(r.aperture.center.truncate(), center).length() < self.chart.span * 0.035
        });
        let position = portal.map_or_else(
            || {
                let p = center.extend(self.flight.position.z);
                p.with_z(
                    self.flight
                        .position
                        .z
                        .max(self.world.airspace.floor(&self.world, p) + 16.0),
                )
            },
            |r| r.aperture.center,
        );
        self.navigation.target = Some(Waypoint {
            map: MapKey::of(&self.world),
            position,
            portal: portal.map(|r| r.id),
        });
        self.navigation.active = false;
        self.navigation.arrived = false;
        self.sounds.push(crate::audio::Sound::Confirm);
    }
    pub(super) fn navigate_chart(&mut self) {
        if self
            .navigation
            .target
            .is_none_or(|t| t.map != MapKey::of(&self.world))
        {
            self.mark_chart();
        }
        self.navigation.active = true;
        self.navigation.arrived = false;
        self.auto = false;
        self.flight.set_demo(false);
        self.page = Page::Playing;
        self.release_keys();
    }
    pub(super) fn navigation_controls(&mut self) -> Option<Controls> {
        if !self.navigation.active {
            return None;
        }
        let target = self.navigation.target?;
        if target.map != MapKey::of(&self.world) {
            self.navigation.active = false;
            return None;
        }
        let delta = offset(target.position.truncate(), self.flight.position.truncate());
        let dz = target.position.z - self.flight.position.z;
        if target.portal.is_none() && delta.length() < 24.0 && dz.abs() < 20.0 {
            self.navigation.active = false;
            self.navigation.arrived = true;
            self.sounds.push(crate::audio::Sound::Confirm);
        }
        let bearing = delta.x.atan2(-delta.y);
        Some(Controls {
            manual: true,
            turn: crate::flight::angle(bearing - self.flight.yaw) * 1.8
                / self.flight.vehicle.turn(),
            climb: (dz * 0.8 / (self.flight.speed * 0.55).max(1.0)).clamp(-1.0, 1.0),
            ..Default::default()
        })
    }
}
