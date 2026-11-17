use super::*;
use crate::app::chart::{CHART_SIZE, offset};
use crate::gameplay::PickupKind;
use crate::world::celestial::Structure;
use glam::{Vec2, Vec3, vec2};

const ORIGIN: Vec2 = vec2(8.0, 24.0);

fn inside(p: Vec2, margin: f32) -> bool {
    p.x >= margin
        && p.y >= margin
        && p.x < CHART_SIZE as f32 - margin
        && p.y < CHART_SIZE as f32 - margin
}
fn marker(canvas: &mut Canvas<'_>, p: Vec2, color: [u8; 3], radius: i32) {
    if inside(p, radius as f32 + 1.0) {
        let p = p + ORIGIN;
        canvas.rect(
            p.x as i32 - radius,
            p.y as i32 - radius,
            radius * 2 + 1,
            radius * 2 + 1,
            color,
        );
    }
}
fn route_line(canvas: &mut Canvas<'_>, start: Vec2, end: Vec2, color: [u8; 3]) {
    let steps = (end - start).length().min(400.0).ceil() as usize;
    for i in 0..=steps {
        let p = start.lerp(end, i as f32 / steps.max(1) as f32);
        if inside(p, 1.0) && i % 4 < 2 {
            canvas.pixel((p.x + ORIGIN.x) as i32, (p.y + ORIGIN.y) as i32, color);
        }
    }
}

