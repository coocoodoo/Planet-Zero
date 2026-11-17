use super::*;
use crate::app::chart::{CHART_SIZE, offset};
use crate::flight::Flight;
use crate::gameplay::{Mission, PickupKind, Weapon};
use crate::world::SIZE;
use glam::{Vec2, Vec3, vec2};

const INK: [u8; 3] = [3, 8, 13];
const MAP_ORIGIN: Vec2 = vec2(5.0, 116.0);
const MAP_CENTER: Vec2 = vec2(29.0, 136.0);
const MAP_SCALE: f32 = 0.05;

fn compass(c: &mut Canvas<'_>, heading: f32) {
    c.line(96, 16, 224, 16, INK);
    c.line(96, 15, 224, 15, MUTED);
    let start = (heading / 15.0).floor() as i32;
    for i in start - 4..=start + 4 {
        let delta = i as f32 * 15.0 - heading;
        if delta.abs() > 46.0 {
            continue;
        }
        let x = 160 + (delta * 1.2).round() as i32;
        let major = i.rem_euclid(3) == 0;
        let y = if major { 10 } else { 12 };
        c.rect(x - 1, y - 1, 3, 17 - y, INK);
        c.line(x, y, x, 15, WHITE);
        if major {
            let label = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]
                [(i.div_euclid(3)).rem_euclid(8) as usize];
            c.osd(x - Canvas::osd_width(label) / 2, 1, label, WHITE);
        }
    }
    c.line(157, 18, 160, 15, CYAN);
    c.line(160, 15, 163, 18, CYAN);
    c.osd(
        152,
        21,
        &format!("{:03}", heading.round().rem_euclid(360.0) as u32),
        CYAN,
    );
}

fn instruments(c: &mut Canvas<'_>, f: &Flight, ground: Option<f32>, chase: bool, cockpit: bool) {
    if chase {
        for (x, value, step, right) in [(38, f.speed, 10.0, false), (270, f.position.z, 25.0, true)]
        {
            c.rect(x - 1, 53, 3, 55, INK);
            c.line(x, 54, x, 106, MUTED);
            let base = (value / step).floor() as i32;
            for i in base - 3..=base + 3 {
                let y = 82 - ((i as f32 * step - value) / step * 9.0).round() as i32;
                if !(54..=106).contains(&y) {
                    continue;
                }
                let width = if i % 2 == 0 { 5 } else { 3 };
                let end = x + if right { width } else { -width };
                c.line(x, y + 1, end, y + 1, INK);
                c.line(x, y, end, y, WHITE);
            }
            let direction = if right { 1 } else { -1 };
            c.line(x, 82, x + direction * 5, 79, CYAN);
            c.line(x, 82, x + direction * 5, 85, CYAN);
        }
    }
    let (left, right, y) = if cockpit { (78, 242, 47) } else { (5, 315, 66) };
    c.osd(left, y, "SPD", MUTED);
    c.osd_value(left, y + 12, &format!("{:03.0}", f.speed), WHITE);
    c.osd(left, y + 28, "U/S", MUTED);
    c.osd_right(right, y, "ALT", MUTED);
    let altitude = format!("{:.0}", f.position.z);
    c.osd_value(
        right - altitude.len() as i32 * 8 + 1,
        y + 12,
        &altitude,
        WHITE,
    );
    c.osd_right(right, y + 28, "U", MUTED);
    if !cockpit {
        c.osd_right(
            right,
            y + 40,
            &ground.map_or_else(
                || "AGL --".to_owned(),
                |h| format!("AGL {:.0}", f.position.z - h),
            ),
            MUTED,
        );
    }
}

