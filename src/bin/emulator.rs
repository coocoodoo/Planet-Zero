//! Cardputer Zero emulator for the PC.
//!
//! Runs the device build's own game object (`cardputer::Game`: evdev key codes with the F/X/Z/C
//! arrow remap, gyroscope steering, RGB565 framebuffer with the device's dither) on a fixed
//! 30 fps clock. The BMI270 is simulated: a virtual tilt produces accelerometer and gyroscope
//! samples at 100 Hz that go through the device's real `imu::Fusion` (calibration included).
//!
//! It plays a script and saves screenshots inside a picture of the device:
//!
//! ```text
//! flight-club-emulator --script scripts/emulator/tour.txt --out target/emulator
//! ```
//!
//! Script lines (`#` starts a comment):
//!
//! | Line | Meaning |
//! | --- | --- |
//! | `fresh` | delete the emulator's save file (new game) |
//! | `start` | select CARDPUTER SKY in the menu and press OK |
//! | `wait S` | run S seconds |
//! | `press KEY [S]` | press and release (hold S seconds, default 0.1) |
//! | `down KEY` / `up KEY` | hold / release |
//! | `tilt ROLL PITCH [S]` | tilt the device (degrees; right roll and nose-up pitch are positive) over S seconds |
//! | `shot NAME` | save `NAME.png` (device frame) and `NAME-screen.png` (raw 320×170) |
//! | `log` | print the game state |
//! | `caption TEXT` | caption shown under the device in a `--video` recording |
//! | `record off` / `record on` | pause and resume a `--video` recording (to skip long waits) |
//! | `expect FIELD VALUE` | check `phase`, `overlay`, `location`, `page`, `imu`; `expect-min FIELD N` for `credits`, `discoveries`, `item:<name>`, `altitude`, `systems` |
//! | `land` | descend toward the ground, then press 6 (helper for scripts) |
//! | `bench S` | time S seconds of frames and count triangles |
//! | `until FIELD VALUE S` | run until a field has a value (at most S seconds) |
//! | `until-min FIELD N S` | run until a number field reaches N (at most S seconds) |
//! | `mine-until-drones S` | on foot: aim at props and hold 5 until a drone patrol arrives |
//! | `swap-to N` | on foot: press 7 until weapon slot N (1–3) is in hand |
//! | `face-drone` | on foot: turn and look at the nearest drone with the arrow keys |
//! | `fight N S` | on foot: face the nearest drone and hold 5 until N drones are destroyed |
//! | `goto-poi KIND S` | on foot: walk to the nearest point of interest of a kind (cache, ruin…) |
//! | `goto-ship` | on foot: turn and walk to the parked ship with the arrow keys |
//! | `aim` | on foot: walk to the nearest prop with the arrow keys until it is under the crosshair |
//!
//! KEY is a key cap name: `0`–`9`, `a`–`z`, `ok`/`enter`, `esc`, `space`, `tab`, `shift`, `alt`,
//! `f1`, or `up`/`down`/`left`/`right` (sent as the physical F/X/Z/C keys).
use flight_club::app::Page;
use flight_club::cardputer::{Game, HEIGHT, WIDTH, imu::Fusion, rgb565};
use flight_club::sky::{Location, Phase};
use glam::{Quat, Vec3, vec3};
use std::path::{Path, PathBuf};

#[path = "emulator/video.rs"]
mod video;

const FRAME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cardputer-frame.rgba"));
const FRAME_SIZE: (usize, usize) = (631, 408);
const SCREEN_AT: (usize, usize) = (155, 22);
const FPS: f32 = 30.0;
const IMU_RATE: usize = 3;

struct Device {
    game: Game,
    fusion: Fusion,
    tilt: (f32, f32),
    tilt_from: (f32, f32),
    tilt_to: (f32, f32),
    tilt_time: (f32, f32),
    gravity: Vec3,
    framebuffer: Vec<u8>,
    out: PathBuf,
    time: f32,
    failures: usize,
    video: Option<video::Video>,
    caption: String,
    held: Vec<u16>,
}

