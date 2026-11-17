use crate::app::{App, Page};
pub use crate::app::{HEIGHT, WIDTH};
use crate::world::World;
use std::ops::{Deref, DerefMut};
mod gestures;
pub mod imu;
#[cfg(target_os = "linux")]
pub mod linux;

pub struct Game {
    pub app: App,
    gestures: gestures::Gestures,
    pub motion: imu::Motion,
    pub imu_enabled: bool,
    pub calibrate_imu: bool,
}
impl Deref for Game {
    type Target = App;
    fn deref(&self) -> &App {
        &self.app
    }
}
impl DerefMut for Game {
    fn deref_mut(&mut self) -> &mut App {
        &mut self.app
    }
}
impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
impl Game {
    pub fn new() -> Self {
        Self::with_world(World::new(7))
    }
    pub fn with_world(world: World) -> Self {
        Self {
            app: App::new(world),
            gestures: gestures::Gestures::default(),
            motion: imu::Motion::default(),
            imu_enabled: false,
            calibrate_imu: false,
        }
    }
    pub fn key(&mut self, code: u16, value: i32) {
        if value == 1 && !self.app.keys.contains(&code) {
            if code == 23 {
                self.imu_enabled = !self.imu_enabled;
                self.auto = false;
                if self.page == Page::Playing {
                    self.flight.set_demo(false);
                }
                if self.imu_enabled {
                    self.calibrate_imu = true;
                }
            }
            if code == 37 {
                self.calibrate_imu = true;
            }
            if code == 15 && self.app.mode == crate::gameplay::Mode::Free && !self.auto {
                self.imu_enabled = false;
            }
        }
        self.app.key(code, value);
        if self.app.page == Page::Playing && self.imu_enabled {
            self.auto = false;
        }
    }
    pub fn release_keys(&mut self) {
        self.app.release_keys();
    }
    pub fn step(&mut self, dt: f32) {
        let mut controls = self.app.controls();
        self.app.input_label = if self.imu_enabled {
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
        };
        if !self.flight.demo && self.imu_enabled && self.motion.available && self.motion.ready {
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
            if self.page == Page::Playing && self.mission.countdown <= 0.0 {
                if let Some(direction) = self.gestures.update(dt, roll)
                    && self.flight.roll_toward(direction)
                {
                    self.mission.dodge();
                    self.sounds.push(crate::audio::Sound::Roll);
                }
            } else {
                self.gestures.reset();
            }
        }
        if self.flight.demo || !self.imu_enabled || !self.motion.available || !self.motion.ready {
            self.gestures.reset();
        }
        self.app.step(dt, Some(controls));
    }
    pub fn draw(&mut self) {
        self.app.draw();
    }
}

pub fn rgb565(rgba: &[u8], destination: &mut [u8]) {
    for (src, dst) in rgba.chunks_exact(4).zip(destination.chunks_exact_mut(2)) {
        let pixel =
            ((src[0] as u16 >> 3) << 11) | ((src[1] as u16 >> 2) << 5) | (src[2] as u16 >> 3);
        dst.copy_from_slice(&pixel.to_le_bytes());
    }
}