fn hardware(c: &mut Canvas<'_>, t: crate::app::telemetry::Telemetry, full: bool) {
    let battery_color = match t.battery_percent {
        Some(0..=15) => RED,
        Some(_) => WHITE,
        None => MUTED,
    };
    c.rect(229, 2, 15, 10, INK);
    c.rect(230, 3, 12, 8, battery_color);
    c.rect(231, 4, 10, 6, INK);
    c.rect(242, 5, 2, 4, battery_color);
    if let Some(percent) = t.battery_percent {
        c.rect(232, 5, (percent as i32 * 8 + 99) / 100, 4, battery_color);
    }
    c.osd(
        249,
        4,
        &t.battery_percent
            .map_or_else(|| "--%".to_owned(), |v| format!("{v}%")),
        battery_color,
    );
    if t.charging {
        c.line(281, 3, 278, 6, CYAN);
        c.line(278, 6, 281, 6, CYAN);
        c.line(281, 6, 278, 9, CYAN);
    }
    let bars = t
        .wifi_dbm
        .map_or(0, |v| ((v as i32 + 95) / 12 + 1).clamp(1, 4));
    let signal_color = match t.wifi_dbm {
        Some(v) if v < -85 => RED,
        Some(v) if v < -75 => GOLD,
        Some(_) => WHITE,
        None => MUTED,
    };
    let (sx, bottom) = if full { (262, 26) } else { (286, 11) };
    for i in 0..4 {
        let h = 2 + i * 2;
        c.rect(sx + i * 3 - 1, bottom - h - 1, 3, h + 2, INK);
        c.rect(
            sx + i * 3,
            bottom - h,
            1,
            h,
            if i < bars { signal_color } else { [56, 70, 81] },
        );
    }
    if full {
        c.osd(
            230,
            19,
            &t.battery_voltage
                .map_or_else(|| "-.--V".to_owned(), |v| format!("{v:.2}V")),
            WHITE,
        );
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            c.small(301 + dx, 21 + dy, "dBm", INK);
        }
        c.small(301, 21, "dBm", signal_color);
        c.osd_right(
            298,
            19,
            &t.wifi_dbm
                .map_or_else(|| "--".to_owned(), |v| v.to_string()),
            signal_color,
        );
    }
    c.shade(302, 1, 17, 12);
    c.icon(307, 3, 'u', WHITE);
}

fn map_point(position: Vec3, player: Vec3) -> Vec2 {
    MAP_CENTER + offset(position.truncate(), player.truncate()) * MAP_SCALE
}
fn map_marker(c: &mut Canvas<'_>, p: Vec2, color: [u8; 3], portal: bool) {
    if p.x < 8.0 || p.x > 50.0 || p.y < 119.0 || p.y > 153.0 {
        return;
    }
    let (x, y) = (p.x as i32, p.y as i32);
    if portal {
        c.line(x, y - 2, x + 2, y, color);
        c.line(x + 2, y, x, y + 2, color);
        c.line(x, y + 2, x - 2, y, color);
        c.line(x - 2, y, x, y - 2, color);
    } else {
        c.rect(x, y, 2, 2, color);
    }
}

fn minimap(app: &mut App) {
    app.minimap.prepare(&app.world, app.flight.position);
    let f = &app.flight;
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    for y in 0..40 {
        for x in 0..48 {
            let point =
                f.position.truncate() + (vec2(x as f32, y as f32) - vec2(24.0, 20.0)) / MAP_SCALE;
            let p = point.rem_euclid(Vec2::splat(SIZE as f32)) / SIZE as f32 * CHART_SIZE as f32;
            let color = app.minimap.pixels
                [(p.y as usize % CHART_SIZE) * CHART_SIZE + p.x as usize % CHART_SIZE]
                .map(|v| v / 3 + 7);
            c.pixel(MAP_ORIGIN.x as i32 + x, MAP_ORIGIN.y as i32 + y, color);
        }
    }
    for (x, y, dx, dy) in [
        (5, 116, 1, 1),
        (52, 116, -1, 1),
        (5, 155, 1, -1),
        (52, 155, -1, -1),
    ] {
        c.line(x, y, x + dx * 5, y, MUTED);
        c.line(x, y, x, y + dy * 5, MUTED);
    }
    if app.network.active {
        for route in app.network.routes(&app.world) {
            if !f.hides_portal(route.aperture) {
                map_marker(
                    &mut c,
                    map_point(route.aperture.center, f.position),
                    CYAN,
                    true,
                );
            }
        }
    }
    for gate in app
        .mission
        .gates
        .iter()
        .skip(app.mission.gate_index)
        .take(4)
    {
        map_marker(&mut c, map_point(gate.position, f.position), GOLD, false);
    }
    for pickup in app
        .mission
        .pickups
        .iter()
        .filter(|p| !p.collected && p.life > 0.0)
    {
        map_marker(
            &mut c,
            map_point(pickup.position, f.position),
            if pickup.kind == PickupKind::Coin {
                GOLD
            } else {
                CYAN
            },
            false,
        );
    }
    for enemy in app.mission.enemies.iter().filter(|e| e.health > 0.0) {
        map_marker(&mut c, map_point(enemy.position, f.position), RED, false);
    }
    if let Some(convoy) = &app.mission.convoy {
        map_marker(&mut c, map_point(convoy.position, f.position), CYAN, false);
    }
    let forward = f.axes()[1].truncate().normalize_or_zero();
    let right = vec2(-forward.y, forward.x);
    let tip = MAP_CENTER + forward * 4.0;
    let left = MAP_CENTER - forward * 3.0 - right * 2.0;
    let right = MAP_CENTER - forward * 3.0 + right * 2.0;
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
    c.osd(8, 119, "N", MUTED);
    c.osd(39, 146, "1K", MUTED);
}

