#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod desktop;
use flight_club::app::{App, HEIGHT, Page, WIDTH};
use flight_club::gameplay::Mode;
use flight_club::world::World;
use macroquad::prelude::*;

fn configuration() -> Conf {
    Conf {
        window_title: "Flight Club".into(),
        window_width: 1152,
        window_height: 612,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

fn draw_frame(texture: &Texture2D) {
    let scale = (screen_width() / WIDTH as f32).min(screen_height() / HEIGHT as f32);
    let width = WIDTH as f32 * scale;
    let height = HEIGHT as f32 * scale;
    draw_texture_ex(
        texture,
        (screen_width() - width) * 0.5,
        (screen_height() - height) * 0.5,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(width, height)),
            ..Default::default()
        },
    );
}

fn save_capture(app: &App, path: &std::path::Path) {
    let mut bytes = Vec::with_capacity(app.screen.len());
    for row in app.screen.as_chunks::<{ WIDTH * 4 }>().0.iter().rev() {
        bytes.extend_from_slice(row);
    }
    Image {
        bytes,
        width: WIDTH as u16,
        height: HEIGHT as u16,
    }
    .export_png(path.to_str().unwrap());
}

fn capture(app: &mut App, path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("无法创建截图目录");
    for (name, mode, frames, view) in [
        ("menu", Mode::Free, 120, 1),
        ("race", Mode::Race, 150, 1),
        ("coins", Mode::Coins, 140, 2),
        ("combat", Mode::Combat, 420, 1),
        ("boss", Mode::Combat, 250, 1),
        ("cockpit", Mode::Combat, 240, 2),
    ] {
        app.mode = mode;
        app.start();
        app.flight.snap_view(view);
        if name == "menu" {
            app.home();
        }
        if name == "boss" {
            app.mission.prepare_benchmark(true);
        }
        app.advance(frames);
        app.draw();
        save_capture(app, &path.join(format!("{name}.png")));
    }
    app.page = Page::Paused;
    app.selection = 0;
    app.draw();
    save_capture(app, &path.join("pause.png"));
    app.mission.outcome = Some(true);
    app.page = Page::Results;
    app.draw();
    save_capture(app, &path.join("results-win.png"));
    app.mission.outcome = Some(false);
    app.mission.message = "AIRCRAFT LOST";
    app.draw();
    save_capture(app, &path.join("results-loss.png"));
    for level in [2, 3, 6] {
        app.level = level;
        let profile = flight_club::content::levels::operation(level);
        app.world = World::generate(profile.map, profile.map.seed()).into();
        app.mode = Mode::Combat;
        app.set_weather(profile.weather);
        app.start();
        app.flight.snap_view(1);
        app.advance(240);
        app.draw();
        save_capture(app, &path.join(format!("stage-{level}.png")));
    }
    for vehicle in flight_club::content::vehicles::Vehicle::ALL {
        app.mode = Mode::Free;
        app.flight.vehicle = vehicle;
        app.start();
        app.flight.snap_view(0);
        app.advance(60);
        app.draw();
        save_capture(app, &path.join(format!("vehicle-{}.png", vehicle.name())));
    }
}

