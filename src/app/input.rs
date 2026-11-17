use super::*;

impl App {
    pub fn release_keys(&mut self) {
        self.keys.clear();
        self.escape_since = None;
    }
    pub fn controls(&self) -> Controls {
        let held = |codes: &[u16]| codes.iter().any(|code| self.keys.contains(code));
        Controls {
            bank: None,
            aerobatic: true,
            turn: f32::from(held(&[46, 106, 32])) - f32::from(held(&[44, 105, 30])),
            climb: f32::from(held(&[33, 103, 17])) - f32::from(held(&[45, 108, 31])),
            boost: held(&[43, 7, 42, 54]),
            manual: !self.auto,
        }
    }
    pub fn key(&mut self, code: u16, value: i32) {
        if value == 0 {
            self.keys.remove(&code);
            if code == 1
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
        if code == 1 {
            self.escape_since = Some(Instant::now());
            return;
        }
        if matches!(code, 10 | 12 | 13) {
            match code {
                10 => self.audio_settings.profile = (self.audio_settings.profile + 1) % 4,
                12 => self.audio_settings.volume = self.audio_settings.volume.saturating_sub(1),
                _ => self.audio_settings.volume = (self.audio_settings.volume + 1).min(10),
            }
            self.sounds.push(crate::audio::Sound::Navigate);
            return;
        }
        if matches!(
            self.page,
            Page::Menu | Page::Paused | Page::Help | Page::Results
        ) {
            if matches!(code, 28 | 57) {
                self.sounds.push(crate::audio::Sound::Confirm);
            } else if matches!(
                code,
                17 | 33 | 103 | 31 | 45 | 108 | 30 | 44 | 105 | 32 | 46 | 106
            ) {
                self.sounds.push(crate::audio::Sound::Navigate);
            }
        }
        if code == 22 {
            self.flight.hud = !self.flight.hud;
            return;
        }
        if self.page == Page::Help {
            match code {
                44 | 46 | 105 | 106 => self.help_page = (self.help_page + 1) % 5,
                28 | 35 | 52 | 9 => self.page = self.help_return,
                _ => {}
            }
            return;
        }
        if self.page == Page::Menu {
            match code {
                17 | 33 | 103 => self.selection = (self.selection + MENU_ROWS - 1) % MENU_ROWS,
                31 | 45 | 108 => self.selection = (self.selection + 1) % MENU_ROWS,
                30 | 44 | 105 => self.adjust(-1),
                32 | 46 | 106 => self.adjust(1),
                28 | 57 => self.activate(),
                35 | 52 | 9 => self.help(),
                _ => {}
            }
            return;
        }
        if self.page == Page::Paused {
            match code {
                44 | 105 => {
                    self.audio_settings.volume = self.audio_settings.volume.saturating_sub(1)
                }
                46 | 106 => self.audio_settings.volume = (self.audio_settings.volume + 1).min(10),
                17 | 33 | 103 => self.selection = (self.selection + 5) % 6,
                31 | 45 | 108 => self.selection = (self.selection + 1) % 6,
                28 | 57 => self.activate(),
                25 | 35 | 52 | 9 => self.page = Page::Playing,
                _ => {}
            }
            return;
        }
        if self.page == Page::Results {
            match code {
                19 => self.start(),
                28 | 57 => self.result_action(),
                44 | 105 => self.reward = (self.reward + 2) % 3,
                46 | 106 => self.reward = (self.reward + 1) % 3,
                35 | 52 | 9 => self.home(),
                _ => {}
            }
            return;
        }
        if self.page == Page::Loading {
            return;
        }
        match code {
            21 | 94 | 11 => self.mission.activate_overdrive(),
            40 | 5 | 57 => self.flight.view = (self.flight.view + 1) % 3,
            41 | 6 | 16 => {
                if self.flight.roll() {
                    self.sounds.push(crate::audio::Sound::Roll);
                    self.mission.dodge();
                }
            }
            51 | 8 | 20 => self.theme = (self.theme + 1) % 3,
            35 => {
                self.network
                    .summon(&self.world, self.flight.position, self.flight.axes()[1]);
                self.flight.portal_transit = None;
                self.sounds.push(crate::audio::Sound::Warp);
            }
            52 | 9 | 25 => {
                self.page = Page::Paused;
                self.selection = 0;
            }
            19 => self.start(),
            24 => self.mission.cycle_weapon(),
            18 | 26 | 2 => self.mission.bomb(),
            27 | 3 => self.mission.autofire = !self.mission.autofire,
            48 if self.mode == Mode::Free => self.flight.exploded = !self.flight.exploded,
            47 if self.mode == Mode::Free => {
                self.flight.wing_count = (self.flight.wing_count + 1) % 3
            }
            34 if self.mode == Mode::Free => self.flight.weather = self.flight.weather.next(),
            50 if self.mode == Mode::Free => {
                let kind = self.world.kind.next();
                self.loader.start(kind, kind.seed());
            }
            49 if self.mode == Mode::Free => self
                .loader
                .start(self.world.kind, self.world.seed.wrapping_add(7919)),
            36 if self.mode == Mode::Free => self.flight.set_demo(!self.flight.demo),
            15 if self.mode == Mode::Free => {
                self.flight.set_demo(false);
                self.auto = !self.auto;
            }
            44 | 45 | 46 | 33 | 103 | 108 | 105 | 106 | 30 | 31 | 32 | 17 => {
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
                7 => self.help(),
                8 => self.quit = true,
                _ => {}
            },
            Page::Paused => match self.selection {
                0 => self.page = Page::Playing,
                1 => self.start(),
                2 => self.flight.hud = !self.flight.hud,
                3 => self.help(),
                4 => self.home(),
                5 => self.quit = true,
                _ => {}
            },
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
                self.flight.weather =
                    crate::content::weather::Weather::ALL[next(self.flight.weather as usize, 6)]
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
