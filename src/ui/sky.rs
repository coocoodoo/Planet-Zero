//! Cardputer Sky screens. Layout follows M5Stack's Recommended Key Mapping Design: content in
//! y 0–131, and a key bar whose 22×22 icons are centred on the 4–8 keys (x 49, 105, 161, 217,
//! 273; y 153) with a tick pointing down at each key. Other keys appear only in the help pages,
//! never in columns that line up with keys 4–8.
use super::sky_icons::{Icon, bitmap};
use super::*;
use crate::app::Context;
use crate::app::sky::{BOARD_DISTANCE, CROSSHAIR, DOCK_DISTANCE, LAND_ALTITUDE, STATION_STOCK};
use crate::app::{HEIGHT, WIDTH};
use crate::keys::BAR;
use crate::sky::combat::EffectKind;
use crate::sky::interior::{self, Role};
use crate::sky::inventory::{Item, RECIPES};
use crate::sky::missions;
use crate::sky::props::Prop;
use crate::sky::station::{self, Upgrade};
use crate::sky::{Location, Overlay, Phase};
use glam::{Vec2, Vec3, vec2, vec3};

pub(super) const KEY_X: [i32; 5] = [49, 105, 161, 217, 273];
pub(super) const KEY_Y: i32 = 153;
const BAR_TOP: i32 = 131;
const DIM: [u8; 3] = [70, 86, 100];
const PANEL: [u8; 3] = [9, 16, 26];
const SELECT: [u8; 3] = [36, 73, 86];

struct Key {
    label: &'static str,
    icon: Icon,
    enabled: bool,
}
const fn key(label: &'static str, icon: Icon, enabled: bool) -> Key {
    Key {
        label,
        icon,
        enabled,
    }
}
const BLANK: Key = key("", Icon::Blank, false);

pub(super) fn draw(app: &mut App) {
    let mut screen = std::mem::take(&mut app.screen);
    let mut c = Canvas {
        pixels: &mut screen,
    };
    match app.sky.overlay {
        Overlay::None => match app.sky.phase {
            Phase::OnFoot => {
                on_foot(&mut c, app);
                tracker(&mut c, app, 68);
            }
            Phase::Station => station_hud(&mut c, app),
            _ => {
                flying(&mut c, app);
                space_hud(&mut c, app);
                tracker(&mut c, app, 96);
            }
        },
        Overlay::Galaxy => galaxy(&mut c, app),
        Overlay::System => system(&mut c, app),
        Overlay::Inventory => inventory(&mut c, app),
        Overlay::Station => station_panel(&mut c, app),
        Overlay::Lore => lore_panel(&mut c, app),
        Overlay::Talk => talk_panel(&mut c, app),
        Overlay::Missions => missions_panel(&mut c, app),
    }
    notices(&mut c, app);
    key_bar(&mut c, app, &keys(app));
    app.screen = screen;
}

fn keys(app: &App) -> [Key; 5] {
    let sky = &app.sky;
    match sky.overlay {
        Overlay::Galaxy => [
            key("PREV", Icon::Prev, true),
            key("NEXT", Icon::Next, true),
            key(
                "WARP",
                Icon::Warp,
                app.world.kind == crate::world::MapKind::Space,
            ),
            key("SYSTEM", Icon::System, true),
            key("CLOSE", Icon::Close, true),
        ],
        Overlay::System => [
            key("PREV", Icon::Prev, true),
            key("NEXT", Icon::Next, true),
            key("COURSE", Icon::Land, sky.location == Location::Space),
            key("GALAXY", Icon::Map, true),
            key("CLOSE", Icon::Close, true),
        ],
        Overlay::Inventory if sky.crafting => [
            BLANK,
            key("ITEMS", Icon::Grid, true),
            key(
                "MAKE",
                Icon::Make,
                RECIPES[sky.craft_select].possible(&sky.inventory),
            ),
            BLANK,
            key("CLOSE", Icon::Close, true),
        ],
        Overlay::Inventory if sky.arms => [
            key("EQUIP", Icon::Equip, true),
            key("SCRAP", Icon::Scrap, sky.loadout.count() > 1),
            key("ITEMS", Icon::Grid, true),
            key("MAP", Icon::Map, true),
            key("CLOSE", Icon::Close, true),
        ],
        Overlay::Lore => [BLANK, BLANK, BLANK, BLANK, key("CLOSE", Icon::Close, true)],
        Overlay::Inventory => [
            key(
                "USE",
                Icon::Use,
                sky.inventory.slots()[sky.inventory_select].is_some(),
            ),
            key("CRAFT", Icon::Craft, true),
            key("ARMS", Icon::Arms, true),
            key("MAP", Icon::Map, true),
            key("CLOSE", Icon::Close, true),
        ],
        Overlay::Talk => {
            let people = app.station_people();
            let role = people.get(sky.talking).map_or(Role::Traveller, |n| n.role);
            let icon = match role {
                Role::Agent => Icon::Read,
                Role::Trader => Icon::Sell,
                Role::Outfitter => Icon::Fire,
                Role::Technician => Icon::Upgrade,
                Role::Traveller => Icon::Talk,
            };
            [
                key(
                    role.service(),
                    icon,
                    role != Role::Traveller || !sky.traveller_gift,
                ),
                key("CHAT", Icon::Talk, true),
                BLANK,
                BLANK,
                key("LEAVE", Icon::Close, true),
            ]
        }
        Overlay::Missions => {
            let board = sky.mission_tab == 0;
            let primary = if board {
                key(
                    "ACCEPT",
                    Icon::Confirm,
                    !sky.board.is_empty() && sky.missions.len() < missions::MAX_ACTIVE,
                )
            } else {
                let done = sky
                    .missions
                    .get(sky.mission_select)
                    .is_some_and(|m| m.done(&sky.inventory));
                key(
                    "CLAIM",
                    Icon::Confirm,
                    done && app.sky.phase == Phase::Station,
                )
            };
            [
                primary,
                key("ABANDON", Icon::Scrap, !board && !sky.missions.is_empty()),
                key(if board { "ACTIVE" } else { "BOARD" }, Icon::Grid, true),
                BLANK,
                key("CLOSE", Icon::Close, true),
            ]
        }
        Overlay::Station => [
            key("SELL", Icon::Sell, true),
            key("BUY", Icon::Buy, true),
            key("UPGRADE", Icon::Upgrade, true),
            key("OK", Icon::Confirm, true),
            if sky.phase == Phase::Station {
                key("LEAVE", Icon::Close, true)
            } else {
                key("UNDOCK", Icon::Undock, true)
            },
        ],
        Overlay::None => match sky.phase {
            Phase::OnFoot => {
                let context = app.sky_context();
                let icon = match context {
                    Context::Board => Icon::Board,
                    Context::Open(_) => Icon::Open,
                    Context::Read(_) => Icon::Read,
                    Context::Signal(_) => Icon::Signal,
                    Context::Swap => Icon::Swap,
                    Context::Talk(_) => Icon::Talk,
                };
                let swap_ready = sky.loadout.equipped.iter().flatten().count() > 1;
                let ready =
                    sky.loadout.overheat[sky.loadout.active] <= 0.0 && sky.vitals.tool > 0.0;
                [
                    key("SCAN", Icon::Scan, true),
                    key("FIRE", Icon::Fire, ready),
                    key("JET", Icon::Jet, sky.vitals.jetpack > 0.0),
                    key(
                        context.label(),
                        icon,
                        context != Context::Swap || swap_ready,
                    ),
                    key("BAG", Icon::Bag, true),
                ]
            }
            Phase::Station => {
                let context = app.sky_context();
                let near = match context {
                    Context::Talk(_) => key("TALK", Icon::Talk, true),
                    Context::Board => key("UNDOCK", Icon::Board, true),
                    _ => key("TALK", Icon::Talk, false),
                };
                [
                    key("LOG", Icon::Read, true),
                    BLANK,
                    BLANK,
                    near,
                    key("BAG", Icon::Bag, true),
                ]
            }
            Phase::Flying => {
                let third = if sky.location == Location::Space {
                    key(if sky.autodock { "AUTO" } else { "DOCK" }, Icon::Dock, true)
                } else {
                    key("LAND", Icon::Land, can_land(app))
                };
                [
                    key("SCAN", Icon::Scan, true),
                    if sky.location != Location::Space {
                        key("FIRE", Icon::Fire, false)
                    } else if app.space_aim_rock() {
                        key("MINE", Icon::Mine, sky.vitals.tool_ready())
                    } else {
                        key("FIRE", Icon::Fire, sky.vitals.overheat <= 0.0)
                    },
                    third,
                    key("MAP", Icon::Map, true),
                    key("BAG", Icon::Bag, true),
                ]
            }
            _ => [BLANK, BLANK, BLANK, BLANK, BLANK],
        },
    }
}

