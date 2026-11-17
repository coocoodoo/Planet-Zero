use super::*;
use crate::world::MapKind;
use crate::world::celestial::{MOON_RADIUS, moon_center};
use glam::vec2;

pub(super) fn draw(app: &mut App) {
    if app.page != Page::Playing || app.mode != Mode::Free || app.flight.demo {
        return;
    }
    if let Some(transition) = &app.orbit.transition {
        let alpha = transition.opacity();
        let tint = if transition.target.kind == MapKind::Space {
            [38, 67, 106]
        } else {
            [146, 153, 166]
        };
        for pixel in app.screen.chunks_exact_mut(4) {
            let old = pixel[3] as f32 / 255.0;
            let combined = alpha + old * (1.0 - alpha);
            if combined > 0.0 {
                for channel in 0..3 {
                    pixel[channel] = ((tint[channel] as f32 * alpha
                        + pixel[channel] as f32 * old * (1.0 - alpha))
                        / combined) as u8;
                }
                pixel[3] = (combined * 255.0).round() as u8;
            }
        }
        if app.flight.hud {
            let label = if transition.target.kind == MapKind::Space {
                "ENTERING ORBIT"
            } else {
                "LUNAR DESCENT"
            };
            let mut c = Canvas {
                pixels: &mut app.screen,
            };
            c.osd(160 - Canvas::osd_width(label) / 2, 112, label, WHITE);
        }
        return;
    }
    if !app.flight.hud {
        return;
    }
    let ready = app.orbit_ready();
    let label = if app.orbit.boost {
        "B CUT ORBIT BOOST"
    } else if ready {
        "B ORBIT BOOST"
    } else {
        ""
    };
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    c.osd(160 - Canvas::osd_width(label) / 2, 120, label, CYAN);
    if app.world.kind == MapKind::Space && app.navigation.target.is_none() {
        let delta = moon_center(app.flight.position) - app.flight.position;
        let bearing = angle(delta.x.atan2(-delta.y) - app.flight.yaw);
        let direction = vec2(bearing.sin(), -bearing.cos());
        let right = vec2(-direction.y, direction.x);
        let center = vec2(160.0, 73.0);
        let tip = center + direction * 16.0;
        for p in [
            center + direction * 10.0 + right * 3.0,
            center + direction * 10.0 - right * 3.0,
        ] {
            c.line(tip.x as i32, tip.y as i32, p.x as i32, p.y as i32, WHITE);
        }
        c.osd(149, 48, "MOON", WHITE);
        c.osd(
            158,
            70,
            if delta.z > 15.0 {
                "▲"
            } else if delta.z < -15.0 {
                "▼"
            } else {
                "-"
            },
            WHITE,
        );
        let range = format!("{:.0}U", (delta.length() - MOON_RADIUS).max(0.0));
        c.osd(160 - Canvas::osd_width(&range) / 2, 94, &range, WHITE);
    }
}
