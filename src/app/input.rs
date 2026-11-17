use super::*;
use crate::keys::*;

impl App {
    pub fn release_keys(&mut self) {
        self.keys.clear();
        self.direction_tap = None;
        self.escape_since = None;
    }
    pub fn controls(&self) -> Controls {
        let held = |key| f32::from(self.keys.contains(&key));
        Controls {
            direction: None,
            throttle: None,
            bank: None,
            aerobatic: true,
            turn: held(RIGHT) - held(LEFT),
            climb: held(UP) - held(DOWN),
            boost: self.keys.contains(&BOOST) || self.keys.contains(&BOOST_RIGHT),
            orbital: false,
            manual: !self.auto,
        }
    }
    pub fn key(&mut self, code: u16, value: i32) {
        if value == 0 {
            self.keys.remove(&code);
            if code == ESC
                && self
                    .escape_since
                    .take()
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
            {
                self.back();
            }
            return;
        }
        if value != 1 || !self.keys.insert(code) {
            return;
        }
        if code == ESC {
            self.escape_since = Some(Instant::now());
            return;
        }
        match code {
            CHART if matches!(self.page, Page::Playing | Page::Paused | Page::Chart) => {
                self.toggle_chart();
                return;
            }
            HUD => {
                self.flight.hud = !self.flight.hud;
                return;
            }
            HELP if self.page != Page::Help => {
                self.help();
                return;
            }
            SETTINGS if self.page != Page::Settings => {
                if self.page == Page::Help && self.help_return == Page::Settings {
                    self.page = Page::Settings;
                    return;
                }
                self.open_settings();
                return;
            }
            SOUND => {
                self.audio_settings.profile = (self.audio_settings.profile + 1) % 4;
                return;
            }
            _ => {}
        }
        if matches!(
            self.page,
            Page::Menu | Page::Paused | Page::Settings | Page::Help | Page::Results
        ) {
            if code == ENTER {
                self.sounds.push(crate::audio::Sound::Confirm);
            } else if matches!(code, UP | LEFT | DOWN | RIGHT) {
                self.sounds.push(crate::audio::Sound::Navigate);
            }
        }
        match self.page {
            Page::Chart => {
                match code {
                    WEATHER => self.chart.zoom_by(1, &self.world, self.flight.position),
                    TIME => self.chart.zoom_by(-1, &self.world, self.flight.position),
                    PORTAL => {
                        let routes = self.network.routes(&self.world);
                        if !routes.is_empty() {
                            let route = routes[self.chart.selected % routes.len()];
                            self.chart.focus = Some(route.aperture.center.truncate());
                            self.chart.selected = (self.chart.selected + 1) % routes.len();
                        }
                    }
                    VEHICLE => self.chart.focus = Some(self.flight.position.truncate()),
                    WINGMEN if self.world.kind == crate::world::MapKind::Space => {
                        self.select_chart_body(crate::world::orbit::Body::Earth)
                    }
                    MODEL if self.world.kind == crate::world::MapKind::Space => {
                        self.select_chart_body(crate::world::orbit::Body::Moon)
                    }
                    AUTOPILOT if self.world.kind == crate::world::MapKind::Space => {
                        self.cycle_landing()
                    }
                    VIEW => self.mark_chart(),
                    ENTER => self.navigate_chart(),
                    _ => {}
                }
                return;
            }
            Page::Settings => {
                match code {
                    UP => self.selection = (self.selection + 5) % 6,
                    DOWN => self.selection = (self.selection + 1) % 6,
                    LEFT => self.adjust_setting(-1),
                    RIGHT | ENTER => self.adjust_setting(1),
                    SETTINGS => self.close_settings(),
                    _ => {}
                }
                return;
            }
            Page::Help => {
                match code {
                    LEFT => {
                        self.help_page = (self.help_page + crate::ui::HELP_PAGE_COUNT - 1)
                            % crate::ui::HELP_PAGE_COUNT;
                    }
                    RIGHT => {
                        self.help_page = (self.help_page + 1) % crate::ui::HELP_PAGE_COUNT;
                    }
                    ENTER | HELP => self.page = self.help_return,
                    _ => {}
                }
                return;
            }
            Page::Menu => {
                match code {
                    UP => self.selection = (self.selection + MENU_ROWS - 1) % MENU_ROWS,
                    DOWN => self.selection = (self.selection + 1) % MENU_ROWS,
                    LEFT => self.adjust(-1),
                    RIGHT => self.adjust(1),
                    ENTER => self.activate(),
                    _ => {}
                }
                return;
            }
            Page::Paused => {
                match code {
                    UP => self.selection = (self.selection + 5) % 6,
                    DOWN => self.selection = (self.selection + 1) % 6,
                    ENTER => self.activate(),
                    PAUSE => self.page = Page::Playing,
                    _ => {}
                }
                return;
            }
            Page::Results => {
                match code {
                    RETRY => self.start(),
                    ENTER => self.result_action(),
                    LEFT => self.reward = (self.reward + 2) % 3,
                    RIGHT => self.reward = (self.reward + 1) % 3,
                    _ => {}
                }
                return;
            }
            Page::Loading => return,
            Page::Playing => {}
        }
        if self.mode == Mode::Free && (WEATHER..=SEED).contains(&code) {
            self.free_flight_key(code);
            return;
        }
        match code {
            OVERDRIVE if self.mode == Mode::Combat => self.mission.activate_overdrive(),
            VIEW => self.flight.view = (self.flight.view + 1) % 3,
            ROLL_LEFT | ROLL_RIGHT if self.orbit.transition.is_none() => {
                let direction = if code == ROLL_LEFT { -1.0 } else { 1.0 };
                if self.flight.roll_toward(direction) {
                    self.sounds.push(crate::audio::Sound::Roll);
                    self.mission.dodge();
                }
            }
            PAUSE => {
                self.page = Page::Paused;
                self.selection = 0;
            }
            RETRY => self.start(),
            WEAPON if self.mode == Mode::Combat => self.mission.cycle_weapon(),
            BOMB if self.mode == Mode::Combat => self.mission.bomb(),
            AUTOFIRE if self.mode == Mode::Combat => self.mission.autofire = !self.mission.autofire,
            UP | LEFT | DOWN | RIGHT if self.orbit.transition.is_none() => {
                self.navigation.active = false;
                self.navigation.arrived = false;
                self.auto = false;
                self.flight.set_demo(false);
                let now = Instant::now();
                let double = self.direction_tap.is_some_and(|(key, time)| {
                    key == code && now.duration_since(time) <= Duration::from_millis(320)
                });
                self.direction_tap = if double { None } else { Some((code, now)) };
                if double && self.mission.countdown <= 0.0 {
                    let started = match code {
                        UP => self.flight.loop_toward(1.0),
                        DOWN => self.flight.loop_toward(-1.0),
                        LEFT => self.flight.roll_toward(-1.0),
                        RIGHT => self.flight.roll_toward(1.0),
                        _ => false,
                    };
                    if started {
                        self.sounds.push(crate::audio::Sound::Roll);
                        self.mission.dodge();
                    }
                }
            }
            _ => {}
        }
    }
    fn free_flight_key(&mut self, code: u16) {
        if self.orbit.transition.is_some() {
            return;
        }
        if matches!(code, AUTOPILOT | DEMO | MAP | SEED) {
            self.orbit = orbit::Orbit::default();
        }
        match code {
            WEATHER => self.flight.weather = self.flight.weather.next(),
            TIME => self.theme = (self.theme + 1) % 3,
            PORTAL => {
                self.network
                    .summon(&self.world, self.flight.position, self.flight.axes()[1]);
                self.flight.portal_transit = None;
                self.sounds.push(crate::audio::Sound::Warp);
            }
            VEHICLE => {
                self.flight.vehicle =
                    Vehicle::ALL[(self.flight.vehicle as usize + 1) % Vehicle::ALL.len()];
                self.mission.max_health = self.flight.vehicle.shield();
                self.mission.health = self.mission.max_health;
            }
            WINGMEN => self.flight.wing_count = (self.flight.wing_count + 1) % 3,
            MODEL => self.flight.exploded = !self.flight.exploded,
            AUTOPILOT => {
                self.navigation.active = false;
                self.flight.set_demo(false);
                self.auto = !self.auto;
            }
            DEMO => {
                self.navigation.active = false;
                self.flight.set_demo(!self.flight.demo);
            }
            MAP => {
                let kind = self.world.kind.next();
                self.loader.start(kind, kind.seed());
            }
            SEED => self
                .loader
                .start(self.world.kind, self.world.seed.wrapping_add(7919)),
            _ => {}
        }
    }
    pub fn back(&mut self) {
        match self.page {
            Page::Chart => self.toggle_chart(),
            Page::Menu => {}
            Page::Playing => {
                self.page = Page::Paused;
                self.selection = 0;
            }
            Page::Paused => self.page = Page::Playing,
            Page::Settings => self.close_settings(),
            Page::Help => self.page = self.help_return,
            Page::Results | Page::Loading => self.home(),
        }
    }
    pub(super) fn help(&mut self) {
        self.help_return = self.page;
        self.page = Page::Help;
        self.help_page = 0;
    }
    pub fn activate(&mut self) {
        match self.page {
            Page::Menu => match self.selection {
                0..=5 => self.selection = START_ROW,
                START_ROW => self.start(),
                7 => self.open_settings(),
                8 => self.quit = true,
                _ => {}
            },
            Page::Paused => match self.selection {
                0 => self.page = Page::Playing,
                1 => self.start(),
                2 => self.flight.hud = !self.flight.hud,
                3 => self.open_settings(),
                4 => self.home(),
                5 => self.quit = true,
                _ => {}
            },
            Page::Settings => self.adjust_setting(1),
            Page::Results => self.result_action(),
            _ => {}
        }
    }
    pub(super) fn adjust(&mut self, direction: i32) {
        let next = |i: usize, len: usize| (i as i32 + direction).rem_euclid(len as i32) as usize;
        match self.selection {
            0 => self.mode = Mode::ALL[next(self.mode as usize, 4)],
            1 => {
                self.flight.vehicle =
                    Vehicle::ALL[next(self.flight.vehicle as usize, Vehicle::ALL.len())]
            }
            2 => {
                let current = self.loader.pending.unwrap_or(self.world.kind);
                let kind = crate::world::MapKind::ALL
                    [next(current as usize, crate::world::MapKind::ALL.len())];
                self.loader.start(kind, kind.seed());
            }
            3 => {
                self.flight.weather = crate::content::weather::Weather::ALL[next(
                    self.flight.weather as usize,
                    crate::content::weather::Weather::ALL.len(),
                )]
            }
            4 => self.difficulty = Difficulty::ALL[next(self.difficulty as usize, 3)],
            5 => {
                self.level = next((self.level - 1) as usize, 12) as u32 + 1;
                self.load_operation();
            }
            _ => {}
        }
    }
    pub fn uses_fpv_hud(&self) -> bool {
        self.flight.fixed_weight < 0.5
    }
    pub fn hud_button_contains(&self, x: usize, y: usize) -> bool {
        if self.mode == Mode::Combat && !self.uses_fpv_hud() {
            x >= 291 && y >= 154
        } else {
            x >= 302 && y < 14
        }
    }
    pub fn click(&mut self, x: usize, y: usize) {
        if self.page == Page::Menu && (36..148).contains(&y) {
            self.selection = ((y - 36) / 12).min(MENU_ROWS - 1);
            if self.selection >= START_ROW {
                self.activate();
            } else if x >= 296 {
                self.adjust(1);
            } else if (102..112).contains(&x) {
                self.adjust(-1);
            }
        } else if self.page == Page::Paused && (45..141).contains(&y) {
            self.selection = ((y - 45) / 16).min(5);
            self.activate();
        } else if self.page == Page::Settings && (45..141).contains(&y) {
            self.selection = ((y - 45) / 16).min(5);
            self.adjust_setting(if x < 160 { -1 } else { 1 });
        } else if self.page == Page::Playing && self.hud_button_contains(x, y) {
            self.flight.hud = !self.flight.hud;
        } else if self.page == Page::Results && (134..153).contains(&y) {
            self.result_action();
        } else if self.page == Page::Results && (108..130).contains(&y) {
            self.reward = (self.reward + 1) % 3;
        }
    }
}
