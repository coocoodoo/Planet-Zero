use crate::app::telemetry::Telemetry;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

fn battery(text: &str) -> Telemetry {
    let field = |name: &str| {
        text.lines().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key == name).then_some(value.trim())
        })
    };
    if field("POWER_SUPPLY_PRESENT") == Some("0") {
        return Telemetry::default();
    }
    Telemetry {
        battery_percent: field("POWER_SUPPLY_CAPACITY")
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|&v| v <= 100),
        battery_voltage: field("POWER_SUPPLY_VOLTAGE_NOW")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0)
            .map(|v| v as f32 / 1_000_000.0),
        charging: field("POWER_SUPPLY_STATUS") == Some("Charging"),
        ..Telemetry::default()
    }
}

fn wireless(text: &str) -> Option<i16> {
    text.lines()
        .filter_map(|line| {
            let (_, values) = line.split_once(':')?;
            values
                .split_whitespace()
                .nth(2)?
                .trim_end_matches('.')
                .parse::<i16>()
                .ok()
        })
        .filter(|v| (-127..0).contains(v))
        .max()
}

pub struct Sensor {
    state: Arc<Mutex<(Telemetry, Instant)>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Sensor {
    pub fn start() -> Self {
        let battery_path = std::fs::read_dir("/sys/class/power_supply")
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|p| p.path())
            .find(|p| std::fs::read_to_string(p.join("type")).is_ok_and(|t| t.trim() == "Battery"))
            .map(|p| p.join("uevent"));
        Self::spawn(battery_path, PathBuf::from("/proc/net/wireless"))
    }
    fn spawn(battery_path: Option<PathBuf>, wireless_path: PathBuf) -> Self {
        let state = Arc::new(Mutex::new((Telemetry::default(), Instant::now())));
        let stop = Arc::new(AtomicBool::new(false));
        let (out, exit) = (state.clone(), stop.clone());
        let worker = std::thread::spawn(move || {
            while !exit.load(Ordering::Relaxed) {
                let mut sample = battery_path
                    .as_ref()
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .map_or_else(Telemetry::default, |s| battery(&s));
                sample.wifi_dbm = std::fs::read_to_string(&wireless_path)
                    .ok()
                    .and_then(|s| wireless(&s));
                *out.lock().unwrap() = (sample, Instant::now());
                std::thread::park_timeout(Duration::from_secs(1));
            }
        });
        Self {
            state,
            stop,
            worker: Some(worker),
        }
    }
    pub fn snapshot(&self) -> Telemetry {
        let (value, updated) = *self.state.lock().unwrap();
        if updated.elapsed() > Duration::from_secs(3) {
            Telemetry::default()
        } else {
            value
        }
    }
}
impl Drop for Sensor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn battery_uses_measured_microvolts_and_explicit_charge_state() {
        let sample = battery(
            "POWER_SUPPLY_CAPACITY=99\nPOWER_SUPPLY_VOLTAGE_NOW=4167000\nPOWER_SUPPLY_STATUS=Not charging\nPOWER_SUPPLY_PRESENT=1\n",
        );
        assert_eq!(sample.battery_percent, Some(99));
        assert!((sample.battery_voltage.unwrap() - 4.167).abs() < 0.0001);
        assert!(!sample.charging);
        assert!(battery("POWER_SUPPLY_STATUS=Charging\n").charging);
        let missing = battery("POWER_SUPPLY_PRESENT=0\nPOWER_SUPPLY_CAPACITY=99\n");
        assert!(missing.battery_percent.is_none());
        assert!(
            battery("POWER_SUPPLY_CAPACITY=101\nPOWER_SUPPLY_VOLTAGE_NOW=bad\n")
                .battery_voltage
                .is_none()
        );
        assert!(battery("").battery_percent.is_none());
    }
    #[test]
    fn wifi_reports_rssi_and_clears_when_disconnected() {
        assert_eq!(
            wireless("face | tus | link level noise\nwlan0: 0000 70. -31. -256 0 0\n"),
            Some(-31)
        );
        assert_eq!(wireless("wlan0: 0000 0. 0. 0 0\n"), None);
        assert_eq!(wireless("Inter-| sta-| Quality\n"), None);
    }
}
