use super::*;
pub fn draw(app: &mut App) {
    if app.flight.hud && app.uses_fpv_hud() {
        super::fpv::draw(app);
        return;
    }
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
    c.shade(302, 1, 17, 12);
    c.icon(307, 3, 'u', WHITE);
    if m.mode == Mode::Free {
        c.shade(2, 1, 112, 12);
        c.weather(5, 3, f.weather.target, app.world.kind.vacuum());
        c.text(17, 3, app.world.kind.name(), MUTED);
        c.shade(124, 1, 174, 12);
        let status = if f.hover {
            format!("HOVER  ALT {:03.0}", f.position.z)
        } else {
            format!("{:03.0}  ALT {:03.0}", f.speed, f.position.z)
        };
        c.text(128, 3, &status, if f.hover { CYAN } else { WHITE });
        if f.demo || app.auto || app.input_label == "IMU" || app.loader.pending.is_some() {
            c.shade(2, 16, 13, 12);
            c.icon(
                5,
                18,
                if app.loader.pending.is_some() {
                    'O'
                } else if f.demo {
                    'd'
                } else if app.input_label == "IMU" {
                    'i'
                } else {
                    'a'
                },
                CYAN,
            );
        }
    } else {
        c.shade(2, 1, 278, 12);
        c.weather(5, 3, f.weather.target, app.world.kind.vacuum());
        c.text(
            17,
            3,
            &format!(
                "{:07} {:03.0}S STAGE {:02}",
                m.score, m.remaining, app.level
            ),
            WHITE,
        );
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
        c.text(64, 82, "HOLD STILL / KEYS ACTIVE", GOLD);
    }
}
