use super::*;
use flight_club::flight::{Controls, angle};
use glam::Vec3;

pub(super) fn high_resolution(
    options: &Options,
    game: &mut Game,
    scale: usize,
) -> Result<(), Box<dyn Error>> {
    game.start();
    game.flight.snap_view(options.view);
    for _ in 0..60 {
        game.step(1.0 / 30.0);
    }
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let started = Instant::now();
    let mut renderer = flight_club::render::Renderer::new();
    renderer.render(&game.world, &game.flight, w, h, 0);
    println!(
        "{{\"scene\":\"hd\",\"scale\":{scale},\"width\":{w},\"height\":{h},\"milliseconds\":{:.2}}}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    if let Some(path) = &options.capture {
        write_ppm(path, &format!("hd-{scale}x"), w, h, &renderer.rgba)?;
    }
    Ok(())
}

pub(super) fn travel(
    options: &Options,
    game: &mut Game,
    hops: usize,
) -> Result<(), Box<dyn Error>> {
    use flight_club::world::network::MapKey;
    game.start();
    game.auto = false;
    game.network.active = true;
    game.flight.wing_count = 0;
    game.flight.snap_view(options.view);
    let controls = Controls {
        manual: true,
        ..Default::default()
    };
    for hop in 0..hops {
        for _ in 0..75 {
            game.app.step(1.0 / 30.0, Some(Controls { ..controls }));
        }
        let world = game.world.clone();
        game.network.ensure(&world);
        let routes = game.network.routes(&world);
        let route = routes[hop % routes.len()];
        let started = Instant::now();
        game.network.request(route.target);
        while game.network.get(route.target).is_none() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            game.network.poll();
            game.network.request(route.target);
        }
        let load_ms = started.elapsed().as_secs_f64() * 1000.0;
        let entry = route.aperture;
        game.flight.position = entry.center + entry.normal() * 200.0;
        game.flight.follow = game.flight.position;
        game.flight.yaw = entry.yaw + std::f32::consts::PI;
        game.flight.pitch = 0.0;
        game.flight.camera_yaw = -game.flight.yaw;
        game.flight.hud = false;
        let mut samples = Vec::new();
        for frame in 0..180 {
            let began = Instant::now();
            game.app.step(1.0 / 30.0, Some(Controls { ..controls }));
            game.draw();
            samples.push(began.elapsed().as_secs_f64() * 1000.0);
            if let Some(path) = &options.capture
                && [0, 25, 40].contains(&frame)
            {
                capture(path, &format!("travel-{hop}-{frame}"), game)?;
            }
            if MapKey::of(&game.world) == route.target {
                break;
            }
        }
        let arrived = MapKey::of(&game.world) == route.target;
        report(&format!("travel-{hop}"), 1, &mut samples);
        println!(
            "{{\"from\":\"{}\",\"to\":\"{}\",\"altitude\":{:.1},\"load_ms\":{load_ms:.1},\"arrived\":{arrived}}}",
            world.kind.name(),
            route.target.kind.name(),
            entry.center.z
        );
        if !arrived {
            return Err("跨地图穿越未到达目标区域".into());
        }
        if let Some(path) = &options.capture {
            capture(path, &format!("travel-{hop}-exit"), game)?;
        }
    }
    Ok(())
}

pub(super) fn maneuvers(
    options: &Options,
    game: &mut Game,
    frames: usize,
) -> Result<(), Box<dyn Error>> {
    for view in [1, 2] {
        for direction in [-1.0, 1.0] {
            game.mode = flight_club::gameplay::Mode::Free;
            game.start();
            game.auto = false;
            game.flight.position.z = game.flight.loop_altitude() + 160.0;
            game.flight.follow = game.flight.position;
            game.flight.snap_view(view);
            game.flight.wing_count = 0;
            game.flight.hud = false;
            let name = format!(
                "loop-{}-{}",
                view,
                if direction > 0.0 { "up" } else { "down" }
            );
            let mut samples = Vec::with_capacity(frames);
            let mut rotation = 0.0;
            let mut stages = [0.0; 3];
            for frame in 0..frames {
                let before = game.flight.pitch;
                let start = Instant::now();
                game.app.step(
                    1.0 / 30.0,
                    Some(Controls {
                        climb: direction,
                        manual: true,
                        aerobatic: true,
                        ..Default::default()
                    }),
                );
                rotation += angle(game.flight.pitch - before);
                game.draw();
                for (total, value) in stages.iter_mut().zip(game.renderer.stages) {
                    *total += value;
                }
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
                if let Some(path) = &options.capture
                    && [0, 40, 80, 120, 160].contains(&frame)
                {
                    capture(path, &format!("{name}-{frame}"), game)?;
                }
            }
            report(&name, 1, &mut samples);
            eprintln!("{name} stages: {:?}", stages.map(|v| v / frames as f64));
            println!(
                "{{\"scene\":\"{name}\",\"rotation_degrees\":{:.1}}}",
                rotation.to_degrees()
            );
            if rotation * direction < std::f32::consts::TAU {
                return Err(format!("{name} 未完成整圈翻转").into());
            }
        }
    }
    game.start();
    game.auto = false;
    game.flight.position.z = (game.world.ceiling as f32 + 130.0).max(180.0);
    let world = game.world.clone();
    game.network.ensure(&world);
    let route = game.network.routes(&world)[0];
    game.network
        .remember(World::generate(route.target.kind, route.target.seed).into());
    let entry = route.aperture;
    for orbit in [0.0_f32, 1.0, 2.0, 3.0] {
        game.flight.position =
            entry.center + Vec3::new(orbit.sin() * 140.0, orbit.cos() * 140.0, 30.0);
        game.flight.follow = game.flight.position;
        game.flight.yaw = (-orbit.sin()).atan2(orbit.cos());
        game.flight.pitch = 0.0;
        game.flight.camera_yaw = -game.flight.yaw;
        game.flight.view = 1;
        game.flight.weights = [0.0, 1.0, 0.0];
        game.flight.hud = false;
        game.flight.wing_count = 0;
        game.draw();
        if let Some(path) = &options.capture {
            capture(path, &format!("portal-orbit-{}", orbit as usize), game)?;
        }
    }
    Ok(())
}
