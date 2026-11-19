use super::*;

const HELP_PAGES: [[&str; 8]; 9] = [
    [
        "FLIGHT",
        "UP / DOWN PITCH",
        "LEFT / RIGHT TURN",
        "SHIFT BOOST / LEAVE HOVER",
        "ALT HOVER ON/OFF",
        "Q / E ROLL LEFT / RIGHT",
        "W A S D LOOK AROUND",
        "SPACE CHANGE VIEW",
    ],
    [
        "INTERFACE",
        "P PAUSE / RESUME",
        "U HUD ON/OFF",
        "R RETRY CURRENT STAGE",
        "F1 HELP / F2 SETTINGS",
        "F9 CHANGE SOUND MODE",
        "F10 PERFORMANCE PANEL",
        "ESC BACK / HOLD 2S TO EXIT",
    ],
    [
        "DISPLAY / AEROBATICS",
        "F11 FULLSCREEN",
        "F12 NEXT HD SCALE",
        "SHIFT+F12 PREVIOUS HD SCALE",
        "UP / DOWN HOLD 3S, RELEASE",
        "THEN TAP TWICE TO LOOP",
        "LOOPS REQUIRE GROUND CLEARANCE",
        "ENTER RETURN FROM HELP",
    ],
    [
        "FREE FLIGHT",
        "1 CHANGE WEATHER",
        "2 CHANGE TIME OF DAY",
        "3 SUMMON WORMHOLE",
        "4 CHANGE AIRCRAFT",
        "5 CHANGE WINGMEN COUNT",
        "6 EXPAND AIRCRAFT MODEL",
        "7 AUTOPILOT ON/OFF",
    ],
    [
        "FREE FLIGHT / WORLD",
        "8 DIRECTOR SHOW ON/OFF",
        "9 CHANGE MAP",
        "0 GENERATE NEW TERRAIN",
        "M OPEN NAVIGATION CHART",
        "UP CLIMB TOWARD SPACE",
        "FLY TOWARD A PLANET TO LAND",
        "DIRECTION KEYS TAKE CONTROL",
    ],
    [
        "COMBAT",
        "1 HOLD TO FIRE",
        "2 NEXT WEAPON",
        "3 RELEASE NOVA",
        "4 ACTIVATE OVERDRIVE",
        "5 AUTO FIRE ON/OFF",
        "LEFT / RIGHT SELECT WIN UPGRADE",
        "ENTER NEXT STAGE / RETRY",
    ],
    [
        "RACE",
        "FLY THROUGH GREEN GATES IN ORDER",
        "SHIFT BOOST",
        "ALT HOVER ON/OFF",
        "P PAUSE / RESUME",
        "R RESTART RACE",
        "ENTER NEXT STAGE AFTER WIN",
        "ENTER RETRY AFTER FAILURE",
    ],
    [
        "COIN RUN",
        "COLLECT COINS TO BUILD A CHAIN",
        "SHIFT BOOST",
        "ALT HOVER ON/OFF",
        "P PAUSE / RESUME",
        "R RESTART COIN RUN",
        "ENTER NEXT STAGE AFTER WIN",
        "ENTER RETRY AFTER FAILURE",
    ],
    [
        "NAVIGATION CHART",
        "ARROWS PAN / 1 AND 2 ZOOM",
        "3 NEXT PORTAL / 4 CENTER",
        "SPACE MARK / ENTER NAVIGATE",
        "5 EARTH / 6 MOON IN SPACE",
        "7 CHANGE LANDING IN SPACE",
        "M RETURN TO FLIGHT",
        "DIRECTION KEYS TAKE CONTROL",
    ],
];

fn help_pages(mode: Mode) -> &'static [usize] {
    match mode {
        Mode::Free => &[0, 1, 2, 3, 4, 8],
        Mode::Combat => &[0, 1, 2, 5],
        Mode::Race => &[0, 1, 2, 6],
        Mode::Coins => &[0, 1, 2, 7],
    }
}
pub(crate) fn help_page_count(mode: Mode) -> usize {
    help_pages(mode).len()
}