fn can_land(app: &App) -> bool {
    let p = app.flight.position;
    matches!(app.sky.location, Location::Planet(_))
        && app.orbit.transition.is_none()
        && p.z - app.world.height(p.x, p.y) <= LAND_ALTITUDE
}

/// The key bar: label, 22×22 icon centred on the key, tick down to the key.
fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

/// The key bar. On the HUD it slides down after a few idle seconds so only the labels and the
/// ticks to the keys stay; any key press brings it back. Pressed keys light up: a bright flash
/// that settles to gold while held (red if the action is unavailable right now).
fn key_bar(c: &mut Canvas, app: &App, keys: &[Key; 5]) {
    let bar = &app.sky.bar;
    let slide = if app.sky.overlay == Overlay::None {
        bar.offset().round() as i32
    } else {
        0
    };
    let top = BAR_TOP + slide;
    c.shade(0, top as usize, 320, (170 - top).max(0) as usize);
    const FLASH: [u8; 3] = [255, 246, 210];
    const PLATE: [u8; 3] = [40, 60, 70];
    for (i, k) in keys.iter().enumerate() {
        let x = KEY_X[i];
        let held = app.keys.contains(&BAR[i]);
        let pulse = bar.pulse(i);
        if k.icon == Icon::Blank {
            c.rect(x, 166, 1, 4, blend(DIM, RED, pulse));
            continue;
        }
        // colour: flash on the press, gold while held, red when the action is unavailable
        let rest = if k.enabled { WHITE } else { DIM };
        let hot = if k.enabled { GOLD } else { RED };
        let color = if held {
            blend(hot, FLASH, pulse)
        } else {
            blend(rest, blend(hot, FLASH, pulse), pulse)
        };
        // the icon sinks a pixel while its key is down, like a real button
        let icon_y = KEY_Y + slide + i32::from(held);
        if held || pulse > 0.0 {
            let glow = blend(PLATE, hot, 0.25 + 0.45 * pulse);
            let plate = if held {
                glow
            } else {
                blend(PANEL, glow, pulse)
            };
            c.rect(
                x - 12,
                icon_y - 12,
                24,
                (165 - (icon_y - 12)).clamp(0, 24),
                plate,
            );
            if pulse > 0.0 {
                // ring that grows and fades from the key
                let grow = (12.0 + 5.0 * (1.0 - pulse)) as i32;
                let ring = blend(PANEL, hot, pulse);
                c.rect(x - grow, icon_y - grow, grow * 2, 1, ring);
                c.rect(x - grow, icon_y + grow - 1, grow * 2, 1, ring);
                c.rect(x - grow, icon_y - grow, 1, grow * 2, ring);
                c.rect(x + grow - 1, icon_y - grow, 1, grow * 2, ring);
            }
        }
        let rows = bitmap(k.icon);
        for (yy, row) in rows.iter().enumerate() {
            let py = icon_y - 11 + yy as i32 * 2;
            // never below the key ticks: a tucked bar shows only its labels
            if py > 163 {
                break;
            }
            for (xx, ch) in row.bytes().enumerate() {
                if ch == b'#' {
                    c.rect(x - 11 + xx as i32 * 2, py, 2, 2, color);
                }
            }
        }
        let label = if !k.enabled {
            blend(DIM, RED, pulse)
        } else if held || pulse > 0.0 {
            blend(GOLD, FLASH, pulse)
        } else {
            CYAN
        };
        c.osd(
            x - Canvas::osd_width(k.label) / 2,
            133 + slide,
            k.label,
            label,
        );
        c.rect(x, 166, 1, 4, color);
    }
}

fn notices(c: &mut Canvas, app: &App) {
    let notices = app.sky_notices();
    if app.sky.overlay != Overlay::None {
        // panels: newest notice on a strip above the key bar
        if let Some(n) = notices.last() {
            c.rect(0, 120, 320, 11, [20, 30, 44]);
            c.osd(160 - Canvas::osd_width(&n.text) / 2, 122, &n.text, n.color);
        }
        return;
    }
    for (i, n) in notices.iter().rev().enumerate() {
        let w = Canvas::osd_width(&n.text);
        c.osd(160 - w / 2, 121 - i as i32 * 10, &n.text, n.color);
    }
}

fn bar(c: &mut Canvas, x: i32, y: i32, label: &str, value: f32, color: [u8; 3]) {
    c.osd(x, y, label, if value < 0.25 { RED } else { MUTED });
    c.rect(x + 20, y + 2, 34, 4, [30, 40, 52]);
    c.rect(
        x + 20,
        y + 2,
        (value.clamp(0.0, 1.0) * 34.0) as i32,
        4,
        if value < 0.25 { RED } else { color },
    );
}

fn credits(c: &mut Canvas, app: &App) {
    c.osd_right(316, 4, &format!("{} CR", app.sky.credits), GOLD);
}

fn heading_tape(c: &mut Canvas, yaw: f32) {
    // compass tape: N at yaw 0, E at +90°
    let degrees = yaw.to_degrees().rem_euclid(360.0);
    c.rect(110, 13, 100, 1, MUTED);
    for mark in (0..360).step_by(15) {
        let offset = crate::flight::angle((mark as f32 - degrees).to_radians()).to_degrees();
        if offset.abs() > 48.0 {
            continue;
        }
        let x = 160 + offset as i32;
        let label = match mark {
            0 => "N",
            90 => "E",
            180 => "S",
            270 => "W",
            _ => "",
        };
        if label.is_empty() {
            c.rect(x, 11, 1, 2, MUTED);
        } else {
            c.osd(x - 2, 2, label, if mark == 0 { GOLD } else { WHITE });
            c.rect(x, 10, 1, 3, WHITE);
        }
    }
    c.rect(160, 14, 1, 3, GOLD);
}

fn reticle(c: &mut Canvas, center: Vec2, color: [u8; 3]) {
    let (x, y) = (center.x as i32, center.y as i32);
    for (dx, dy) in [(-6, 0), (3, 0)] {
        c.rect(x + dx, y + dy, 3, 1, color);
    }
    for (dx, dy) in [(0, -6), (0, 3)] {
        c.rect(x + dx, y + dy, 1, 3, color);
    }
}

