use crate::app::{App, HEIGHT, Page, WIDTH};
use crate::arcade::Mode;
use crate::flight::angle;
use font8x8::UnicodeFonts;

const WHITE: [u8; 3] = [225, 238, 244];
const MUTED: [u8; 3] = [145, 171, 188];
const CYAN: [u8; 3] = [111, 237, 218];
const GOLD: [u8; 3] = [255, 208, 105];
const RED: [u8; 3] = [255, 112, 130];

struct Canvas<'a> {
    pixels: &'a mut [u8],
}
impl Canvas<'_> {
    fn pixel(&mut self, x: i32, y: i32, c: [u8; 3]) {
        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
            let i = (y as usize * WIDTH + x as usize) * 4;
            self.pixels[i..i + 3].copy_from_slice(&c);
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.pixel(xx, yy, c);
            }
        }
    }
    fn shade(&mut self, x: usize, y: usize, w: usize, h: usize) {
        for yy in y..(y + h).min(HEIGHT) {
            for xx in x..(x + w).min(WIDTH) {
                let i = (yy * WIDTH + xx) * 4;
                for c in 0..3 {
                    self.pixels[i + c] = (self.pixels[i + c] as u16 / 4 + [7, 13, 22][c]) as u8;
                }
            }
        }
    }
    fn text(&mut self, x: i32, y: i32, value: &str, c: [u8; 3]) {
        self.text_scale(x, y, value, c, 1);
    }
    fn text_scale(&mut self, x: i32, y: i32, value: &str, c: [u8; 3], scale: i32) {
        for (i, ch) in value.chars().enumerate() {
            if let Some(glyph) = font8x8::BASIC_FONTS.get(ch) {
                for (gy, row) in glyph.iter().enumerate() {
                    for gx in 0..8 {
                        if row & (1 << gx) != 0 {
                            self.rect(
                                x + (i as i32 * 8 + gx) * scale,
                                y + gy as i32 * scale,
                                scale,
                                scale,
                                c,
                            );
                        }
                    }
                }
            }
        }
    }
    fn line(&mut self, x: i32, y: i32, xx: i32, yy: i32, c: [u8; 3]) {
        let steps = (xx - x).abs().max((yy - y).abs()).max(1);
        for i in 0..=steps {
            self.pixel(x + (xx - x) * i / steps, y + (yy - y) * i / steps, c);
        }
    }
    fn bar(&mut self, x: i32, y: i32, w: i32, value: f32, c: [u8; 3]) {
        self.rect(x, y, w, 4, [38, 48, 65]);
        self.rect(x, y, (value.clamp(0.0, 1.0) * w as f32) as i32, 4, c);
    }
}

