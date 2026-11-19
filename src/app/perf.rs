use std::collections::VecDeque;
use std::time::{Duration, Instant};

const FRAME_SAMPLES: usize = 180;
const RESOURCE_SAMPLES: usize = 60;

#[derive(Clone, Copy, Default)]
pub struct FrameTiming {
    pub logic: f32,
    pub draw: f32,
    pub present: f32,
    pub wait: f32,
}

#[derive(Clone, Copy)]
pub struct FrameSample {
    pub ms: f32,
    pub timing: Option<FrameTiming>,
}

#[derive(Clone, Copy, Default)]
pub struct Resources {
    pub elapsed: f32,
    pub cpu: Option<f32>,
    pub rss_mb: Option<f32>,
    pub swap_mb: Option<f32>,
    pub available_mb: Option<f32>,
    pub faults_per_second: Option<f32>,
    pub temperature: Option<f32>,
}

pub struct PerfStats {
    pub target_fps: f32,
    pub asynchronous: bool,
    pub cpu_capacity: f32,
    pub fps: f32,
    pub fps_avg: f32,
    pub frame_ms: f32,
    pub frame_peak: f32,
    pub frame_p95: f32,
    pub slow_percent: f32,
    pub render_ms: f32,
    pub triangles: u32,
    pub detail: [f32; 4],
    pub frames: VecDeque<FrameSample>,
    pub resources: VecDeque<Resources>,
    pub timing: Option<FrameTiming>,
    pub last_timing: Option<FrameTiming>,
    window: f32,
    window_frames: usize,
    resource_clock: Instant,
    counters: Option<(f64, i64)>,
}

impl Default for PerfStats {
    fn default() -> Self {
        Self {
            target_fps: 30.0,
            asynchronous: false,
            cpu_capacity: std::thread::available_parallelism().map_or(1, usize::from) as f32
                * 100.0,
            fps: 0.0,
            fps_avg: 0.0,
            frame_ms: 0.0,
            frame_peak: 0.0,
            frame_p95: 0.0,
            slow_percent: 0.0,
            render_ms: 0.0,
            triangles: 0,
            detail: [0.0; 4],
            frames: VecDeque::with_capacity(FRAME_SAMPLES),
            resources: VecDeque::with_capacity(RESOURCE_SAMPLES),
            timing: None,
            last_timing: None,
            window: 0.0,
            window_frames: 0,
            resource_clock: Instant::now(),
            counters: None,
        }
    }
}

impl PerfStats {
    pub fn record_render(&mut self, ms: f32, triangles: u32, detail: [f64; 4]) {
        self.render_ms = ms;
        self.triangles = triangles;
        self.detail = detail.map(|v| v as f32);
    }

    pub(super) fn record(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if self.frames.len() == FRAME_SAMPLES {
            self.frames.pop_front();
        }
        self.frames.push_back(FrameSample {
            ms: dt * 1000.0,
            timing: self.last_timing,
        });
        self.window += dt;
        self.window_frames += 1;
        if self.window >= 0.25 {
            self.fps = self.window_frames as f32 / self.window;
            self.frame_ms = self.window * 1000.0 / self.window_frames as f32;
            self.window = 0.0;
            self.window_frames = 0;
            let mut sorted: Vec<_> = self.frames.iter().map(|s| s.ms).collect();
            sorted.sort_unstable_by(f32::total_cmp);
            self.fps_avg = sorted.len() as f32 * 1000.0 / sorted.iter().sum::<f32>();
            self.frame_peak = *sorted.last().unwrap();
            self.frame_p95 = sorted[(sorted.len() * 95).div_ceil(100) - 1];
            let budget = 1000.0 / self.target_fps;
            self.slow_percent = sorted.iter().filter(|&&ms| ms > budget * 1.1).count() as f32
                * 100.0
                / sorted.len() as f32;
            let mut timing = FrameTiming::default();
            let mut count = 0;
            for t in self.frames.iter().filter_map(|s| s.timing) {
                timing.logic += t.logic;
                timing.draw += t.draw;
                timing.present += t.present;
                timing.wait += t.wait;
                count += 1;
            }
            self.timing = (count > 0).then(|| {
                let n = count as f32;
                FrameTiming {
                    logic: timing.logic / n,
                    draw: timing.draw / n,
                    present: timing.present / n,
                    wait: timing.wait / n,
                }
            });
        }
        if self.resources.is_empty() || self.resource_clock.elapsed() >= Duration::from_secs(1) {
            self.sample_resources();
        }
    }

