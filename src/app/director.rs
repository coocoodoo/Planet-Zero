use super::*;
use crate::world::network::Route;

#[derive(Default)]
pub(super) struct Director {
    slot: Option<u32>,
    route: Option<Route>,
    last_link: Option<u32>,
}

impl App {
    pub(super) fn direct_demo(&mut self) {
        if !self.flight.demo || self.loader.pending.is_some() {
            self.director = Director::default();
            self.flight.demo_target = None;
            return;
        }
        let cycle = (self.flight.demo_time / 96.0) as u32;
        let phase = self.flight.demo_time.rem_euclid(96.0);
        let hop = usize::from(phase >= 48.0);
        let slot = cycle * 2 + hop as u32;
        let cue = if hop == 0 { 28.0 } else { 70.0 };
        if self.director.slot != Some(slot) && self.flight.demo_target.is_none() {
            self.network.ensure(&self.world);
            self.director.route = self
                .network
                .routes(&self.world)
                .into_iter()
                .find(|route| Some(route.id) != self.director.last_link);
            self.director.slot = Some(slot);
        }
        if let Some(route) = self.director.route {
            self.network.request(route.target);
            if phase >= cue && self.network.get(route.target).is_some() {
                let portal = self.network.stage_portal(
                    &self.world,
                    route.id,
                    self.flight.position,
                    self.flight.axes()[1],
                );
                self.flight.demo_target = Some(portal);
                self.director.last_link = Some(route.id);
                self.director.route = None;
            }
        }
    }
}