/// Key cap label for the video's status line.
fn key_label(code: u16) -> String {
    match code {
        1 => "ESC".into(),
        2..=10 => (code - 1).to_string(),
        11 => "0".into(),
        15 => "TAB".into(),
        28 => "OK".into(),
        42 => "SHIFT".into(),
        57 => "SPACE".into(),
        59 => "F1".into(),
        16..=25 => "QWERTYUIOP"[(code - 16) as usize..][..1].into(),
        30..=38 => "ASDFGHJKL"[(code - 30) as usize..][..1].into(),
        44..=50 => "ZXCVBNM"[(code - 44) as usize..][..1].into(),
        _ => format!("#{code}"),
    }
}

fn key_code(name: &str) -> Option<u16> {
    let name = name.to_ascii_lowercase();
    Some(match name.as_str() {
        "esc" => 1,
        "0" => 11,
        "ok" | "enter" => 28,
        "space" => 57,
        "tab" => 15,
        "shift" => 42,
        "alt" => 56,
        "f1" => 59,
        "f2" => 60,
        "up" => 33,
        "down" => 45,
        "left" => 44,
        "right" => 46,
        n if n.len() == 1 => {
            let c = n.chars().next()?;
            if let Some(d) = c.to_digit(10) {
                return Some(d as u16 + 1);
            }
            let row = "qwertyuiop";
            let home = "asdfghjkl";
            let bottom = "zxcvbnm";
            if let Some(i) = row.find(c) {
                16 + i as u16
            } else if let Some(i) = home.find(c) {
                30 + i as u16
            } else if let Some(i) = bottom.find(c) {
                44 + i as u16
            } else {
                return None;
            }
        }
        _ => return None,
    })
}

