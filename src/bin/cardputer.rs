use flight_club::cardputer::{Game, HEIGHT, WIDTH};
use flight_club::world::{MapKind, World};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::time::Duration;
use std::time::Instant;

#[path = "cardputer/scenarios.rs"]
mod scenarios;

struct Options {
    audio: bool,
    audio_benchmark: Option<f32>,
    level: u32,
    mode: Option<flight_club::gameplay::Mode>,
    vehicle: usize,
    benchmark: Option<usize>,
    maneuvers: Option<usize>,
    travel: Option<usize>,
    hd: Option<usize>,
    frames: Option<usize>,
    capture: Option<PathBuf>,
    scale: usize,
    view: usize,
    fps: u32,
    imu_test: Option<f32>,
    imu_csv: Option<PathBuf>,
    imu: bool,
    demo: bool,
    map: MapKind,
    weather: flight_club::content::weather::Weather,
    seed: Option<u32>,
}

fn options() -> Result<Options, Box<dyn Error>> {
    let mut result = Options {
        audio: true,
        audio_benchmark: None,
        level: 1,
        mode: None,
        vehicle: 0,
        benchmark: None,
        maneuvers: None,
        travel: None,
        hd: None,
        frames: None,
        capture: None,
        scale: 1,
        view: 2,
        fps: 30,
        imu_test: None,
        imu_csv: None,
        imu: true,
        demo: false,
        map: MapKind::Islands,
        weather: flight_club::content::weather::Weather::Clear,
        seed: None,
    };
    let mut map_explicit = false;
    let mut weather_explicit = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" {
            println!(
                "flight-club-cardputer [--audio on|off] [--level 1..999] [--mode free|race|coins|combat] [--vehicle 0..4] [--scale 1] [--view top|chase|cockpit] [--fps 30] [--frames N] [--benchmark N] [--maneuvers N] [--travel N] [--hd 1|2|4|8] [--capture-dir PATH] [--imu on|off] [--demo on|off] [--map islands|alpine|canyon|desert|wetlands|volcanic|moon|space|fjords|icebergs|city|highlands] [--seed N] [--weather clear|rain|snow|storm|fog|sandstorm] [--imu-test SECONDS] [--imu-csv PATH]"
            );
            std::process::exit(0);
        }
        let value = args.next().ok_or_else(|| format!("{arg} 缺少参数值"))?;
        match arg.as_str() {
            "--audio" => result.audio = switch("audio", &value)?,
            "--audio-benchmark" => result.audio_benchmark = Some(value.parse()?),
            "--level" => result.level = value.parse()?,
            "--mode" => {
                result.mode = Some(
                    flight_club::gameplay::Mode::parse(&value)
                        .ok_or("未知模式，请使用 free、race、coins 或 combat")?,
                )
            }
            "--vehicle" => result.vehicle = value.parse()?,
            "--weather" => {
                weather_explicit = true;
                result.weather = flight_club::content::weather::Weather::parse(&value)
                    .ok_or("未知天气，请使用 --help 查看天气名称")?
            }
            "--map" => {
                map_explicit = true;
                result.map = MapKind::parse(&value).ok_or("未知地图，请使用 --help 查看地图名称")?
            }
            "--seed" => result.seed = Some(value.parse()?),
            "--demo" => result.demo = switch("demo", &value)?,
            "--imu-test" => result.imu_test = Some(value.parse()?),
            "--imu-csv" => result.imu_csv = Some(value.into()),
            "--imu" => result.imu = switch("imu", &value)?,
            "--travel" => result.travel = Some(value.parse()?),
            "--hd" => result.hd = Some(value.parse()?),
            "--maneuvers" => result.maneuvers = Some(value.parse()?),
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
    if !(1..=999).contains(&result.level) {
        return Err("关卡编号范围为 1–999".into());
    }
    let operation = flight_club::content::levels::operation(result.level);
    if !map_explicit {
        result.map = operation.map;
    }
    if !weather_explicit {
        result.weather = operation.weather;
    }
    if result.vehicle >= flight_club::content::vehicles::Vehicle::ALL.len()
        || result.scale != 1
        || !(1..=60).contains(&result.fps)
        || result.maneuvers.is_some_and(|n| n < 180)
        || result.benchmark == Some(0)
        || result.frames == Some(0)
        || result.imu_test.is_some_and(|s| !s.is_finite() || s <= 0.0)
    {
        return Err("全像素模式的 scale 为 1，fps 范围为 1–60，帧数需要大于零".into());
    }
    if result.hd.is_some_and(|n| ![1, 2, 4, 8].contains(&n)) || result.travel == Some(0) {
        return Err("高清倍率支持 1、2、4、8，穿越次数需要大于零".into());
    }
    Ok(result)
}

