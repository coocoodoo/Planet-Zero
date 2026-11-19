use glam::{Quat, Vec3, vec3};
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default)]
pub struct Motion {
    pub roll: f32,
    pub pitch: f32,
    pub yaw_rate: f32,
    pub ready: bool,
    pub available: bool,
    pub samples: u64,
    pub read_ms: f32,
    pub acceleration: [f32; 3],
    pub gyro: [f32; 3],
}

pub struct Fusion {
    gravity: Vec3,
    neutral: Vec3,
    right: Vec3,
    forward: Vec3,
    bias: Vec3,
    sum_accel: Vec3,
    sum_gyro: Vec3,
    sum_accel_squared: f32,
    sum_gyro_squared: f32,
    calibration_time: f32,
    calibration_count: usize,
    pub motion: Motion,
}

impl Default for Fusion {
    fn default() -> Self {
        Self {
            gravity: Vec3::Z,
            neutral: Vec3::Z,
            right: Vec3::X,
            forward: Vec3::Y,
            bias: Vec3::ZERO,
            sum_accel: Vec3::ZERO,
            sum_gyro: Vec3::ZERO,
            sum_accel_squared: 0.0,
            sum_gyro_squared: 0.0,
            calibration_time: 0.0,
            calibration_count: 0,
            motion: Motion::default(),
        }
    }
}

impl Fusion {
    pub fn calibrate(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, acceleration: Vec3, gyro: Vec3, dt: f32) -> Motion {
        self.motion.samples += 1;
        self.motion.available = true;
        self.motion.acceleration = acceleration.to_array();
        self.motion.gyro = gyro.to_array();
        let magnitude = acceleration.length();
        if !self.motion.ready {
            self.sum_accel += acceleration;
            self.sum_gyro += gyro;
            self.sum_accel_squared += acceleration.length_squared();
            self.sum_gyro_squared += gyro.length_squared();
            self.calibration_time += dt;
            self.calibration_count += 1;
            if self.calibration_time < 0.35 {
                return self.motion;
            }
            let count = self.calibration_count as f32;
            let mean_accel = self.sum_accel / count;
            let variance = self.sum_accel_squared / count - mean_accel.length_squared();
            let stable = (8.5..10.8).contains(&mean_accel.length())
                && self.sum_gyro_squared / count < 0.15_f32.powi(2)
                && variance < 0.4_f32.powi(2);
            if stable {
                self.neutral = self.sum_accel.normalize();
                self.gravity = self.neutral;
                self.bias = self.sum_gyro / self.calibration_count as f32;
                // 设备屏幕长边沿 BMI270 的 Y 轴
                let screen_right = Vec3::Y;
                self.right = (screen_right - self.neutral * screen_right.dot(self.neutral))
                    .normalize_or_zero();
                if self.right.length_squared() < 0.5 {
                    self.right = Vec3::Z.cross(self.neutral).normalize();
                }
                self.forward = self.neutral.cross(self.right);
                self.motion.ready = true;
            } else {
                self.sum_accel = Vec3::ZERO;
                self.sum_gyro = Vec3::ZERO;
                self.sum_accel_squared = 0.0;
                self.sum_gyro_squared = 0.0;
                self.calibration_time = 0.0;
                self.calibration_count = 0;
            }
            return self.motion;
        }
        let angular = gyro - self.bias;
        self.gravity = Quat::from_scaled_axis(-angular * dt) * self.gravity;
        if (8.3..11.3).contains(&magnitude) {
            self.gravity = self
                .gravity
                .lerp(acceleration / magnitude, 1.0 - (-2.5 * dt).exp())
                .normalize();
        }
        let up = self.gravity.dot(self.neutral);
        let roll = -self.gravity.dot(self.right).atan2(up);
        let pitch = self.gravity.dot(self.forward).atan2(up);
        let gain = 1.0 - (-14.0 * dt).exp();
        self.motion.roll += (roll - self.motion.roll) * gain;
        self.motion.pitch += (pitch - self.motion.pitch) * gain;
        self.motion.yaw_rate += (-angular.dot(self.gravity) - self.motion.yaw_rate) * gain;
        self.motion
    }
}

pub fn axis(value: f32, extent: f32) -> f32 {
    let deadzone = 2.5_f32.to_radians();
    value.signum() * ((value.abs() - deadzone) / (extent - deadzone)).clamp(0.0, 1.0)
}