impl Device {
    fn new(out: PathBuf) -> Self {
        let mut game = Game::new();
        game.mode = flight_club::gameplay::Mode::Explore;
        game.imu_enabled = true;
        game.calibrate_imu = true;
        Self {
            game,
            fusion: Fusion::default(),
            tilt: (0.0, 0.0),
            tilt_from: (0.0, 0.0),
            tilt_to: (0.0, 0.0),
            tilt_time: (0.0, 0.0),
            gravity: Vec3::Z,
            framebuffer: vec![0; WIDTH * HEIGHT * 2],
            out,
            time: 0.0,
            failures: 0,
            video: None,
            caption: String::new(),
            held: Vec::new(),
        }
    }
    /// Accelerometer reading for a tilt, in the BMI270 frame the device uses: the screen's
    /// long edge is +Y and the device lies with +Z up when level.
    fn gravity_for(roll: f32, pitch: f32) -> Vec3 {
        vec3(-pitch.to_radians().tan(), -roll.to_radians().tan(), 1.0).normalize()
    }
    fn sample_imu(&mut self, dt: f32) {
        let (t, length) = self.tilt_time;
        let k = if length <= 0.0 {
            1.0
        } else {
            (t / length).clamp(0.0, 1.0)
        };
        let k = k * k * (3.0 - 2.0 * k);
        self.tilt = (
            self.tilt_from.0 + (self.tilt_to.0 - self.tilt_from.0) * k,
            self.tilt_from.1 + (self.tilt_to.1 - self.tilt_from.1) * k,
        );
        self.tilt_time.0 += dt;
        let gravity = Self::gravity_for(self.tilt.0, self.tilt.1);
        // gyro rate that rotates the previous gravity onto the new one (as the fusion expects)
        let rotation = Quat::from_rotation_arc(self.gravity, gravity);
        let gyro = -rotation.to_scaled_axis() / dt;
        self.gravity = gravity;
        self.game.motion = self.fusion.update(gravity * 9.81, gyro, dt);
    }
    fn frame(&mut self) {
        let dt = 1.0 / FPS;
        if self.game.calibrate_imu {
            self.fusion.calibrate();
            self.game.calibrate_imu = false;
        }
        for _ in 0..IMU_RATE {
            self.sample_imu(dt / IMU_RATE as f32);
        }
        self.game.step(dt);
        self.game.draw();
        rgb565(&self.game.screen, &mut self.framebuffer);
        self.time += dt;
        // take the recorder out while building its frame (that reads the rest of the device)
        if let Some(mut video) = self.video.take() {
            video.audio(self.game.audio_frame());
            let framed = self.framed();
            video
                .frame(&framed, &self.caption, &self.status())
                .expect("ffmpeg stopped");
            self.video = Some(video);
        }
    }
    /// Held keys and the device tilt, for the video's status line.
    fn status(&self) -> String {
        let mut parts = Vec::new();
        if !self.held.is_empty() {
            let keys: Vec<String> = self.held.iter().map(|&k| key_label(k)).collect();
            parts.push(format!("KEYS {}", keys.join(" ")));
        }
        if self.tilt.0.abs() > 0.5 || self.tilt.1.abs() > 0.5 {
            parts.push(format!("TILT R{:+.0} P{:+.0}", self.tilt.0, self.tilt.1));
        }
        parts.join("   ")
    }
    fn wait(&mut self, seconds: f32) {
        for _ in 0..(seconds * FPS).round().max(1.0) as usize {
            self.frame();
        }
    }
    fn key(&mut self, code: u16, value: i32) {
        if value == 1 && !self.held.contains(&code) {
            self.held.push(code);
        } else if value == 0 {
            self.held.retain(|&k| k != code);
        }
        self.game.key(code, value);
    }
    /// Framebuffer decoded back to RGB, as the LCD shows it.
    fn screen_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(WIDTH * HEIGHT * 4);
        for p in self.framebuffer.chunks_exact(2) {
            let v = u16::from_le_bytes([p[0], p[1]]);
            let (r, g, b) = ((v >> 11) & 31, (v >> 5) & 63, v & 31);
            rgba.extend_from_slice(&[
                (r * 255 / 31) as u8,
                (g * 255 / 63) as u8,
                (b * 255 / 31) as u8,
                255,
            ]);
        }
        rgba
    }
    /// The screen inside the picture of the device (631×408 RGBA).
    fn framed(&self) -> Vec<u8> {
        let screen = self.screen_rgba();
        let (fw, _) = FRAME_SIZE;
        let mut framed = FRAME.to_vec();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let at = ((SCREEN_AT.1 + y) * fw + SCREEN_AT.0 + x) * 4;
                framed[at..at + 4].copy_from_slice(&screen[(y * WIDTH + x) * 4..][..4]);
            }
        }
        for p in framed.chunks_exact_mut(4) {
            if p[3] < 255 {
                let a = p[3] as u32;
                for channel in p.iter_mut().take(3) {
                    *channel = ((*channel as u32 * a + 24 * (255 - a)) / 255) as u8;
                }
                p[3] = 255;
            }
        }
        framed
    }
    fn shot(&self, name: &str) {
        let screen = self.screen_rgba();
        let (fw, fh) = FRAME_SIZE;
        let mut framed = FRAME.to_vec();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let at = ((SCREEN_AT.1 + y) * fw + SCREEN_AT.0 + x) * 4;
                framed[at..at + 4].copy_from_slice(&screen[(y * WIDTH + x) * 4..][..4]);
            }
        }
        // the bezel image has a transparent background: put it on dark grey
        for p in framed.chunks_exact_mut(4) {
            if p[3] < 255 {
                let a = p[3] as u32;
                for channel in p.iter_mut().take(3) {
                    *channel = ((*channel as u32 * a + 24 * (255 - a)) / 255) as u8;
                }
                p[3] = 255;
            }
        }
        let png = flight_club::sky::png::write_rgba;
        png(&self.out.join(format!("{name}.png")), &framed, fw, fh).expect("write screenshot");
        png(
            &self.out.join(format!("{name}-screen.png")),
            &screen,
            WIDTH,
            HEIGHT,
        )
        .expect("write screenshot");
        println!("shot {name} at {:.1}s", self.time);
    }
    fn field(&self, field: &str) -> String {
        let g = &self.game;
        let sky = &g.sky;
        match field {
            "page" => format!("{:?}", g.page).to_lowercase(),
            "phase" => match sky.phase {
                Phase::Flying => "flying",
                Phase::Landing { .. } => "landing",
                Phase::OnFoot => "onfoot",
                Phase::Takeoff { .. } => "takeoff",
                Phase::Docking { .. } => "docking",
                Phase::Docked => "docked",
            }
            .into(),
            "overlay" => format!("{:?}", sky.overlay).to_lowercase(),
            "location" => match sky.location {
                Location::Space => "space".into(),
                Location::Planet(_) => "planet".into(),
                Location::Elsewhere => "elsewhere".into(),
            },
            "imu" => g.input_label.to_lowercase(),
            "context" => g.sky_context().label().to_lowercase(),
            _ => "?".into(),
        }
    }
    fn number(&self, field: &str) -> f32 {
        let g = &self.game;
        let sky = &g.sky;
        if let Some(item) = field.strip_prefix("item:") {
            return flight_club::sky::inventory::Item::ALL
                .iter()
                .find(|i| i.name().eq_ignore_ascii_case(&item.replace('_', " ")))
                .map_or(-1.0, |&i| sky.inventory.count(i) as f32);
        }
        match field {
            "credits" => sky.credits as f32,
            "drones" => sky.drones.len() as f32,
            "drones_destroyed" => sky.drones_destroyed as f32,
            "weapons" => sky.loadout.count() as f32,
            "alert" => sky.alert.level,
            "suit" => sky.vitals.suit,
            "contract" => f32::from(sky.contract.is_some()),
            "discoveries" => sky.discoveries as f32,
            "systems" => sky.visited.len() as f32,
            "altitude" => {
                let p = g.flight.position;
                p.z - g.world.height(p.x, p.y)
            }
            "life" => sky.vitals.life,
            _ => f32::NAN,
        }
    }
    fn log(&self) {
        let g = &self.game;
        let eye = g.sky.walker.position;
        let drones: Vec<String> = g
            .sky
            .drones
            .iter()
            .map(|d| {
                format!(
                    "{}:{:.0}u/{:.0}hp/z{:+.0}/{}",
                    d.kind.spec().name,
                    d.position.distance(eye),
                    d.health,
                    d.position.z - eye.z,
                    if d.sight { "sight" } else { "hidden" }
                )
            })
            .collect();
        {
            let slots: Vec<String> = g
                .sky
                .loadout
                .all()
                .iter()
                .map(|w| w.map_or("-".to_owned(), |w| w.archetype.spec().short.to_owned()))
                .collect();
            println!(
                "     context {} active {} slots {}",
                g.sky_context().label(),
                g.sky.loadout.active,
                slots.join(",")
            );
        }
        if !drones.is_empty() {
            let weapon = g.sky.loadout.weapon().map(|w| w.name()).unwrap_or_default();
            println!(
                "     drones {} weapon {weapon} heat {:.2}",
                drones.join(" "),
                g.sky.loadout.heat[g.sky.loadout.active]
            );
        }
        let sky = &g.sky;
        let p = g.flight.position;
        let place = match sky.location {
            Location::Planet(i) => {
                let planet = &sky.system.planets[i];
                format!("{} ({})", planet.name, planet.biome.name())
            }
            Location::Space => "space".into(),
            Location::Elsewhere => format!("{:?}", g.world.kind),
        };
        println!(
            "t={:.1}s page={} phase={} overlay={} system={} at={} pos=({:.0},{:.0},{:.0}) alt={:.0} speed={:.0} credits={} data={} discoveries={} imu={} roll={:.1} pitch={:.1} life={:.2} hazard={:.2} props={} walker=({:.1},{:.1},{:.1}) grounded={} autodock={} forward=({:.2},{:.2},{:.2}) transition={} inventory={:?}",
            self.time,
            self.field("page"),
            self.field("phase"),
            self.field("overlay"),
            sky.system.name,
            place,
            p.x,
            p.y,
            p.z,
            self.number("altitude"),
            g.flight.speed,
            sky.credits,
            sky.data,
            sky.discoveries,
            g.input_label,
            g.motion.roll.to_degrees(),
            g.motion.pitch.to_degrees(),
            sky.vitals.life,
            sky.vitals.hazard,
            sky.props.len(),
            sky.walker.position.x,
            sky.walker.position.y,
            sky.walker.position.z,
            sky.walker.grounded,
            sky.autodock,
            g.flight.axes()[1].x,
            g.flight.axes()[1].y,
            g.flight.axes()[1].z,
            g.orbit_transition(),
            sky.inventory.counts,
        );
    }
    /// Turn toward the nearest prop and nudge the view until it is under the crosshair.
    fn aim_nearest_prop(&mut self) {
        let eye = self.game.sky.walker.position;
        let Some(prop) = self
            .game
            .sky
            .props
            .iter()
            .filter(|p| p.harvest(flight_club::sky::biome::Biome::Lush).is_some())
            .min_by(|a, b| {
                a.position
                    .distance(eye)
                    .total_cmp(&b.position.distance(eye))
            })
            .copied()
        else {
            return;
        };
        self.walk_to(prop.position, 7.0, 20.0);
        for _ in 0..150 {
            if self.game.aimed_prop().is_some() {
                return;
            }
            let camera = flight_club::flight::Camera::new(&self.game.flight, WIDTH, HEIGHT);
            let Some((p, _, _)) = camera.project(prop.center()) else {
                return;
            };
            self.nudge(p);
        }
    }
    /// One frame of arrow-key aiming toward a screen point.
    fn nudge(&mut self, p: glam::Vec2) {
        let (dx, dy) = (p.x - 160.0, p.y - 71.4);
        let mut held = Vec::new();
        if dx.abs() > 3.0 {
            held.push(if dx > 0.0 { 46 } else { 44 });
        }
        if dy.abs() > 3.0 {
            held.push(if dy > 0.0 { 31 } else { 17 });
        }
        for &k in &held {
            self.key(k, 1);
        }
        self.frame();
        for &k in &held {
            self.key(k, 0);
        }
    }
    /// Aim at the nearest drone for up to `frames` frames (turning first if it is behind).
    fn face_drone(&mut self, frames: usize) {
        for _ in 0..frames {
            let eye = self.game.sky.walker.eye().position;
            let Some(target) = self
                .game
                .sky
                .drones
                .iter()
                .min_by(|a, b| {
                    a.position
                        .distance(eye)
                        .total_cmp(&b.position.distance(eye))
                })
                .map(|d| d.position)
            else {
                self.frame();
                return;
            };
            let camera = flight_club::flight::Camera::new(&self.game.flight, WIDTH, HEIGHT);
            match camera.project(target) {
                Some((p, depth, _)) if depth > -3.0 && (0.0..320.0).contains(&p.x) => self.nudge(p),
                _ => {
                    self.key(46, 1);
                    self.frame();
                    self.key(46, 0);
                }
            }
        }
    }
    /// Walk with the arrow keys (as the player would) until within `stop` units of `target`.
    fn walk_to(&mut self, target: Vec3, stop: f32, seconds: f32) {
        let (up, left, right, jet) = (33, 44, 46, 7);
        let mut last = self.game.sky.walker.position;
        for frame in 0..(seconds * FPS) as usize {
            // stuck against a slope: jet over it, as a player would
            if frame % 30 == 29 {
                let now = self.game.sky.walker.position;
                if now.truncate().distance(last.truncate()) < 2.0 {
                    self.key(jet, 1);
                    self.key(up, 1);
                    self.wait(0.6);
                    self.key(jet, 0);
                    self.key(up, 0);
                }
                last = self.game.sky.walker.position;
            }
            let walker = self.game.sky.walker;
            let d = target - walker.position;
            if d.truncate().length() < stop {
                break;
            }
            let want = d.x.atan2(-d.y);
            let diff = flight_club::flight::angle(want - walker.yaw);
            let turn = if diff > 0.12 {
                Some(right)
            } else if diff < -0.12 {
                Some(left)
            } else {
                None
            };
            if let Some(k) = turn {
                self.key(k, 1);
            }
            if diff.abs() < 0.5 {
                self.key(up, 1);
                // run (shift) on long walks
                if d.truncate().length() > 30.0 {
                    self.key(42, 1);
                }
            }
            self.frame();
            for k in [up, left, right, 42] {
                self.key(k, 0);
            }
        }
        self.frame();
    }
    fn check(&mut self, ok: bool, what: String) {
        if ok {
            println!("ok   {what}");
        } else {
            self.failures += 1;
            println!("FAIL {what}");
            self.log();
        }
    }
    fn run(&mut self, script: &str) {
        for (n, line) in script.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let arg = |i: usize| words.get(i).copied().unwrap_or("");
            let number = |i: usize, default: f32| arg(i).parse::<f32>().unwrap_or(default);
            let code = || {
                key_code(arg(1)).unwrap_or_else(|| panic!("line {}: unknown key {}", n + 1, arg(1)))
            };
            match words[0] {
                "fresh" => {
                    if let Some(path) = flight_club::sky::save::path() {
                        let _ = std::fs::remove_file(path);
                    }
                }
                "start" => {
                    self.game.mode = flight_club::gameplay::Mode::Explore;
                    self.game.selection = flight_club::app::START_ROW;
                    self.key(28, 1);
                    self.frame();
                    self.key(28, 0);
                    // wait for the planet to generate and the session to begin
                    let mut guard = 0;
                    while self.game.page != Page::Playing && guard < 1200 {
                        self.frame();
                        guard += 1;
                    }
                }
                "wait" => self.wait(number(1, 1.0)),
                "press" => {
                    let code = code();
                    self.key(code, 1);
                    self.wait(number(2, 0.1));
                    self.key(code, 0);
                    self.frame();
                }
                "down" => {
                    let code = code();
                    self.key(code, 1);
                }
                "up" => {
                    let code = code();
                    self.key(code, 0);
                }
                "tilt" => {
                    self.tilt_from = self.tilt;
                    self.tilt_to = (number(1, 0.0), number(2, 0.0));
                    self.tilt_time = (0.0, number(3, 0.4));
                }
                "shot" => self.shot(arg(1)),
                "caption" => {
                    self.caption = words[1..].join(" ");
                }
                "record" => {
                    if let Some(video) = &mut self.video {
                        video.recording = arg(1) != "off";
                    }
                }
                "log" => self.log(),
                "expect" => {
                    let value = self.field(arg(1));
                    let ok = value == arg(2).to_lowercase();
                    self.check(ok, format!("{} = {} (is {value})", arg(1), arg(2)));
                }
                "expect-min" => {
                    let value = self.number(arg(1));
                    let ok = value >= number(2, 0.0);
                    self.check(ok, format!("{} >= {} (is {value:.1})", arg(1), arg(2)));
                }
                "land" => {
                    // dive with the arrow key until low enough, level off, then press 6; a
                    // player over water or rough ground keeps flying and tries again
                    for _ in 0..8 {
                        let mut guard = 0;
                        while self.number("altitude") > 150.0 && guard < 900 {
                            self.key(45, 1);
                            self.frame();
                            guard += 1;
                        }
                        self.key(45, 0);
                        self.wait(0.6);
                        self.key(7, 1);
                        self.frame();
                        self.key(7, 0);
                        if self.field("phase") != "flying" {
                            break;
                        }
                        self.wait(2.0);
                    }
                    self.wait(3.6);
                }
                "until" => {
                    let limit = (number(3, 60.0) * FPS) as usize;
                    let mut frames = 0;
                    while self.field(arg(1)) != arg(2).to_lowercase() && frames < limit {
                        self.frame();
                        frames += 1;
                    }
                    let value = self.field(arg(1));
                    self.check(
                        value == arg(2).to_lowercase(),
                        format!(
                            "{} = {} after {:.1}s (is {value})",
                            arg(1),
                            arg(2),
                            frames as f32 / FPS
                        ),
                    );
                }
                "until-min" => {
                    let limit = (number(3, 60.0) * FPS) as usize;
                    let goal = number(2, 0.0);
                    let mut frames = 0;
                    while self.number(arg(1)) < goal && frames < limit {
                        self.frame();
                        frames += 1;
                    }
                    let value = self.number(arg(1));
                    self.check(
                        value >= goal,
                        format!(
                            "{} >= {goal} after {:.1}s (is {value:.1})",
                            arg(1),
                            frames as f32 / FPS
                        ),
                    );
                }
                "mine-until-drones" => {
                    let limit = (number(1, 60.0) * FPS) as usize;
                    let mut frames = 0;
                    while self.game.sky.drones.is_empty() && frames < limit {
                        if self.game.aimed_prop().is_none() {
                            self.key(6, 0);
                            self.aim_nearest_prop();
                        }
                        self.key(6, 1);
                        self.frame();
                        frames += 1;
                    }
                    self.key(6, 0);
                    let n = self.game.sky.drones.len();
                    self.check(
                        n > 0,
                        format!("a drone patrol arrived after {:.1}s", frames as f32 / FPS),
                    );
                }
                "swap-to" => {
                    // press 7 (as a player would) until the wanted slot is in hand
                    let slot = (number(1, 1.0) as usize).saturating_sub(1).min(2);
                    for _ in 0..3 {
                        if self.game.sky.loadout.active == slot {
                            break;
                        }
                        self.key(8, 1);
                        self.wait(0.1);
                        self.key(8, 0);
                        self.wait(0.6);
                    }
                    let active = self.game.sky.loadout.active;
                    self.check(active == slot, format!("slot {} in hand", slot + 1));
                }
                "face-drone" => {
                    self.face_drone(90);
                }
                "fight" => {
                    let goal = self.game.sky.drones_destroyed + number(1, 1.0) as u32;
                    let limit = (number(2, 60.0) * FPS) as usize;
                    let mut frames = 0;
                    while self.game.sky.drones_destroyed < goal && frames < limit {
                        if self.game.sky.drones.is_empty() {
                            self.frame();
                            frames += 1;
                            continue;
                        }
                        // aim with the arrows while holding fire, like a player
                        self.key(6, 1);
                        self.face_drone(1);
                        frames += 1;
                        // switch weapons when this one is hot or cannot reach, as a player would
                        let eye = self.game.sky.walker.position;
                        let nearest = self
                            .game
                            .sky
                            .drones
                            .iter()
                            .map(|d| d.position.distance(eye))
                            .fold(f32::INFINITY, f32::min);
                        let l = &self.game.sky.loadout;
                        let short =
                            l.stats().is_some_and(|s| nearest > s.range * 0.9) && frames % 30 == 0;
                        if l.overheat[l.active] > 0.0 || short {
                            self.key(6, 0);
                            self.key(16, 1);
                            self.frame();
                            self.key(16, 0);
                        }
                    }
                    self.key(6, 0);
                    self.frame();
                    let done = self.game.sky.drones_destroyed;
                    self.check(
                        done >= goal,
                        format!(
                            "destroyed {done} drones (goal {goal}) in {:.1}s",
                            frames as f32 / FPS
                        ),
                    );
                }
                "goto-poi" => {
                    let kind = arg(1).to_uppercase();
                    let here = self.game.sky.walker.position;
                    let planet = self.game.sky.planet().cloned();
                    let mut found = Vec::new();
                    if let Some(planet) = planet {
                        flight_club::sky::props::nearby(
                            &self.game.world,
                            planet.seed,
                            planet.biome,
                            here.truncate(),
                            number(3, 500.0),
                            &self.game.sky.harvested,
                            &mut found,
                        );
                    }
                    let target = found
                        .iter()
                        .filter(|p| p.kind.label().contains(kind.as_str()))
                        .min_by(|a, b| {
                            a.position
                                .distance(here)
                                .total_cmp(&b.position.distance(here))
                        })
                        .copied();
                    match target {
                        Some(poi) => {
                            println!(
                                "     walking {:.0} units to a {}",
                                poi.position.distance(here),
                                poi.kind.label()
                            );
                            self.walk_to(poi.position, 3.5, number(2, 90.0));
                            let d = (self.game.sky.walker.position - poi.position)
                                .truncate()
                                .length();
                            self.check(
                                d < 8.0,
                                format!("reached the {} ({d:.1} units away)", poi.kind.label()),
                            );
                        }
                        None => self.check(false, format!("no {kind} within range")),
                    }
                }
                "bench" => {
                    // time the frame (game step + full render) and count what Sky adds
                    let frames = (number(1, 3.0) * FPS) as usize;
                    let started = std::time::Instant::now();
                    let mut triangles = 0_u64;
                    let mut extra = 0_u64;
                    for _ in 0..frames {
                        self.frame();
                        triangles += self.game.renderer.triangles as u64;
                        extra += self.game.renderer.extras.triangles.len() as u64;
                    }
                    let ms = started.elapsed().as_secs_f64() * 1000.0 / frames as f64;
                    println!(
                        "bench {}: {ms:.2} ms/frame on this PC (one thread), {} triangles/frame, {} from Cardputer Sky",
                        self.field("phase"),
                        triangles / frames as u64,
                        extra / frames as u64
                    );
                }
                "goto-ship" => {
                    let Some(ship) = self.game.sky.parked.map(|p| p.position) else {
                        self.check(false, "goto-ship: no parked ship".into());
                        continue;
                    };
                    self.walk_to(ship, 7.0, number(1, 120.0));
                    let d = self.game.sky.walker.position.distance(ship);
                    self.check(d < 9.0, format!("reached the ship ({d:.1} units away)"));
                }
                "aim" => {
                    let mut aimed = false;
                    for _ in 0..4 {
                        let eye = self.game.sky.walker.position;
                        let Some(prop) = self
                            .game
                            .sky
                            .props
                            .iter()
                            .filter(|p| {
                                !p.kind.is_poi()
                                    || p.kind == flight_club::sky::props::PropKind::Deposit
                            })
                            .min_by(|a, b| {
                                a.position
                                    .distance(eye)
                                    .total_cmp(&b.position.distance(eye))
                            })
                            .copied()
                        else {
                            break;
                        };
                        self.walk_to(prop.position, 7.0, 20.0);
                        // nudge the view toward the prop, as a player would
                        for _ in 0..150 {
                            if self.game.aimed_prop().is_some() {
                                aimed = true;
                                break;
                            }
                            let camera =
                                flight_club::flight::Camera::new(&self.game.flight, WIDTH, HEIGHT);
                            let Some((p, _, _)) = camera.project(prop.center()) else {
                                break;
                            };
                            let (dx, dy) = (p.x - 160.0, p.y - 71.4);
                            let mut held = Vec::new();
                            if dx.abs() > 3.0 {
                                held.push(if dx > 0.0 { 46 } else { 44 });
                            }
                            if dy.abs() > 3.0 {
                                held.push(if dy > 0.0 { 31 } else { 17 });
                            }
                            for &k in &held {
                                self.key(k, 1);
                            }
                            self.frame();
                            for &k in &held {
                                self.key(k, 0);
                            }
                        }
                        if aimed {
                            break;
                        }
                    }
                    self.check(aimed, "a prop is under the crosshair".into());
                }
                other => panic!("line {}: unknown command {other}", n + 1),
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("flight-club-emulator --script FILE [--out DIR] [--save FILE] [--video FILE.mp4]");
        return;
    }
    let out = PathBuf::from(value("--out").unwrap_or_else(|| "target/emulator".into()));
    // keep the emulator's progress apart from a real save
    let save = value("--save").map_or_else(|| out.join("sky-save.txt"), PathBuf::from);
    // SAFETY: single-threaded at this point; the variable is read when the game saves
    unsafe {
        std::env::set_var("FLIGHT_CLUB_SKY_SAVE", &save);
        std::env::set_var("FLIGHT_CLUB_RECORDS", out.join("records.txt"));
        std::env::set_var("FLIGHT_CLUB_SETTINGS", out.join("settings.toml"));
    }
    std::fs::create_dir_all(&out).expect("create output directory");
    let script = match value("--script") {
        Some(path) => std::fs::read_to_string(Path::new(&path)).expect("read script"),
        None => include_str!("../../scripts/emulator/tour.txt").to_owned(),
    };
    let mut device = Device::new(out);
    // the device boots into the launcher menu with the gyroscope calibrating
    device.wait(1.0);
    if let Some(path) = value("--video") {
        device.video = Some(video::Video::start(Path::new(&path)).expect("start ffmpeg"));
    }
    device.run(&script);
    if let Some(video) = device.video.take() {
        let frames = video.frames;
        let path = video.finish().expect("finish the video");
        println!("video {} ({:.1}s)", path.display(), frames as f32 / FPS);
    }
    if device.failures > 0 {
        eprintln!("{} check(s) failed", device.failures);
        std::process::exit(1);
    }
    println!("all checks passed");
}
