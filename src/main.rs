use macroquad::prelude::*;
use rust_fly::app::{App, HEIGHT, Page, WIDTH};
use rust_fly::arcade::Mode;
use rust_fly::world::World;

fn configuration() -> Conf {
    Conf {
        window_title: "Rust Fly / Flight Club".into(),
        window_width: 1152,
        window_height: 612,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

fn draw_frame(texture: &Texture2D) {
    clear_background(BLACK);
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
    for row in app.screen.chunks_exact(WIDTH * 4).rev() {
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
        app.flight.view = view;
        app.flight.weights = [0.0; 3];
        app.flight.weights[view] = 1.0;
        if name == "menu" {
            app.home();
        }
        if name == "boss" {
            app.mission.wave = 4;
        }
        for _ in 0..frames {
            app.step(1.0 / 30.0, None);
        }
        app.draw();
        save_capture(app, &path.join(format!("{name}.png")));
    }
    app.page = Page::Paused;
    app.selection = 0;
    app.draw();
    save_capture(app, &path.join("pause.png"));
    for vehicle in rust_fly::vehicle::Vehicle::ALL {
        app.mode = Mode::Free;
        app.flight.vehicle = vehicle;
        app.start();
        app.flight.view = 0;
        app.flight.weights = [1.0, 0.0, 0.0];
        for _ in 0..60 {
            app.step(1.0 / 30.0, None);
        }
        app.draw();
        save_capture(app, &path.join(format!("vehicle-{}.png", vehicle.name())));
    }
}

#[macroquad::main(configuration)]
async fn main() {
    let mut app = App::new(World::new(7));
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--capture") {
        capture(
            &mut app,
            std::path::Path::new(args.get(i + 1).expect("请指定截图目录")),
        );
        return;
    }
    let texture = Texture2D::from_rgba8(WIDTH as u16, HEIGHT as u16, &app.screen);
    texture.set_filter(FilterMode::Nearest);
    let mut fullscreen = false;
    let mut pointer = mouse_position();
    let mut pointer_idle = 10.0_f32;
    let mapping = [
        (KeyCode::Escape, 1),
        (KeyCode::Enter, 28),
        (KeyCode::Space, 57),
        (KeyCode::Up, 103),
        (KeyCode::Down, 108),
        (KeyCode::Left, 105),
        (KeyCode::Right, 106),
        (KeyCode::W, 17),
        (KeyCode::A, 30),
        (KeyCode::S, 31),
        (KeyCode::D, 32),
        (KeyCode::Z, 44),
        (KeyCode::X, 45),
        (KeyCode::C, 46),
        (KeyCode::Q, 16),
        (KeyCode::LeftShift, 42),
        (KeyCode::RightShift, 54),
        (KeyCode::U, 22),
        (KeyCode::H, 35),
        (KeyCode::P, 25),
        (KeyCode::R, 19),
        (KeyCode::T, 20),
        (KeyCode::J, 36),
        (KeyCode::V, 47),
        (KeyCode::B, 48),
        (KeyCode::G, 34),
        (KeyCode::M, 50),
        (KeyCode::N, 49),
        (KeyCode::Tab, 15),
        (KeyCode::L, 38),
        (KeyCode::O, 24),
        (KeyCode::E, 18),
        (KeyCode::Key1, 26),
        (KeyCode::Key2, 27),
        (KeyCode::Key3, 39),
        (KeyCode::Key4, 40),
        (KeyCode::Key5, 41),
        (KeyCode::Key6, 43),
        (KeyCode::Key7, 51),
        (KeyCode::Key8, 52),
    ];
    loop {
        let dt = get_frame_time().min(0.1);
        for (key, code) in mapping {
            if is_key_pressed(key) {
                app.key(code, 1);
            }
            if is_key_released(key) {
                app.key(code, 0);
            }
        }
        if is_key_pressed(KeyCode::F) {
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
            if app.page != Page::Playing || (mx >= 280.0 && my < 14.0) {
                app.click(mx.max(0.0) as usize, my.max(0.0) as usize);
            } else if app.mode == Mode::Free {
                app.flight.roll();
            }
        }
        if is_mouse_button_down(MouseButton::Left) && app.mode == Mode::Combat {
            app.keys.insert(38);
        } else {
            app.keys.remove(&38);
            if is_key_down(KeyCode::L) {
                app.keys.insert(38);
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
        app.step(dt, Some(controls));
        app.draw();
        texture.update_from_bytes(WIDTH as u32, HEIGHT as u32, &app.screen);
        draw_frame(&texture);
        if app.quit {
            break;
        }
        next_frame().await;
    }
}