struct Reader {
    files: Vec<File>,
    accel_scale: f32,
    gyro_scale: f32,
    pub path: PathBuf,
}
impl Reader {
    fn open() -> io::Result<Self> {
        let path = if let Some(path) = std::env::var_os("FLIGHT_CLUB_IMU") {
            PathBuf::from(path)
        } else {
            std::fs::read_dir("/sys/bus/iio/devices")?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    std::fs::read_to_string(path.join("name"))
                        .is_ok_and(|name| name.trim() == "bmi270")
                })
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到 BMI270"))?
        };
        let scale = |name: &str| -> io::Result<f32> {
            std::fs::read_to_string(path.join(name))?
                .trim()
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "IMU 比例参数无效"))
        };
        let files = [
            "in_accel_x_raw",
            "in_accel_y_raw",
            "in_accel_z_raw",
            "in_anglvel_x_raw",
            "in_anglvel_y_raw",
            "in_anglvel_z_raw",
        ]
        .iter()
        .map(|name| File::open(path.join(name)))
        .collect::<io::Result<Vec<_>>>()?;
        Ok(Self {
            files,
            accel_scale: scale("in_accel_scale")?,
            gyro_scale: scale("in_anglvel_scale")?,
            path,
        })
    }
    fn read(&mut self) -> io::Result<(Vec3, Vec3)> {
        let mut values = [0.0; 6];
        for (file, value) in self.files.iter_mut().zip(&mut values) {
            file.rewind()?;
            let mut text = [0_u8; 32];
            let n = file.read(&mut text)?;
            *value = std::str::from_utf8(&text[..n])
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "IMU 采样值无效"))?;
        }
        Ok((
            vec3(values[0], values[1], values[2]) * self.accel_scale,
            vec3(values[3], values[4], values[5]) * self.gyro_scale,
        ))
    }
}

