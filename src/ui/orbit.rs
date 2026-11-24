use super::*;
use crate::world::MapKind;
use glam::vec2;

pub(super) fn draw(app: &mut App) {
    if app.page != Page::Playing || !app.mode.roams() || app.flight.demo {
        return;
    }
    if app.orbit.arrival_notice > 0.0 && !app.explore() {
        let mut c = Canvas {
            pixels: &mut app.screen,
        };
        c.shade(62, 40, 196, 36);
        c.text_scale(64, 44, "SPACE REACHED", CYAN, 2);
        c.osd(88, 65, "M OPEN ORBITAL CHART", WHITE);
    }
    if !app.flight.hud {
        return;
    }
    let label = if app.orbit.transition.is_some() {
        if app.flight.velocity().z >= 0.0 {
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
    let body_name = app.body_name(body);
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    c.osd(160 - Canvas::osd_width(label) / 2, 120, label, CYAN);
    if app.world.kind == MapKind::Space
        && app.navigation.target.is_none()
        && app.orbit.arrival_notice <= 0.0
    {
        let delta = body.center() - app.flight.position;
        let [right_axis, forward, up] = app.flight.axes();
        let d = delta.normalize_or_zero();
        let v = vec2(d.dot(right_axis), -d.dot(up));
        let direction = if v.length_squared() > 0.0001 {
            v.normalize()
        } else {
            vec2(0.0, if d.dot(forward) >= 0.0 { -1.0 } else { 1.0 })
        };
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
            160 - Canvas::osd_width(&body_name) / 2,
            48,
            &body_name,
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