fn combat(c: &mut Canvas<'_>, m: &Mission, full: bool) {
    use crate::gameplay::progression::Phase;
    let (left, status_y, values_y) = if full { (64, 119, 132) } else { (5, 145, 158) };
    c.osd(left, status_y, &format!("ST {:02}", m.level), GOLD);
    c.icon(left + 38, status_y, m.phase().icon(), CYAN);
    let objective = if matches!(m.phase(), Phase::Patrol | Phase::Strike) {
        format!("{:02}/{:02}", m.objective, m.objective_goal())
    } else {
        format!("{:03.0}S", (m.phase_duration() - m.phase_elapsed).max(0.0))
    };
    c.osd(left + 50, status_y, &objective, WHITE);
    let (health, energy, weapon_x, bomb, overdrive, auto) = if full {
        (64, 122, 184, 207, 240, 291)
    } else {
        (5, 59, 118, 145, 181, 291)
    };
    c.icon(health, values_y, 'S', CYAN);
    c.osd(health + 11, values_y, &format!("{:.0}", m.health), WHITE);
    c.bar(health + 33, values_y + 2, 16, m.health / m.max_health, CYAN);
    c.icon(energy, values_y, 'A', CYAN);
    c.osd(energy + 11, values_y, &format!("{:.0}", m.energy), WHITE);
    c.bar(energy + 33, values_y + 2, 16, m.energy / 100.0, CYAN);
    let weapon = match m.weapon {
        Weapon::Gun => 'G',
        Weapon::Missile => 'M',
        Weapon::Laser => 'L',
        Weapon::Plasma => 'P',
        Weapon::Orb => 'O',
    };
    c.osd(
        weapon_x,
        values_y,
        &format!("{weapon}{}", m.levels[m.weapon as usize]),
        m.weapon.color(),
    );
    c.icon(bomb, values_y, 'B', GOLD);
    c.osd(bomb + 11, values_y, &m.bombs.to_string(), WHITE);
    c.icon(overdrive, values_y, 'Y', GOLD);
    c.bar(
        overdrive + 12,
        values_y + 2,
        if full { 27 } else { 25 },
        if m.overdrive_time > 0.0 {
            m.overdrive_time / 8.0
        } else {
            m.overdrive / 100.0
        },
        GOLD,
    );
    c.osd(
        auto,
        values_y,
        if m.autofire { "AUTO" } else { "MAN" },
        MUTED,
    );
    let (boss_x, boss_y) = if full { (64, 145) } else { (99, 145) };
    if let Some(boss) = m.enemies.iter().find(|e| e.is_boss()) {
        c.osd(boss_x, boss_y, "BOSS", RED);
        c.bar(
            boss_x + 30,
            boss_y + 2,
            70,
            boss.health / boss.max_health,
            RED,
        );
    } else if let Some(convoy) = &m.convoy {
        c.osd(boss_x, boss_y, "ESCORT", CYAN);
        c.bar(
            boss_x + 42,
            boss_y + 2,
            58,
            convoy.health / convoy.max_health,
            CYAN,
        );
    }
    if m.support > 0.0 {
        let (x, y) = if full { (188, 145) } else { (232, 158) };
        c.icon(x, y, 'W', CYAN);
        c.osd(x + 11, y, &format!("{:.0}S", m.support), WHITE);
    }
    for (i, n) in m.notices.iter().flatten().enumerate() {
        c.icon(
            286 + i as i32 * 10,
            145,
            n.kind.icon(),
            if n.kind == crate::gameplay::progression::NoticeKind::Hit {
                RED
            } else {
                GOLD
            },
        );
    }
    if m.flash > 0.0 {
        c.rect(0, 0, 2, 170, RED);
        c.rect(318, 0, 2, 170, RED);
    }
}

