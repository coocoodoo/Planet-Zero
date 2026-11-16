use macroquad::prelude::*;
use rust_fly::flight::{Camera, Controls, Flight, angle};
use rust_fly::render::Renderer;
use rust_fly::world::World;
use std::path::Path;
use std::time::Instant;

fn configuration() -> Conf {
    Conf {
        window_title: "Rust Fly".into(),
        window_width: 1152,
        window_height: 648,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

fn draw_frame(texture: &Texture2D) {
    clear_background(BLACK);
    draw_texture_ex(
        texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            ..Default::default()
        },
    );
}

fn help(flight: &Flight, theme: usize, milliseconds: f32, paused: bool) {
    let mode = ["OVERHEAD", "CHASE", "COCKPIT"][flight.view];
    let light = ["DAY", "SUNSET", "NIGHT"][theme];
    let size = 18.0;
    let title = format!("RUST / FLY    {mode}    {light}");
    draw_rectangle(18.0, 16.0, 344.0, 28.0, Color::new(0.035, 0.07, 0.12, 0.68));
    draw_text(
        &title,
        28.0,
        36.0,
        size,
        Color::from_rgba(226, 239, 243, 255),
    );
    if flight.demo {
        draw_text(flight.demonstration(), 28.0, 66.0, 18.0, ORANGE);
    }
    let status = format!(
        "SPD {:03.0}   ALT {:03.0}   {:.1} ms{}",
        flight.speed,
        flight.position.z,
        milliseconds,
        if paused { "   PAUSED" } else { "" }
    );
    let measure = measure_text(&status, None, 16, 1.0);
    let x = screen_width() - measure.width - 28.0;
    draw_rectangle(
        x - 10.0,
        16.0,
        measure.width + 20.0,
        28.0,
        Color::new(0.035, 0.07, 0.12, 0.68),
    );
    draw_text(&status, x, 35.0, 16.0, Color::from_rgba(226, 239, 243, 255));
    let hint = "MOVE TO STEER  /  CLICK TO ROLL  /  HOLD TO BOOST  /  SPACE VIEW";
    let secondary =
        "WASD FLY   SHIFT BOOST   J DEMO   B SLICES   T LIGHT   P PAUSE   R RESET   H HIDE";
    for (text, bottom, size) in [(hint, 39.0, 18), (secondary, 17.0, 14)] {
        let measure = measure_text(text, None, size, 1.0);
        let x = (screen_width() - measure.width) * 0.5;
        let y = screen_height() - bottom;
        draw_rectangle(
            x - 10.0,
            y - size as f32,
            measure.width + 20.0,
            size as f32 + 7.0,
            Color::new(0.035, 0.07, 0.12, 0.7),
        );
        draw_text(
            text,
            x,
            y,
            size as f32,
            Color::from_rgba(226, 239, 243, 255),
        );
    }
}

async fn capture(directory: &Path, world: &World, renderer: &mut Renderer) {
    std::fs::create_dir_all(directory).expect("无法创建截图目录");
    let mut flight = Flight::new(world);
    for _ in 0..180 {
        flight.step(1.0 / 60.0, &Controls::default(), world);
    }
    let (w, h) = (384, 216);
    let mut measurements = Vec::new();
    for (name, view, theme, exploded) in [
        ("overhead", 0, 0, false),
        ("chase", 1, 0, false),
        ("cockpit", 2, 0, false),
        ("sunset", 1, 1, false),
        ("night", 1, 2, false),
        ("slices", 0, 0, true),
    ] {
        flight.view = view;
        flight.weights = [0.0; 3];
        flight.weights[view] = 1.0;
        flight.spread = if exploded { 5.0 } else { 1.0 };
        let start = Instant::now();
        for _ in 0..30 {
            renderer.render(world, &flight, w, h, theme);
        }
        let milliseconds = start.elapsed().as_secs_f64() * 1000.0 / 30.0;
        let texture = Texture2D::from_rgba8(w as u16, h as u16, &renderer.rgba);
        texture.set_filter(FilterMode::Nearest);
        draw_frame(&texture);
        help(&flight, theme, milliseconds as f32, false);
        get_screen_data().export_png(directory.join(format!("{name}.png")).to_str().unwrap());
        next_frame().await;
        measurements.push(format!("\"{name}\": {milliseconds:.3}"));
        println!("{name}: {milliseconds:.2} ms / frame");
    }
    let mut checks = Flight::new(world);
    let before = checks.position;
    checks.roll();
    for _ in 0..90 {
        checks.step(
            1.0 / 60.0,
            &Controls {
                bank: None,
                turn: 0.8,
                climb: 0.5,
                boost: true,
                manual: true,
            },
            world,
        );
    }
    assert!(checks.position.distance(before) > 100.0);
    assert!(checks.position.z > before.z);
    assert!(checks.speed > 150.0 && checks.rate > 0.8);
    assert!(checks.spin == 0.0 && !checks.trails.is_empty());
    checks.view = 2;
    for _ in 0..180 {
        checks.step(1.0 / 60.0, &Controls::default(), world);
    }
    assert!(checks.weights[2] > 0.99);
    assert!(checks.position.is_finite());
    std::fs::write(directory.join("verification.json"), format!("{{\n  \"resolution\": [384, 216],\n  \"render_ms\": {{{}}},\n  \"flight_checks\": \"passed: movement, climb, boost, turn, roll, trails, camera transition\"\n}}\n",measurements.join(", "))).unwrap();
    println!("飞行状态检查通过，截图已保存至 {}", directory.display());
}

#[macroquad::main(configuration)]
async fn main() {
    let started = Instant::now();
    let world = World::new(7);
    println!(
        "地形生成完成：{:.0} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    let mut flight = Flight::new(&world);
    let mut renderer = Renderer::new();
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(i) = arguments.iter().position(|a| a == "--capture") {
        let directory = arguments.get(i + 1).expect("请在 --capture 后指定截图目录");
        capture(Path::new(directory), &world, &mut renderer).await;
        return;
    }
    let mut texture = Texture2D::empty();
    let mut dimensions = (0, 0);
    let mut theme = 0;
    let mut hint = true;
    let mut paused = false;
    let mut fullscreen = false;
    let mut pointer = mouse_position();
    let mut pointer_idle = 10.0;
    let mut held_for = 0.0;
    let mut render_ms = 0.0;
    loop {
        let dt = get_frame_time().min(0.05);
        let width = (screen_width() / 3.0).round().clamp(160.0, 720.0) as usize;
        let height = (width as f32 * screen_height() / screen_width())
            .round()
            .clamp(90.0, 720.0) as usize;
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::Space) {
            flight.view = (flight.view + 1) % 3;
        }
        if is_key_pressed(KeyCode::J) {
            flight.set_demo(!flight.demo);
        }
        if is_key_pressed(KeyCode::V) {
            flight.wing_count = (flight.wing_count + 1) % 3;
        }
        if is_key_pressed(KeyCode::T) {
            theme = (theme + 1) % 3;
        }
        if is_key_pressed(KeyCode::B) {
            flight.exploded = !flight.exploded;
        }
        if is_key_pressed(KeyCode::H) {
            hint = !hint;
        }
        if is_key_pressed(KeyCode::P) {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::R) {
            flight = Flight::new(&world);
        }
        if is_key_pressed(KeyCode::F) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_key_pressed(KeyCode::Q) {
            flight.roll();
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            held_for = 0.0;
        }
        if is_mouse_button_down(MouseButton::Left) {
            held_for += dt;
        }
        if is_mouse_button_released(MouseButton::Left) && held_for < 0.2 {
            flight.roll();
        }
        let next_pointer = mouse_position();
        pointer_idle += dt;
        if (next_pointer.0 - pointer.0).abs() + (next_pointer.1 - pointer.1).abs() > 1.0 {
            pointer_idle = 0.0;
        }
        pointer = next_pointer;
        let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
        let right = is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);
        let up = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
        let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
        let keyboard = left || right || up || down;
        if keyboard {
            flight.set_demo(false);
        }
        let mut controls = Controls {
            bank: None,
            turn: f32::from(right) - f32::from(left),
            climb: f32::from(up) - f32::from(down),
            boost: is_key_down(KeyCode::LeftShift)
                || is_key_down(KeyCode::RightShift)
                || (is_mouse_button_down(MouseButton::Left) && held_for >= 0.2),
            manual: keyboard,
        };
        if !keyboard && pointer_idle < 5.0 {
            controls.manual = true;
            let mouse = vec2(
                pointer.0 / screen_width() * width as f32,
                pointer.1 / screen_height() * height as f32,
            );
            let camera = Camera::new(&flight, width, height);
            if flight.view == 0 {
                if let Some((center, _, _)) = camera.project(flight.position) {
                    let delta = mouse - center;
                    if delta.length() > 18.0 {
                        let (s, c) = camera.yaw.sin_cos();
                        let direction = vec2(delta.x * c + delta.y * s, -delta.x * s + delta.y * c);
                        controls.turn = (angle(direction.x.atan2(-direction.y) - flight.yaw) * 2.6
                            / 1.25)
                            .clamp(-1.0, 1.0);
                    }
                }
            } else {
                controls.turn =
                    ((mouse.x - width as f32 * 0.5) / (width as f32 * 0.26)).clamp(-1.0, 1.0);
                controls.climb =
                    ((mouse.y - height as f32 * 0.5) / (height as f32 * 0.4)).clamp(-1.0, 1.0);
            }
        }
        if !paused {
            flight.step(dt, &controls, &world);
        }
        let start = Instant::now();
        renderer.render(&world, &flight, width, height, theme);
        render_ms += (start.elapsed().as_secs_f32() * 1000.0 - render_ms) * 0.1;
        if dimensions != (width, height) {
            texture = Texture2D::from_rgba8(width as u16, height as u16, &renderer.rgba);
            texture.set_filter(FilterMode::Nearest);
            dimensions = (width, height);
        } else {
            texture.update_from_bytes(width as u32, height as u32, &renderer.rgba);
        }
        draw_frame(&texture);
        if hint {
            help(&flight, theme, render_ms, paused);
        }
        next_frame().await;
    }
}
