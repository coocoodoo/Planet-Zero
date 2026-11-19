mod audio;
pub(crate) mod chart;
mod director;
mod input;
pub mod navigation;
pub(crate) mod orbit;
mod records;
mod settings;
pub mod telemetry;
#[cfg(test)]
mod tests;
mod travel;
use crate::content::vehicles::Vehicle;
use crate::flight::{Controls, Flight};
use crate::gameplay::progression::{self as campaign, Loadout, Upgrade};
use crate::gameplay::{Difficulty, Mission, Mode};
use crate::render::Renderer;
use crate::world::{World, WorldLoader};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 170;
pub const START_ROW: usize = 6;
pub const MENU_ROWS: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Menu,
    Playing,
    Paused,
    Settings,
    Chart,
    Results,
    Help,
    Loading,
}

#[derive(Default)]
pub struct PerfStats {
    pub fps: f32,
    pub fps_avg: f32,
    pub fps_min: f32,
    pub frame_ms: f32,
    pub frame_peak: f32,
    pub render_ms: f32,
    pub cpu: f32,
    pub mem_mb: f32,
    pub curve: Vec<f32>,
    window: f32,
    age: f32,
    cpu_secs: Option<f64>,
}

impl PerfStats {
    fn record(&mut self, dt: f32) {
        if self.frame_ms <= 0.0 {
            self.frame_ms = dt * 1000.0;
        }
        self.frame_ms += (dt * 1000.0 - self.frame_ms) * 0.15;
        self.frame_peak = self.frame_peak.max(self.frame_ms);
        let instant = (1000.0 / self.frame_ms.max(0.1)).min(240.0);
        self.fps += (instant - self.fps) * 0.15;
        self.age += dt;
        self.window += dt;
        if self.age < 1.5 || self.window < 0.1 {
            return;
        }
        let cpu = process_cpu_seconds();
        if let (Some(now), Some(previous)) = (cpu, self.cpu_secs) {
            let cores = cpu_cores().max(1.0);
            let load = (now - previous) / self.window as f64 / cores as f64 * 100.0;
            self.cpu += (load as f32 - self.cpu) * 0.4;
        }
        self.cpu_secs = cpu;
        self.mem_mb = process_max_rss_mb().unwrap_or(0.0);
        self.window = 0.0;
        self.curve.push(self.fps);
        if self.curve.len() > 96 {
            self.curve.remove(0);
        }
        let settled = &self.curve[self.curve.len().min(5)..];
        if !settled.is_empty() {
            self.fps_avg = settled.iter().sum::<f32>() / settled.len() as f32;
            self.fps_min = settled.iter().copied().fold(f32::INFINITY, f32::min);
        }
    }
}

#[cfg(unix)]
fn process_cpu_seconds() -> Option<f64> {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return None;
        }
        let user = usage.ru_utime.tv_sec as f64 + usage.ru_utime.tv_usec as f64 / 1_000_000.0;
        let system = usage.ru_stime.tv_sec as f64 + usage.ru_stime.tv_usec as f64 / 1_000_000.0;
        Some(user + system)
    }
}

#[cfg(not(unix))]
fn process_cpu_seconds() -> Option<f64> {
    None
}

#[cfg(unix)]
fn cpu_cores() -> f32 {
    unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN).max(1) as f32 }
}

#[cfg(not(unix))]
fn cpu_cores() -> f32 {
    1.0
}

#[cfg(unix)]
fn process_max_rss_mb() -> Option<f32> {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return None;
        }
        #[cfg(target_os = "macos")]
        let bytes = usage.ru_maxrss as f64;
        #[cfg(not(target_os = "macos"))]
        let bytes = usage.ru_maxrss as f64 * 1024.0;
        Some((bytes / 1_048_576.0) as f32)
    }
}

#[cfg(not(unix))]
fn process_max_rss_mb() -> Option<f32> {
    None
}

pub struct App {
    pub(crate) orbit: orbit::Orbit,
    pub navigation: navigation::Navigation,
    pitch_gesture: input::PitchGesture,
    pub telemetry: telemetry::Telemetry,
    pub(crate) minimap: chart::Chart,
    pub device_controls: bool,
    pub(crate) chart: chart::Chart,
    chart_return: Page,
    director: director::Director,
    pub portal_labels: bool,
    settings_path: Option<PathBuf>,
    settings_return: (Page, usize),
    pub audio_settings: crate::audio::Settings,
    pub sounds: crate::audio::Events,
    pub level: u32,
    pub reward: usize,
    pub run_score: u32,
    loadout: Loadout,
    pending_start: bool,

    pub world: std::sync::Arc<World>,
    pub network: crate::world::network::Network,
    pub travel: Option<crate::world::network::Route>,
    travel_return: Page,
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
    pub perf_show: bool,
    pub perf: PerfStats,
    escape_since: Option<Instant>,
}