/// Marker for a world point: diamond with label, or an edge arrow when off-screen.
fn marker(
    c: &mut Canvas,
    camera: &crate::flight::Camera,
    point: Vec3,
    label: &str,
    color: [u8; 3],
) {
    let projected = camera.project(point);
    let on_screen = projected.is_some_and(|(p, depth, _)| {
        depth > 0.0 && (6.0..314.0).contains(&p.x) && (18.0..126.0).contains(&p.y)
    });
    if let (true, Some((p, _, _))) = (on_screen, projected) {
        let (x, y) = (p.x as i32, p.y as i32);
        for (dx, dy) in [(0, -3), (3, 0), (0, 3), (-3, 0)] {
            c.line(x + dx, y + dy, x + dy, y - dx, color);
        }
        let w = Canvas::osd_width(label);
        c.osd((x - w / 2).clamp(2, 318 - w), y + 6, label, color);
        return;
    }
    // off-screen: arrow on the edge toward the point
    let delta = point - camera.position;
    let v = vec2(delta.dot(camera.right), -delta.dot(camera.up));
    let v = if delta.dot(camera.forward) < 0.0 && v.length_squared() < 1.0 {
        vec2(0.0, 1.0)
    } else {
        v.normalize_or_zero()
    };
    let center = vec2(160.0, 72.0);
    let scale = (150.0 / v.x.abs().max(0.001)).min(52.0 / v.y.abs().max(0.001));
    let tip = center + v * scale;
    let right = vec2(-v.y, v.x);
    for side in [-1.0, 1.0] {
        let base = tip - v * 6.0 + right * 4.0 * side;
        c.line(
            tip.x as i32,
            tip.y as i32,
            base.x as i32,
            base.y as i32,
            color,
        );
    }
    // the left edge belongs to the flight panel; its arrow speaks for itself
    if tip.x < 80.0 && tip.y < 110.0 {
        return;
    }
    let w = Canvas::osd_width(label);
    let at = tip - v * 14.0;
    c.osd(
        (at.x as i32 - w / 2).clamp(2, 318 - w),
        (at.y as i32 - 3).clamp(18, 124),
        label,
        color,
    );
}

fn distance_label(d: f32) -> String {
    if d >= 1000.0 {
        format!("{:.1}K", d / 1000.0)
    } else {
        format!("{d:.0}")
    }
}

fn flying(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let f = &app.flight;
    let camera = crate::flight::Camera::new(f, WIDTH, HEIGHT);
    // top left: where am I
    let (title, detail) = match sky.location {
        Location::Planet(i) => {
            let p = &sky.system.planets[i];
            (
                p.name.clone(),
                format!("{} / {}", p.biome.name(), p.biome.hazard().0.name()),
            )
        }
        _ => (
            sky.system.name.clone(),
            format!("{} STAR", sky.system.class.name()),
        ),
    };
    c.osd(4, 4, &title, CYAN);
    c.osd(4, 13, &detail, MUTED);
    credits(c, app);
    c.osd_right(
        316,
        13,
        app.input_label,
        if app.input_label == "IMU" {
            CYAN
        } else {
            MUTED
        },
    );
    if !matches!(sky.location, Location::Space) {
        heading_tape(c, f.yaw);
    }
    // left: flight data and ship systems
    let ground = app.world.height(f.position.x, f.position.y);
    c.osd(4, 30, &format!("SPD {:.0}", f.speed), WHITE);
    let altitude = if matches!(sky.location, Location::Planet(_)) {
        format!("ALT {:.0}", f.position.z - ground)
    } else {
        format!(
            "STN {}",
            distance_label(f.position.distance(sky.system.station))
        )
    };
    c.osd(4, 40, &altitude, WHITE);
    bar(c, 4, 54, "PLS", sky.vitals.pulse, CYAN);
    bar(c, 4, 64, "SHD", sky.vitals.shield, WHITE);
    bar(c, 4, 74, "LFT", sky.vitals.lift, [120, 170, 255]);
    if sky.mining || sky.vitals.heat > 0.0 {
        bar(c, 4, 84, "HOT", 1.0 - sky.vitals.heat, GOLD);
    }
    beam(c, app, &camera, vec2(160.0, 82.0));
    reticle(c, vec2(160.0, 73.0), if sky.mining { GOLD } else { WHITE });
    match sky.phase {
        Phase::Landing { .. } => centered(c, 34, "LANDING", CYAN),
        Phase::Takeoff { .. } => centered(c, 34, "LIFT OFF", CYAN),
        Phase::Docking { .. } => centered(c, 34, "DOCKING", CYAN),
        _ => {}
    }
    match sky.location {
        Location::Space => {
            let station = sky.system.station;
            let d = f.position.distance(station);
            marker(
                c,
                &camera,
                station,
                &format!("STATION {}", distance_label(d)),
                CYAN,
            );
            if sky.autodock {
                centered(c, 34, "AUTOPILOT", CYAN);
            }
            if d < DOCK_DISTANCE * 3.0 && sky.phase == Phase::Flying && sky.warp_target.is_none() {
                centered(c, 44, "APPROACH TO DOCK", WHITE);
            }
            if let Some(id) = sky.warp_target {
                let name = sky.galaxy.system(id).name;
                centered(c, 24, &format!("WARP TO {name}"), GOLD);
            }
        }
        Location::Planet(_) => {
            if sky.phase == Phase::Flying && app.orbit.transition.is_none() {
                let above = f.position.z - ground;
                if above <= LAND_ALTITUDE {
                    centered(c, 24, "6 LAND", GOLD);
                }
            }
            if sky.scan_age < 12.0 {
                for prop in sky.revealed.iter().filter(|p| p.kind.is_poi()) {
                    let d = prop.position.distance(f.position);
                    marker(
                        c,
                        &camera,
                        prop.center(),
                        &format!("{} {}", prop.kind.label(), distance_label(d)),
                        GOLD,
                    );
                }
            }
        }
        Location::Elsewhere => {}
    }
    scan_wave(c, sky.scan_age);
}

/// Mining beam from the tool (or the ship's nose) to the target, drawn over the 3D view.
fn beam(c: &mut Canvas, app: &App, camera: &crate::flight::Camera, from: Vec2) {
    let sky = &app.sky;
    let (true, Some(end)) = (sky.mining, sky.beam) else {
        return;
    };
    // project() already rejects points behind the eye (depth is measured from a point ahead of it)
    let Some((to, _, _)) = camera.project(end) else {
        return;
    };
    let to = to.clamp(vec2(0.0, 0.0), vec2(319.0, BAR_TOP as f32 - 1.0));
    let flicker = (sky.time * 30.0) as i32 % 2;
    let outer = if flicker == 0 {
        [255, 120, 80]
    } else {
        [255, 160, 90]
    };
    for (dx, dy, color) in [(0, -1, outer), (0, 1, outer), (0, 0, [255, 236, 190])] {
        c.line(
            from.x as i32 + dx,
            from.y as i32 + dy,
            to.x as i32 + dx,
            to.y as i32 + dy,
            color,
        );
    }
    for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2), (0, 0)] {
        c.pixel(to.x as i32 + dx, to.y as i32 + dy, [255, 236, 190]);
    }
}

fn centered(c: &mut Canvas, y: i32, text: &str, color: [u8; 3]) {
    c.osd(160 - Canvas::osd_width(text) / 2, y, text, color);
}

fn scan_wave(c: &mut Canvas, age: f32) {
    if age >= 1.2 {
        return;
    }
    let r = age / 1.2 * 150.0;
    let color = [90, (255.0 * (1.0 - age / 1.2)) as u8, 230];
    for i in 0..96 {
        let a = i as f32 / 96.0 * std::f32::consts::TAU;
        let p = vec2(160.0, 72.0) + vec2(a.cos() * r, a.sin() * r * 0.55);
        if p.y < BAR_TOP as f32 {
            c.pixel(p.x as i32, p.y as i32, color);
        }
    }
}

/// Wireframe box around a scanned prop with its name and yield.
fn scan_box(
    c: &mut Canvas,
    camera: &crate::flight::Camera,
    prop: &Prop,
    biome: crate::sky::biome::Biome,
    new: bool,
    label: bool,
) {
    let h = prop.half_extent();
    let center = prop.center();
    let corners: [Option<Vec2>; 8] = std::array::from_fn(|i| {
        let s = vec3(
            if i & 1 == 0 { -1.0 } else { 1.0 },
            if i & 2 == 0 { -1.0 } else { 1.0 },
            if i & 4 == 0 { -1.0 } else { 1.0 },
        );
        camera
            .project(center + h * s)
            .filter(|(_, depth, _)| *depth > 0.0)
            .map(|(p, _, _)| p)
    });
    let color = if prop.kind.is_poi() {
        GOLD
    } else if new {
        CYAN
    } else {
        MUTED
    };
    for (a, b) in [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        if let (Some(p), Some(q)) = (corners[a], corners[b])
            && p.distance(q) < 200.0
            && p.y < BAR_TOP as f32
            && q.y < BAR_TOP as f32
        {
            c.line(p.x as i32, p.y as i32, q.x as i32, q.y as i32, color);
        }
    }
    if !label {
        return;
    }
    if let Some(top) = corners.iter().flatten().min_by(|a, b| a.y.total_cmp(&b.y)) {
        let name = prop.name();
        let yield_text = prop
            .harvest(biome)
            .map_or(prop.kind.label().to_owned(), |(item, n)| {
                format!("{} {n}", item.symbol().to_uppercase())
            });
        let x = top.x as i32;
        if top.y > 22.0 && top.y < 120.0 {
            c.osd(
                x - Canvas::osd_width(&name) / 2,
                top.y as i32 - 18,
                &name,
                color,
            );
            c.osd(
                x - Canvas::osd_width(&yield_text) / 2,
                top.y as i32 - 9,
                &yield_text,
                WHITE,
            );
        }
    }
}

