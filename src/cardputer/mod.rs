use crate::app::{App, Page};
pub use crate::app::{HEIGHT, WIDTH};
use crate::world::World;
use std::ops::{Deref, DerefMut};
pub mod imu;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod telemetry;

pub struct Game {
    pub app: App,
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
        let mut app = App::new(world);
        app.device_controls = true;
        Self {
            app,
            motion: imu::Motion::default(),
            imu_enabled: false,
            calibrate_imu: false,
        }
    }
    pub fn key(&mut self, code: u16, value: i32) {
        let code = crate::keys::cardputer_direction(code);
        if value == 1 && !self.app.keys.contains(&code) {
            if code == crate::keys::IMU {
                self.imu_enabled = !self.imu_enabled;
                self.auto = false;
                if self.page == Page::Playing {
                    self.flight.set_demo(false);
                }
                if self.imu_enabled {
                    self.calibrate_imu = true;
                }
            }
            if code == crate::keys::CALIBRATE {
                self.calibrate_imu = true;
            }
            if code == crate::keys::AUTOPILOT
                && self.page == Page::Playing
                && self.app.mode == crate::gameplay::Mode::Free
                && !self.auto
            {
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
        if !self.navigation.active
            && !self.flight.demo
            && self.imu_enabled
            && self.motion.available
            && self.motion.ready
        {
            let roll = imu::axis(self.motion.roll, 35.0_f32.to_radians());
            let pitch = imu::axis(self.motion.pitch, 30.0_f32.to_radians());
            if controls.turn == 0.0 {
                controls.turn = (roll + self.motion.yaw_rate * 0.28).clamp(-1.0, 1.0);
                controls.bank = Some(roll * 65.0_f32.to_radians());
            }
            if controls.climb == 0.0 {
                controls.climb = pitch;
                if self.flight.space {
                    controls.pitch_target = Some(pitch * (std::f32::consts::FRAC_PI_2 - 0.02));
                }
            }
            controls.manual = true;
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
