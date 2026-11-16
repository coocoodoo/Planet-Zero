use rust_fly::cardputer::{Game, HEIGHT, WIDTH};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::time::Duration;
use std::time::Instant;

struct Options {
    benchmark: Option<usize>,
    frames: Option<usize>,
    capture: Option<PathBuf>,
    scale: usize,
    view: usize,
    fps: u32,
    imu_test: Option<f32>,
    imu_csv: Option<PathBuf>,
    imu: bool,
}

fn options() -> Result<Options, Box<dyn Error>> {
    let mut result = Options {
        benchmark: None,
        frames: None,
        capture: None,
        scale: 1,
        view: 2,
        fps: 30,
        imu_test: None,
        imu_csv: None,
        imu: true,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" {
            println!(
                "rust-fly-cardputer [--scale 1] [--view top|chase|cockpit] [--fps 30] [--frames N] [--benchmark N] [--capture-dir PATH] [--imu on|off] [--imu-test SECONDS] [--imu-csv PATH]"
            );
            std::process::exit(0);
        }
        let value = args.next().ok_or_else(|| format!("{arg} 缺少参数值"))?;
        match arg.as_str() {
            "--imu-test" => result.imu_test = Some(value.parse()?),
            "--imu-csv" => result.imu_csv = Some(value.into()),
            "--imu" => {
                result.imu = match value.as_str() {
                    "on" => true,
                    "off" => false,
                    _ => return Err("imu 支持 on 或 off".into()),
                }
            }
            "--benchmark" => result.benchmark = Some(value.parse()?),
            "--frames" => result.frames = Some(value.parse()?),
            "--capture-dir" => result.capture = Some(value.into()),
            "--scale" => result.scale = value.parse()?,
            "--fps" => result.fps = value.parse()?,
            "--view" => {
                result.view = match value.as_str() {
                    "top" => 0,
                    "chase" => 1,
                    "cockpit" => 2,
                    _ => return Err("视角支持 top、chase 或 cockpit".into()),
                }
            }
            _ => return Err(format!("未知参数 {arg}").into()),
        }
    }
    if result.scale != 1
        || !(1..=60).contains(&result.fps)
        || result.benchmark == Some(0)
        || result.frames == Some(0)
        || result.imu_test.is_some_and(|s| !s.is_finite() || s <= 0.0)
    {
        return Err("全像素模式的 scale 为 1，fps 范围为 1–60，帧数需要大于零".into());
    }
    Ok(result)
}

fn capture(path: &Path, name: &str, game: &Game) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    let mut file =
        std::io::BufWriter::new(std::fs::File::create(path.join(format!("{name}.ppm")))?);
    write!(file, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
    for pixel in game.screen.chunks_exact(4) {
        file.write_all(&pixel[..3])?;
    }
    file.flush()
}

fn report(name: &str, scale: usize, samples: &mut [f64]) {
    samples.sort_by(f64::total_cmp);
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let p95 = samples[(samples.len() - 1) * 95 / 100];
    println!(
        "{{\"scene\":\"{name}\",\"scale\":{scale},\"frames\":{},\"mean_ms\":{mean:.3},\"p95_ms\":{p95:.3}}}",
        samples.len()
    );
}

