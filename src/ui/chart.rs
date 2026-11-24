use super::*;
use crate::app::chart::CHART_SIZE;
use crate::world::{SIZE, network::MapKey, orbit::Body};
use glam::{Vec2, vec2};

const ORIGIN: Vec2 = vec2(24.0, 28.0);
fn inside(p: Vec2, margin: f32) -> bool {
    p.min_element() >= margin && p.max_element() < CHART_SIZE as f32 - margin
}
fn marker(c: &mut Canvas<'_>, p: Vec2, color: [u8; 3], radius: i32) {
    if inside(p, radius as f32 + 1.0) {
        let p = p + ORIGIN;
        c.rect(
            p.x as i32 - radius,
            p.y as i32 - radius,
            radius * 2 + 1,
            radius * 2 + 1,
            color,
        );
    }
}
fn route_line(c: &mut Canvas<'_>, start: Vec2, end: Vec2, color: [u8; 3]) {
    let d = end - start;
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for axis in 0..2 {
        if d[axis].abs() < 0.001 {
            if !(1.0..135.0).contains(&start[axis]) {
                return;
            }
        } else {
            let a = (1.0 - start[axis]) / d[axis];
            let b = (134.0 - start[axis]) / d[axis];
            lo = lo.max(a.min(b));
            hi = hi.min(a.max(b));
        }
    }
    if lo > hi {
        return;
    }
    let a = start + d * lo;
    let b = start + d * hi;
    let steps = a.distance(b).ceil() as usize;
    for i in 0..=steps {
        let p = a.lerp(b, i as f32 / steps.max(1) as f32) + ORIGIN;
        if i % 4 < 2 {
            c.pixel(p.x as i32, p.y as i32, color);
        }
    }
}