#[macroquad::main(configuration)]
async fn main() {
    let mut app = App::new(World::new(7));
    app.perf.asynchronous = true;
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--capture") {
        capture(
            &mut app,
            std::path::Path::new(args.get(i + 1).expect("请指定截图目录")),
        );
        return;
    }
    let audio = flight_club::audio::Output::start()
        .map_err(|error| eprintln!("音频暂不可用：{error}"))
        .ok();
    app.draw();
    let mut texture = Texture2D::from_rgba8(WIDTH as u16, HEIGHT as u16, &app.screen);
    texture.set_filter(FilterMode::Nearest);
    let overlay = Texture2D::from_rgba8(WIDTH as u16, HEIGHT as u16, &app.screen);
    overlay.set_filter(FilterMode::Nearest);
    let mut drawing = desktop::Drawing::new();
    let mut quality = 0_usize;
    let mut displayed_scale = 1;
    let mut frame_ms = 0.0;
    let mut quality_notice = 0.0_f32;
    let mut fullscreen = false;
    let mut pointer = mouse_position();
    let mut pointer_idle = 10.0_f32;
    use flight_club::keys as k;
    let mapping = [
        (KeyCode::Escape, k::ESC),
        (KeyCode::Enter, k::ENTER),
        (KeyCode::Up, k::UP),
        (KeyCode::Down, k::DOWN),
        (KeyCode::Left, k::LEFT),
        (KeyCode::Right, k::RIGHT),
        (KeyCode::Q, k::ROLL_LEFT),
        (KeyCode::E, k::ROLL_RIGHT),
        (KeyCode::W, k::LOOK_UP),
        (KeyCode::A, k::LOOK_LEFT),
        (KeyCode::S, k::LOOK_DOWN),
        (KeyCode::D, k::LOOK_RIGHT),
        (KeyCode::LeftAlt, k::HOVER),
        (KeyCode::RightAlt, k::HOVER),
        (KeyCode::LeftShift, k::BOOST),
        (KeyCode::RightShift, k::BOOST_RIGHT),
        (KeyCode::Space, k::VIEW),
        (KeyCode::M, k::CHART),
        (KeyCode::U, k::HUD),
        (KeyCode::P, k::PAUSE),
        (KeyCode::R, k::RETRY),
        (KeyCode::Key1, k::WEATHER),
        (KeyCode::Key2, k::TIME),
        (KeyCode::Key3, k::PORTAL),
        (KeyCode::Key4, k::VEHICLE),
        (KeyCode::Key5, k::WINGMEN),
        (KeyCode::Key6, k::MODEL),
        (KeyCode::Key7, k::AUTOPILOT),
        (KeyCode::Key8, k::DEMO),
        (KeyCode::Key9, k::MAP),
        (KeyCode::Key0, k::SEED),
        (KeyCode::F1, k::HELP),
        (KeyCode::F2, k::SETTINGS),
        (KeyCode::F9, k::SOUND),
        (KeyCode::F10, k::PERF),
    ];
    loop {
        let frame_dt = get_frame_time();
        let dt = frame_dt.min(0.1);
        quality_notice = (quality_notice - dt).max(0.0);
        if is_key_pressed(KeyCode::F12) {
            let count = desktop::SCALES.len();
            let reverse = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            quality = (quality + if reverse { count - 1 } else { 1 }) % count;
            quality_notice = 5.0;
        }
        for (key, code) in mapping {
            if is_key_pressed(key) {
                app.key(code, 1);
            }
            if is_key_released(key) {
                app.key(code, 0);
            }
        }
        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        let next = mouse_position();
        pointer_idle += dt;
        if (next.0 - pointer.0).abs() + (next.1 - pointer.1).abs() > 1.0 {
            pointer_idle = 0.0;
        }
        pointer = next;
        let scale = (screen_width() / WIDTH as f32).min(screen_height() / HEIGHT as f32);
        let mx = (pointer.0 - (screen_width() - WIDTH as f32 * scale) * 0.5) / scale;
        let my = (pointer.1 - (screen_height() - HEIGHT as f32 * scale) * 0.5) / scale;
        if is_mouse_button_pressed(MouseButton::Left) {
            if app.page != Page::Playing
                || app.hud_button_contains(mx.max(0.0) as usize, my.max(0.0) as usize)
            {
                app.click(mx.max(0.0) as usize, my.max(0.0) as usize);
            } else if app.mode == Mode::Free && app.flight.roll() {
                app.sounds.push(flight_club::audio::Sound::Roll);
            }
        }
        if is_mouse_button_down(MouseButton::Left) && app.mode == Mode::Combat {
            app.keys.insert(k::FIRE);
        } else {
            app.keys.remove(&k::FIRE);
            if is_key_down(KeyCode::Key1) {
                app.keys.insert(k::FIRE);
            }
        }
        let mut controls = app.controls();
        if app.page == Page::Playing
            && pointer_idle < 4.0
            && controls.turn == 0.0
            && controls.climb == 0.0
        {
            controls.manual = true;
            controls.turn = ((mx - 160.0) / 100.0).clamp(-1.0, 1.0);
            controls.climb = ((85.0 - my) / 65.0).clamp(-1.0, 1.0);
        }
        app.input_label = if pointer_idle < 4.0 {
            "MOUSE"
        } else if app.auto {
            "AUTO"
        } else {
            "KEY"
        };
        app.step(frame_dt, Some(controls));
        if let Some(audio) = &audio {
            audio.update(&mut app);
        }
        app.prepare_scene();
        let scale = desktop::SCALES[quality];
        if let Some(frame) = drawing.poll()
            && frame.scale == scale
            && frame.map == flight_club::world::network::MapKey::of(&app.world)
        {
            let (w, h) = (WIDTH * scale, HEIGHT * scale);
            if displayed_scale == scale {
                texture.update_from_bytes(w as u32, h as u32, &frame.pixels);
            } else {
                texture = Texture2D::from_rgba8(w as u16, h as u16, &frame.pixels);
                texture.set_filter(FilterMode::Nearest);
                displayed_scale = scale;
            }
            frame_ms = frame.milliseconds;
            app.perf
                .record_render(frame.milliseconds as f32, frame.triangles, frame.detail);
            drawing.recycle(frame.pixels);
        }
        if app.page != Page::Chart {
            drawing.submit(&app, scale);
        }
        app.draw_overlay();
        overlay.update_from_bytes(WIDTH as u32, HEIGHT as u32, &app.screen);
        clear_background(BLACK);
        draw_frame(&texture);
        draw_frame(&overlay);
        if quality_notice > 0.0 || displayed_scale != scale {
            let status = if displayed_scale == scale {
                format!("{frame_ms:.0} ms")
            } else {
                "RENDERING".into()
            };
            let text = format!(
                "F12 {scale}x  {} x {}  {status}",
                WIDTH * scale,
                HEIGHT * scale
            );
            draw_rectangle(
                12.0,
                screen_height() - 40.0,
                380.0,
                28.0,
                Color::new(0.02, 0.05, 0.09, 0.85),
            );
            draw_text(&text, 22.0, screen_height() - 20.0, 20.0, WHITE);
        }
        if app.quit {
            break;
        }
        next_frame().await;
    }
}