pub fn draw(app: &mut App) {
    let cockpit = app.flight.weights[2] >= 0.5;
    let chase = app.flight.weights[1] >= 0.5;
    if !cockpit {
        minimap(app);
    }
    let f = &app.flight;
    let m = &app.mission;
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let forward = f.axes()[1];
    let heading = forward.x.atan2(-forward.y).to_degrees().rem_euclid(360.0);
    compass(&mut c, heading);
    instruments(
        &mut c,
        f,
        (app.world.kind != crate::world::MapKind::Space)
            .then(|| app.world.flight_height(f.position)),
        chase,
        cockpit,
    );
    hardware(&mut c, app.telemetry, chase);
    c.icon(5, 4, 'O', WHITE);
    let seconds = m.elapsed as u32;
    let time = if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    };
    c.osd(16, 4, &time, WHITE);
    c.weather(5, 20, f.weather, app.world.kind.vacuum());
    let status = if app.loader.pending.is_some() {
        'O'
    } else if f.demo {
        'd'
    } else if app.input_label == "IMU" {
        'i'
    } else if app.auto {
        'a'
    } else {
        ' '
    };
    c.icon(17, 20, status, CYAN);
    c.osd(30, 20, app.world.kind.name(), MUTED);
    if app.input_label == "CAL" {
        let text = "HOLD STILL / K CALIBRATE";
        c.osd(160 - Canvas::osd_width(text) / 2, 35, text, GOLD);
    }
    if app.input_label == "FAIL" {
        c.osd(5, if m.mode == Mode::Free { 33 } else { 44 }, "IMU --", RED);
    }
    if m.mode != Mode::Free {
        c.osd(5, 33, &format!("T-{:03.0}", m.remaining), GOLD);
        c.osd_right(
            315,
            if chase { 159 } else { 33 },
            &format!("{:07}", m.score),
            GOLD,
        );
        if m.mode == Mode::Combat {
            combat(&mut c, m, !cockpit);
        } else {
            let (x, y) = if !cockpit { (64, 119) } else { (5, 145) };
            let label = if m.mode == Mode::Race {
                format!("GATE {}/{}", m.gate_index + 1, m.gates.len())
            } else {
                format!("COINS {} X{}", m.collected, m.combo)
            };
            c.osd(x, y, &label, GOLD);
            if m.mode == Mode::Coins && m.combo > 0 {
                c.bar(
                    x,
                    y + 13,
                    68,
                    m.combo_time / (5.0 - m.difficulty as usize as f32),
                    GOLD,
                );
            }
        }
        if (chase || m.mode != Mode::Combat)
            && let Some(target) = m.target(f)
        {
            let d = target - f.position;
            let bearing = d.x.atan2(-d.y).to_degrees().rem_euclid(360.0);
            c.osd_right(
                315,
                if !cockpit { 119 } else { 145 },
                &format!("BRG {:03} {:.0}U", bearing as u32, d.length()),
                CYAN,
            );
        }
        if m.countdown > 0.0 {
            c.osd_value(157, 48, &format!("{}", m.countdown.ceil() as u32), GOLD);
        }
    } else if chase {
        c.osd(64, 144, "LOC", MUTED);
        c.osd_right(315, 159, f.vehicle.name(), MUTED);
    }
    if chase {
        c.osd(
            64,
            159,
            &format!(
                "X{:04} Y{:04}",
                f.position.x.rem_euclid(SIZE as f32) as usize % SIZE,
                f.position.y.rem_euclid(SIZE as f32) as usize % SIZE
            ),
            WHITE,
        );
    }
}