fn control_label(line: &str) -> String {
    for (direction, key) in [
        ("UP ", "▲ "),
        ("DOWN ", "▼ "),
        ("LEFT ", "◀ "),
        ("RIGHT ", "▶ "),
    ] {
        if let Some(action) = line.strip_prefix(direction) {
            return format!("{key}{action}");
        }
    }
    line.to_owned()
}
fn help_page(mode: Mode, index: usize, device: bool) -> [&'static str; 8] {
    let index = help_pages(mode)[index];
    if device && index == 2 {
        [
            "MOTION / AEROBATICS",
            "I IMU ON/OFF",
            "K RECENTER IMU",
            "HOLD STILL WHILE CALIBRATING",
            "UP / DOWN HOLD 3S, RELEASE",
            "THEN TAP TWICE TO LOOP",
            "LOOPS REQUIRE GROUND CLEARANCE",
            "ENTER RETURN FROM HELP",
        ]
    } else {
        HELP_PAGES[index]
    }
}

pub fn draw(app: &mut App) {
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let m = &app.mission;
    let f = &app.flight;
    if app.page == Page::Loading {
        c.shade(30, 48, 260, 72);
        let heading = if app.travel.is_some() {
            "WORMHOLE / CONNECTING".into()
        } else {
            format!("STAGE {:02} / PREPARING", app.level)
        };
        c.text(46, 57, &heading, GOLD);
        c.text(
            46,
            78,
            app.travel.map_or_else(
                || {
                    crate::gameplay::progression::operation(app.level)
                        .name
                        .as_str()
                },
                |route| route.target.kind.name(),
            ),
            WHITE,
        );
        c.text(46, 100, "ESC MAIN MENU", MUTED);
        return;
    }
    if app.page == Page::Menu {
        c.shade(0, 0, 320, 32);
        c.text_scale(10, 7, "FLIGHT CLUB", WHITE, 2);
        c.text(224, 8, "ARCADE", CYAN);
        c.text(224, 20, concat!("V", env!("CARGO_PKG_VERSION")), MUTED);
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
            ("", "SETTINGS"),
            (
                "",
                if app.device_controls {
                    "EXIT TO LAUNCHER"
                } else {
                    "EXIT TO DESKTOP"
                },
            ),
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
        c.text(8, 162, "ENTER CONFIRM", MUTED);
        return;
    }
    if app.page == Page::Help {
        c.shade(6, 6, 308, 158);
        c.rect(6, 6, 2, 158, CYAN);
        let page = help_page(app.mode, app.help_page, app.device_controls);
        c.text(14, 16, page[0], GOLD);
        c.text(
            266,
            16,
            &format!("{}/{}", app.help_page + 1, help_page_count(app.mode)),
            GOLD,
        );
        for (i, line) in page[1..].iter().enumerate() {
            c.text(14, 33 + i as i32 * 13, &control_label(line), WHITE);
        }
        c.rect(14, 123, 292, 1, MUTED);
        c.text(14, 130, &control_label("LEFT PREVIOUS PAGE"), CYAN);
        c.text(14, 141, &control_label("RIGHT NEXT PAGE"), CYAN);
        c.text(14, 152, "ENTER RETURN", CYAN);
        return;
    }
    if app.page == Page::Settings {
        c.shade(24, 16, 272, 141);
        c.rect(24, 16, 2, 141, CYAN);
        c.text_scale(43, 23, "SETTINGS", WHITE, 2);
        let toggle = |value| if value { "ON" } else { "OFF" };
        let rows = [
            format!("FACETED TERRAIN   {}", toggle(f.faceted)),
            format!("PORTAL LABELS     {}", toggle(app.portal_labels)),
            format!("DYNAMIC CLOUDS    {}", toggle(f.dynamic_clouds)),
            format!("FLIGHT HUD        {}", toggle(f.hud)),
            format!("SOUND   {}", app.audio_settings.name()),
            format!("VOLUME           {:3}%", app.audio_settings.volume * 10),
            "BACK".to_owned(),
        ];
        for (i, line) in rows.iter().enumerate() {
            let y = 45 + i as i32 * 14;
            if i == app.selection {
                c.rect(32, y - 2, 256, 13, [36, 73, 86]);
                c.text(34, y, ">", GOLD);
            }
            c.text(47, y, line, if i == app.selection { GOLD } else { WHITE });
        }
        c.text(
            34,
            146,
            if app.selection == 2 {
                "OFF: SIMPLE CLOUDS"
            } else {
                "ESC RETURN"
            },
            MUTED,
        );
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
            "SETTINGS / F2",
            "MAIN MENU",
            if app.device_controls {
                "EXIT TO LAUNCHER"
            } else {
                "EXIT TO DESKTOP"
            },
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
                "F10 {}  VOL {}%",
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
                    "REWARD {}",
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