impl App {
    pub fn new(world: World) -> Self {
        let mut flight = Flight::new(&world);
        let mission = Mission::new(Mode::Free, Difficulty::Cadet, &mut flight, &world);
        flight.set_demo(true);
        let record_path = std::env::var_os("FLIGHT_CLUB_RECORDS")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share"))
                    })
                    .map(|p| p.join("flight-club/records-v1.txt"))
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
        let settings_path = std::env::var_os("FLIGHT_CLUB_SETTINGS")
            .map(PathBuf::from)
            .or_else(|| {
                record_path
                    .as_ref()
                    .map(|p| p.with_file_name("settings-v1.toml"))
            });
        let settings = settings::Settings::load(settings_path.as_deref());
        flight.faceted = settings.faceted;
        Self {
            orbit: orbit::Orbit::default(),
            navigation: navigation::Navigation::default(),
            pitch_gesture: Default::default(),
            telemetry: telemetry::Telemetry::default(),
            minimap: chart::Chart::default(),
            device_controls: false,
            chart: chart::Chart::default(),
            chart_return: Page::Playing,
            portal_labels: settings.portal_labels,
            settings_path,
            settings_return: (Page::Menu, 0),
            director: director::Director::default(),
            audio_settings: crate::audio::Settings::default(),
            sounds: crate::audio::Events::default(),
            level: 1,
            reward: 0,
            run_score: 0,
            loadout: Loadout::default(),
            pending_start: false,
            network: crate::world::network::Network::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(world.seed, |time| time.subsec_nanos() ^ world.seed),
            ),
            world: std::sync::Arc::new(world),
            travel: None,
            travel_return: Page::Playing,
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
            perf_show: false,
            perf: PerfStats::default(),
            escape_since: None,
        }
    }
    pub fn start(&mut self) {
        self.orbit = orbit::Orbit::default();
        self.navigation = navigation::Navigation::default();
        self.pitch_gesture = Default::default();
        self.director = director::Director::default();
        if self.page == Page::Menu {
            self.loadout = Loadout::default();
            if self.flight.vehicle == Vehicle::Atlas {
                self.loadout.bombs = 3;
            }
            self.run_score = 0;
        }
        if self.loader.pending.is_some() {
            self.pending_start = true;
            self.page = Page::Loading;
            return;
        }
        self.pending_start = false;
        self.mission = Mission::for_level(
            self.mode,
            self.difficulty,
            &mut self.flight,
            &self.world,
            self.level,
        );
        if self.mode == Mode::Combat {
            self.mission.equip(&self.loadout);
        }
        if let Ok(time) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            self.mission
                .shuffle_encounters(time.subsec_nanos() ^ self.world.seed);
        }
        self.auto = self.mode == Mode::Free;
        self.flight.hover = false;
        self.page = Page::Playing;
        self.new_record = false;
        self.best_score = self
            .records
            .iter()
            .find(|(key, _)| key == &self.record_key())
            .map_or(0, |r| r.1);
        self.keys.clear();
        if self.mission.countdown > 0.0 {
            self.sounds.push(crate::audio::Sound::Countdown(3));
        }
    }
    pub fn result_action(&mut self) {
        if self.page != Page::Results {
            return;
        }
        if self.mission.outcome == Some(true) {
            if self.mode == Mode::Combat {
                self.loadout = self.mission.loadout();
                self.loadout.upgrade(Upgrade::ALL[self.reward]);
            }
            self.run_score += self.mission.score;
            self.level = self.level.saturating_add(1);
            self.load_operation();
        }
        self.start();
    }
    fn load_operation(&mut self) {
        let op = campaign::operation(self.level);
        self.flight.weather = op.weather;
        let seed = op
            .map
            .seed()
            .wrapping_add(((self.level - 1) / 6).wrapping_mul(7919));
        if self.world.kind != op.map || self.world.seed != seed {
            self.loader.start(op.map, seed);
        }
    }
    pub fn home(&mut self) {
        self.orbit = orbit::Orbit::default();
        self.flight.surface = None;
        self.flight.sky_rotation = glam::Quat::IDENTITY;
        self.flight.camera_adjustment = None;
        self.flight.orbit_quiet = false;
        self.flight.orbital_scenery = None;
        self.page = Page::Menu;
        self.travel = None;
        self.flight.portal_transit = None;
        self.selection = START_ROW;
        self.pending_start = false;
        self.flight.wing_count = 2;
        self.flight.exploded = false;
        self.flight.set_demo(true);
        self.keys.clear();
    }
    pub fn step(&mut self, dt: f32, external: Option<Controls>) {
        let dt = dt.clamp(0.0, 0.1);
        self.perf.record(dt);
        self.network.poll();
        if let Some(route) = self.travel {
            if self.arrive(route) {
                self.travel = None;
                self.page = self.travel_return;
            } else {
                self.network.request(route.target);
                return;
            }
        }
        if let Some(world) = self.loader.poll() {
            self.orbit = orbit::Orbit::default();
            self.navigation = navigation::Navigation::default();
            self.flight.change_world(&world);
            self.world = std::sync::Arc::new(world);
            if self.pending_start {
                self.start();
            }
        }
        if self
            .escape_since
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(2))
        {
            self.quit = true;
        }
        if self.page != Page::Playing {
            self.pitch_gesture = Default::default();
        }
        if self.page == Page::Chart {
            let held = |key| f32::from(self.keys.contains(&key));
            let movement = glam::vec2(
                held(crate::keys::RIGHT) - held(crate::keys::LEFT),
                held(crate::keys::DOWN) - held(crate::keys::UP),
            );
            if movement.length_squared() > 0.0 {
                self.chart
                    .pan(&self.world, self.flight.position, movement * dt * 0.65);
            }
            self.network.ensure(&self.world);
            if self.world.kind == crate::world::MapKind::Space {
                self.prepare_orbit();
            } else {
                let routes = self.network.routes(&self.world);
                if let Some(route) = routes.get(self.chart.selected % routes.len().max(1)) {
                    self.network.request(route.target);
                }
            }
            return;
        }
        self.direct_demo();
        if self.page == Page::Loading
            || self.page == Page::Menu
            || (self.page == Page::Help && self.help_return == Page::Menu)
        {
            let previous = self.flight.position;
            self.flight.step(dt, &Controls::default(), &self.world);
            if !self.cross_world(previous) && !self.network.active {
                self.flight.traverse_portal(&self.world, previous);
            }
            return;
        }
        if self.page != Page::Playing {
            return;
        }
        let held = |key| f32::from(self.keys.contains(&key));
        let look = glam::vec2(
            held(crate::keys::LOOK_RIGHT) - held(crate::keys::LOOK_LEFT),
            held(crate::keys::LOOK_UP) - held(crate::keys::LOOK_DOWN),
        )
        .clamp_length_max(1.0)
            * 30.0_f32.to_radians();
        self.flight.camera_look += (look - self.flight.camera_look) * crate::flight::ease(7.0, dt);
        let mut controls = external.unwrap_or_else(|| self.controls());
        // 俯瞰限制作用于玩家输入，导航与地形保护保持完整控制
        controls.heading_only = self.flight.weights[0] > 0.5 && controls.manual;
        if controls.heading_only {
            controls.climb = 0.0;
            controls.bank = None;
            controls.pitch_target = None;
        }
        let mut controls = self.navigation_controls().unwrap_or(controls);
        // 悬停中按加速恢复油门，方向键调整姿态
        if self.flight.hover && controls.manual && controls.boost {
            self.flight.hover = false;
            controls.throttle = Some(1.0);
        }
        self.orbit_controls(&mut controls, dt);
        let mut previous = self.flight.position;
        if self.mission.countdown <= 0.0 {
            self.flight.step(dt, &controls, &self.world);
        }
        if self.step_orbit(dt) {
            return;
        }
        if self.cross_world(previous) {
            return;
        }
        if !self.network.active && self.flight.traverse_portal(&self.world, previous) {
            previous = self.flight.position;
            self.sounds.push(crate::audio::Sound::Warp);
        }
        let fire = self.mode == Mode::Combat && self.keys.contains(&crate::keys::FIRE);
        self.mission
            .step(dt, &mut self.flight, &self.world, previous, fire);
        if self.mission.outcome.is_some() {
            self.page = Page::Results;
            self.reward = 0;
            self.save_record();
        }
    }
    pub fn prepare_scene(&mut self) {
        if (self.page == Page::Playing || self.flight.demo) && self.loader.pending.is_none() {
            self.network.active = true;
            if self.prepare_orbit() {
                self.network.ensure(&self.world);
                return;
            }
            self.network
                .preload(&self.world, self.flight.position, self.flight.axes()[1]);
        }
    }
    pub fn scene_is_menu(&self) -> bool {
        self.page == Page::Loading
            || self.page == Page::Menu
            || (self.page == Page::Help && self.help_return == Page::Menu)
    }
    pub fn draw_overlay(&mut self) {
        self.screen.fill(0);
        crate::ui::draw(self);
    }
    pub fn draw(&mut self) {
        if self.page == Page::Chart {
            crate::ui::draw(self);
            return;
        }
        self.prepare_scene();
        let menu = self.scene_is_menu();
        self.renderer.render_connected(
            &self.world,
            &self.flight,
            (WIDTH, HEIGHT),
            self.theme,
            if menu { None } else { Some(&self.mission) },
            Some(&self.network),
        );
        self.screen.copy_from_slice(&self.renderer.rgba);
        self.perf.render_ms = self.renderer.stages.iter().sum::<f64>() as f32;
        crate::ui::draw(self);
    }
}
