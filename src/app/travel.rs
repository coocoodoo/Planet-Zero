use super::*;
use crate::world::network::Route;

impl App {
    pub(super) fn cross_world(&mut self, previous: glam::Vec3) -> bool {
        if !self.network.active || !self.flight.can_warp() {
            return false;
        }
        for mut route in self.network.routes(&self.world) {
            if route.aperture.crossed(previous, self.flight.position) {
                route.aperture = route.aperture.facing(previous);
                if !self.arrive(route) {
                    self.travel = Some(route);
                    self.network.request(route.target);
                    self.page = Page::Loading;
                }
                return true;
            }
        }
        false
    }
    pub(super) fn arrive(&mut self, route: Route) -> bool {
        let Some(destination) = self.network.get(route.target).cloned() else {
            return false;
        };
        let exit = self.network.exit(&destination, route.id);
        let before = self.flight.position;
        self.flight.warp(route.aperture, exit);
        self.flight.enter_region(&destination);
        self.mission.relocate(
            route.aperture,
            exit,
            self.flight.position - route.aperture.transform(exit, before),
            &destination,
        );
        let previous = std::mem::replace(&mut self.world, destination);
        self.network.remember(previous);
        self.sounds.push(crate::audio::Sound::Warp);
        true
    }
}
