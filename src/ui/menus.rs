use super::*;

pub(super) const HELP_PAGES: [[&str; 8]; 11] = [
    [
        "FLIGHT",
        "UP PITCH UP",
        "DOWN PITCH DOWN",
        "LEFT TURN LEFT",
        "RIGHT TURN RIGHT",
        "SHIFT HOLD TO BOOST",
        "D ROLL LEFT",
        "G ROLL RIGHT",
    ],
    [
        "INTERFACE",
        "SPACE CHANGE VIEW",
        "M NAVIGATION CHART",
        "U HUD ON/OFF",
        "P PAUSE / RESUME",
        "R RETRY CURRENT STAGE",
        "F1 HELP ON/OFF",
        "F2 SETTINGS ON/OFF",
    ],
    [
        "FREE FLIGHT / EXPLORATION",
        "1 CHANGE WEATHER",
        "2 CHANGE TIME OF DAY",
        "3 SUMMON ONE WORMHOLE",
        "4 CHANGE AIRCRAFT",
        "5 CHANGE WINGMEN COUNT",
        "6 MODEL EXPANSION ON/OFF",
        "7 AUTOPILOT ON/OFF",
    ],
    [
        "FREE FLIGHT / DIRECTOR",
        "8 DIRECTOR SHOW ON/OFF",
        "9 CHANGE MAP",
        "0 GENERATE NEW TERRAIN",
        "PORTAL APPEARS BEHIND CAMERA",
        "CAMERA TURNS BEFORE WARP ENTRY",
        "DIRECTOR : TWO WARP SEQUENCES",
        "DIRECTION KEYS TAKE CONTROL",
    ],
    [
        "COMBAT",
        "1 HOLD TO FIRE",
        "2 NEXT WEAPON",
        "3 RELEASE NOVA",
        "4 ACTIVATE OVERDRIVE",
        "5 AUTO FIRE ON/OFF",
        "GRAZE BULLETS TO CHARGE OVERDRIVE",
        "COLLECT WEAPONS SHIELDS ENERGY",
    ],
    [
        "MISSIONS",
        "RACE : CROSS GREEN GATES IN ORDER",
        "COINS : 3-5 SECOND CHAIN / X10",
        "ENTER NEXT STAGE AFTER WIN",
        "ENTER RETRY AFTER FAILURE",
        "LEFT PREVIOUS COMBAT UPGRADE",
        "RIGHT NEXT COMBAT UPGRADE",
        "R REPLAY CURRENT STAGE",
    ],
    [
        "MENU / SETTINGS",
        "UP SELECT PREVIOUS ROW",
        "DOWN SELECT NEXT ROW",
        "LEFT SELECT PREVIOUS VALUE",
        "RIGHT SELECT NEXT VALUE",
        "ENTER ACTIVATE SELECTION",
        "ENTER ON CONFIG : GO TO START",
        "ESC RETURN",
    ],
    [
        "NAVIGATION CHART",
        "UP PAN NORTH",
        "DOWN PAN SOUTH",
        "LEFT PAN WEST",
        "RIGHT PAN EAST",
        "1 ZOOM IN / MAX 512X",
        "2 ZOOM OUT",
        "SPACE MARK CROSSHAIR",
    ],
    [
        "WAYPOINT NAVIGATION",
        "3 FIND NEXT PORTAL",
        "4 CENTER ON AIRCRAFT",
        "ENTER AUTO NAV TO MARKER",
        "M RETURN TO FLIGHT",
        "DIRECTION KEY TAKES CONTROL",
        "RING ARROW : FULL BEARING",
        "CENTER ARROW : HEIGHT OFFSET",
    ],
    [
        "AEROBATICS / DESKTOP",
        "UP DOUBLE TAP : INSIDE LOOP",
        "DOWN DOUBLE TAP : OUTSIDE LOOP",
        "LEFT DOUBLE TAP : LEFT ROLL",
        "RIGHT DOUBLE TAP : RIGHT ROLL",
        "TAP TWICE WITHIN 0.32 SECONDS",
        "HIGH ALTITUDE REQUIRED",
        "HOLD DIRECTION : NORMAL FLIGHT",
    ],
    [
        "DESKTOP / DISPLAY / SOUND",
        "F10 CHANGE SOUND MODE",
        "F11 FULLSCREEN",
        "F12 NEXT HD SCALE",
        "SHIFT+F12 PREVIOUS HD SCALE",
        "HD SCALE : 1X / 2X / 4X / 8X",
        "ESC BACK / PAUSE / RESUME",
        "ESC HOLD 2S TO EXIT",
    ],
];

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
fn help_page(index: usize, device: bool) -> [&'static str; 8] {
    if device && index == 9 {
        [
            "AEROBATICS / CARDPUTER",
            "UP DOUBLE TAP : INSIDE LOOP",
            "DOWN DOUBLE TAP : OUTSIDE LOOP",
            "LEFT DOUBLE TAP : LEFT ROLL",
            "RIGHT DOUBLE TAP : RIGHT ROLL",
            "HIGH ALTITUDE REQUIRED",
            "IMU TILT : NORMAL FLIGHT",
            "HOLD DIRECTION : NORMAL FLIGHT",
        ]
    } else if device && index == 10 {
        [
            "CARDPUTER / MOTION / SOUND",
            "I IMU ON/OFF",
            "K RECENTER IMU",
            "HOLD STILL WHILE CALIBRATING",
            "F10 CHANGE SOUND MODE",
            "DISPLAY : NATIVE 320 X 170",
            "ESC BACK / PAUSE / RESUME",
            "ESC HOLD 2S TO LAUNCHER",
        ]
    } else {
        let mut page = HELP_PAGES[index];
        if index == 0 {
            page[0] = if device {
                "FLIGHT / CARDPUTER"
            } else {
                "FLIGHT / DESKTOP"
            };
        }
        page
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
        c.text_scale(10, 7, "RUST / FLY", WHITE, 2);
        c.text(224, 8, "FLIGHT CLUB", CYAN);
        c.text(224, 20, "ARCADE 0.12", MUTED);
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
        let page = help_page(app.help_page, app.device_controls);
        c.text(14, 16, page[0], GOLD);
        c.text(
            266,
            16,
            &format!("{}/{}", app.help_page + 1, HELP_PAGES.len()),
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
            format!("FLIGHT HUD        {}", toggle(f.hud)),
            format!("SOUND   {}", app.audio_settings.name()),
            format!("VOLUME           {:3}%", app.audio_settings.volume * 10),
            "BACK".to_owned(),
        ];
        for (i, line) in rows.iter().enumerate() {
            let y = 48 + i as i32 * 16;
            if i == app.selection {
                c.rect(32, y - 2, 256, 13, [36, 73, 86]);
                c.text(34, y, ">", GOLD);
            }
            c.text(47, y, line, if i == app.selection { GOLD } else { WHITE });
        }
        c.text(34, 146, "ESC RETURN", MUTED);
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
