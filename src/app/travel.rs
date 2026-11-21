use super::*;
use crate::world::network::{MapKey, Route};

impl App {
    /// 菜单背景演示期间，包括从菜单打开的帮助页与演示穿越加载
    fn on_menu(&self) -> bool {
        let menu = |page| {
            matches!(page, Page::Menu) || (page == Page::Help && self.help_return == Page::Menu)
        };
        menu(self.page)
            || (self.page == Page::Loading && self.travel.is_some() && menu(self.travel_return))
    }
    /// 菜单所选的地图，演示穿越到别的地图时仍显示玩家的选择
    pub fn menu_map(&self) -> crate::world::MapKind {
        self.loader
            .pending
            .or(self.menu_home.map(|(key, _)| key.kind))
            .unwrap_or(self.world.kind)
    }
    pub fn menu_weather(&self) -> &crate::content::weather::WeatherState {
        self.menu_home
            .as_ref()
            .map_or(&self.flight.weather, |(_, weather)| weather)
    }
    pub(super) fn cycle_menu_weather(&mut self, direction: i32) {
        let kind = self.menu_map();
        match &mut self.menu_home {
            Some((_, weather)) => weather.cycle(kind, direction),
            None => self.flight.weather.cycle(kind, direction),
        }
    }
    /// 开始任务前回到玩家在菜单选择的地图和天气
    pub(super) fn restore_menu_home(&mut self) {
        let Some((key, weather)) = self.menu_home.take() else {
            return;
        };
        if MapKey::of(&self.world) != key && self.loader.pending.is_none() {
            if let Some(world) = self.network.take(key) {
                self.flight.change_world(&world);
                let previous = std::mem::replace(&mut self.world, world);
                self.network.remember(previous);
            } else {
                self.loader.start(key.kind, key.seed);
            }
        }
        self.flight.weather = weather;
    }
    pub(super) fn forget_menu_home(&mut self) {
        self.menu_home = None;
    }
    pub(super) fn cross_world(&mut self, previous: glam::Vec3) -> bool {
        if !self.network.active || !self.flight.can_warp() || self.flight.orbit_quiet {
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
        if self.on_menu() && self.menu_home.is_none() {
            self.menu_home = Some((MapKey::of(&self.world), self.flight.weather));
        }
        let exit = self.network.exit(&destination, route.id);
        let before = self.flight.position;
        self.orbit = orbit::Orbit::default();
        self.flight.warp(route.aperture, exit);
        self.flight.enter_region(&destination);
        if !self.flight.space {
            self.flight.settle_surface_attitude();
        }
        self.flight.demo_target = None;
        let environment = self.network.environment(route.target);
        self.flight.weather = environment.weather;
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