fn on_foot(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let camera = crate::flight::Camera::new(&app.flight, WIDTH, HEIGHT);
    let biome = sky
        .planet()
        .map_or(crate::sky::biome::Biome::Lush, |p| p.biome);
    if let Some(planet) = sky.planet() {
        c.osd(4, 4, &planet.name, CYAN);
        let (hazard, strength) = planet.biome.hazard();
        let sheltered = sky
            .parked
            .is_some_and(|p| p.position.distance(sky.walker.position) < 10.0);
        let text = if sheltered {
            "SHELTERED".to_owned()
        } else if strength > 0.0 {
            format!("{} {:.0}%", hazard.name(), strength * 100.0)
        } else {
            hazard.name().to_owned()
        };
        c.osd(
            4,
            13,
            &text,
            if strength > 0.0 && !sheltered {
                GOLD
            } else {
                MUTED
            },
        );
    }
    credits(c, app);
    heading_tape(c, sky.walker.yaw);
    // ship beacon
    if let Some(parked) = sky.parked {
        let d = parked.position.distance(sky.walker.position);
        let label = if d < BOARD_DISTANCE {
            "SHIP / 7 BOARD".to_owned()
        } else {
            format!("SHIP {}", distance_label(d))
        };
        marker(c, &camera, parked.position + Vec3::Z * 4.0, &label, CYAN);
    }
    // scanner frames
    if sky.scan_age < 10.0 {
        // frame the six nearest finds in front of you; name the three nearest
        let eye = sky.walker.eye().position;
        let mut near: Vec<&Prop> = sky
            .revealed
            .iter()
            .filter(|p| !sky.harvested.contains(&p.id))
            .filter(|p| (p.center() - eye).dot(camera.forward) > 0.0)
            .collect();
        near.sort_by(|a, b| {
            a.position
                .distance(eye)
                .total_cmp(&b.position.distance(eye))
        });
        for (i, prop) in near.iter().take(6).enumerate() {
            scan_box(c, &camera, prop, biome, sky.scan_age < 3.0, i < 3);
        }
    }
    scan_wave(c, sky.scan_age);
    // beacon marks: the four nearest unvisited points of interest
    let here = sky.walker.position;
    let mut marks: Vec<&Prop> = sky
        .marked
        .iter()
        .filter(|p| !sky.harvested.contains(&p.id))
        .collect();
    marks.sort_by(|a, b| {
        a.position
            .distance(here)
            .total_cmp(&b.position.distance(here))
    });
    for poi in marks.iter().take(4) {
        let d = poi.position.distance(here);
        marker(
            c,
            &camera,
            poi.center(),
            &format!("{} {}", poi.kind.label(), distance_label(d)),
            GOLD,
        );
    }
    // drones: health over them, red arrows when off screen
    // only the nearest off-screen drone gets a name, so edge labels never pile up
    let nearest = sky.drones.iter().map(|d| d.id).min_by_key(|&id| {
        sky.drones
            .iter()
            .find(|d| d.id == id)
            .map_or(u32::MAX, |d| d.position.distance(here) as u32)
    });
    for drone in &sky.drones {
        let top = drone.position + Vec3::Z * (drone.radius() + 0.8);
        match camera.project(top) {
            Some((p, depth, _))
                if depth > -3.0 && (0.0..320.0).contains(&p.x) && (18.0..126.0).contains(&p.y) =>
            {
                let hp = (drone.health / drone.kind.spec().health).clamp(0.0, 1.0);
                c.rect(p.x as i32 - 8, p.y as i32 - 3, 16, 2, [70, 20, 26]);
                c.rect(p.x as i32 - 8, p.y as i32 - 3, (hp * 16.0) as i32, 2, RED);
                if drone.telegraph > 0.0 {
                    c.osd(p.x as i32 - 2, p.y as i32 - 13, "!", GOLD);
                }
            }
            _ => {
                let label = if Some(drone.id) == nearest {
                    drone.kind.spec().name
                } else {
                    ""
                };
                marker(c, &camera, drone.position, label, RED)
            }
        }
    }
    // lances and arcs
    for e in &sky.effects {
        if let EffectKind::Line(to) = e.kind
            && let (Some((a, _, _)), Some((b, _, _))) =
                (camera.project(e.position), camera.project(to))
        {
            let fade = 1.0 - e.age();
            let color = e.color.map(|v| (v as f32 * (0.4 + 0.6 * fade)) as u8);
            let (a, b) = (
                a.clamp(vec2(0.0, 0.0), vec2(319.0, 130.0)),
                b.clamp(vec2(0.0, 0.0), vec2(319.0, 130.0)),
            );
            c.line(a.x as i32, a.y as i32, b.x as i32, b.y as i32, color);
            c.line(
                a.x as i32,
                a.y as i32 + 1,
                b.x as i32,
                b.y as i32 + 1,
                [255, 255, 240],
            );
        }
    }
    // aim, prompt and mining progress
    let aimed = app.aimed_prop();
    let context = app.sky_context();
    beam(c, app, &camera, vec2(236.0, 126.0));
    let reticle_color = if sky.hit_marker > 0.0 {
        RED
    } else if aimed.is_some() {
        GOLD
    } else {
        WHITE
    };
    reticle(c, CROSSHAIR, reticle_color);
    if sky.hit_marker > 0.0 {
        let (x, y) = (CROSSHAIR.x as i32, CROSSHAIR.y as i32);
        for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            c.line(x + dx * 5, y + dy * 5, x + dx * 8, y + dy * 8, WHITE);
        }
    }
    let prompt = match context {
        Context::Swap | Context::Talk(_) => None,
        Context::Board => Some("7 BOARD SHIP".to_owned()),
        Context::Open(p) | Context::Read(p) | Context::Signal(p) => {
            Some(format!("7 {} {}", context.label(), p.kind.label()))
        }
    };
    if let Some(prompt) = prompt {
        c.osd(160 - Canvas::osd_width(&prompt) / 2, 80, &prompt, CYAN);
    } else if let Some(prop) = aimed {
        let label = prop.name();
        c.osd(160 - Canvas::osd_width(&label) / 2, 80, &label, WHITE);
        if let Some(&damage) = sky.prop_damage.get(&prop.id) {
            let progress = (damage / prop.toughness()).clamp(0.0, 1.0);
            c.rect(140, 90, 40, 3, [30, 40, 52]);
            c.rect(140, 90, (progress * 40.0) as i32, 3, GOLD);
        }
    }
    // where hits came from
    for &(source, age) in &sky.hurt {
        let angle = app.sky_heading_to(source);
        let fade = 1.0 - age / 0.8;
        let color = [255, (90.0 * fade) as u8, (90.0 * fade) as u8];
        for k in -5..=5 {
            let a = angle + k as f32 * 0.06;
            let p = CROSSHAIR + vec2(a.sin() * 36.0, -a.cos() * 26.0);
            c.rect(p.x as i32, p.y as i32, 2, 2, color);
        }
    }
    if sky.vitals.suit < 0.25 || !sky.hurt.is_empty() {
        let edge = if sky.vitals.suit <= 0.0 { 3 } else { 2 };
        c.rect(0, 0, edge, BAR_TOP, RED);
        c.rect(320 - edge, 0, edge, BAR_TOP, RED);
    }
    radar(c, app);
    // suit on the left, weapon on the right (drawn last so markers never cover them)
    let suit = [140, 200, 255];
    bar(c, 4, 26, "SHD", sky.vitals.suit, suit);
    bar(c, 4, 36, "O2", sky.vitals.life, Item::Oxygen.color());
    bar(c, 4, 46, "HAZ", sky.vitals.hazard, Item::Sodium.color());
    bar(c, 4, 56, "JET", sky.vitals.jetpack, CYAN);
    weapon_card(c, app);
}

