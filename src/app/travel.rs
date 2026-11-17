use super::*;
use crate::world::network::Route;

impl App {
    pub(super) fn cross_world(&mut self, previous: glam::Vec3) -> bool {
        if !self.network.active || !self.flight.can_warp() {
            return false;
        }
        for mut route in self.network.routes(&self.world) {
            if self.flight.demo
                && self
                    .flight
                    .demo_target
                    .is_none_or(|p| p.center != route.aperture.center)
            {
                continue;
            }
            if let Some(entry) = self.flight.approach_portal(route.aperture, previous) {
                route.aperture = entry;
                if !self.arrive(route) {
                    self.travel = Some(route);
                    self.travel_return = self.page;
                    self.network.request(route.target);
                    self.page = Page::Loading;
                }
                return true;
            }
        }
        false
    }
    pub(super) fn arrive(&mut self, route: Route) -> bool {
        let Some(destination) = self.network.take(route.target) else {
            return false;
        };
        let exit = self.network.exit(&destination, route.id);
        let before = self.flight.position;
        self.flight.warp(route.aperture, exit);
        self.flight.enter_region(&destination);
        self.flight.demo_target = None;
        let environment = self.network.environment(route.target);
        self.flight.weather = environment.weather;
        self.theme = environment.theme;
        self.mission.relocate(
            route.aperture,
            exit,
            self.flight.position - route.aperture.transform(exit, before),
            &destination,
        );
        self.navigation = navigation::Navigation::default();
        let previous = std::mem::replace(&mut self.world, destination);
        self.network.remember(previous);
        self.sounds.push(crate::audio::Sound::Warp);
        true
    }
}
