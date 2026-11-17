use super::*;
use crate::keys::*;

impl App {
    pub fn release_keys(&mut self) {
        self.keys.clear();
        self.escape_since = None;
    }
    pub fn controls(&self) -> Controls {
        let held = |key| f32::from(self.keys.contains(&key));
        Controls {
            bank: None,
            aerobatic: true,
            turn: held(D) - held(A),
            climb: held(W) - held(S),
            boost: self.keys.contains(&BOOST),
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
            } else if matches!(code, W | A | S | D | UP | LEFT | DOWN | RIGHT) {
                self.sounds.push(crate::audio::Sound::Navigate);
            }
        }
        match self.page {
            Page::Settings => {
                match code {
                    UP | W => self.selection = (self.selection + 5) % 6,
                    DOWN | S => self.selection = (self.selection + 1) % 6,
                    LEFT | A => self.adjust_setting(-1),
                    RIGHT | D | ENTER => self.adjust_setting(1),
                    SETTINGS => self.close_settings(),
                    _ => {}
                }
                return;
            }
            Page::Help => {
                match code {
                    LEFT | A => self.help_page = (self.help_page + 4) % 5,
                    RIGHT | D => self.help_page = (self.help_page + 1) % 5,
                    ENTER | HELP => self.page = self.help_return,
                    _ => {}
                }
                return;
            }
            Page::Menu => {
                match code {
                    W | UP => self.selection = (self.selection + MENU_ROWS - 1) % MENU_ROWS,
                    S | DOWN => self.selection = (self.selection + 1) % MENU_ROWS,
                    A | LEFT => self.adjust(-1),
                    D | RIGHT => self.adjust(1),
                    ENTER => self.activate(),
                    _ => {}
                }
                return;
            }
            Page::Paused => {
                match code {
                    W | UP => self.selection = (self.selection + 5) % 6,
                    S | DOWN => self.selection = (self.selection + 1) % 6,
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
                    A | LEFT => self.reward = (self.reward + 2) % 3,
                    D | RIGHT => self.reward = (self.reward + 1) % 3,
                    _ => {}
                }
                return;
            }
            Page::Loading => return,
            Page::Playing => {}
        }
        match code {
            OVERDRIVE => self.mission.activate_overdrive(),
            VIEW => self.flight.view = (self.flight.view + 1) % 3,
            ROLL => {
                if self.flight.roll() {
                    self.sounds.push(crate::audio::Sound::Roll);
                    self.mission.dodge();
                }
            }
            PORTAL => {
                self.network
                    .summon(&self.world, self.flight.position, self.flight.axes()[1]);
                self.flight.portal_transit = None;
                self.sounds.push(crate::audio::Sound::Warp);
            }
            PAUSE => {
                self.page = Page::Paused;
                self.selection = 0;
            }
            RETRY => self.start(),
            WEAPON => self.mission.cycle_weapon(),
            BOMB => self.mission.bomb(),
            AUTOFIRE => self.mission.autofire = !self.mission.autofire,
            MODEL if self.mode == Mode::Free => self.flight.exploded = !self.flight.exploded,
            WINGMEN if self.mode == Mode::Free => {
                self.flight.wing_count = (self.flight.wing_count + 1) % 3
            }
            WEATHER if self.mode == Mode::Free => self.flight.weather = self.flight.weather.next(),
            TIME if self.mode == Mode::Free => self.theme = (self.theme + 1) % 3,
            MAP if self.mode == Mode::Free => {
                let kind = self.world.kind.next();
                self.loader.start(kind, kind.seed());
            }
            SEED if self.mode == Mode::Free => self
                .loader
                .start(self.world.kind, self.world.seed.wrapping_add(7919)),
            DEMO if self.mode == Mode::Free => self.flight.set_demo(!self.flight.demo),
            AUTOPILOT if self.mode == Mode::Free => {
                self.flight.set_demo(false);
                self.auto = !self.auto;
            }
            W | A | S | D => {
                self.auto = false;
                self.flight.set_demo(false);
            }
            _ => {}
        }
    }
    pub fn back(&mut self) {
        match self.page {
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
        } else if self.page == Page::Playing
            && ((self.mode == Mode::Combat && x >= 291 && y >= 154)
                || (self.mode != Mode::Combat && x >= 280 && y < 14))
        {
            self.flight.hud = !self.flight.hud;
        } else if self.page == Page::Results && (134..153).contains(&y) {
            self.result_action();
        } else if self.page == Page::Results && (108..130).contains(&y) {
            self.reward = (self.reward + 1) % 3;
        }
    }
}