/// Active weapon: name in its rarity colour, perk letters, slot dots, battery and heat.
fn weapon_card(c: &mut Canvas, app: &App) {
    let l = &app.sky.loadout;
    let Some(weapon) = l.weapon() else { return };
    let mut name = weapon.name();
    name.truncate(30);
    c.osd_right(316, 20, &name, weapon.rarity.color());
    let mut tags = String::new();
    for perk in weapon.perks().into_iter().flatten() {
        tags.push(perk.letter());
    }
    c.osd_right(286, 29, &tags, GOLD);
    for slot in 0..3 {
        let color = if slot == l.active {
            WHITE
        } else if l.equipped[slot].is_some() {
            MUTED
        } else {
            DIM
        };
        c.rect(296 + slot as i32 * 7, 31, 5, 4, color);
    }
    bar(c, 262, 39, "BAT", app.sky.vitals.tool, Item::Carbon.color());
    let hot = l.overheat[l.active] > 0.0;
    bar(
        c,
        262,
        49,
        "HOT",
        1.0 - l.heat[l.active],
        if hot { RED } else { GOLD },
    );
}

/// 30 px radar, forward is up: ship, drones, marked points of interest and scanned props.
fn radar(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let center = vec2(301.0, 104.0);
    let radius = 14.0;
    let range = 120.0;
    for i in 0..40 {
        let a = i as f32 / 40.0 * std::f32::consts::TAU;
        c.pixel(
            (center.x + a.cos() * radius) as i32,
            (center.y + a.sin() * radius) as i32,
            [50, 70, 86],
        );
    }
    c.pixel(center.x as i32, center.y as i32, WHITE);
    c.pixel(center.x as i32, center.y as i32 - 1, WHITE);
    let here = sky.walker.position;
    let (s, co) = sky.walker.yaw.sin_cos();
    let mut dot = |point: Vec3, color: [u8; 3], size: i32, clamp: bool| {
        let d = (point - here).truncate() / range * radius;
        // world heading(yaw) = (sin, -cos): rotate so it points up
        let v = vec2(d.x * co + d.y * s, d.x * s - d.y * co);
        let v = vec2(v.x, -v.y);
        let v = if v.length() > radius {
            if !clamp {
                return;
            }
            v.normalize() * radius
        } else {
            v
        };
        c.rect(
            (center.x + v.x) as i32 - size / 2,
            (center.y + v.y) as i32 - size / 2,
            size,
            size,
            color,
        );
    };
    if sky.scan_age < 12.0 {
        for p in &sky.revealed {
            if !sky.harvested.contains(&p.id) {
                dot(p.position, [90, 110, 120], 1, false);
            }
        }
    }
    for p in &sky.marked {
        if !sky.harvested.contains(&p.id) {
            dot(p.position, GOLD, 2, true);
        }
    }
    if let Some(parked) = sky.parked {
        dot(parked.position, CYAN, 3, true);
    }
    for d in &sky.drones {
        dot(d.position, RED, 2, true);
    }
    // alert meter
    let alert = sky.alert.level;
    if alert > 0.01 {
        c.rect(287, 121, 28, 2, [50, 30, 34]);
        c.rect(
            287,
            121,
            (alert * 28.0) as i32,
            2,
            if alert > 0.5 { RED } else { GOLD },
        );
    }
}

fn panel(c: &mut Canvas, title: &str) {
    c.rect(0, 0, 320, BAR_TOP, PANEL);
    c.rect(0, 0, 320, 1, CYAN);
    c.osd(4, 4, title, CYAN);
    c.rect(4, 14, 312, 1, [36, 52, 66]);
}

fn galaxy(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    panel(c, &format!("GALAXY / {}", sky.system.name));
    let jumps = sky.inventory.count(Item::JumpCell);
    c.osd_right(
        316,
        4,
        &format!("JUMP CELLS {jumps}"),
        if jumps > 0 { CYAN } else { RED },
    );
    // 2.5D star field around the current system, slowly turning
    let spin = sky.time * 0.12;
    let (s, co) = (
        crate::sky::rng::fast_sin(spin),
        crate::sky::rng::fast_cos(spin),
    );
    let (st, ct) = sky.map_tilt.sin_cos();
    let origin = sky.system.id.vec();
    let center = vec2(104.0, 72.0);
    let scale = 13.0;
    let project = |p: Vec3| {
        let d = p - origin;
        let x = d.x * co - d.y * s;
        let y = d.x * s + d.y * co;
        center + vec2(x, y * st - d.z * ct) * scale
    };
    // range ring
    let range = sky.upgrades.warp_range();
    for i in 0..72 {
        let a = i as f32 / 72.0 * std::f32::consts::TAU;
        let p = center + vec2(a.cos(), a.sin() * st) * range * scale;
        if p.y > 16.0 && p.y < 128.0 && p.x < 206.0 {
            c.pixel(p.x as i32, p.y as i32, [40, 80, 96]);
        }
    }
    let neighbors = app.sky_neighbors();
    let selected = neighbors.get(sky.map_select).map(|n| n.0);
    for &(id, in_range, visited) in &neighbors {
        let p = project(id.vec());
        if !(2.0..206.0).contains(&p.x) || !(17.0..128.0).contains(&p.y) {
            continue;
        }
        let class = sky.galaxy.system(id).class;
        let color = if in_range { class.rgb() } else { [90, 96, 110] };
        c.rect(p.x as i32 - 1, p.y as i32 - 1, 2, 2, color);
        if visited {
            c.pixel(p.x as i32 + 2, p.y as i32 - 2, MUTED);
        }
        if Some(id) == selected {
            let (x, y) = (p.x as i32, p.y as i32);
            for (dx, dy) in [(-5, -5), (3, -5), (-5, 3), (3, 3)] {
                c.rect(x + dx, y + dy, 3, 1, GOLD);
                c.rect(
                    x + dx + if dx < 0 { 0 } else { 2 },
                    y + dy + if dy < 0 { 0 } else { -1 },
                    1,
                    2,
                    GOLD,
                );
            }
            c.line(
                center.x as i32,
                center.y as i32,
                x,
                y,
                if in_range { CYAN } else { RED },
            );
        }
    }
    // you are here
    c.rect(center.x as i32 - 2, center.y as i32 - 2, 4, 4, WHITE);
    c.osd(center.x as i32 - 8, center.y as i32 + 5, "YOU", WHITE);
    // info panel
    c.rect(210, 18, 1, 110, [36, 52, 66]);
    if let Some(&(id, in_range, visited)) = neighbors.get(sky.map_select) {
        let system = sky.galaxy.system(id);
        c.osd(216, 22, &system.name, system.class.rgb());
        c.osd(216, 34, &format!("{} STAR", system.class.name()), MUTED);
        c.osd(
            216,
            46,
            &format!("{:.1} LY", sky.system.id.light_years(id)),
            WHITE,
        );
        c.osd(216, 56, &format!("{} PLANETS", system.atmospheric()), WHITE);
        c.osd(216, 66, "1 MOON", WHITE);
        c.osd(
            216,
            80,
            if in_range { "IN RANGE" } else { "OUT OF RANGE" },
            if in_range { CYAN } else { RED },
        );
        if visited {
            c.osd(216, 90, "VISITED", MUTED);
        }
        c.osd(
            216,
            104,
            &format!("{}/{}", sky.map_select + 1, neighbors.len()),
            MUTED,
        );
    } else {
        c.osd(216, 22, "NO STARS NEAR", MUTED);
    }
    if app.world.kind != crate::world::MapKind::Space {
        c.osd(216, 116, "WARP FROM SPACE", GOLD);
    }
}

