use super::*;
pub fn draw(app: &mut App) {
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let m = &app.mission;
    let f = &app.flight;
    if app.page == Page::Loading {
        c.shade(30, 48, 260, 72);
        c.text(46, 57, &format!("STAGE {:02} / PREPARING", app.level), GOLD);
        c.text(
            46,
            78,
            crate::gameplay::progression::operation(app.level)
                .name
                .as_str(),
            WHITE,
        );
        c.text(46, 100, "ESC MAIN MENU", MUTED);
        return;
    }
    if app.page == Page::Menu {
        c.shade(0, 0, 320, 32);
        c.text_scale(10, 7, "RUST / FLY", WHITE, 2);
        c.text(224, 8, "FLIGHT CLUB", CYAN);
        c.text(224, 20, "ARCADE 0.7", MUTED);
        c.shade(8, 34, 304, 115);
        c.rect(8, 34, 2, 115, CYAN);
        let stage = format!(
            "{:02} {}",
            app.level,
            crate::gameplay::progression::operation(app.level).name
        );
        let rows = [
            ("MODE", app.mode.name()),
            ("AIRCRAFT", f.vehicle.name()),
            ("MAP", app.loader.pending.unwrap_or(app.world.kind).name()),
            (
                "WEATHER",
                if app.loader.pending.unwrap_or(app.world.kind).vacuum() {
                    "VACUUM"
                } else {
                    f.weather.name()
                },
            ),
            ("SKILL", app.difficulty.name()),
            ("STAGE", stage.as_str()),
            (
                "",
                if app.loader.pending.is_some() {
                    "START WHEN READY"
                } else {
                    "START FLIGHT"
                },
            ),
            ("", "FLIGHT MANUAL"),
            ("", "EXIT TO LAUNCHER"),
        ];
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = 37 + i as i32 * 12;
            if i == app.selection {
                c.rect(12, y - 1, 296, 11, [36, 73, 86]);
                c.text(16, y, ">", GOLD);
            }
            c.text(30, y, label, MUTED);
            c.text(112, y, value, if i == app.selection { GOLD } else { WHITE });
            if i < crate::app::START_ROW && i == app.selection {
                c.text(103, y, "<", CYAN);
                c.text(298, y, ">", CYAN);
            }
        }
        let description = if app.selection == 1 {
            f.vehicle.description()
        } else {
            app.mode.description()
        };
        c.shade(0, 151, 320, 19);
        c.text(8, 152, description, CYAN);
        c.text(8, 162, "Z/C CHANGE  ENTER START  9 SOUND", MUTED);
        return;
    }
    if app.page == Page::Help {
        c.shade(6, 6, 308, 158);
        c.rect(6, 6, 2, 158, CYAN);
        let pages = [
            [
                "FLIGHT MANUAL / 1 OF 4",
                "Z X C F / WASD : STEER / CLIMB",
                "4 VIEW  5 Q ROLL  6 SHIFT BOOST",
                "I IMU ON/OFF   K RECALIBRATE",
                "PAUSE Z/C VOLUME  U UI  9 SOUND",
                "HOLD ESC 2 SECONDS TO EXIT",
                "8 P PAUSE / H CREATE WORMHOLE",
                "Z/C PAGE       ENTER RETURN",
            ],
            [
                "MISSIONS / 2 OF 4",
                "RACE : CROSS GREEN GATES IN ORDER",
                "COINS : 3-5 SECOND CHAIN / X10",
                "WIN : ENTER ADVANCES TO NEXT MAP",
                "FAIL : ENTER RETRIES THIS STAGE",
                "R REPLAYS CURRENT STAGE",
                "WIN IN COMBAT : Z/C PICK UPGRADE",
                "Z/C PAGE       ENTER RETURN",
            ],
            [
                "COMBAT / 3 OF 4",
                "3 L FIRE   2 AUTO   O WEAPON",
                "1 E NOVA   0 Y OVERDRIVE",
                "GRAZE BULLETS TO CHARGE OVERDRIVE",
                "G M L P O : WEAPON UPGRADES",
                "B NOVA  S SHIELD  A ENERGY",
                "W WINGMEN     Q ROLL TO EVADE",
                "Z/C PAGE       ENTER RETURN",
            ],
            [
                "OBJECTIVES / 4 OF 4",
                "P PATROL : DEFEAT THE QUOTA",
                "C CONVOY : KEEP TRANSPORT ALIVE",
                "T STRIKE : DESTROY GROUND GUNS",
                "S SURVIVE : HOLD UNTIL TIMER ENDS",
                "B BOSS : DESTROY THE FLAGSHIP",
                "SIDES : SHIELD ENERGY GOAL ALERTS",
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
        c.text(
            45,
            145,
            &format!(
                "9 SOUND {}  Z/C VOL {}%",
                app.audio_settings.name(),
                app.audio_settings.volume * 10
            ),
            MUTED,
        );
        return;
    }
    if app.page == Page::Results {
        let won = m.outcome == Some(true);
        c.shade(12, 12, 296, 146);
        c.rect(12, 12, 296, 2, if won { CYAN } else { RED });
        c.text_scale(
            28,
            22,
            if won { "MISSION CLEAR" } else { "FLIGHT ENDED" },
            if won { CYAN } else { RED },
            2,
        );
        c.text(
            28,
            46,
            &format!(
                "STAGE {:02}  {}",
                app.level,
                if won { "CLEARED" } else { m.message }
            ),
            GOLD,
        );
        c.text(
            28,
            62,
            &format!("SCORE {:07}  TIME {:03.0}", m.score, m.elapsed),
            WHITE,
        );
        let detail = match m.mode {
            Mode::Race => format!(
                "GATES {}/{}  PERFECT {}",
                m.gate_index,
                m.gates.len(),
                m.perfect
            ),
            Mode::Coins => format!("COINS {}  CHAIN X{}", m.collected, m.best_combo),
            Mode::Combat => format!("KILLS {}  GRAZE {}", m.kills, m.grazes),
            Mode::Free => String::new(),
        };
        c.text(28, 78, &detail, WHITE);
        c.text(
            28,
            94,
            &format!(
                "RUN {:07} BEST {:07}",
                app.run_score + if won { m.score } else { 0 },
                app.best_score
            ),
            CYAN,
        );
        if app.new_record {
            c.icon(289, 94, '+', GOLD);
        }
        if won && m.mode == Mode::Combat {
            c.text(
                28,
                110,
                &format!(
                    "Z/C REWARD {}",
                    crate::gameplay::progression::Upgrade::ALL[app.reward].name()
                ),
                GOLD,
            );
        } else {
            c.text(
                28,
                110,
                if app.record_error {
                    "SCORE SAVE FAILED"
                } else if won {
                    "NEW ROUTE IN THE NEXT REGION"
                } else {
                    "R REPLAY / ESC MAIN MENU"
                },
                MUTED,
            );
        }
        c.text(28, 123, "R REPLAY    ESC MAIN MENU", MUTED);
        c.rect(24, 135, 272, 17, [36, 73, 86]);
        c.text(
            32,
            140,
            if won {
                "ENTER NEXT STAGE"
            } else {
                "ENTER RETRY"
            },
            GOLD,
        );
    }
}