fn benchmark(options: &Options, game: &mut Game, frames: usize) -> Result<(), Box<dyn Error>> {
    for (view, name) in ["top", "chase", "cockpit"].into_iter().enumerate() {
        game.flight = rust_fly::flight::Flight::new(&game.world);
        game.flight.view = view;
        game.flight.weights = [0.0; 3];
        game.flight.weights[view] = 1.0;
        for _ in 0..60 {
            game.step(1.0 / 30.0);
        }
        let mut samples = Vec::with_capacity(frames);
        let mut stages = [0.0; 3];
        for _ in 0..frames {
            let start = Instant::now();
            game.step(1.0 / 30.0);
            game.draw();
            for (total, value) in stages.iter_mut().zip(game.renderer.stages) {
                *total += value;
            }
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        report(name, options.scale, &mut samples);
        eprintln!(
            "{name} stages [cockpit, world, jets]: {:?}",
            stages.map(|t| t / frames as f64)
        );
        if let Some(path) = &options.capture {
            capture(path, &format!("{name}-{}x", options.scale), game)?;
        }
    }
    if let Some(path) = &options.capture {
        game.menu = true;
        game.draw();
        capture(path, "menu", game)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn run(options: &Options, game: &mut Game) -> Result<(), Box<dyn Error>> {
    use rust_fly::cardputer::linux::{Display, Keyboard, install_signals, stopped};
    install_signals()?;
    let sensor = match rust_fly::cardputer::imu::Sensor::open() {
        Ok(sensor) => {
            eprintln!("BMI270: {}", sensor.path.display());
            Some(sensor)
        }
        Err(error) => {
            eprintln!("IMU 暂不可用，使用键盘：{error}");
            None
        }
    };
    game.imu_enabled = options.imu && sensor.is_some();
    if game.imu_enabled {
        game.auto = false;
    }
    let mut keyboard = Keyboard::open()?;
    let mut display = Display::open()?;
    let period = Duration::from_secs_f64(1.0 / options.fps as f64);
    let mut previous = Instant::now();
    let mut count = 0;
    let mut rendering = Vec::new();
    let mut presenting = Vec::new();
    let began = Instant::now();
    while !stopped() && !game.quit && options.frames.is_none_or(|limit| count < limit) {
        let start = Instant::now();
        keyboard.poll(game)?;
        if let Some(sensor) = &sensor {
            if game.calibrate_imu {
                sensor.calibrate();
                game.calibrate_imu = false;
            }
            game.motion = sensor.motion();
        }
        game.step((start - previous).as_secs_f32());
        previous = start;
        game.draw();
        let drawn = Instant::now();
        display.present(&game.screen)?;
        count += 1;
        if options.frames.is_some() {
            rendering.push((drawn - start).as_secs_f64() * 1000.0);
            presenting.push(drawn.elapsed().as_secs_f64() * 1000.0);
        }
        if let Some(wait) = period.checked_sub(start.elapsed()) {
            std::thread::sleep(wait);
        }
    }
    if !rendering.is_empty() {
        report("render", 1, &mut rendering);
        report("present", 1, &mut presenting);
    }
    println!(
        "{{\"display_frames\":{count},\"elapsed_seconds\":{:.3},\"view\":{},\"auto\":{},\"speed\":{:.2},\"altitude\":{:.2},\"yaw\":{:.4},\"bank\":{:.4},\"pitch\":{:.4},\"wings\":{},\"imu_enabled\":{},\"imu_ready\":{},\"width\":320,\"height\":170}}",
        began.elapsed().as_secs_f64(),
        game.flight.view,
        game.auto,
        game.flight.speed,
        game.flight.position.z,
        game.flight.yaw,
        game.flight.bank,
        game.flight.pitch,
        game.flight.wing_count,
        game.imu_enabled,
        game.motion.ready
    );
    if let Some(path) = &options.capture {
        capture(path, "screen", game)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = options()?;
    if let Some(seconds) = options.imu_test {
        rust_fly::cardputer::imu::diagnose(seconds, options.imu_csv.as_deref())?;
        return Ok(());
    }
    let start = Instant::now();
    let mut game = Game::new();
    eprintln!("地形生成：{:.0} ms", start.elapsed().as_secs_f64() * 1000.0);
    game.flight.view = options.view;
    game.flight.weights = [0.0; 3];
    game.flight.weights[options.view] = 1.0;
    if let Some(frames) = options.benchmark {
        return benchmark(&options, &mut game, frames);
    }
    #[cfg(target_os = "linux")]
    return run(&options, &mut game);
    #[cfg(not(target_os = "linux"))]
    Err("设备显示后端需要 Linux，本机可使用 --benchmark N 验证渲染".into())
}
