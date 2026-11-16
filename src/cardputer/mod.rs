use font8x8::UnicodeFonts;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::flight::{Controls, Flight};
use crate::render::Renderer;
use crate::world::World;

#[cfg(target_os = "linux")]
pub mod linux;

pub mod imu;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 170;

pub struct Game {
    pub world: World,
    pub flight: Flight,
    pub renderer: Renderer,
    pub screen: Vec<u8>,
    pub theme: usize,
    pub menu: bool,
    pub paused: bool,
    pub quit: bool,
    pub auto: bool,
    pub motion: imu::Motion,
    pub imu_enabled: bool,
    pub calibrate_imu: bool,
    keys: HashSet<u16>,
    escape_since: Option<Instant>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        let world = World::new(7);
        let flight = Flight::new(&world);
        Self {
            world,
            flight,
            renderer: Renderer::new(),
            screen: vec![0; WIDTH * HEIGHT * 4],
            theme: 0,
            menu: false,
            paused: false,
            quit: false,
            auto: true,
            motion: imu::Motion::default(),
            imu_enabled: false,
            calibrate_imu: false,
            keys: HashSet::new(),
            escape_since: None,
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
                self.menu = !self.menu;
            }
            return;
        }
        if value != 1 || !self.keys.insert(code) {
            return;
        }
        match code {
            1 => self.escape_since = Some(Instant::now()),
            40 | 5 | 57 => self.flight.view = (self.flight.view + 1) % 3,
            41 | 6 | 16 => self.flight.roll(),
            51 | 8 | 20 => self.theme = (self.theme + 1) % 3,
            52 | 9 | 35 => self.menu = !self.menu,
            28 => {
                self.menu = false;
                self.paused = false;
            }
            25 => self.paused = !self.paused,
            48 => self.flight.exploded = !self.flight.exploded,
            47 => self.flight.wing_count = (self.flight.wing_count + 1) % 3,
            19 => {
                self.flight = Flight::new(&self.world);
                self.auto = !self.imu_enabled;
            }
            23 => {
                self.imu_enabled = !self.imu_enabled;
                self.auto = false;
                if self.imu_enabled {
                    self.calibrate_imu = true;
                }
            }
            37 => self.calibrate_imu = true,
            15 => {
                self.auto = !self.auto;
                if self.auto {
                    self.imu_enabled = false;
                }
            }
            44 | 45 | 46 | 33 | 103 | 108 | 105 | 106 | 30 | 31 | 32 | 17 => self.auto = false,
            _ => {}
        }
    }

    pub fn release_keys(&mut self) {
        self.keys.clear();
        self.escape_since = None;
    }

    pub fn step(&mut self, dt: f32) {
        if self
            .escape_since
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(2))
        {
            self.quit = true;
        }
        if self.menu || self.paused {
            return;
        }
        let held = |codes: &[u16]| codes.iter().any(|code| self.keys.contains(code));
        let mut controls = Controls {
            bank: None,
            turn: f32::from(held(&[46, 106, 32])) - f32::from(held(&[44, 105, 30])),
            climb: f32::from(held(&[33, 103, 17])) - f32::from(held(&[45, 108, 31])),
            boost: held(&[43, 7, 42, 54]),
            manual: !self.auto,
        };
        if self.imu_enabled && self.motion.available && self.motion.ready {
            let roll = imu::axis(self.motion.roll, 35.0_f32.to_radians());
            let pitch = imu::axis(self.motion.pitch, 30.0_f32.to_radians());
            if controls.turn == 0.0 {
                controls.turn = (roll + self.motion.yaw_rate * 0.28).clamp(-1.0, 1.0);
                controls.bank = Some(roll * 65.0_f32.to_radians());
            }
            if controls.climb == 0.0 {
                controls.climb = pitch;
            }
            controls.manual = true;
        }
        self.flight.step(dt.min(0.1), &controls, &self.world);
    }

    pub fn draw(&mut self) {
        self.renderer
            .render(&self.world, &self.flight, WIDTH, HEIGHT, self.theme);
        self.screen.copy_from_slice(&self.renderer.rgba);
        shade(&mut self.screen, 0, 0, WIDTH, 11);
        text(
            &mut self.screen,
            3,
            2,
            ["TOP", "CHASE", "COCKPIT"][self.flight.view],
            [224, 241, 245],
        );
        text(
            &mut self.screen,
            76,
            2,
            &format!(
                "SPD {:03.0} ALT {:03.0}",
                self.flight.speed, self.flight.position.z
            ),
            [224, 241, 245],
        );
        text(&mut self.screen, 244, 2, "1X", [224, 241, 245]);
        text(
            &mut self.screen,
            276,
            2,
            if self.imu_enabled {
                if !self.motion.available {
                    "FAIL"
                } else if self.motion.ready {
                    "IMU"
                } else {
                    "CAL"
                }
            } else if self.auto {
                "AUTO"
            } else {
                "KEY"
            },
            [224, 241, 245],
        );
        if self.imu_enabled && !self.motion.ready && !self.menu {
            shade(&mut self.screen, 76, 18, 168, 12);
            text(
                &mut self.screen,
                80,
                20,
                "HOLD STILL / K ZERO",
                [255, 205, 135],
            );
        }
        if self.menu || self.paused {
            shade(&mut self.screen, 10, 15, 300, 150);
            let lines = [
                "RUST FLY / PAUSED",
                "Z X C F   LEFT DOWN RIGHT UP",
                "4 SPACE   VIEW     5 Q  ROLL",
                "6 SHIFT   BOOST    7 T  LIGHT",
                "8 H ESC   MENU     V    WINGS",
                "I MOTION  K ZERO   TAB  AUTO",
                "B SLICES  R RESET  P    PAUSE",
                "ENTER RESUME  HOLD ESC EXIT",
            ];
            for (row, line) in lines.iter().enumerate() {
                text(
                    &mut self.screen,
                    22,
                    23 + row * 17,
                    line,
                    if row == 0 {
                        [255, 178, 91]
                    } else {
                        [225, 237, 240]
                    },
                );
            }
        } else if self.flight.view != 2 {
            shade(&mut self.screen, 0, HEIGHT - 11, WIDTH, 11);
            text(
                &mut self.screen,
                4,
                HEIGHT - 9,
                "4 VIEW 5 ROLL 6 BOOST 7 LIGHT 8 MENU",
                [225, 237, 240],
            );
        }
    }
}

