use crate::arcade::{Difficulty, Mission, Mode};
use crate::flight::{Controls, Flight};
use crate::render::Renderer;
use crate::vehicle::Vehicle;
use crate::world::{World, WorldLoader};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 170;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Menu,
    Playing,
    Paused,
    Results,
    Help,
}

pub struct App {
    pub world: World,
    pub loader: WorldLoader,
    pub flight: Flight,
    pub renderer: Renderer,
    pub screen: Vec<u8>,
    pub theme: usize,
    pub quit: bool,
    pub auto: bool,
    pub page: Page,
    pub selection: usize,
    pub help_page: usize,
    help_return: Page,
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub mission: Mission,
    pub input_label: &'static str,
    pub best_score: u32,
    pub new_record: bool,
    pub record_error: bool,
    records: Vec<(String, u32)>,
    record_path: Option<PathBuf>,
    pub keys: HashSet<u16>,
    escape_since: Option<Instant>,
}

impl App {
    pub fn new(world: World) -> Self {
        let mut flight = Flight::new(&world);
        let mission = Mission::new(Mode::Free, Difficulty::Cadet, &mut flight, &world);
        flight.set_demo(true);
        let record_path = std::env::var_os("RUST_FLY_RECORDS")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share"))
                    })
                    .map(|p| p.join("rust-fly/records-v1.txt"))
            });
        let records = record_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| {
                s.lines()
                    .filter_map(|line| {
                        let (key, value) = line.split_once(' ')?;
                        Some((key.to_owned(), value.parse().ok()?))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            world,
            loader: WorldLoader::default(),
            flight,
            renderer: Renderer::new(),
            screen: vec![0; WIDTH * HEIGHT * 4],
            theme: 0,
            quit: false,
            auto: true,
            page: Page::Menu,
            selection: 0,
            help_page: 0,
            help_return: Page::Menu,
            mode: Mode::Free,
            difficulty: Difficulty::Cadet,
            mission,
            input_label: "KEY",
            best_score: 0,
            new_record: false,
            record_error: false,
            records,
            record_path,
            keys: HashSet::new(),
            escape_since: None,
        }
    }

    pub fn start(&mut self) {
        if self.loader.pending.is_some() {
            return;
        }
        self.mission = Mission::new(self.mode, self.difficulty, &mut self.flight, &self.world);
        self.auto = self.mode == Mode::Free;
        self.page = Page::Playing;
        self.new_record = false;
        self.best_score = self
            .records
            .iter()
            .find(|(key, _)| key == &self.record_key())
            .map_or(0, |r| r.1);
        self.keys.clear();
    }

    pub fn home(&mut self) {
        self.page = Page::Menu;
        self.selection = 5;
        self.flight.wing_count = 2;
        self.flight.exploded = false;
        self.flight.set_demo(true);
        self.keys.clear();
    }

    pub fn controls(&self) -> Controls {
        let held = |codes: &[u16]| codes.iter().any(|code| self.keys.contains(code));
        Controls {
            bank: None,
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
        if code == 22 {
            self.flight.hud = !self.flight.hud;
            return;
        }
        if self.page == Page::Help {
            match code {
                44 | 46 | 105 | 106 => self.help_page = (self.help_page + 1) % 3,
                28 | 35 | 52 | 9 => self.page = self.help_return,
                _ => {}
            }
            return;
        }
        if self.page == Page::Menu {
            match code {
                17 | 33 | 103 => self.selection = (self.selection + 7) % 8,
                31 | 45 | 108 => self.selection = (self.selection + 1) % 8,
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
                19 | 28 | 57 => self.start(),
                35 | 52 | 9 => self.home(),
                _ => {}
            }
            return;
        }
        match code {
            40 | 5 | 57 => self.flight.view = (self.flight.view + 1) % 3,
            41 | 6 | 16 => {
                self.flight.roll();
                self.mission.dodge();
            }
            51 | 8 | 20 => self.theme = (self.theme + 1) % 3,
            52 | 9 | 35 | 25 => {
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
            Page::Results => self.home(),
        }
    }

    fn help(&mut self) {
        self.help_return = self.page;
        self.page = Page::Help;
        self.help_page = 0;
    }

    pub fn activate(&mut self) {
        match self.page {
            Page::Menu => match self.selection {
                0..=4 => self.adjust(1),
                5 => self.start(),
                6 => self.help(),
                7 => self.quit = true,
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
            Page::Results => self.start(),
            _ => {}
        }
    }

    fn adjust(&mut self, direction: i32) {
        let next = |i: usize, len: usize| (i as i32 + direction).rem_euclid(len as i32) as usize;
        match self.selection {
            0 => self.mode = Mode::ALL[next(self.mode as usize, 4)],
            1 => self.flight.vehicle = Vehicle::ALL[next(self.flight.vehicle as usize, 4)],
            2 => {
                let current = self.loader.pending.unwrap_or(self.world.kind);
                let kind = crate::world::MapKind::ALL[next(current as usize, 6)];
                self.loader.start(kind, kind.seed());
            }
            3 => {
                self.flight.weather =
                    crate::weather::Weather::ALL[next(self.flight.weather as usize, 6)]
            }
            4 => self.difficulty = Difficulty::ALL[next(self.difficulty as usize, 3)],
            _ => {}
        }
    }

    pub fn step(&mut self, dt: f32, external: Option<Controls>) {
        let dt = dt.clamp(0.0, 0.1);
        if let Some(world) = self.loader.poll() {
            self.flight.change_world(&world);
            self.world = world;
        }
        if self
            .escape_since
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(2))
        {
            self.quit = true;
        }
        if self.page == Page::Menu || (self.page == Page::Help && self.help_return == Page::Menu) {
            self.flight.step(dt, &Controls::default(), &self.world);
            return;
        }
        if self.page != Page::Playing {
            return;
        }
        let controls = external.unwrap_or_else(|| self.controls());
        let previous = self.flight.position;
        if self.mission.countdown <= 0.0 {
            self.flight.step(dt, &controls, &self.world);
        }
        let fire = self.keys.contains(&38) || self.keys.contains(&39) || self.keys.contains(&4);
        self.mission
            .step(dt, &mut self.flight, &self.world, previous, fire);
        if self.mission.outcome.is_some() {
            self.page = Page::Results;
            self.save_record();
        }
    }

    pub fn draw(&mut self) {
        let menu =
            self.page == Page::Menu || (self.page == Page::Help && self.help_return == Page::Menu);
        self.renderer.render_mission(
            &self.world,
            &self.flight,
            (WIDTH, HEIGHT),
            self.theme,
            if menu { None } else { Some(&self.mission) },
        );
        self.screen.copy_from_slice(&self.renderer.rgba);
        crate::ui::draw(self);
    }

    pub fn click(&mut self, x: usize, y: usize) {
        if self.page == Page::Menu && (36..148).contains(&y) {
            self.selection = ((y - 36) / 14).min(7);
            self.activate();
        } else if self.page == Page::Paused && (45..141).contains(&y) {
            self.selection = ((y - 45) / 16).min(5);
            self.activate();
        } else if self.page == Page::Playing && x >= 280 && y < 14 {
            self.flight.hud = !self.flight.hud;
        } else if self.page == Page::Results && y > 145 {
            self.start();
        }
    }

    fn record_key(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.mode as usize,
            self.difficulty as usize,
            self.flight.vehicle as usize,
            self.world.kind as usize,
            self.world.seed
        )
    }

    fn save_record(&mut self) {
        if self.mode == Mode::Free || self.mission.score <= self.best_score {
            return;
        }
        if self.mode == Mode::Race && self.mission.outcome != Some(true) {
            return;
        }
        let key = self.record_key();
        self.new_record = true;
        self.best_score = self.mission.score;
        if let Some(record) = self.records.iter_mut().find(|r| r.0 == key) {
            record.1 = self.best_score;
        } else {
            self.records.push((key, self.best_score));
        }
        if let Some(path) = &self.record_path {
            let data = self
                .records
                .iter()
                .map(|(key, value)| format!("{key} {value}\n"))
                .collect::<String>();
            let result = (|| -> std::io::Result<()> {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let temp = path.with_extension("tmp");
                std::fs::write(&temp, data)?;
                std::fs::rename(temp, path)
            })();
            self.record_error = result.is_err();
        }
    }
}
