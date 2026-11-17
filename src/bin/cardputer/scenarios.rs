use super::*;
use glam::Vec3;
use rust_fly::flight::{Controls, angle};

pub(super) fn maneuvers(
    options: &Options,
    game: &mut Game,
    frames: usize,
) -> Result<(), Box<dyn Error>> {
    for view in [1, 2] {
        for direction in [-1.0, 1.0] {
            game.mode = rust_fly::gameplay::Mode::Free;
            game.start();
            game.auto = false;
            game.flight.position.z = game.world.ceiling as f32 + 330.0;
            game.flight.follow = game.flight.position;
            game.flight.view = view;
            game.flight.weights = [0.0; 3];
            game.flight.weights[view] = 1.0;
            game.flight.wing_count = 0;
            game.flight.hud = false;
            let name = format!(
                "loop-{}-{}",
                view,
                if direction > 0.0 { "up" } else { "down" }
            );
            let mut samples = Vec::with_capacity(frames);
            let mut rotation = 0.0;
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
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
                if let Some(path) = &options.capture
                    && [0, 40, 80, 120, 160].contains(&frame)
                {
                    capture(path, &format!("{name}-{frame}"), game)?;
                }
            }
            report(&name, 1, &mut samples);
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
    let (position, forward) = (game.flight.position, game.flight.axes()[1]);
    game.world.spawn_portals(position, forward);
    let entry = game.world.portals[0];
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
