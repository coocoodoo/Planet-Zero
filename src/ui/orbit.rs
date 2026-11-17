use super::*;
use crate::world::MapKind;
use glam::vec2;

pub(super) fn draw(app: &mut App) {
    if app.page != Page::Playing || app.mode != Mode::Free || app.flight.demo {
        return;
    }
    if !app.flight.hud {
        return;
    }
    let label = if let Some(transition) = &app.orbit.transition {
        if transition.ascent {
            "ORBITAL ASCENT"
        } else {
            "SURFACE APPROACH"
        }
    } else if app.orbit.boost {
        "ORBIT BOOST"
    } else {
        ""
    };
    let body = app.orbit_body();
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    c.osd(160 - Canvas::osd_width(label) / 2, 120, label, CYAN);
    if app.world.kind == MapKind::Space && app.navigation.target.is_none() {
        let delta = body.center() - app.flight.position;
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
        c.osd(
            160 - Canvas::osd_width(body.name()) / 2,
            48,
            body.name(),
            WHITE,
        );
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
        let range = format!("{:.0}U", (delta.length() - body.radius()).max(0.0));
        c.osd(160 - Canvas::osd_width(&range) / 2, 94, &range, WHITE);
    }
}
