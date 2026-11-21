use super::*;
use crate::keys::*;

#[derive(Default)]
pub(super) struct PitchGesture {
    held: Option<(u16, f32)>,
    armed: Option<(u16, f32, u8)>,
}
impl PitchGesture {
    fn release(&mut self, code: u16, now: f32) {
        if let Some((key, start)) = self.held
            && key == code
        {
            if now - start >= 3.0 {
                self.armed = Some((key, now, 0));
            }
            self.held = None;
        }
    }
    fn press(&mut self, code: u16, now: f32) -> bool {
        self.held = Some((code, now));
        if let Some((key, released, taps)) = &mut self.armed
            && *key == code
            && now - *released <= 0.3
        {
            *taps += 1;
            if *taps == 2 {
                self.held = None;
                self.armed = None;
                return true;
            }
        } else {
            self.armed = None;
        }
        false
    }
}

impl App {
    pub fn release_keys(&mut self) {
        self.keys.clear();
        self.pitch_gesture = Default::default();
        self.escape_since = None;
    }
    pub fn controls(&self) -> Controls {
        let held = |key| f32::from(self.keys.contains(&key));
        Controls {
            direction: None,
            throttle: if self.auto {
                None
            } else if self.flight.hover {
                Some(0.0)
            } else {
                Some(1.0)
            },
            bank: None,
            pitch_target: None,
            heading_only: false,
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
            if self.page == Page::Playing {
                self.pitch_gesture.release(code, self.flight.time);
            }
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
                self.pitch_gesture = Default::default();
                self.toggle_chart();
                return;
            }
            HUD => {
                self.flight.hud = !self.flight.hud;
                self.save_settings();
                return;
            }
            HELP if self.page != Page::Help => {
                self.pitch_gesture = Default::default();
                self.help();
                return;
            }
            SETTINGS if self.page != Page::Settings => {
                if self.page == Page::Help && self.help_return == Page::Settings {
                    self.page = Page::Settings;
                    return;
                }
                self.pitch_gesture = Default::default();
                self.open_settings();
                return;
            }
            PERF => {
                self.perf_panel = match self.perf_panel {
                    PerfPanel::Off => PerfPanel::Full,
                    PerfPanel::Full => PerfPanel::Half,
                    PerfPanel::Half => PerfPanel::Mini,
                    PerfPanel::Mini => PerfPanel::Off,
                };
                if self.perf_panel == PerfPanel::Full {
                    let target_fps = self.perf.target_fps;
                    let asynchronous = self.perf.asynchronous;
                    self.perf = PerfStats::default();
                    self.perf.target_fps = target_fps;
                    self.perf.asynchronous = asynchronous;
                }
            }
            SOUND => {
                self.audio_settings.profile = (self.audio_settings.profile + 1) % 4;
                self.save_settings();
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
                let rows = self.settings_rows();
                match code {
                    UP => self.selection = (self.selection + rows - 1) % rows,
                    DOWN => self.selection = (self.selection + 1) % rows,
                    LEFT => self.adjust_setting(-1),
                    RIGHT if self.settings_section.is_none() => self.activate_setting(),
                    RIGHT => self.adjust_setting(1),
                    ENTER => self.activate_setting(),
                    SETTINGS => self.close_settings(),
                    _ => {}
                }
                return;
            }
            Page::Help => {
                match code {
                    LEFT => {
                        self.help_page = (self.help_page + crate::ui::help_page_count(self.mode)
                            - 1)
                            % crate::ui::help_page_count(self.mode);
                    }
                    RIGHT => {
                        self.help_page =
                            (self.help_page + 1) % crate::ui::help_page_count(self.mode);
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
            VIEW => {
                self.flight.view = (self.flight.view + 1) % 3;
                self.sounds.push(crate::audio::Sound::View);
            }
            ROLL_LEFT | ROLL_RIGHT => {
                let direction = if code == ROLL_LEFT { -1.0 } else { 1.0 };
                if self.flight.roll_toward(direction) {
                    self.sounds.push(crate::audio::Sound::Roll);
                    self.mission.dodge();
                }
            }
            HOVER => {
                self.flight.hover = !self.flight.hover;
                self.navigation.active = false;
                self.navigation.arrived = false;
                self.auto = false;
                self.flight.set_demo(false);
                self.sounds
                    .push(crate::audio::Sound::Toggle(self.flight.hover));
            }
            PAUSE => {
                self.pitch_gesture = Default::default();
                self.page = Page::Paused;
                self.selection = 0;
            }
            RETRY => self.start(),
            WEAPON if self.mode == Mode::Combat => self.mission.cycle_weapon(),
            BOMB if self.mode == Mode::Combat => self.mission.bomb(),
            AUTOFIRE if self.mode == Mode::Combat => {
                self.mission.autofire = !self.mission.autofire;
                self.sounds
                    .push(crate::audio::Sound::Toggle(self.mission.autofire));
            }
            UP | LEFT | DOWN | RIGHT => {
                self.navigation.active = false;
                self.navigation.arrived = false;
                self.auto = false;
                self.flight.set_demo(false);
                if matches!(code, UP | DOWN) {
                    let opposite = if code == UP { DOWN } else { UP };
                    let double = if self.keys.contains(&opposite) {
                        self.pitch_gesture = Default::default();
                        false
                    } else {
                        self.pitch_gesture.press(code, self.flight.time)
                    };
                    if double
                        && self.mission.countdown <= 0.0
                        && self.flight.loop_toward(if code == UP { 1.0 } else { -1.0 })
                    {
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
            WEATHER => self.flight.weather.cycle(self.world.kind, 1),
            TIME => {
                self.theme = (self.theme + 1) % 3;
                self.flight.day_time = 0.0;
            }
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
                self.flight.hover = false;
                self.auto = !self.auto;
            }
            DEMO => {
                self.navigation.active = false;
                self.flight.hover = false;
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
        self.pitch_gesture = Default::default();
        match self.page {
            Page::Chart => self.toggle_chart(),
            Page::Menu => {}
            Page::Playing => {
                self.page = Page::Paused;
                self.selection = 0;
            }
            Page::Paused => self.page = Page::Playing,
            Page::Settings => self.settings_back(),
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
            Page::Settings => self.activate_setting(),
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
                let current = self.menu_map();
                let kind = crate::world::MapKind::ALL
                    [next(current as usize, crate::world::MapKind::ALL.len())];
                self.forget_menu_home();
                self.loader.start(kind, kind.seed());
            }
            3 => {
                self.cycle_menu_weather(direction);
            }
            4 => self.difficulty = Difficulty::ALL[next(self.difficulty as usize, 3)],
            5 => {
                self.level = next((self.level - 1) as usize, 12) as u32 + 1;
                self.forget_menu_home();
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
        } else if self.page == Page::Settings && (24..296).contains(&x) {
            let row_height = self.settings_row_height();
            if self.settings_section.is_some() && (20..42).contains(&y) {
                self.settings_back();
            } else if (49..49 + self.settings_rows() * row_height).contains(&y) {
                self.selection = (y - 49) / row_height;
                if self.settings_section == Some(SettingsSection::General)
                    && matches!(self.selection, 2 | 3)
                    && (184..208).contains(&x)
                {
                    self.adjust_setting(-1);
                } else {
                    self.activate_setting();
                }
            }
        } else if self.page == Page::Playing && self.hud_button_contains(x, y) {
            self.flight.hud = !self.flight.hud;
        } else if self.page == Page::Results && (134..153).contains(&y) {
            self.result_action();
        } else if self.page == Page::Results && (108..130).contains(&y) {
            self.reward = (self.reward + 1) % 3;
        }
    }
}
