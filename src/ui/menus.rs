use super::*;

const HELP_PAGES: [[&str; 8]; 18] = [
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
        "F10 CYCLE PERFORMANCE PANEL",
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
    [
        "SKY / FLYING",
        "TILT THE DEVICE TO STEER",
        "F X Z C CLIMB DIVE TURN",
        "4 SCAN  5 MINE  6 LAND",
        "7 GALAXY MAP  8 INVENTORY",
        "SHIFT PULSE BOOST",
        "Q / E ROLL  SPACE CAMERA",
        "I GYRO ON/OFF  K RECENTER",
    ],
    [
        "SKY / ON FOOT",
        "F / X WALK  Z / C TURN",
        "TAP Z / C TO AIM FINELY",
        "DOUBLE TAP X: TURN AROUND",
        "A / D SIDESTEP  W / S LOOK",
        "SHIFT RUN  SPACE JETPACK",
        "TILT THE DEVICE TO FINE AIM",
        "K RECENTRE  F2 AIM SETTINGS",
    ],
    [
        "SKY / SPACE",
        "HOLD CLIMB HIGH TO REACH ORBIT",
        "FLY AT A PLANET TO LAND ON IT",
        "6 AUTOPILOT TO THE STATION",
        "5 MINE ASTEROIDS FOR TRITIUM",
        "7 MAP: WARP USES A JUMP CELL",
        "WARP AWAY FROM PLANETS",
        "FLY INTO THE TUNNEL TO JUMP",
    ],
    [
        "SKY / GALAXY AND SYSTEM MAP",
        "Z / C OR 4 / 5 PICK A STAR",
        "F / X TILT THE STAR FIELD",
        "6 OPEN A WARP TUNNEL",
        "7 SYSTEM VIEW AND PLANETS",
        "SYSTEM: 6 SET COURSE",
        "8 OR ESC CLOSE",
        "CYAN STARS ARE IN RANGE",
    ],
    [
        "SKY / INVENTORY AND STATION",
        "ARROWS PICK A SLOT",
        "4 USE: RECHARGE A SYSTEM",
        "5 RECIPES  6 MAKE",
        "STATION: 4 SELL 5 BUY 6 UPGRADE",
        "STATION: 7 OR ENTER CONFIRM",
        "STATION: 8 UNDOCK",
        "SCAN DATA SELLS AT STATIONS",
    ],
    [
        "SKY / WEAPONS AND DRONES",
        "4 SCAN  5 FIRE  6 JETPACK",
        "7 OR E: BOARD OPEN READ",
        "7 WITH NOTHING NEAR: SWAP",
        "Q NEXT WEAPON",
        "MINING AND LOOTING ALERT",
        "GUARD DRONES / RED ! = SHOT",
        "SHIELD SHD RECHARGES IN 3S",
    ],
    [
        "SKY / ARSENAL",
        "8 BAG THEN 6 ARMS",
        "4 EQUIP  5 SCRAP FOR METAL",
        "RARITY: COMMON TO EXOTIC",
        "EACH RARITY ADDS A PERK",
        "CACHES HOLD RARE WEAPONS",
        "REROLL CELL REROLLS PERKS",
        "STATIONS: ARMORY + MISSIONS",
    ],
    [
        "SKY / STATIONS AND MISSIONS",
        "6 AUTOPILOT DOCKS YOU",
        "WALK UP TO SOMEONE: 7 TALK",
        "AGENT: MISSION BOARD",
        "TRADER OUTFITTER TECHNICIAN",
        "THREE MISSIONS AT A TIME",
        "CLAIM DONE JOBS AT ANY AGENT",
        "4 LOG  7 AT THE SHIP: UNDOCK",
    ],
    [
        "SKY / PIRATES AND ASTEROIDS",
        "RAIDS ARE RARE: 30M TO 3H",
        "BOUNTY JOBS CALL TARGETS",
        "5 FIRES THE SHIP CANNONS",
        "TILT TO TURN ONTO THEM",
        "5 ON AN ASTEROID MINES IT",
        "BLUE VEINS RICH  GOLD RARE",
        "NO SHIELD: TOWED, CARGO LOST",
    ],
];

fn help_pages(mode: Mode) -> &'static [usize] {
    match mode {
        Mode::Explore => &[9, 10, 14, 15, 11, 16, 17, 12, 13, 1, 2],
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
    // 画布暂时取走屏幕缓冲，各页面函数只读 App
    let mut screen = std::mem::take(&mut app.screen);
    let mut c = Canvas {
        pixels: &mut screen,
    };
    match app.page {
        Page::Loading => loading(&mut c, app),
        Page::Menu => main_menu(&mut c, app),
        Page::Help => help(&mut c, app),
        Page::Settings => settings(&mut c, app),
        Page::Paused => paused(&mut c, app),
        Page::Results => results(&mut c, app),
        _ => {}
    }
    app.screen = screen;
}

/// 加载与虫洞连接提示
fn loading(c: &mut Canvas, app: &App) {
    c.shade(30, 48, 260, 72);
    if app.mode == Mode::Explore && app.travel.is_none() {
        c.text(46, 57, "CARDPUTER SKY / LANDFALL", GOLD);
        c.text(46, 78, &format!("{} SYSTEM", app.sky.system.name), WHITE);
        c.text(46, 100, "GENERATING TERRAIN", MUTED);
        return;
    }
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
}

