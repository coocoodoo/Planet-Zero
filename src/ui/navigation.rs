use super::*;
use crate::app::chart::offset;
use crate::world::network::MapKey;
use glam::vec2;

pub(super) fn draw(app: &mut App) {
    let Some(target) = app
        .navigation
        .target
        .filter(|t| t.map == MapKey::of(&app.world))
    else {
        return;
    };
    let delta = offset(target.position.truncate(), app.flight.position.truncate());
    let bearing = angle(delta.x.atan2(-delta.y) - app.flight.yaw);
    let height = target.position.z - app.flight.position.z;
    let color = if app.navigation.arrived {
        WHITE
    } else if app.navigation.active {
        CYAN
    } else {
        GOLD
    };
    let mut c = Canvas {
        pixels: &mut app.screen,
    };
    let center = vec2(160.0, 73.0);
    for i in 0..16 {
        let a = i as f32 * std::f32::consts::TAU / 16.0;
        c.pixel(
            160 + (a.sin() * 14.0) as i32,
            73 - (a.cos() * 14.0) as i32,
            MUTED,
        );
    }
    let direction = vec2(bearing.sin(), -bearing.cos());
    let right = vec2(-direction.y, direction.x);
    let tip = center + direction * 14.0;
    for p in [
        center + direction * 8.0 + right * 3.0,
        center + direction * 8.0 - right * 3.0,
    ] {
        c.line(tip.x as i32, tip.y as i32, p.x as i32, p.y as i32, color);
    }
    c.osd(
        158,
        70,
        if height > 12.0 {
            "▲"
        } else if height < -12.0 {
            "▼"
        } else {
            "-"
        },
        color,
    );
    c.osd(
        152,
        48,
        if app.navigation.active {
            "NAV"
        } else if app.navigation.arrived {
            "OK"
        } else {
            "LOC"
        },
        color,
    );
    let distance = format!("{:.0}U", delta.length());
    c.osd(160 - Canvas::osd_width(&distance) / 2, 92, &distance, color);
    if height.abs() > 12.0 {
        c.osd(179, 70, &format!("{:+.0}", height), MUTED);
    }
}