pub fn draw(app: &mut App) {
    app.chart.prepare(&app.world, app.flight.position);
    let names = Body::ALL.map(|body| app.body_name(body));
    let name = |body: Body| names[Body::ALL.iter().position(|b| *b == body).unwrap()].as_str();
    let chart = &app.chart;
    let f = &app.flight;
    let routes = app.network.routes(&app.world);
    let target = app
        .navigation
        .target
        .filter(|t| t.map == MapKey::of(&app.world));
    let focused = routes.iter().min_by(|a, b| {
        (a.aperture.center.truncate() - chart.center)
            .length_squared()
            .total_cmp(&(b.aperture.center.truncate() - chart.center).length_squared())
    });
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    c.rect(0, 0, 320, 170, [7, 13, 23]);
    c.osd(5, 4, &format!("NAV / {}", app.world.kind.name()), CYAN);
    c.osd_right(314, 4, &format!("{}X / PAUSED", 1 << chart.zoom), MUTED);
    c.rect(23, 27, 138, 138, [57, 87, 105]);
    for y in 0..CHART_SIZE {
        for x in 0..CHART_SIZE {
            c.pixel(
                24 + x as i32,
                28 + y as i32,
                chart.pixels[y * CHART_SIZE + x],
            );
        }
    }
    let grid = chart.span / 4.0;
    let corner = chart.center - Vec2::splat(chart.span * 0.5);
    for axis in 0..2 {
        for i in 0..=4 {
            let coordinate = (corner[axis] / grid).ceil() * grid + i as f32 * grid;
            let at = ((coordinate - corner[axis]) / chart.span * CHART_SIZE as f32).round() as i32;
            if !(0..136).contains(&at) {
                continue;
            }
            for n in (0..136).step_by(3) {
                let (x, y) = if axis == 0 {
                    (24 + at, 28 + n)
                } else {
                    (24 + n, 28 + at)
                };
                c.pixel(x, y, [74, 96, 109]);
            }
            let label = if chart.space {
                format!("{:.0}K", coordinate / 1000.0)
            } else {
                format!("{coordinate:.0}")
            };
            if axis == 0 && at <= 112 {
                c.osd(25 + at, 17, &label, MUTED);
            }
            if axis == 1 && at <= 126 {
                c.osd(0, 30 + at, &label, MUTED);
            }
        }
    }
    if chart.space {
        for body in Body::ALL {
            let p = chart.point(body.center());
            let radius = (body.radius() * CHART_SIZE as f32 / chart.span).max(3.0);
            let color = if body == Body::Earth { CYAN } else { WHITE };
            for i in 0..64 {
                let a = i as f32 * std::f32::consts::TAU / 64.0;
                let q = p + vec2(a.sin(), a.cos()) * radius;
                if inside(q, 1.0) {
                    c.pixel((q.x + ORIGIN.x) as i32, (q.y + ORIGIN.y) as i32, color);
                }
            }
            if inside(p, 22.0) {
                c.osd(
                    (p.x + ORIGIN.x - 12.0) as i32,
                    (p.y + ORIGIN.y + radius + 3.0) as i32,
                    name(body),
                    color,
                );
            }
        }
    }
    let player = chart.point(f.position);
    if let Some(t) = target {
        route_line(
            &mut c,
            player,
            player + t.offset(f.position) * CHART_SIZE as f32 / chart.span,
            GOLD,
        );
        let p = chart.point(t.position);
        if inside(p, 5.0) {
            let p = p + ORIGIN;
            for side in [-1, 1] {
                c.line(
                    p.x as i32,
                    p.y as i32 - 4,
                    p.x as i32 + side * 4,
                    p.y as i32,
                    GOLD,
                );
                c.line(
                    p.x as i32,
                    p.y as i32 + 4,
                    p.x as i32 + side * 4,
                    p.y as i32,
                    GOLD,
                );
            }
        }
    }
    for (i, route) in routes.iter().enumerate() {
        let p = chart.point(route.aperture.center);
        if inside(p, 5.0) {
            let q = p + ORIGIN;
            let (x, y) = (q.x as i32, q.y as i32);
            c.line(x, y - 3, x + 3, y, CYAN);
            c.line(x + 3, y, x, y + 3, CYAN);
            c.line(x, y + 3, x - 3, y, CYAN);
            c.line(x - 3, y, x, y - 3, CYAN);
            if inside(p, 12.0) {
                c.osd(x + 5, y - 3, &(i + 1).to_string(), CYAN);
            }
        }
    }
    for gate in app.mission.gates.iter().skip(app.mission.gate_index) {
        marker(&mut c, chart.point(gate.position), GOLD, 1);
    }
    for pickup in app
        .mission
        .pickups
        .iter()
        .filter(|p| !p.collected && p.life > 0.0)
    {
        marker(&mut c, chart.point(pickup.position), GOLD, 0);
    }
    for enemy in app.mission.enemies.iter().filter(|e| e.health > 0.0) {
        marker(
            &mut c,
            chart.point(enemy.position),
            RED,
            if enemy.is_boss() { 2 } else { 1 },
        );
    }
    if inside(player, 5.0) {
        let forward = f.axes()[1].truncate().normalize_or_zero();
        let side = vec2(-forward.y, forward.x);
        let tip = player + forward * 5.0 + ORIGIN;
        let left = player - forward * 3.0 - side * 3.0 + ORIGIN;
        let right = player - forward * 3.0 + side * 3.0 + ORIGIN;
        for (a, b) in [(tip, left), (tip, right), (left, right)] {
            c.line(a.x as i32, a.y as i32, b.x as i32, b.y as i32, WHITE);
        }
    }
    let center = ORIGIN + Vec2::splat(CHART_SIZE as f32 * 0.5);
    for side in [-1, 1] {
        c.line(
            center.x as i32 + side * 4,
            center.y as i32,
            center.x as i32 + side * 9,
            center.y as i32,
            WHITE,
        );
        c.line(
            center.x as i32,
            center.y as i32 + side * 4,
            center.x as i32,
            center.y as i32 + side * 9,
            WHITE,
        );
    }
    if chart.space {
        c.osd(171, 18, "LANDING / EARTH", CYAN);
        c.osd(
            171,
            29,
            app.orbit.earth.map_or("PREPARING", |key| key.kind.name()),
            GOLD,
        );
        c.osd(
            171,
            41,
            &format!("X{:.0} Y{:.0}", chart.center.x, chart.center.y),
            WHITE,
        );
        if let Some(t) = target {
            c.osd(171, 53, t.body.map_or("WAYPOINT", name), CYAN);
            c.osd(
                171,
                64,
                &format!(
                    "Z{:.0} D{:.0}",
                    t.position.z,
                    t.position.distance(f.position)
                ),
                MUTED,
            );
        }
        for (i, line) in [
            "▲ ▼ ◀ ▶ PAN",
            "1 ZOOM IN",
            "2 ZOOM OUT",
            "3 FIND PORTAL",
            "4 FIND AIRCRAFT",
            "5 SELECT EARTH",
            "6 SELECT MOON",
            "7 CHANGE LANDING",
            "ENTER AUTO NAV",
            "M RETURN",
        ]
        .iter()
        .enumerate()
        {
            c.osd(
                171,
                77 + i as i32 * 9,
                line,
                if i == 8 { GOLD } else { MUTED },
            );
        }
        return;
    }
    let x = chart.center.x.rem_euclid(SIZE as f32) as u32;
    let y = chart.center.y.rem_euclid(SIZE as f32) as u32;
    c.osd(
        171,
        18,
        &format!("GRID {}{}", (b'A' + (x / 512) as u8) as char, y / 512 + 1),
        GOLD,
    );
    c.osd(
        171,
        30,
        &format!("X{:.0} Y{:.0}", chart.center.x, chart.center.y),
        WHITE,
    );
    c.osd(171, 42, &format!("SPAN {:.0} U", chart.span), MUTED);
    c.osd(
        171,
        57,
        if target.is_some() {
            "WAYPOINT MARKED"
        } else {
            "SPACE MARK TARGET"
        },
        CYAN,
    );
    if let Some(t) = target {
        let delta = t.offset(f.position);
        c.osd(171, 69, &format!("RANGE {:.0} U", delta.length()), WHITE);
    } else if let Some(route) = focused {
        c.osd(171, 69, route.target.kind.name(), WHITE);
    }
    for (i, line) in [
        "▲ ▼ ◀ ▶ PAN",
        "1 ZOOM IN",
        "2 ZOOM OUT",
        "3 FIND PORTAL",
        "4 FIND AIRCRAFT",
        "ENTER AUTO NAV",
        "M RETURN",
    ]
    .iter()
    .enumerate()
    {
        c.osd(
            171,
            85 + i as i32 * 11,
            line,
            if i == 5 { GOLD } else { MUTED },
        );
    }
}