    fn sample_resources(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.resource_clock).as_secs_f64();
        let current = process_counters();
        let mut resource = process_memory();
        resource.elapsed = self
            .resources
            .back()
            .map_or(0.0, |last| last.elapsed + elapsed as f32);
        if let (Some((cpu, faults)), Some((previous_cpu, previous_faults))) =
            (current, self.counters)
        {
            resource.cpu = Some(((cpu - previous_cpu) / elapsed * 100.0) as f32);
            resource.faults_per_second = Some(((faults - previous_faults) as f64 / elapsed) as f32);
        }
        self.counters = current;
        self.resource_clock = now;
        if self.resources.len() == RESOURCE_SAMPLES {
            self.resources.pop_front();
        }
        self.resources.push_back(resource);
    }
}

#[cfg(unix)]
fn process_counters() -> Option<(f64, i64)> {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return None;
        }
        let user = usage.ru_utime.tv_sec as f64 + usage.ru_utime.tv_usec as f64 / 1_000_000.0;
        let system = usage.ru_stime.tv_sec as f64 + usage.ru_stime.tv_usec as f64 / 1_000_000.0;
        Some((user + system, usage.ru_majflt))
    }
}

#[cfg(not(unix))]
fn process_counters() -> Option<(f64, i64)> {
    None
}

#[cfg(any(target_os = "linux", test))]
fn memory_field(text: &str, field: &str) -> Option<f32> {
    text.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name == field {
            value
                .split_whitespace()
                .next()?
                .parse::<f32>()
                .ok()
                .map(|kb| kb / 1024.0)
        } else {
            None
        }
    })
}

#[cfg(target_os = "linux")]
fn process_memory() -> Resources {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let memory = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    Resources {
        rss_mb: memory_field(&status, "VmRSS"),
        swap_mb: memory_field(&status, "VmSwap"),
        available_mb: memory_field(&memory, "MemAvailable"),
        temperature: std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|t| t / 1000.0),
        ..Resources::default()
    }
}

#[cfg(target_os = "macos")]
fn process_memory() -> Resources {
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            libc::getpid(),
            libc::PROC_PIDTASKINFO,
            0,
            (&mut info as *mut libc::proc_taskinfo).cast(),
            size,
        )
    };
    Resources {
        rss_mb: (read == size).then_some(info.pti_resident_size as f32 / 1_048_576.0),
        ..Resources::default()
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process_memory() -> Resources {
    Resources::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_stalls_drive_percentiles_and_slow_frames() {
        let mut stats = PerfStats::default();
        for _ in 0..19 {
            stats.record(1.0 / 30.0);
        }
        stats.record(0.3);
        assert_eq!(stats.frames.back().unwrap().ms, 300.0);
        assert_eq!(stats.frame_peak, 300.0);
        assert!((stats.frame_p95 - 1000.0 / 30.0).abs() < 0.001);
        assert_eq!(stats.slow_percent, 5.0);
        assert!((stats.fps_avg - 20.0 / (19.0 / 30.0 + 0.3)).abs() < 0.001);
        for _ in 0..190 {
            stats.record(1.0 / 30.0);
        }
        assert_eq!(stats.frames.len(), FRAME_SAMPLES);
        assert!(stats.frame_peak < 34.0);
    }

    #[test]
    fn stage_averages_use_completed_frame_timings() {
        let mut stats = PerfStats {
            last_timing: Some(FrameTiming {
                logic: 1.0,
                draw: 23.0,
                present: 3.0,
                wait: 6.0,
            }),
            ..PerfStats::default()
        };
        for _ in 0..30 {
            stats.record(1.0 / 30.0);
        }
        let timing = stats.timing.unwrap();
        assert_eq!(timing.draw, 23.0);
        assert_eq!(
            timing.logic + timing.draw + timing.present + timing.wait,
            33.0
        );
    }

    #[test]
    fn current_memory_is_distinct_from_peak_and_missing_values() {
        let status = "VmHWM: 131072 kB\nVmRSS: 65536 kB\nVmSwap: 256 kB\n";
        assert_eq!(memory_field(status, "VmRSS"), Some(64.0));
        assert_eq!(memory_field(status, "VmSwap"), Some(0.25));
        assert_eq!(memory_field(status, "MemAvailable"), None);
    }
}
