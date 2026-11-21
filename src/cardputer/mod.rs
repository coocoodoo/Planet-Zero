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
                controls.pitch_target = Some(pitch * std::f32::consts::FRAC_PI_2);
            }
            controls.manual = true;
        }
        self.app.step(dt, Some(controls));
        // 加载完成后的自动开局同样交给体感控制
        if self.app.page == Page::Playing && self.imu_enabled {
            self.auto = false;
        }
    }
    pub fn draw(&mut self) {
        self.app.draw();
    }
}

pub fn rgb565(rgba: &[u8], destination: &mut [u8]) {
    const DITHER: [u16; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    for (i, (src, dst)) in rgba
        .chunks_exact(4)
        .zip(destination.chunks_exact_mut(2))
        .enumerate()
    {
        let threshold = DITHER[(i / WIDTH % 4) * 4 + i % 4] * 16 + 8;
        let r = (src[0] as u16 * 31 + threshold) / 255;
        let g = (src[1] as u16 * 63 + threshold) / 255;
        let b = (src[2] as u16 * 31 + threshold) / 255;
        let pixel = (r << 11) | (g << 5) | b;
        dst.copy_from_slice(&pixel.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_preserves_endpoints_and_smooths_gradual_brightness() {
        let count = WIDTH * 4;
        let mut rgba = vec![255; count * 4];
        let mut packed = vec![0; count * 2];
        let mut previous = [0; 3];
        for value in 0..=255 {
            for p in rgba.chunks_exact_mut(4) {
                p[..3].fill(value);
            }
            rgb565(&rgba, &mut packed);
            let mut totals = [0_usize; 3];
            for p in packed.chunks_exact(2) {
                let color = u16::from_le_bytes([p[0], p[1]]);
                for (sum, channel) in
                    totals
                        .iter_mut()
                        .zip([color >> 11, (color >> 5) & 63, color & 31])
                {
                    *sum += channel as usize;
                }
            }
            for channel in 0..3 {
                assert!(totals[channel] >= previous[channel]);
                let limit = if channel == 1 { count / 4 } else { count / 8 };
                assert!(totals[channel] - previous[channel] <= limit);
            }
            if value == 0 || value == 255 {
                assert!(packed.iter().all(|&b| b == value));
            }
            previous = totals;
        }
    }
}
