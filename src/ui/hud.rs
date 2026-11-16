use super::*;
pub fn draw(app: &mut App) {
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let m = &app.mission;
    let f = &app.flight;
    if !f.hud {
        return;
    }
    if m.mode == Mode::Combat {
        combat_hud(&mut c, m, f, app.input_label);
        return;
    }
    c.shade(0, 0, 320, 13);
    if m.mode == Mode::Free {
        c.text(
            4,
            3,
            &format!(
                "{} {:03.0} ALT {:03.0}",
                ["TOP", "CHASE", "PIT"][f.view],
                f.speed,
                f.position.z
            ),
            WHITE,
        );
        c.text(200, 3, if f.demo { "DEMO" } else { app.input_label }, CYAN);
    } else {
        c.text(
            4,
            3,
            &format!(
                "{:07} {:03.0}S STAGE {:02}",
                m.score, m.remaining, app.level
            ),
            WHITE,
        );
    }
    c.rect(279, 1, 40, 11, [36, 73, 86]);
    c.text(280, 3, "U UI", WHITE);
    if m.mode == Mode::Free {
        let label = app.loader.pending.map_or_else(
            || {
                format!(
                    "{} / {}",
                    app.world.kind.name(),
                    if app.world.kind.vacuum() {
                        "VACUUM"
                    } else {
                        f.weather.name()
                    }
                )
            },
            |map| format!("LOADING {}", map.name()),
        );
        c.shade(4, 16, (label.len() * 8 + 8).min(312), 12);
        c.text(8, 18, &label, MUTED);
        if f.demo {
            c.shade(4, 29, 192, 12);
            c.text(8, 31, f.demonstration(), GOLD);
        }
        c.shade(0, 158, 320, 12);
        c.text(4, 161, "4 VIEW 5 ROLL 6 BOOST 8 MENU", WHITE);
    } else {
        if let Some(target) = m.target(f) {
            let delta = target - f.position;
            let heading = angle(delta.x.atan2(-delta.y) - f.yaw);
            let direction = if heading.abs() < 0.14 {
                "AHEAD"
            } else if heading > 0.0 {
                "RIGHT"
            } else {
                "LEFT"
            };
            let altitude = if delta.z > 12.0 {
                "UP"
            } else if delta.z < -12.0 {
                "DOWN"
            } else {
                "LEVEL"
            };
            let label = if m.mode == Mode::Race {
                format!(
                    "GATE {:02}/{} {} {} {:03.0}",
                    m.gate_index + 1,
                    m.gates.len(),
                    direction,
                    altitude,
                    delta.length()
                )
            } else {
                format!("{} {} {:03.0}", direction, altitude, delta.length())
            };
            c.shade(4, 16, (label.len() * 8 + 8).min(312), 11);
            c.text(8, 18, &label, CYAN);
        }
        if m.mode == Mode::Coins && m.combo > 0 {
            c.shade(4, 30, 120, 23);
            c.text(8, 32, &format!("CHAIN X{}", m.combo), GOLD);
            c.bar(
                8,
                46,
                104,
                m.combo_time / (5.0 - m.difficulty as usize as f32),
                GOLD,
            );
        }
        c.shade(0, 158, 320, 12);
        c.text(4, 161, "4 VIEW 6 BOOST R RETRY 8 PAUSE", WHITE);
        if m.message_time > 0.0 && m.countdown <= 0.0 {
            let width = (m.message.len() * 8 + 8).min(312);
            let x = (320 - width) / 2;
            c.shade(x, 61, width, 12);
            c.text(x as i32 + 4, 63, m.message, GOLD);
        }
        if m.countdown > 0.0 {
            c.shade(132, 51, 56, 42);
            c.text_scale(148, 59, &format!("{}", m.countdown.ceil() as u32), GOLD, 3);
        }
    }
    if app.input_label == "CAL" {
        c.shade(60, 79, 200, 13);
        c.text(64, 82, "HOLD STILL / K CALIBRATE", GOLD);
    }
}
