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
    c.line(84, 13, 236, 13, INK);
    c.line(84, 12, 236, 12, MUTED);
    let start = (heading / 15.0).floor() as i32;
    for i in start - 5..=start + 5 {
        let bearing = i as f32 * 15.0;
        let delta = bearing - heading;
        if delta.abs() > 55.0 {
            continue;
        }
        let x = 160 + (delta * 1.25).round() as i32;
        let major = i.rem_euclid(3) == 0;
        c.rect(
            x - 1,
            if major { 7 } else { 9 },
            3,
            if major { 6 } else { 4 },
            INK,
        );
        c.line(x, if major { 7 } else { 9 }, x, 12, WHITE);
        if major {
            let label = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]
                [(i.div_euclid(3)).rem_euclid(8) as usize];
            c.osd(x - (label.len() as i32 * 4 - 1) / 2, 0, label, WHITE);
        }
    }
    c.line(157, 14, 160, 11, CYAN);
    c.line(160, 11, 163, 14, CYAN);
    c.osd(
        154,
        18,
        &format!("{:03}", heading.round().rem_euclid(360.0) as u32),
        CYAN,
    );
}

fn tapes(c: &mut Canvas<'_>, f: &Flight, ground: Option<f32>) {
    for (x, value, step, right) in [(38, f.speed, 10.0, false), (270, f.position.z, 25.0, true)] {
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
    c.osd(6, 66, "SPD", MUTED);
    c.osd_value(5, 78, &format!("{:03.0}", f.speed), WHITE);
    c.osd(6, 93, "U/S", MUTED);
    c.osd(300, 66, "ALT", MUTED);
    let altitude = format!("{:.0}", f.position.z);
    c.osd_value(316 - altitude.len() as i32 * 8, 78, &altitude, WHITE);
    c.osd(310, 93, "U", MUTED);
    c.osd(
        282,
        105,
        &ground.map_or_else(
            || "AGL --".to_owned(),
            |h| format!("AGL {:.0}", f.position.z - h),
        ),
        MUTED,
    );
}

fn hardware(c: &mut Canvas<'_>, t: crate::app::telemetry::Telemetry) {
    let battery_color = match t.battery_percent {
        Some(0..=15) => RED,
        Some(_) => WHITE,
        None => MUTED,
    };
    c.rect(234, 2, 14, 8, INK);
    c.rect(235, 3, 11, 6, battery_color);
    c.rect(236, 4, 9, 4, INK);
    c.rect(246, 5, 1, 2, battery_color);
    if let Some(percent) = t.battery_percent {
        c.rect(237, 5, (percent as i32 * 7 + 99) / 100, 2, battery_color);
    }
    c.osd(
        251,
        4,
        &t.battery_percent
            .map_or_else(|| "--%".to_owned(), |v| format!("{v}%")),
        battery_color,
    );
    c.osd(
        270,
        4,
        &t.battery_voltage
            .map_or_else(|| "-.--V".to_owned(), |v| format!("{v:.2}V")),
        WHITE,
    );
    if t.charging {
        c.line(297, 2, 294, 5, CYAN);
        c.line(294, 5, 297, 5, CYAN);
        c.line(297, 5, 294, 8, CYAN);
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
    for i in 0..4 {
        let h = 2 + i * 2;
        c.rect(234 + i * 3, 23 - h, 3, h + 1, INK);
        c.rect(
            235 + i * 3,
            23 - h,
            1,
            h,
            if i < bars { signal_color } else { [56, 70, 81] },
        );
    }
    c.osd(251, 17, "WIFI", MUTED);
    c.osd(
        271,
        17,
        &t.wifi_dbm
            .map_or_else(|| "--DBM".to_owned(), |v| format!("{v}DBM")),
        signal_color,
    );
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
    c.osd(42, 148, "1K", MUTED);
}

fn combat(c: &mut Canvas<'_>, m: &Mission, input: &str) {
    use crate::gameplay::progression::Phase;
    c.osd(64, 119, &format!("ST {:02}", m.level), GOLD);
    c.icon(93, 118, m.phase().icon(), CYAN);
    let objective = if m.phase() == Phase::Patrol || m.phase() == Phase::Strike {
        format!("{:02}/{:02}", m.objective, m.objective_goal())
    } else {
        format!("{:03.0}S", (m.phase_duration() - m.phase_elapsed).max(0.0))
    };
    c.osd(105, 119, &objective, WHITE);
    for (i, n) in m.notices.iter().flatten().enumerate() {
        c.icon(
            286 + i as i32 * 10,
            118,
            n.kind.icon(),
            if n.kind == crate::gameplay::progression::NoticeKind::Hit {
                RED
            } else {
                GOLD
            },
        );
    }
    c.icon(64, 131, 'S', CYAN);
    c.osd(74, 132, &format!("{:.0}", m.health), WHITE);
    c.bar(91, 133, 27, m.health / m.max_health, CYAN);
    c.icon(125, 131, 'A', CYAN);
    c.osd(135, 132, &format!("{:.0}", m.energy), WHITE);
    c.bar(151, 133, 24, m.energy / 100.0, CYAN);
    let weapon = match m.weapon {
        Weapon::Gun => 'G',
        Weapon::Missile => 'M',
        Weapon::Laser => 'L',
        Weapon::Plasma => 'P',
        Weapon::Orb => 'O',
    };
    c.osd(
        186,
        132,
        &format!("{weapon}{}", m.levels[m.weapon as usize]),
        m.weapon.color(),
    );
    c.icon(207, 131, 'B', GOLD);
    c.osd(217, 132, &m.bombs.to_string(), WHITE);
    c.icon(238, 131, 'Y', GOLD);
    c.bar(
        249,
        133,
        37,
        if m.overdrive_time > 0.0 {
            m.overdrive_time / 8.0
        } else {
            m.overdrive / 100.0
        },
        GOLD,
    );
    c.osd(295, 132, if m.autofire { "AUTO" } else { "MAN" }, MUTED);
    if let Some(boss) = m.enemies.iter().find(|e| e.is_boss()) {
        c.osd(64, 144, "BOSS", RED);
        c.bar(88, 145, 96, boss.health / boss.max_health, RED);
    } else if let Some(convoy) = &m.convoy {
        c.osd(64, 144, "ESCORT", CYAN);
        c.bar(92, 145, 92, convoy.health / convoy.max_health, CYAN);
    }
    if m.support > 0.0 {
        c.icon(207, 143, 'W', CYAN);
        c.osd(217, 144, &format!("{:.0}S", m.support), WHITE);
    }
    if input == "FAIL" {
        c.icon(300, 143, '!', RED);
    }
    c.osd(252, 159, &format!("{:07}", m.score), GOLD);
    if m.flash > 0.0 {
        c.rect(0, 0, 2, 170, RED);
        c.rect(318, 0, 2, 170, RED);
    }
}

pub fn draw(app: &mut App) {
    minimap(app);
    let f = &app.flight;
    let m = &app.mission;
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let forward = f.axes()[1];
    let heading = forward.x.atan2(-forward.y).to_degrees().rem_euclid(360.0);
    compass(&mut c, heading);
    tapes(
        &mut c,
        f,
        (app.world.kind != crate::world::MapKind::Space)
            .then(|| app.world.flight_height(f.position)),
    );
    hardware(&mut c, app.telemetry);
    c.icon(5, 3, 'O', WHITE);
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
    c.osd(16, 5, &time, WHITE);
    c.weather(5, 18, f.weather, app.world.kind.vacuum());
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
    c.icon(17, 18, status, CYAN);
    c.osd(30, 20, app.world.kind.name(), MUTED);
    if app.input_label == "CAL" {
        c.osd(96, 35, "HOLD STILL / K CALIBRATE", GOLD);
    }
    if app.input_label == "FAIL" {
        c.osd(5, if m.mode == Mode::Free { 31 } else { 41 }, "IMU --", RED);
    }
    if m.mode != Mode::Free {
        c.osd(5, 31, &format!("T-{:03.0}", m.remaining), GOLD);
        if m.mode == Mode::Combat {
            combat(&mut c, m, app.input_label);
        } else {
            let label = if m.mode == Mode::Race {
                format!("GATE {}/{}", m.gate_index + 1, m.gates.len())
            } else {
                format!("COINS {} X{}", m.collected, m.combo)
            };
            c.osd(64, 122, &label, GOLD);
            c.osd(252, 159, &format!("{:07}", m.score), GOLD);
            if m.mode == Mode::Coins && m.combo > 0 {
                c.bar(
                    64,
                    132,
                    54,
                    m.combo_time / (5.0 - m.difficulty as usize as f32),
                    GOLD,
                );
            }
        }
        if let Some(target) = m.target(f) {
            let d = target - f.position;
            let bearing = d.x.atan2(-d.y).to_degrees().rem_euclid(360.0);
            c.osd(
                148,
                119,
                &format!("BRG {:03} {:.0}U", bearing as u32, d.length()),
                CYAN,
            );
        }
        if m.countdown > 0.0 {
            c.osd_value(156, 48, &format!("{}", m.countdown.ceil() as u32), GOLD);
        }
    } else {
        c.osd(64, 144, "GPS SIM", MUTED);
        c.osd(252, 159, f.vehicle.name(), MUTED);
    }
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