fn system(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    panel(c, &format!("SYSTEM / {}", sky.system.name));
    c.osd_right(
        316,
        4,
        &format!("{} STAR", sky.system.class.name()),
        sky.system.class.rgb(),
    );
    // orbit diagram
    let star = vec2(70.0, 72.0);
    c.rect(
        star.x as i32 - 3,
        star.y as i32 - 3,
        6,
        6,
        sky.system.class.rgb(),
    );
    let first = &sky.system.planets[0];
    let first_at =
        star + vec2(first.angle.cos(), first.angle.sin() * 0.5) * (12.0 + first.distance * 20.0);
    for (i, planet) in sky.system.planets.iter().enumerate() {
        let p = if planet.is_moon() {
            first_at + vec2(9.0, -6.0)
        } else {
            let r = 12.0 + planet.distance * 20.0;
            for k in 0..64 {
                let a = k as f32 / 64.0 * std::f32::consts::TAU;
                let q = star + vec2(a.cos(), a.sin() * 0.5) * r;
                if q.y > 16.0 && q.y < 128.0 {
                    c.pixel(q.x as i32, q.y as i32, [34, 50, 64]);
                }
            }
            star + vec2(planet.angle.cos(), planet.angle.sin() * 0.5) * r
        };
        let color = biome_color(planet.biome);
        let size = if planet.is_moon() { 2 } else { 4 };
        c.rect(
            p.x as i32 - size / 2,
            p.y as i32 - size / 2,
            size,
            size,
            color,
        );
        if i == sky.system_select {
            c.line(
                p.x as i32 - 6,
                p.y as i32 - 6,
                p.x as i32 - 3,
                p.y as i32 - 6,
                GOLD,
            );
            c.line(
                p.x as i32 - 6,
                p.y as i32 - 6,
                p.x as i32 - 6,
                p.y as i32 - 3,
                GOLD,
            );
            c.line(
                p.x as i32 + 6,
                p.y as i32 + 6,
                p.x as i32 + 3,
                p.y as i32 + 6,
                GOLD,
            );
            c.line(
                p.x as i32 + 6,
                p.y as i32 + 6,
                p.x as i32 + 6,
                p.y as i32 + 3,
                GOLD,
            );
        }
    }
    // planet list
    for (i, planet) in sky.system.planets.iter().enumerate() {
        let y = 20 + i as i32 * 13;
        if i == sky.system_select {
            c.rect(146, y - 2, 170, 12, SELECT);
        }
        let mark = if i == sky.course {
            ">"
        } else if Location::Planet(i) == sky.location {
            "-"
        } else {
            ""
        };
        c.osd(148, y, mark, GOLD);
        c.osd(
            156,
            y,
            &planet.name,
            if i == sky.system_select { GOLD } else { WHITE },
        );
        c.osd_right(
            314,
            y,
            planet.biome.name().split(' ').next().unwrap_or(""),
            biome_color(planet.biome),
        );
    }
    let p = &sky.system.planets[sky.system_select];
    let (hazard, strength) = p.biome.hazard();
    let y = 20 + sky.system.planets.len() as i32 * 13 + 4;
    c.osd(
        148,
        y,
        &format!("{} KM / {}", p.radius_km, hazard.name()),
        if strength > 0.0 { GOLD } else { MUTED },
    );
    c.osd(
        148,
        y + 10,
        &format!("YIELDS {}", p.biome.mineral().name()),
        MUTED,
    );
    if sky.system_select == sky.course {
        c.osd(148, y + 20, "CURRENT COURSE", CYAN);
    }
}

fn biome_color(b: crate::sky::biome::Biome) -> [u8; 3] {
    use crate::sky::biome::Biome;
    match b {
        Biome::Lush => [110, 220, 120],
        Biome::Desert => [235, 180, 110],
        Biome::Frozen => [190, 225, 255],
        Biome::Toxic => [200, 230, 70],
        Biome::Volcanic => [255, 110, 70],
        Biome::Exotic => [220, 120, 255],
        Biome::Barren => [170, 170, 180],
    }
}

fn inventory(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    panel(c, "INVENTORY");
    c.osd_right(
        316,
        4,
        &format!("{} CR  DATA {}", sky.credits, sky.data),
        GOLD,
    );
    if sky.crafting {
        crafting(c, app);
    } else if sky.arms {
        arms(c, app);
    } else {
        let slots = sky.inventory.slots();
        for (i, slot) in slots.iter().enumerate() {
            let (x, y) = (6 + (i % 4) as i32 * 33, 19 + (i / 4) as i32 * 28);
            let selected = i == sky.inventory_select;
            c.rect(x, y, 31, 26, if selected { SELECT } else { [18, 28, 40] });
            if selected {
                c.rect(x, y, 31, 1, GOLD);
                c.rect(x, y + 25, 31, 1, GOLD);
            }
            if let Some((item, n)) = slot {
                c.rect(x + 2, y + 2, 4, 4, item.color());
                c.osd(
                    x + 8,
                    y + 3,
                    item.symbol().to_uppercase().as_str(),
                    item.color(),
                );
                c.osd_right(x + 29, y + 16, &n.to_string(), WHITE);
            }
        }
        let info_x = 142;
        if let Some((item, _)) = slots[sky.inventory_select] {
            c.osd(info_x, 20, item.name(), item.color());
            c.osd(
                info_x,
                30,
                &format!("HELD {}", sky.inventory.count(item)),
                WHITE,
            );
            c.osd(info_x, 40, &format!("4 {}", item.use_hint()), CYAN);
        } else {
            c.osd(info_x, 20, "EMPTY SLOT", MUTED);
        }
        let v = &sky.vitals;
        bar(c, info_x, 58, "O2", v.life, Item::Oxygen.color());
        bar(c, info_x, 68, "HAZ", v.hazard, Item::Sodium.color());
        bar(c, info_x, 78, "TL", v.tool, Item::Carbon.color());
        bar(c, info_x + 88, 58, "SHD", v.shield, WHITE);
        bar(c, info_x + 88, 68, "PLS", v.pulse, CYAN);
        bar(c, info_x + 88, 78, "LFT", v.lift, [120, 170, 255]);
        c.osd(
            info_x,
            96,
            &format!("SLOTS {}/16", sky.inventory.slots_used()),
            MUTED,
        );
        c.osd(
            info_x,
            106,
            &format!("DISCOVERIES {}", sky.discoveries),
            MUTED,
        );
        let rank = missions::RANKS[missions::rank(sky.standing)].0;
        let line = match sky.missions.first() {
            Some(m) => (
                missions::tracker(m, &sky.inventory),
                if m.done(&sky.inventory) { GOLD } else { CYAN },
            ),
            None => (
                format!("RANK {rank}  JOBS DONE {}", sky.missions_done),
                MUTED,
            ),
        };
        c.osd(info_x, 116, &line.0, line.1);
    }
}