fn shade(buffer: &mut [u8], x: usize, y: usize, width: usize, height: usize) {
    for yy in y..(y + height).min(HEIGHT) {
        for xx in x..(x + width).min(WIDTH) {
            let at = (yy * WIDTH + xx) * 4;
            for channel in 0..3 {
                buffer[at + channel] /= 3;
            }
        }
    }
}

fn text(buffer: &mut [u8], x: usize, y: usize, value: &str, color: [u8; 3]) {
    for (i, ch) in value.chars().enumerate() {
        if let Some(glyph) = font8x8::BASIC_FONTS.get(ch) {
            for (gy, row) in glyph.iter().enumerate() {
                for gx in 0..8 {
                    let px = x + i * 8 + gx;
                    let py = y + gy;
                    if row & (1 << gx) != 0 && px < WIDTH && py < HEIGHT {
                        let at = (py * WIDTH + px) * 4;
                        buffer[at..at + 3].copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

pub fn rgb565(rgba: &[u8], destination: &mut [u8]) {
    for (src, dst) in rgba.chunks_exact(4).zip(destination.chunks_exact_mut(2)) {
        let pixel =
            ((src[0] as u16 >> 3) << 11) | ((src[1] as u16 >> 2) << 5) | (src[2] as u16 >> 3);
        dst.copy_from_slice(&pixel.to_le_bytes());
    }
}