/// 主菜单：模式、机型、地图等选项
fn main_menu(c: &mut Canvas, app: &App) {
    let f = &app.flight;
    let menu_map = app.menu_map();
    let menu_weather = app.menu_weather().name();
    c.shade(0, 0, 320, 32);
    if app.mode == Mode::Explore {
        c.text_scale(10, 7, "CARDPUTER SKY", WHITE, 2);
        c.text(224, 8, "EXPLORE", CYAN);
    } else {
        c.text_scale(10, 7, "FLIGHT CLUB", WHITE, 2);
        c.text(224, 8, "ARCADE", CYAN);
    }
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
        ("MAP", menu_map.name()),
        (
            "WEATHER",
            if menu_map.vacuum() {
                "VACUUM"
            } else {
                menu_weather
            },
        ),
        ("SKILL", app.difficulty.name()),
        ("STAGE", stage.as_str()),
        (
            "",
            if app.loader.pending.is_some() {
                "START WHEN READY"
            } else if app.mode == Mode::Explore {
                "START EXPLORING"
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
    // 描述栏留出上下边距，第二行不再贴着屏幕底边
    c.shade(0, 145, 320, 25);
    c.text(8, 148, description, CYAN);
    c.text(8, 159, "ENTER CONFIRM", MUTED);
}

/// 分页操作说明
fn help(c: &mut Canvas, app: &App) {
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
}

/// 设置页及其常规、高级子页
fn settings(c: &mut Canvas, app: &App) {
    let f = &app.flight;
    let settings_row_height = app.settings_row_height() as i32;
    use crate::app::SettingsSection;
    c.shade(24, 16, 272, 141);
    c.rect(24, 16, 2, 141, CYAN);
    let title = match app.settings_section {
        None => "SETTINGS",
        Some(SettingsSection::General) => "GENERAL",
        Some(SettingsSection::Advanced) => "ADVANCED",
    };
    if app.settings_section.is_some() {
        c.text(34, 28, "<", CYAN);
    }
    c.text_scale(52, 23, title, WHITE, 2);
    c.rect(34, 44, 252, 1, MUTED);
    let toggle = |value| if value { "ON" } else { "OFF" }.to_owned();
    let rows = match app.settings_section {
        None => vec![
            ("GENERAL", ">".to_owned()),
            ("ADVANCED", ">".to_owned()),
            ("BACK", String::new()),
        ],
        Some(SettingsSection::General) => vec![
            ("FLIGHT HUD", toggle(f.hud)),
            ("PORTAL LABELS", toggle(app.portal_labels)),
            ("SOUND", app.audio_settings.name().to_owned()),
            ("VOLUME", format!("{}%", app.audio_settings.volume * 10)),
            ("AIM ASSIST", toggle(app.aim_assist)),
            ("TILT AIM ON FOOT", toggle(app.tilt_aim)),
            ("BACK", String::new()),
        ],
        Some(SettingsSection::Advanced) => vec![
            ("FACETED TERRAIN", toggle(f.faceted)),
            ("DYNAMIC CLOUDS", toggle(f.dynamic_clouds)),
            ("CLOUD PIXEL TEXTURE", toggle(f.cloud_pixel_texture)),
            ("DYNAMIC LIGHTING", toggle(f.dynamic_lighting)),
            ("SMOOTH SKY GRADIENT", toggle(f.smooth_sky_gradient)),
            ("PORTAL PREVIEW", toggle(f.portal_preview)),
            ("BACK", String::new()),
        ],
    };
    let row_height = settings_row_height;
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = 52 + i as i32 * row_height;
        if i == app.selection {
            c.rect(32, y - 3, 256, row_height - 1, [36, 73, 86]);
            c.text(34, y, ">", GOLD);
        }
        let color = if i == app.selection { GOLD } else { WHITE };
        c.text(47, y, label, color);
        if app.settings_section == Some(SettingsSection::General) && matches!(i, 2 | 3) {
            c.text(192, y, "<", color);
            c.text(232 - value.len() as i32 * 4, y, value, color);
            c.text(272, y, ">", color);
        } else {
            c.text(280 - value.len() as i32 * 8, y, value, color);
        }
        if app.settings_section.is_none() && i < 2 {
            c.text(
                47,
                y + 11,
                ["HUD, LABELS AND AUDIO", "VISUALS AND PERFORMANCE"][i],
                MUTED,
            );
        }
    }
    let hint = if app.selection == rows.len() - 1 {
        "ENTER BACK"
    } else {
        match (app.settings_section, app.selection) {
            (None, _) => "ENTER OPEN",
            (Some(SettingsSection::Advanced), 1) => "OFF: SIMPLE CLOUDS / FASTER",
            (Some(SettingsSection::Advanced), 2) => "ON: DOTTED CLOUD SHADING",
            (Some(SettingsSection::Advanced), 3) => "OFF: FREEZE SUN POSITION",
            (Some(SettingsSection::Advanced), 4) => "ON: CONTINUOUS SKY COLORS",
            (Some(SettingsSection::Advanced), 5) => "OFF: ENERGY FIELD / FASTER",
            (Some(SettingsSection::General), 2 | 3) => "LEFT / RIGHT ADJUST",
            _ => "ENTER TOGGLE",
        }
    };
    c.text(34, 132, hint, CYAN);
    c.text(34, 149, "ESC BACK", MUTED);
}

/// 暂停菜单
fn paused(c: &mut Canvas, app: &App) {
    let f = &app.flight;
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
}

/// 结算页
fn results(c: &mut Canvas, app: &App) {
    let m = &app.mission;
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
        Mode::Free | Mode::Explore => String::new(),
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