pub fn draw(app: &mut App) {
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let m = &app.mission;
    let f = &app.flight;
    if app.page == Page::Menu {
        c.shade(0, 0, 320, 32);
        c.text_scale(10, 7, "RUST / FLY", WHITE, 2);
        c.text(224, 8, "FLIGHT CLUB", CYAN);
        c.text(224, 20, "ARCADE 0.4", MUTED);
        c.shade(8, 34, 304, 115);
        c.rect(8, 34, 2, 115, CYAN);
        let rows = [
            ("MODE", app.mode.name()),
            ("AIRCRAFT", f.vehicle.name()),
            ("MAP", app.loader.pending.unwrap_or(app.world.kind).name()),
            ("WEATHER", f.weather.name()),
            ("LEVEL", app.difficulty.name()),
            (
                "",
                if app.loader.pending.is_some() {
                    "GENERATING MAP"
                } else {
                    "START FLIGHT"
                },
            ),
            ("", "FLIGHT MANUAL"),
            ("", "EXIT TO LAUNCHER"),
        ];
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = 37 + i as i32 * 14;
            if i == app.selection {
                c.rect(12, y - 1, 296, 12, [36, 73, 86]);
                c.text(16, y, ">", GOLD);
            }
            c.text(30, y, label, MUTED);
            c.text(112, y, value, if i == app.selection { GOLD } else { WHITE });
        }
        let description = if app.selection == 1 {
            f.vehicle.description()
        } else {
            app.mode.description()
        };
        c.shade(0, 151, 320, 19);
        c.text(8, 152, description, CYAN);
        c.text(8, 162, "Z/C CHANGE  F/X SELECT  ENTER GO", MUTED);
        return;
    }
    if app.page == Page::Help {
        c.shade(6, 6, 308, 158);
        c.rect(6, 6, 2, 158, CYAN);
        let pages = [
            [
                "FLIGHT MANUAL / 1 OF 3",
                "Z X C F / WASD : STEER / CLIMB",
                "4 SPACE VIEW   5 Q ROLL/EVADE",
                "6 SHIFT BOOST  7 T LIGHT",
                "I IMU ON/OFF   K RECALIBRATE",
                "U HIDE UI      8 H P PAUSE",
                "HOLD ESC 2 SECONDS TO EXIT",
                "Z/C PAGE       ENTER RETURN",
            ],
            [
                "MISSION MANUAL / 2 OF 3",
                "RACE : CROSS GREEN GATES IN ORDER",
                "GATE CENTER GIVES PERFECT BONUS",
                "COINS : 5 SECOND CHAIN / MAX X10",
                "WHITE PICKUP ADDS 8 SECONDS",
                "RADAR AND ARROW POINT TO TARGET",
                "R RETRY        ESC PAUSE",
                "Z/C PAGE       ENTER RETURN",
            ],
            [
                "COMBAT MANUAL / 3 OF 3",
                "3 L FIRE       2 AUTO FIRE",
                "O WEAPON       1 E NOVA BOMB",
                "Q ROLL : BRIEF DAMAGE PROTECTION",
                "COLOR ORBS UPGRADE EACH WEAPON",
                "RED NOVA  GREEN SHIELD  BLUE AMMO",
                "VIOLET WINGMEN SUPPORT / 30 SEC",
                "Z/C PAGE       ENTER RETURN",
            ],
        ];
        for (i, line) in pages[app.help_page].iter().enumerate() {
            c.text(
                14,
                17 + i as i32 * 19,
                line,
                if i == 0 { GOLD } else { WHITE },
            );
        }
        return;
    }
    if app.page == Page::Paused {
        c.shade(35, 16, 250, 140);
        c.rect(35, 16, 2, 140, CYAN);
        c.text_scale(56, 23, "PAUSED", WHITE, 2);
        let rows = [
            "RESUME FLIGHT",
            "RETRY MISSION",
            if f.hud { "HIDE UI / U" } else { "SHOW UI / U" },
            "FLIGHT MANUAL",
            "MAIN MENU",
            "EXIT TO LAUNCHER",
        ];
        for (i, line) in rows.iter().enumerate() {
            let y = 48 + i as i32 * 16;
            if i == app.selection {
                c.rect(45, y - 2, 230, 13, [36, 73, 86]);
                c.text(48, y, ">", GOLD);
            }
            c.text(64, y, line, if i == app.selection { GOLD } else { WHITE });
        }
        return;
    }
    if app.page == Page::Results {
        c.shade(12, 12, 296, 146);
        let won = m.outcome == Some(true);
        c.rect(12, 12, 296, 2, if won { CYAN } else { RED });
        c.text_scale(
            28,
            22,
            if won { "MISSION CLEAR" } else { "FLIGHT ENDED" },
            if won { CYAN } else { RED },
            2,
        );
        let medal = if !won {
            "KEEP FLYING"
        } else if m.mode == Mode::Race {
            if m.perfect >= 12 {
                "GOLD WINGS"
            } else if m.perfect >= 6 {
                "SILVER WINGS"
            } else {
                "BRONZE WINGS"
            }
        } else if m.score >= 20000 {
            "GOLD WINGS"
        } else if m.score >= 8000 {
            "SILVER WINGS"
        } else {
            "BRONZE WINGS"
        };
        c.text(28, 46, medal, GOLD);
        c.text(
            28,
            62,
            &format!("SCORE {:07}   TIME {:03.1}", m.score, m.elapsed),
            WHITE,
        );
        match m.mode {
            Mode::Race => {
                c.text(
                    28,
                    78,
                    &format!("GATES {:02}/18   PERFECT {:02}", m.gate_index, m.perfect),
                    WHITE,
                );
            }
            Mode::Coins => {
                c.text(
                    28,
                    78,
                    &format!("COINS {:02}      BEST CHAIN X{}", m.collected, m.best_combo),
                    WHITE,
                );
            }
            Mode::Combat => {
                c.text(
                    28,
                    78,
                    &format!(
                        "KILLS {:02}  WAVE {}  CHAIN X{}",
                        m.kills, m.wave, m.best_combo
                    ),
                    WHITE,
                );
            }
            Mode::Free => {}
        }
        c.text(
            28,
            94,
            &format!(
                "BEST  {:07}{}",
                app.best_score,
                if app.new_record { "  NEW RECORD" } else { "" }
            ),
            CYAN,
        );
        c.text(
            28,
            110,
            if app.record_error {
                "SAVE FAILED / SCORE IN MEMORY"
            } else {
                f.vehicle.description()
            },
            MUTED,
        );
        c.rect(24, 130, 272, 17, [36, 73, 86]);
        c.text(32, 135, "ENTER RETRY  /  ESC MAIN MENU", GOLD);
        return;
    }
    if !f.hud {
        return;
    }
    c.shade(0, 0, 320, 13);
    if m.mode == Mode::Free {
        c.text(
            4,
            3,
            &format!(
                "{} {:03.0}  ALT {:03.0}",
                ["TOP", "CHASE", "PIT"][f.view],
                f.speed,
                f.position.z
            ),
            WHITE,
        );
        c.text(196, 3, if f.demo { "DEMO" } else { app.input_label }, CYAN);
    } else {
        c.text(
            4,
            3,
            &format!("{:07}  {:03.0}S", m.score, m.remaining),
            WHITE,
        );
        c.text(
            152,
            3,
            match m.mode {
                Mode::Race => "RACE",
                Mode::Coins => "COINS",
                _ => "ASSAULT",
            },
            CYAN,
        );
        c.text(224, 3, app.input_label, MUTED);
    }
    c.rect(279, 1, 40, 11, [36, 73, 86]);
    c.text(280, 3, "U UI", WHITE);
    if m.mode == Mode::Free {
        let label = app.loader.pending.map_or_else(
            || format!("{} / {}", app.world.kind.name(), f.weather.name()),
            |map| format!("LOADING {}", map.name()),
        );
        c.shade(4, 16, (label.len() * 8 + 8).min(312), 12);
        c.text(8, 18, &label, MUTED);
        if f.demo {
            c.shade(4, 29, 192, 12);
            c.text(8, 31, f.demonstration(), GOLD);
        }
        c.shade(0, 158, 320, 12);
        c.text(4, 161, "4 VIEW  5 ROLL  6 BOOST  8 MENU", WHITE);
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
                    "GATE {:02}/18 {} {} {:03.0}",
                    m.gate_index + 1,
                    direction,
                    altitude,
                    delta.length()
                )
            } else {
                format!("{} {} {:03.0}", direction, altitude, delta.length())
            };
            c.shade(4, 16, (label.len() * 8 + 8).min(312), 11);
            c.text(8, 18, &label, CYAN);
            c.shade(266, 91, 50, 53);
            for i in 0..32 {
                let a = i as f32 * std::f32::consts::TAU / 32.0;
                c.pixel(
                    291 + (a.sin() * 21.0) as i32,
                    116 + (a.cos() * 21.0) as i32,
                    [79, 126, 142],
                );
            }
            c.line(287, 119, 291, 112, WHITE);
            c.line(291, 112, 295, 119, WHITE);
            let px = 291 + (heading.sin() * 18.0) as i32;
            let py = 116 - (heading.cos() * 18.0) as i32;
            c.rect(px - 1, py - 1, 3, 3, GOLD);
            for enemy in &m.enemies {
                let d = enemy.position - f.position;
                let a = angle(d.x.atan2(-d.y) - f.yaw);
                let r = (d.length() / 20.0).min(20.0);
                c.pixel(291 + (a.sin() * r) as i32, 116 - (a.cos() * r) as i32, RED);
            }
        }
        if m.mode == Mode::Combat {
            c.shade(0, 145, 320, 25);
            c.text(
                4,
                147,
                &format!(
                    "{} L{}  E{:03.0}  NOVA {}",
                    m.weapon.name(),
                    m.levels[m.weapon as usize],
                    m.energy,
                    m.bombs
                ),
                GOLD,
            );
            c.bar(
                4,
                158,
                104,
                m.health / m.max_health,
                if m.health / m.max_health < 0.3 {
                    RED
                } else {
                    CYAN
                },
            );
            c.bar(4, 164, 104, m.energy / 100.0, [98, 183, 255]);
            c.text(
                116,
                160,
                if m.autofire {
                    "2 AUTO ON  O GUN  E NOVA"
                } else {
                    "3 FIRE  O GUN  E NOVA"
                },
                WHITE,
            );
            if let Some(boss) = m.enemies.iter().find(|e| e.kind >= 3) {
                c.shade(70, 29, 180, 19);
                c.text(
                    78,
                    30,
                    &format!(
                        "{} / {}",
                        boss.boss_name(),
                        if boss.health < boss.max_health * 0.5 {
                            "RAGE"
                        } else {
                            "ARMOR"
                        }
                    ),
                    RED,
                );
                c.bar(78, 42, 164, boss.health / boss.max_health, RED);
            } else {
                c.shade(4, 29, 104, 11);
                c.text(8, 31, &format!("WAVE {} / 5", m.wave), MUTED);
            }
            if m.support > 0.0 {
                c.text(8, 132, &format!("WINGMEN {:02.0}S", m.support), CYAN);
            }
        } else {
            c.shade(0, 158, 320, 12);
            c.text(
                4,
                161,
                if m.mode == Mode::Race {
                    "4 VIEW 6 BOOST  R RETRY  8 PAUSE"
                } else {
                    "GOLD COINS / WHITE +8S / GOAL 3000"
                },
                WHITE,
            );
            if m.mode == Mode::Coins && m.combo > 0 {
                c.shade(4, 30, 120, 23);
                c.text(8, 32, &format!("CHAIN X{}", m.combo), GOLD);
                c.bar(8, 46, 104, m.combo_time / 5.0, GOLD);
            }
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
        if m.flash > 0.0 {
            for y in [14, 142] {
                c.rect(0, y, 320, 2, CYAN);
            }
        }
    }
    if app.input_label == "CAL" {
        c.shade(60, 79, 200, 13);
        c.text(64, 82, "HOLD STILL / K CALIBRATE", GOLD);
    }
}