/// Weapons view: slots and locker on the left, the selected weapon's card on the right.
fn arms(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let all = sky.loadout.all();
    for (i, weapon) in all.iter().enumerate() {
        let y = 19 + i as i32 * 11;
        let selected = i == sky.arms_select;
        if selected {
            c.rect(4, y - 2, 146, 10, SELECT);
        }
        let tag = if i < 3 {
            if i == sky.loadout.active { ">" } else { " " }
        } else {
            "L"
        };
        c.osd(6, y, tag, if i < 3 { GOLD } else { MUTED });
        match weapon {
            Some(w) => {
                let mut name = format!("{} MK{}", w.archetype.spec().name, w.mark());
                name.truncate(22);
                c.osd(14, y, &name, w.rarity.color());
            }
            None => c.osd(14, y, "EMPTY SLOT", DIM),
        }
    }
    c.rect(154, 18, 1, 110, [36, 52, 66]);
    let Some(Some(w)) = all.get(sky.arms_select) else {
        c.osd(160, 20, "NO WEAPON", MUTED);
        return;
    };
    let s = w.stats();
    let name = w.name();
    let (first, second) = if name.len() > 26 {
        let cut = name[..26].rfind(' ').unwrap_or(26);
        (name[..cut].to_owned(), name[cut..].trim().to_owned())
    } else {
        (name, String::new())
    };
    c.osd(160, 20, &first, w.rarity.color());
    c.osd(160, 29, &second, w.rarity.color());
    c.osd(160, 40, w.rarity.name(), w.rarity.color());
    c.osd(
        160,
        50,
        &format!("DPS {:.0}  RANGE {:.0}", w.dps(), s.range),
        WHITE,
    );
    let fire = match s.fire {
        crate::sky::arsenal::Fire::Beam => "BEAM".to_owned(),
        crate::sky::arsenal::Fire::Bolt if s.pellets > 1 => format!("{} BOLTS", s.pellets),
        crate::sky::arsenal::Fire::Bolt => "BOLTS".to_owned(),
        crate::sky::arsenal::Fire::Lance => "PIERCING LANCE".to_owned(),
        crate::sky::arsenal::Fire::Lob => format!("MORTAR {:.0}M BLAST", s.splash),
        crate::sky::arsenal::Fire::Chain => format!("ARC TO {}", s.chain + 1),
    };
    c.osd(160, 59, &fire, MUTED);
    let mut y = 71;
    for perk in w.perks().into_iter().flatten() {
        c.osd(160, y, &format!("{} {}", perk.letter(), perk.name()), GOLD);
        c.osd(166, y + 9, perk.effect(), MUTED);
        y += 19;
    }
    if w.rarity == crate::sky::arsenal::Rarity::Common {
        c.osd(160, 71, "NO PERKS", DIM);
    }
}

fn crafting(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    c.osd(6, 20, "RECIPES", CYAN);
    for (i, recipe) in RECIPES.iter().enumerate() {
        let y = 32 + i as i32 * 18;
        let selected = i == sky.craft_select;
        if selected {
            c.rect(4, y - 2, 312, 18, SELECT);
        }
        let possible = recipe.possible(&sky.inventory);
        c.osd(
            8,
            y,
            recipe.output.name(),
            if possible { recipe.output.color() } else { DIM },
        );
        c.osd_right(
            312,
            y,
            &format!("HELD {}", sky.inventory.count(recipe.output)),
            MUTED,
        );
        let mut x = 8;
        for &(item, n) in recipe.inputs {
            let have = sky.inventory.count(item);
            let text = format!("{n} {}", item.name());
            c.osd(x, y + 8, &text, if have >= n { WHITE } else { RED });
            x += Canvas::osd_width(&text) + 12;
        }
    }
}

fn station_panel(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    panel(c, &format!("STATION / {}", sky.system.name));
    c.osd_right(316, 4, &format!("{} CR", sky.credits), GOLD);
    let tabs = ["SELL", "BUY", "UPGRADE"];
    for (i, tab) in tabs.iter().enumerate() {
        let x = 6 + i as i32 * 64;
        let active = i == sky.station_tab;
        if active {
            c.rect(x - 2, 17, Canvas::osd_width(tab) + 4, 10, SELECT);
        }
        c.osd(x, 18, tab, if active { GOLD } else { MUTED });
    }
    let market = sky.system.market;
    let mut rows: Vec<(String, String, [u8; 3])> = Vec::new();
    match sky.station_tab {
        0 => {
            rows.push((
                format!("SURVEY DATA X{}", sky.data),
                format!("+{}", sky.data * station::DATA_PRICE),
                CYAN,
            ));
            for item in Item::ALL {
                let (_, sell) = station::prices(item, market);
                rows.push((
                    format!("{} X{}", item.name(), sky.inventory.count(item)),
                    format!("{sell} EA"),
                    item.color(),
                ));
            }
        }
        1 => {
            for weapon in sky.armory {
                match weapon {
                    Some(w) => {
                        let mut name = w.name();
                        name.truncate(34);
                        let price = (w.value() as f32 * market) as u32;
                        rows.push((name, format!("{price}"), w.rarity.color()));
                    }
                    None => rows.push(("SOLD".to_owned(), String::new(), DIM)),
                }
            }
            for (item, n) in STATION_STOCK {
                let (buy, _) = station::prices(item, market);
                rows.push((
                    format!("{n} {}", item.name()),
                    format!("{}", buy * n as u32),
                    item.color(),
                ));
            }
            let cost = ((1.0 - sky.vitals.shield) * 400.0).ceil() as u32;
            rows.push(("REPAIR SHIELD".to_owned(), format!("{cost}"), WHITE));
        }
        _ => {
            for upgrade in Upgrade::ALL {
                let level = sky.upgrades.level(upgrade);
                let price = if level >= Upgrade::MAX {
                    "MAX".to_owned()
                } else {
                    format!("{}", upgrade.cost(level))
                };
                rows.push((
                    format!("{} MK{} {}", upgrade.name(), level + 1, upgrade.effect()),
                    price,
                    WHITE,
                ));
            }
        }
    }
    let visible = 9;
    let first = sky.station_select.saturating_sub(visible - 1);
    for (i, (label, value, color)) in rows.iter().enumerate().skip(first).take(visible) {
        let y = 31 + (i - first) as i32 * 11;
        let selected = i == sky.station_select;
        if selected {
            c.rect(4, y - 2, 312, 10, SELECT);
        }
        c.osd(8, y, label, if selected { GOLD } else { *color });
        c.osd_right(312, y, value, if selected { GOLD } else { WHITE });
    }
}

fn lore_panel(c: &mut Canvas, app: &App) {
    let Some((title, lines)) = &app.sky.lore else {
        return;
    };
    panel(c, &format!("INSCRIPTION / {title}"));
    for (i, line) in lines.iter().enumerate() {
        let y = 44 + i as i32 * 18;
        c.osd(
            160 - Canvas::osd_width(line) / 2,
            y,
            line,
            if i == 1 { GOLD } else { WHITE },
        );
    }
    c.osd(
        160 - Canvas::osd_width("SURVEY DATA RECORDED") / 2,
        104,
        "SURVEY DATA RECORDED",
        MUTED,
    );
}

/// First active missions in the corner of the HUD.
fn tracker(c: &mut Canvas, app: &App, y: i32) {
    for (i, m) in app.sky.missions.iter().take(2).enumerate() {
        let done = m.done(&app.sky.inventory);
        let text = missions::tracker(m, &app.sky.inventory);
        c.osd(
            4,
            y + i as i32 * 9,
            &text,
            if done { GOLD } else { [150, 200, 210] },
        );
    }
}

