use super::*;
use crate::app::chart::map_offset;
use crate::world::network::MapKey;
use crate::world::{MapKind, orbit::Body};
use glam::Vec3;

#[derive(Clone, Copy)]
pub struct Waypoint {
    pub map: MapKey,
    pub position: Vec3,
    pub portal: Option<u32>,
    pub body: Option<Body>,
}
impl Waypoint {
    pub(crate) fn offset(self, kind: MapKind, position: Vec3) -> glam::Vec2 {
        if self.portal.is_some() {
            (self.position - position).truncate()
        } else {
            map_offset(kind, self.position.truncate(), position.truncate())
        }
    }
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
            map_offset(self.world.kind, r.aperture.center.truncate(), center).length()
                < self.chart.span * 0.035
        });
        let body = if self.world.kind == MapKind::Space {
            Body::ALL.into_iter().find(|body| {
                (body.center().truncate() - center).length()
                    < body.radius().max(self.chart.span * 0.035)
            })
        } else {
            None
        };
        let mut position = portal.map_or_else(
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
        if let Some(body) = body {
            position = body.center();
        }
        self.navigation.target = Some(Waypoint {
            map: MapKey::of(&self.world),
            position,
            portal: if body.is_some() {
                None
            } else {
                portal.map(|r| r.id)
            },
            body,
        });
        self.navigation.active = false;
        self.navigation.arrived = false;
        self.sounds.push(crate::audio::Sound::Confirm);
    }
    pub(super) fn select_chart_body(&mut self, body: Body) {
        self.chart.focus = Some(body.center().truncate());
        self.mark_chart();
    }
    pub(super) fn cycle_landing(&mut self) {
        let current = self.orbit.earth.map(|key| key.kind);
        let kinds: Vec<_> = MapKind::ALL
            .into_iter()
            .filter(|kind| !kind.vacuum())
            .collect();
        let at = kinds
            .iter()
            .position(|kind| Some(*kind) == current)
            .map_or(0, |i| (i + 1) % kinds.len());
        let kind = kinds[at];
        let key = MapKey {
            kind,
            seed: kind.seed(),
        };
        self.orbit.earth = Some(key);
        self.network.request(key);
        self.sounds.push(crate::audio::Sound::Navigate);
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
            return (self.navigation.arrived && self.world.kind == MapKind::Space).then_some(
                Controls {
                    manual: true,
                    throttle: Some(0.0),
                    ..Default::default()
                },
            );
        }
        let target = self.navigation.target?;
        if target.map != MapKey::of(&self.world) {
            self.navigation.active = false;
            return None;
        }
        if self.world.kind == MapKind::Space {
            let delta = target.position - self.flight.position;
            let distance = delta.length();
            if target.portal.is_none() && target.body.is_none() && distance < 24.0 {
                self.navigation.active = false;
                self.navigation.arrived = true;
                self.sounds.push(crate::audio::Sound::Confirm);
            }
            let direction = delta.normalize_or_zero();
            let alignment = self.flight.axes()[1].dot(direction).max(0.0);
            let range = distance - target.body.map_or(0.0, |body| body.radius());
            let throttle = if self.navigation.arrived {
                0.0
            } else {
                (distance / 220.0).min(1.0) * alignment.powi(2)
            };
            return Some(Controls {
                manual: true,
                aerobatic: true,
                direction: Some(direction),
                throttle: Some(throttle),
                boost: range > 2500.0 && alignment > 0.9,
                ..Default::default()
            });
        }
        let delta = target.offset(self.world.kind, self.flight.position);
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

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;
    fn app() -> App {
        let mut app = App::new(World::generate(MapKind::Space, 1999));
        app.start();
        app.auto = false;
        app.record_path = None;
        app.flight.position = vec3(45000.0, 40000.0, 25000.0);
        app.flight.follow = app.flight.position;
        app.flight.yaw = 0.0;
        app
    }
    #[test]
    fn distant_surface_portals_use_their_actual_position_for_navigation() {
        let mut app = App::new(World::generate(MapKind::Moon, MapKind::Moon.seed()));
        app.start();
        app.auto = false;
        app.flight.yaw = 0.0;
        app.navigation.active = true;
        app.navigation.target = Some(Waypoint {
            map: MapKey::of(&app.world),
            position: app.flight.position - Vec3::Y * 3000.0,
            portal: Some(1),
            body: None,
        });
        let controls = app.navigation_controls().unwrap();
        assert!(controls.turn.abs() < 0.001);
    }

    #[test]
    fn space_navigation_reaches_vertical_and_far_targets_and_stops() {
        for delta in [
            vec3(0.0, 0.0, 4000.0),
            vec3(0.0, 0.0, -6000.0),
            vec3(12000.0, 9000.0, -12000.0),
        ] {
            let mut app = app();
            let target = app.flight.position + delta;
            app.navigation.target = Some(Waypoint {
                map: MapKey::of(&app.world),
                position: target,
                body: None,
                portal: None,
            });
            app.navigation.active = true;
            for _ in 0..6000 {
                app.step(1.0 / 30.0, None);
                if app.navigation.arrived {
                    break;
                }
            }
            assert!(
                app.navigation.arrived,
                "delta {delta:?}, remaining {}",
                app.flight.position.distance(target)
            );
            for _ in 0..90 {
                app.step(1.0 / 30.0, None);
            }
            assert!(app.flight.position.distance(target) < 26.0);
            assert!(app.flight.speed < 0.01);
        }
    }
    #[test]
    fn chart_selects_real_planet_altitudes_and_landing_can_be_changed() {
        let mut app = app();
        app.page = Page::Chart;
        for body in Body::ALL {
            app.select_chart_body(body);
            let target = app.navigation.target.unwrap();
            assert_eq!(target.body, Some(body));
            assert_eq!(target.position, body.center());
        }
        let key = MapKey {
            kind: MapKind::Islands,
            seed: MapKind::Islands.seed(),
        };
        app.orbit.earth = Some(key);
        let mut kinds = std::collections::HashSet::new();
        for _ in 0..10 {
            app.cycle_landing();
            kinds.insert(app.orbit.earth.unwrap().kind);
        }
        assert_eq!(kinds.len(), 10);
        app.chart.focus = Some(glam::vec2(-27000.0, 82000.0));
        app.chart.prepare(&app.world, app.flight.position);
        assert_eq!(app.chart.center, glam::vec2(-27000.0, 82000.0));
        app.chart
            .pan(&app.world, app.flight.position, glam::vec2(-0.1, 0.0));
        app.chart.prepare(&app.world, app.flight.position);
        assert!(app.chart.center.x < -27000.0);
    }
}