pub struct Sensor {
    state: Arc<Mutex<(Motion, Instant)>>,
    calibrate: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    pub path: PathBuf,
}
impl Sensor {
    pub fn open() -> io::Result<Self> {
        let mut reader = Reader::open()?;
        reader.read()?;
        let state = Arc::new(Mutex::new((Motion::default(), Instant::now())));
        let calibrate = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (out, reset, exit) = (state.clone(), calibrate.clone(), stop.clone());
        let path = reader.path.clone();
        let worker = std::thread::spawn(move || {
            let mut fusion = Fusion::default();
            let mut previous = Instant::now();
            while !exit.load(Ordering::Relaxed) {
                let start = Instant::now();
                if reset.swap(false, Ordering::Relaxed) {
                    fusion.calibrate();
                }
                match reader.read() {
                    Ok((accel, gyro)) => {
                        let mut motion = fusion.update(
                            accel,
                            gyro,
                            (start - previous).as_secs_f32().clamp(0.001, 0.05),
                        );
                        motion.read_ms = start.elapsed().as_secs_f32() * 1000.0;
                        *out.lock().unwrap() = (motion, Instant::now());
                    }
                    Err(_) => {
                        out.lock().unwrap().0.available = false;
                    }
                }
                previous = start;
                if let Some(wait) = Duration::from_millis(10).checked_sub(start.elapsed()) {
                    std::thread::sleep(wait);
                }
            }
        });
        Ok(Self {
            state,
            calibrate,
            stop,
            worker: Some(worker),
            path,
        })
    }
    pub fn motion(&self) -> Motion {
        let (mut motion, time) = *self.state.lock().unwrap();
        if time.elapsed() > Duration::from_millis(150) {
            motion.available = false;
        }
        motion
    }
    pub fn calibrate(&self) {
        self.calibrate.store(true, Ordering::Relaxed);
    }
}
impl Drop for Sensor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn diagnose(seconds: f32, output: Option<&Path>) -> io::Result<()> {
    use std::io::Write;
    let sensor = Sensor::open()?;
    eprintln!("BMI270: {}", sensor.path.display());
    let mut file = output.map(File::create).transpose()?;
    if let Some(file) = &mut file {
        writeln!(
            file,
            "seconds,ready,roll_deg,pitch_deg,yaw_deg_s,ax,ay,az,gx,gy,gz,read_ms"
        )?;
    }
    let start = Instant::now();
    while start.elapsed().as_secs_f32() < seconds {
        std::thread::sleep(Duration::from_millis(100));
        let m = sensor.motion();
        let line = format!(
            "{:.3},{},{:.3},{:.3},{:.3},{:.5},{:.5},{:.5},{:.5},{:.5},{:.5},{:.3}",
            start.elapsed().as_secs_f32(),
            m.ready,
            m.roll.to_degrees(),
            m.pitch.to_degrees(),
            m.yaw_rate.to_degrees(),
            m.acceleration[0],
            m.acceleration[1],
            m.acceleration[2],
            m.gyro[0],
            m.gyro[1],
            m.gyro[2],
            m.read_ms
        );
        if let Some(file) = &mut file {
            writeln!(file, "{line}")?;
        } else {
            println!("{line}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibrated(gravity: Vec3, bias: Vec3) -> Fusion {
        let mut filter = Fusion::default();
        for _ in 0..100 {
            filter.update(gravity, bias, 0.01);
        }
        assert!(filter.motion.ready);
        filter
    }

    #[test]
    fn calibration_tolerates_isolated_sensor_noise() {
        let mut filter = Fusion::default();
        for i in 0..50 {
            let noise = if i % 25 == 0 { 0.65 } else { 0.0 };
            filter.update(vec3(noise, 0.0, 9.81), vec3(0.01, -0.02, 0.03), 0.01);
        }
        assert!(filter.motion.ready);
        assert!(filter.motion.pitch.abs() < 0.01);
        assert!(filter.bias.distance(vec3(0.01, -0.02, 0.03)) < 0.0001);
    }

    #[test]
    fn tilted_grip_calibrates_and_removes_gyro_bias() {
        let gravity = vec3(-8.6, 0.1, 4.6).normalize() * 9.81;
        let bias = vec3(0.01, -0.02, 0.03);
        let mut filter = calibrated(gravity, bias);
        for _ in 0..1000 {
            filter.update(gravity, bias, 0.01);
        }
        assert!(filter.motion.roll.abs() < 0.001);
        assert!(filter.motion.pitch.abs() < 0.001);
        assert!(filter.motion.yaw_rate.abs() < 0.001);
    }

    #[test]
    fn screen_right_tilt_turns_right_and_top_lift_climbs() {
        let mut right = calibrated(Vec3::Z * 9.81, Vec3::ZERO);
        let rad = 25.0_f32.to_radians();
        let acceleration = vec3(0.0, -rad.sin(), rad.cos()) * 9.81;
        for _ in 0..200 {
            right.update(acceleration, Vec3::ZERO, 0.01);
        }
        assert!((right.motion.roll.to_degrees() - 25.0).abs() < 0.3);
        assert!(axis(right.motion.roll, 35.0_f32.to_radians()) > 0.6);
        let mut up = calibrated(Vec3::Z * 9.81, Vec3::ZERO);
        for _ in 0..200 {
            up.update(vec3(-rad.sin(), 0.0, rad.cos()) * 9.81, Vec3::ZERO, 0.01);
        }
        assert!((up.motion.pitch.to_degrees() - 25.0).abs() < 0.3);
    }

    #[test]
    fn moving_device_waits_for_calibration_and_recenter_resets() {
        let mut filter = Fusion::default();
        for _ in 0..200 {
            filter.update(Vec3::Z * 9.81, Vec3::X, 0.01);
        }
        assert!(!filter.motion.ready);
        for _ in 0..100 {
            filter.update(Vec3::Z * 9.81, Vec3::ZERO, 0.01);
        }
        assert!(filter.motion.ready);
        filter.calibrate();
        assert!(!filter.motion.ready);
        assert_eq!(axis(2.0_f32.to_radians(), 0.6), 0.0);
    }

    #[test]
    fn gyro_predicts_tilt_between_accelerometer_samples() {
        let mut filter = calibrated(Vec3::Z * 9.81, Vec3::ZERO);
        for _ in 0..20 {
            filter.update(Vec3::ZERO, vec3(-1.0, 0.0, 0.0), 0.01);
        }
        assert!(filter.motion.roll > 0.1);
        assert!(filter.gravity.is_normalized());
    }
}