fn switch(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("{name} 支持 on 或 off")),
    }
}

/// 把 RGBA 画面写成 `<path>/<name>.ppm`
fn write_ppm(
    path: &Path,
    name: &str,
    width: usize,
    height: usize,
    rgba: &[u8],
) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    let mut file =
        std::io::BufWriter::new(std::fs::File::create(path.join(format!("{name}.ppm")))?);
    write!(file, "P6\n{width} {height}\n255\n")?;
    for pixel in rgba.as_chunks::<4>().0 {
        file.write_all(&pixel[..3])?;
    }
    file.flush()
}

fn capture(path: &Path, name: &str, game: &Game) -> std::io::Result<()> {
    write_ppm(path, name, WIDTH, HEIGHT, &game.screen)
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

fn keep_benchmark_active(game: &mut Game) {
    if game.mode == flight_club::gameplay::Mode::Combat {
        // 性能采样期间维持双方护盾和僚机，持续绘制战斗场景
        game.mission.health = game.mission.max_health;
        game.mission.support = 30.0;
        for enemy in &mut game.mission.enemies {
            enemy.health = enemy.max_health;
        }
    }
}

fn benchmark(options: &Options, game: &mut Game, frames: usize) -> Result<(), Box<dyn Error>> {
    let scenes = if game.mode == flight_club::gameplay::Mode::Combat {
        6
    } else {
        3
    };
    for scene in 0..scenes {
        let view = scene % 3;
        let base = ["top", "chase", "cockpit"][view];
        let name = if scene >= 3 {
            format!("boss-{base}")
        } else {
            base.to_owned()
        };
        game.start();
        game.mission.shuffle_encounters(7109);
        if game.mode == flight_club::gameplay::Mode::Combat {
            game.mission.prepare_benchmark(scene >= 3);
            game.mission.support = 30.0;
        }

        game.flight.set_demo(options.demo);
        game.flight.weather = flight_club::content::weather::WeatherState::new(
            options.weather,
            game.world.kind,
            game.world.seed,
        );
        game.flight.snap_view(view);
        for _ in 0..if game.mode == flight_club::gameplay::Mode::Free {
            60
        } else {
            240
        } {
            keep_benchmark_active(game);
            game.step(1.0 / 30.0);
        }
        let mut samples = Vec::with_capacity(frames);
        let mut stages = [0.0; 3];
        let mut detail = [0.0; 4];
        let mut peak_projectiles = 0;
        for _ in 0..frames {
            let start = Instant::now();
            keep_benchmark_active(game);
            game.step(1.0 / 30.0);
            peak_projectiles = peak_projectiles.max(game.mission.shots.len());
            game.draw();
            for (total, value) in stages.iter_mut().zip(game.renderer.stages) {
                *total += value;
            }
            for (total, value) in detail.iter_mut().zip(game.renderer.detail) {
                *total += value;
            }
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        eprintln!(
            "{name} detail [terrain, clouds, scenery, portals] {:?}",
            detail.map(|v| v / frames as f64)
        );
        report(&name, options.scale, &mut samples);
        println!(
            "{{\"scene\":\"{name}\",\"mode\":\"{}\",\"wave\":{},\"enemies\":{},\"projectiles\":{},\"peak_projectiles\":{peak_projectiles},\"page\":\"{:?}\"}}",
            game.mode.name(),
            game.mission.wave,
            game.mission.enemies.len(),
            game.mission.shots.len(),
            game.page
        );
        eprintln!(
            "{name} stages [cockpit, world, jets]: {:?}",
            stages.map(|t| t / frames as f64)
        );
        if let Some(path) = &options.capture {
            capture(path, &format!("{name}-{}x", options.scale), game)?;
        }
    }
    if let Some(path) = &options.capture {
        capture_landmarks(game, path)?;
    }
    if game.world.kind == MapKind::Space {
        scenarios::travel(options, game, 2)?;
    }
    if let Some(path) = &options.capture {
        game.home();
        game.draw();
        capture(path, "menu", game)?;
    }
    Ok(())
}

fn capture_landmarks(game: &mut Game, path: &Path) -> Result<(), Box<dyn Error>> {
    use glam::vec3;
    let scenes = match game.world.kind {
        MapKind::Moon => vec![
            ("lunar-base", vec3(2180.0, 2725.0, 150.0), 0.0, 0),
            ("lunar-crater", vec3(2210.0, 3080.0, 240.0), 0.10, 1),
        ],
        MapKind::Space => vec![("stellar-sun", vec3(2048.0, 2900.0, 150.0), -0.6, 1)],
        MapKind::Canyon => vec![
            (
                "canyon-floor",
                vec3(
                    flight_club::world::regions_detail::canyon_center(2870.0),
                    2870.0,
                    38.0,
                ),
                0.0,
                1,
            ),
            (
                "canyon-cave",
                vec3(2120.0, 2320.0, 31.0),
                std::f32::consts::FRAC_PI_2,
                1,
            ),
        ],
        MapKind::Icebergs => vec![("ice-channel", vec3(1760.0, 3040.0, 78.0), 0.1, 1)],
        MapKind::City => vec![("city-skyline", vec3(2112.0, 2670.0, 205.0), 0.0, 1)],
        MapKind::Highlands => vec![(
            "wind-farm",
            vec3(1960.0, 3120.0, game.world.height(1960.0, 3120.0) + 85.0),
            0.0,
            1,
        )],
        MapKind::Fjords => vec![("fjord-channel", vec3(1680.0, 3100.0, 110.0), 0.0, 1)],
        _ => vec![],
    };
    for (name, position, yaw, view) in scenes {
        game.mode = flight_club::gameplay::Mode::Free;
        game.start();
        game.flight.position = position;
        game.flight.follow = position;
        game.flight.yaw = yaw;
        game.flight.camera_yaw = -yaw;
        game.flight.snap_view(view);
        game.flight.wing_count = 0;
        game.flight.hud = false;
        game.draw();
        capture(path, name, game)?;
    }
    game.flight.hud = true;
    Ok(())
}

#[cfg(target_os = "linux")]
fn run(options: &Options, game: &mut Game) -> Result<(), Box<dyn Error>> {
    use flight_club::cardputer::linux::{Display, Keyboard, Presenter, install_signals, stopped};
    install_signals()?;
    let sensor = match flight_club::cardputer::imu::Sensor::open() {
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
    let telemetry = flight_club::cardputer::telemetry::Sensor::start();
    let mut keyboard = Keyboard::open()?;
    let mut display = Presenter::start(Display::open()?);
    let audio = if options.audio {
        flight_club::audio::Output::start()
            .map_err(|error| eprintln!("音频暂不可用：{error}"))
            .ok()
    } else {
        None
    };
    game.perf.target_fps = options.fps as f32;
    let period = Duration::from_secs_f64(1.0 / options.fps as f64);
    let mut previous = Instant::now();
    let mut count = 0;
    let mut rendering = Vec::new();
    let mut presenting = Vec::new();
    let began = Instant::now();
    let mut previous_state = None;
    let mut maneuvers = [0_u32; 3];
    let mut pitch_rotation = 0.0_f32;
    let mut minimum_clearance = f32::INFINITY;
    while !stopped() && !game.quit && options.frames.is_none_or(|limit| count < limit) {
        let start = Instant::now();
        keyboard.poll(game)?;
        game.telemetry = telemetry.snapshot();
        if let Some(sensor) = &sensor {
            if game.calibrate_imu {
                sensor.calibrate();
                game.calibrate_imu = false;
            }
            game.motion = sensor.motion();
        }
        let (pitch, spin, warp) = (game.flight.pitch, game.flight.spin, game.flight.warp_flash);
        game.step((start - previous).as_secs_f32());
        if options.frames.is_some() {
            pitch_rotation += flight_club::flight::angle(game.flight.pitch - pitch);
            if spin == 0.0 && game.flight.spin != 0.0 {
                maneuvers[usize::from(game.flight.spin > 0.0)] += 1;
            }
            if game.flight.warp_flash > warp {
                maneuvers[2] += 1;
            }
            let p = game.flight.position;
            minimum_clearance = minimum_clearance.min(p.z - game.world.flight_height(p));
        }
        if let Some(audio) = &audio {
            audio.update(&mut game.app);
        }
        let state = (
            game.page,
            game.level,
            game.mission.phase_index,
            game.selection,
        );
        if previous_state != Some(state) {
            eprintln!(
                "{{\"event\":\"state\",\"page\":\"{:?}\",\"level\":{},\"phase\":{},\"elapsed\":{:.2},\"score\":{},\"health\":{:.1},\"outcome\":\"{}\",\"selection\":{}}}",
                game.page,
                game.level,
                game.mission.phase_index,
                game.mission.elapsed,
                game.mission.score,
                game.mission.health,
                match game.mission.outcome {
                    Some(true) => "win",
                    Some(false) => "loss",
                    None => "active",
                },
                game.selection
            );
            previous_state = Some(state);
        }
        previous = start;
        let draw_start = Instant::now();
        game.draw();
        let drawn = Instant::now();
        display.present(&game.screen)?;
        let presented = Instant::now();
        count += 1;
        if options.frames.is_some() {
            rendering.push((drawn - start).as_secs_f64() * 1000.0);
            presenting.push((presented - drawn).as_secs_f64() * 1000.0);
        }
        let waiting = Instant::now();
        if let Some(wait) = period.checked_sub(start.elapsed()) {
            std::thread::sleep(wait);
        }
        if game.perf_panel != flight_club::app::PerfPanel::Off {
            game.perf.last_timing = Some(flight_club::app::perf::FrameTiming {
                logic: (draw_start - start).as_secs_f32() * 1000.0,
                draw: (drawn - draw_start).as_secs_f32() * 1000.0,
                present: (presented - drawn).as_secs_f32() * 1000.0,
                wait: waiting.elapsed().as_secs_f32() * 1000.0,
            });
        }
    }
    if !rendering.is_empty() {
        report("render", 1, &mut rendering);
        report("present", 1, &mut presenting);
        println!(
            "{{\"roll_left\":{},\"roll_right\":{},\"warps\":{},\"portals\":{},\"routes\":{},\"pitch_rotation_degrees\":{:.1},\"minimum_clearance\":{minimum_clearance:.2}}}",
            maneuvers[0],
            maneuvers[1],
            maneuvers[2],
            if game.network.active {
                game.network.routes(&game.world).len()
            } else {
                game.world.portals.len()
            },
            game.network.routes(&game.world).len(),
            pitch_rotation.to_degrees()
        );
    }
    println!(
        "{{\"display_frames\":{count},\"elapsed_seconds\":{:.3},\"view\":{},\"auto\":{},\"speed\":{:.2},\"altitude\":{:.2},\"yaw\":{:.4},\"bank\":{:.4},\"pitch\":{:.4},\"wings\":{},\"imu_enabled\":{},\"imu_ready\":{},\"width\":320,\"height\":170,\"map\":\"{}\",\"seed\":{},\"weather\":\"{}\",\"demo\":{},\"clearance\":{:.2}}}",
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
        game.motion.ready,
        game.world.kind.name(),
        game.world.seed,
        game.flight.weather.name(),
        game.flight.demo,
        game.flight.position.z
            - game
                .world
                .height(game.flight.position.x, game.flight.position.y)
    );
    println!(
        "{{\"level\":{},\"phase\":\"{}\",\"phase_seconds\":{:.2},\"objective\":{},\"overdrive\":{:.1},\"graze\":{},\"convoy_health\":{:.1}}}",
        game.level,
        game.mission.phase().name(),
        game.mission.phase_elapsed,
        game.mission.objective,
        game.mission.overdrive,
        game.mission.grazes,
        game.mission.convoy.as_ref().map_or(-1.0, |c| c.health)
    );
    println!(
        "{{\"mode\":\"{}\",\"page\":\"{:?}\",\"score\":{},\"wave\":{},\"kills\":{},\"enemies\":{},\"shots\":{},\"health\":{:.1},\"gate\":{},\"coins\":{},\"ui\":{}}}",
        game.mode.name(),
        game.page,
        game.mission.score,
        game.mission.wave,
        game.mission.kills,
        game.mission.enemies.len(),
        game.mission.shots.len(),
        game.mission.health,
        game.mission.gate_index,
        game.mission.collected,
        game.flight.hud
    );
    if let Some(path) = &options.capture {
        capture(path, "screen", game)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = options()?;
    if let Some(seconds) = options.imu_test {
        flight_club::cardputer::imu::diagnose(seconds, options.imu_csv.as_deref())?;
        return Ok(());
    }
    if let Some(seconds) = options.audio_benchmark {
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err("音频测试时长需要大于零".into());
        }
        flight_club::audio::export::benchmark(seconds);
        return Ok(());
    }
    let start = Instant::now();
    let mut game = Game::with_world(World::generate(
        options.map,
        options.seed.unwrap_or_else(|| {
            options
                .map
                .seed()
                .wrapping_add((options.level - 1) / 6 * 7919)
        }),
    ));
    if options.benchmark.is_some() || options.travel.is_some() || options.maneuvers.is_some() {
        game.network = flight_club::world::network::Network::new(7109);
    }
    eprintln!("地形生成：{:.0} ms", start.elapsed().as_secs_f64() * 1000.0);
    game.level = options.level;
    game.flight.vehicle = flight_club::content::vehicles::Vehicle::ALL[options.vehicle];
    if options.mode.is_some()
        || options.benchmark.is_some()
        || options.frames.is_some()
        || options.demo
    {
        game.mode = options.mode.unwrap_or(flight_club::gameplay::Mode::Free);
        game.start();
        game.flight.set_demo(options.demo);
    }
    game.flight.weather = flight_club::content::weather::WeatherState::new(
        options.weather,
        game.world.kind,
        game.world.seed,
    );
    game.flight
        .snap_view(if options.demo { 1 } else { options.view });
    if let Some(scale) = options.hd {
        return scenarios::high_resolution(&options, &mut game, scale);
    }
    if let Some(hops) = options.travel {
        return scenarios::travel(&options, &mut game, hops);
    }
    if let Some(frames) = options.maneuvers {
        return scenarios::maneuvers(&options, &mut game, frames);
    }
    if let Some(frames) = options.benchmark {
        return benchmark(&options, &mut game, frames);
    }
    #[cfg(target_os = "linux")]
    return run(&options, &mut game);
    #[cfg(not(target_os = "linux"))]
    Err("设备显示后端需要 Linux，本机可使用 --benchmark N 验证渲染".into())
}