/// Pirates, the asteroid field and hits on the ship, over the flight HUD in space.
fn space_hud(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    if sky.location != Location::Space {
        return;
    }
    let camera = crate::flight::Camera::new(&app.flight, WIDTH, HEIGHT);
    let ship = app.flight.position;
    // only the nearest raider (and an ace) is named, so labels do not pile up
    let nearest = sky
        .pirates
        .iter()
        .map(|p| p.position.distance(ship))
        .fold(f32::INFINITY, f32::min);
    for p in &sky.pirates {
        let d = p.position.distance(ship);
        let named = p.ace || d <= nearest;
        let label = if p.ace {
            format!("{} {}", p.name, distance_label(d))
        } else {
            format!("RAIDER {}", distance_label(d))
        };
        match camera.project(p.position) {
            Some((q, depth, _))
                if depth > 0.0 && (4.0..316.0).contains(&q.x) && (20.0..124.0).contains(&q.y) =>
            {
                let hp = (p.health / p.max_health).clamp(0.0, 1.0);
                let (x, y) = (q.x as i32, q.y as i32);
                c.rect(x - 9, y - 9, 18, 2, [70, 20, 26]);
                c.rect(x - 9, y - 9, (hp * 18.0) as i32, 2, RED);
                for (dx, dy) in [(-6, -6), (4, -6), (-6, 4), (4, 4)] {
                    c.rect(x + dx, y + dy, 3, 1, RED);
                }
                if named {
                    c.osd(
                        x - Canvas::osd_width(&label) / 2,
                        y + 8,
                        &label,
                        if p.ace { GOLD } else { RED },
                    );
                }
            }
            _ => marker(c, &camera, p.position, if named { &label } else { "" }, RED),
        }
    }
    if !sky.pirates.is_empty() && (sky.time * 2.0).fract() < 0.6 {
        centered(c, 34, "PIRATES", RED);
    }
    let field = sky.system.asteroid_center;
    let d = field.distance(ship);
    if (1200.0..9000.0).contains(&d) && sky.pirates.is_empty() {
        marker(
            c,
            &camera,
            field,
            &format!("ASTEROIDS {}", distance_label(d)),
            [200, 180, 150],
        );
    }
    if !sky.hurt.is_empty() || sky.vitals.shield < 0.25 {
        c.rect(0, 0, 2, BAR_TOP, RED);
        c.rect(318, 0, 2, BAR_TOP, RED);
    }
    if sky.hit_marker > 0.0 {
        for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            c.line(160 + dx * 5, 73 + dy * 5, 160 + dx * 8, 73 + dy * 8, WHITE);
        }
    }
}

/// Walking the station: who is who, what key 7 does.
fn station_hud(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let camera = crate::flight::Camera::new(&app.flight, WIDTH, HEIGHT);
    c.osd(4, 4, &format!("{} STATION", sky.system.name), CYAN);
    let rank = missions::RANKS[missions::rank(sky.standing)].0;
    c.osd(4, 13, &format!("GUILD {rank}"), MUTED);
    credits(c, app);
    heading_tape(c, sky.walker.yaw);
    let here = sky.walker.position;
    for npc in app.station_people() {
        let head = npc.position + Vec3::Z * 3.2;
        if npc.position.distance(here) > 40.0 {
            continue;
        }
        if let Some((p, depth, _)) = camera.project(head)
            && depth > -3.0
            && (0.0..320.0).contains(&p.x)
            && (16.0..124.0).contains(&p.y)
        {
            let [r, g, b, _] = npc.role.sign().to_le_bytes();
            let title = npc.role.title();
            c.osd(
                p.x as i32 - Canvas::osd_width(title) / 2,
                p.y as i32 - 10,
                title,
                [r, g, b],
            );
            c.osd(
                p.x as i32 - Canvas::osd_width(&npc.name) / 2,
                p.y as i32 - 1,
                &npc.name,
                WHITE,
            );
        }
    }
    reticle(c, CROSSHAIR, WHITE);
    let prompt = match app.sky_context() {
        Context::Talk(i) => app
            .station_people()
            .get(i)
            .map(|n| format!("7 TALK TO {}", n.role.title())),
        Context::Board => Some("7 BOARD AND UNDOCK".to_owned()),
        _ => None,
    };
    if let Some(prompt) = prompt {
        c.osd(160 - Canvas::osd_width(&prompt) / 2, 84, &prompt, CYAN);
    }
    tracker(c, app, 26);
}

/// A conversation: portrait by species, what they say, what they offer.
fn talk_panel(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let people = app.station_people();
    let Some(npc) = people.get(sky.talking) else {
        return;
    };
    let [r, g, b, _] = npc.role.sign().to_le_bytes();
    panel(c, &format!("{} / {}", npc.role.title(), npc.name));
    // portrait: the person's voxel bust, turning a little as they talk
    c.rect(8, 20, 72, 96, [18, 26, 38]);
    for row in 0..24 {
        let t = row as f32 / 24.0;
        c.rect(
            8,
            92 + row,
            72,
            1,
            interior_blend([18, 26, 38], [r, g, b], t * 0.18),
        );
    }
    let bust = crate::sky::bust::bust(npc);
    super::sky_bust::portrait(c, &bust, sky.time, 8, 20, 72, 96);
    c.osd(8, 120, SPECIES_LABEL[npc.species as usize % 3], MUTED);
    // words
    let greeting = interior::greeting(npc, sky.visits);
    c.osd(90, 24, &format!("\"{greeting}\""), [r, g, b]);
    if sky.chat > 0 {
        let line = interior::chat(npc, sky.chat - 1);
        c.osd(90, 40, &format!("\"{line}\""), WHITE);
    }
    let offer = match npc.role {
        Role::Agent => format!("4 MISSIONS: {} ON THE BOARD", sky.board.len()),
        Role::Trader => "4 TRADE ELEMENTS AND DATA".to_owned(),
        Role::Outfitter => "4 WEAPONS FOR SALE".to_owned(),
        Role::Technician => "4 SHIP AND SUIT UPGRADES".to_owned(),
        Role::Traveller if sky.traveller_gift => "NOTHING MORE TODAY".to_owned(),
        Role::Traveller => "4 ASK ABOUT THE ROAD".to_owned(),
    };
    c.osd(90, 72, &offer, CYAN);
    c.osd(90, 84, "5 CHAT", MUTED);
    if npc.role == Role::Agent {
        let rank = missions::rank(sky.standing);
        c.osd(
            90,
            100,
            &format!("GUILD RANK {}", missions::RANKS[rank].0),
            GOLD,
        );
        if let Some(&(_, need)) = missions::RANKS.get(rank + 1) {
            c.osd(90, 110, &format!("STANDING {}/{need}", sky.standing), MUTED);
        }
    }
}

const SPECIES_LABEL: [&str; 3] = interior::SPECIES;

fn interior_blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

/// The mission board and your active missions.
fn missions_panel(c: &mut Canvas, app: &App) {
    let sky = &app.sky;
    let rank = missions::RANKS[missions::rank(sky.standing)].0;
    panel(c, &format!("MISSIONS / GUILD {rank}"));
    c.osd_right(316, 4, &format!("{} CR", sky.credits), GOLD);
    let board = sky.mission_tab == 0;
    let tabs = [
        format!("BOARD {}", sky.board.len()),
        format!("ACTIVE {}/{}", sky.missions.len(), missions::MAX_ACTIVE),
    ];
    for (i, tab) in tabs.iter().enumerate() {
        let x = 6 + i as i32 * 90;
        let active = (i == 0) == board;
        if active {
            c.rect(x - 2, 17, Canvas::osd_width(tab) + 4, 10, SELECT);
        }
        c.osd(x, 18, tab, if active { GOLD } else { MUTED });
    }
    let list = if board { &sky.board } else { &sky.missions };
    if list.is_empty() {
        let text = if board {
            "NO WORK LEFT TODAY"
        } else if app.sky.phase == Phase::Station {
            "TALK TO THE MISSION AGENT"
        } else {
            "NO ACTIVE MISSIONS"
        };
        centered(c, 60, text, MUTED);
        return;
    }
    for (i, m) in list.iter().enumerate().take(6) {
        let y = 30 + i as i32 * 15;
        let selected = i == sky.mission_select;
        if selected {
            c.rect(4, y - 2, 312, 15, SELECT);
        }
        let done = !board && m.done(&sky.inventory);
        let title = if board {
            m.kind.title().to_owned()
        } else {
            format!("{} {}/{}", m.kind.title(), m.shown(&sky.inventory), m.count)
        };
        c.osd(
            8,
            y,
            &title,
            if done {
                GOLD
            } else if selected {
                WHITE
            } else {
                CYAN
            },
        );
        let prize = match m.prize {
            missions::Prize::Weapon => " +W",
            missions::Prize::Item(..) => " +I",
            missions::Prize::None => "",
        };
        c.osd_right(312, y, &format!("{}{prize}", m.reward), GOLD);
        c.osd(14, y + 7, &m.describe(&sky.galaxy), MUTED);
    }
    if !board && app.sky.phase != Phase::Station {
        c.osd(8, 121, "CLAIM REWARDS ABOARD ANY STATION", MUTED);
    }
}