pub fn draw(app: &mut App) {
    app.chart.prepare(&app.world, app.flight.position);
    let chart = &app.chart;
    let flight = &app.flight;
    let mission = &app.mission;
    let routes = app.network.routes(&app.world);
    let selected = routes.get(chart.selected % routes.len().max(1));
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    c.rect(0, 0, 320, 170, [7, 13, 23]);
    c.text(8, 7, &format!("NAV / {}", app.world.kind.name()), CYAN);
    c.text(208, 7, "FLIGHT PAUSED", MUTED);
    c.rect(
        7,
        23,
        CHART_SIZE as i32 + 2,
        CHART_SIZE as i32 + 2,
        [57, 87, 105],
    );
    for y in 0..CHART_SIZE {
        for x in 0..CHART_SIZE {
            c.pixel(
                8 + x as i32,
                24 + y as i32,
                chart.pixels[y * CHART_SIZE + x],
            );
        }
    }
    for object in &app.world.scenery {
        let color = match object.kind {
            Structure::Base | Structure::Cruiser | Structure::Satellite => MUTED,
            Structure::Building | Structure::House | Structure::Warehouse => [100, 114, 127],
            _ => continue,
        };
        marker(&mut c, chart.point(object.position), color, 0);
    }
    let player = chart
        .point(flight.position)
        .clamp(Vec2::splat(6.0), Vec2::splat(CHART_SIZE as f32 - 7.0));
    if let Some(route) = selected {
        let delta = offset(route.aperture.center.truncate(), flight.position.truncate());
        route_line(
            &mut c,
            player,
            player + delta * CHART_SIZE as f32 / chart.span,
            CYAN,
        );
    }
    for (i, route) in routes.iter().enumerate() {
        let p = chart.point(route.aperture.center);
        let color = if Some(route.id) == selected.map(|r| r.id) {
            CYAN
        } else {
            [123, 163, 249]
        };
        if inside(p, 4.0) {
            let pos = p + ORIGIN;
            let (x, y) = (pos.x as i32, pos.y as i32);
            c.line(x, y - 3, x + 3, y, color);
            c.line(x + 3, y, x, y + 3, color);
            c.line(x, y + 3, x - 3, y, color);
            c.line(x - 3, y, x, y - 3, color);
            if inside(p, 12.0) {
                c.small(x + 5, y - 2, &(i + 1).to_string(), color);
            }
        }
    }
    for (i, gate) in mission.gates.iter().enumerate().skip(mission.gate_index) {
        marker(
            &mut c,
            chart.point(gate.position),
            if i == mission.gate_index {
                GOLD
            } else {
                [125, 140, 73]
            },
            1,
        );
    }
    for pickup in mission
        .pickups
        .iter()
        .filter(|p| !p.collected && p.life > 0.0)
    {
        marker(
            &mut c,
            chart.point(pickup.position),
            if pickup.kind == PickupKind::Coin {
                GOLD
            } else {
                CYAN
            },
            0,
        );
    }
    for enemy in mission.enemies.iter().filter(|e| e.health > 0.0) {
        marker(
            &mut c,
            chart.point(enemy.position),
            RED,
            if enemy.is_boss() { 2 } else { 1 },
        );
    }
    if let Some(convoy) = &mission.convoy {
        marker(&mut c, chart.point(convoy.position), CYAN, 2);
    }
    if inside(player, 5.0) {
        let forward = flight.axes()[1].truncate().normalize_or_zero();
        let side = vec2(-forward.y, forward.x);
        let tip = player + forward * 5.0 + ORIGIN;
        let left = player - forward * 3.0 - side * 3.0 + ORIGIN;
        let right = player - forward * 3.0 + side * 3.0 + ORIGIN;
        c.line(
            tip.x as i32,
            tip.y as i32,
            left.x as i32,
            left.y as i32,
            WHITE,
        );
        c.line(
            tip.x as i32,
            tip.y as i32,
            right.x as i32,
            right.y as i32,
            WHITE,
        );
        c.line(
            left.x as i32,
            left.y as i32,
            right.x as i32,
            right.y as i32,
            WHITE,
        );
    }
    c.text(12, 28, "N", WHITE);
    c.small(12, 151, &format!("{:.0}", chart.span), WHITE);
    c.text(156, 25, flight.vehicle.name(), WHITE);
    c.text(
        156,
        37,
        &format!(
            "HDG {:03} ALT {:.0}",
            flight.yaw.to_degrees().rem_euclid(360.0) as u32,
            flight.position.z
        ),
        MUTED,
    );
    if let Some(route) = selected {
        let delta = offset(route.aperture.center.truncate(), flight.position.truncate());
        let distance = Vec3::new(
            delta.x,
            delta.y,
            route.aperture.center.z - flight.position.z,
        )
        .length();
        let bearing = delta.x.atan2(-delta.y).to_degrees().rem_euclid(360.0);
        c.text(
            156,
            53,
            &format!(
                "PORTAL {}/{}",
                chart.selected % routes.len() + 1,
                routes.len()
            ),
            CYAN,
        );
        c.text(156, 65, route.target.kind.name(), WHITE);
        c.text(156, 77, &format!("RANGE {:.0} U", distance), MUTED);
        c.text(
            156,
            89,
            &format!(
                "BRG {:03} ALT {:.0}",
                bearing as u32, route.aperture.center.z
            ),
            MUTED,
        );
        c.text(
            156,
            101,
            if app.network.get(route.target).is_some() {
                "DESTINATION READY"
            } else {
                "PRELOAD PENDING"
            },
            MUTED,
        );
    }
    let objective = match app.mode {
        Mode::Free => "CYAN: PORTAL".to_owned(),
        Mode::Race => format!("GATE {}/{}", mission.gate_index + 1, mission.gates.len()),
        Mode::Coins => format!("COINS {}", mission.collected),
        Mode::Combat => format!(
            "HOSTILES {}",
            mission.enemies.iter().filter(|e| e.health > 0.0).count()
        ),
    };
    c.text(
        156,
        117,
        &objective,
        if app.mode == Mode::Combat { RED } else { GOLD },
    );
    c.text(
        156,
        135,
        if app.device_controls {
            "F ZOOM IN"
        } else {
            "UP ZOOM IN"
        },
        CYAN,
    );
    c.text(
        156,
        147,
        if app.device_controls {
            "X ZOOM OUT"
        } else {
            "DOWN ZOOM OUT"
        },
        CYAN,
    );
    c.text(156, 159, "M CLOSE CHART", CYAN);
}
