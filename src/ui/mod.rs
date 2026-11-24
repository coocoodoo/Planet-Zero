mod canvas;
mod chart;
mod combat;
mod fpv;
mod hud;
mod menus;
mod navigation;
mod orbit;
mod osd;
mod perf;
mod sky;
mod sky_icons;
use crate::app::{App, Page};
use crate::flight::angle;
use crate::gameplay::Mode;
use canvas::*;
use combat::combat_hud;

pub(crate) use menus::help_page_count;

pub fn draw(app: &mut App) {
    if app.page == Page::Chart {
        chart::draw(app);
    } else if app.page == Page::Playing && app.explore() {
        sky::draw(app);
    } else if app.page == Page::Playing {
        hud::draw(app);
        if app.flight.hud {
            navigation::draw(app);
        }
        if app.flight.hud && app.portal_labels {
            portal_labels(app);
        }
    } else {
        menus::draw(app);
    }
    // Cardputer Sky panels cover the view; the engine's orbit labels would sit on top of them
    if !(app.explore() && app.sky.overlay != crate::sky::Overlay::None) {
        orbit::draw(app);
    }
    perf::draw(app);
}

fn portal_labels(app: &mut App) {
    if !app.network.active || app.flight.orbit_quiet {
        return;
    }
    let camera = crate::flight::Camera::new(&app.flight, 320, 170);
    let pivot = glam::vec2(160.0, 170.0 * 0.43);
    let (s, c) = (-app.flight.screen_roll() * app.flight.weights[2]).sin_cos();
    let routes = app.network.routes(&app.world);
    let fpv = app.uses_fpv_hud();
    let cockpit = app.flight.weights[2] >= 0.5;
    let mut canvas = Canvas {
        pixels: &mut app.screen,
    };
    for route in routes {
        if app.flight.hides_portal(route.aperture) {
            continue;
        }
        let Some((point, depth, scale)) = camera.project(route.aperture.center) else {
            continue;
        };
        if !(50.0..1400.0).contains(&depth) {
            continue;
        }
        let delta = point - pivot;
        let p = glam::vec2(c * delta.x - s * delta.y, s * delta.x + c * delta.y) + pivot;
        let y = p.y + route.aperture.radius * scale + 5.0;
        let label = route.target.kind.name();
        let glyph = if fpv { 6.0 } else { 8.0 };
        let x = p.x - label.len() as f32 * glyph * 0.5;
        if x < 3.0 || x + label.len() as f32 * glyph > 317.0 || !(18.0..148.0).contains(&y) {
            continue;
        }
        let color = if app.network.get(route.target).is_some() {
            CYAN
        } else {
            MUTED
        };
        if fpv {
            let (left, right, bottom) = if cockpit {
                (108.0, 212.0, 92.0)
            } else {
                (48.0, 268.0, 110.0)
            };
            if x >= left && x + label.len() as f32 * glyph <= right && (33.0..bottom).contains(&y) {
                canvas.osd(x as i32, y as i32, label, color);
            }
        } else {
            canvas.text(x as i32, y as i32, label, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combat_notices_leave_center_view_untouched() {
        let mut app = App::new(crate::world::World::new(7));
        app.mode = Mode::Combat;
        app.start();
        app.mission.countdown = 0.0;
        for view in 0..4 {
            app.flight.weights = [0.0; 3];
            app.flight.weights[view.min(2)] = 1.0;
            app.flight.fixed_weight = if view == 3 { 1.0 } else { 0.0 };
            app.mission.notices.fill(None);
            app.draw_overlay();
            let baseline = app.screen.clone();
            app.mission.announce("WEAPON UPGRADE");
            app.mission.announce("WARNING / BOSS APPROACH");
            app.mission.announce("SHIELD HIT / ROLL TO EVADE");
            app.draw_overlay();
            for y in 45..110 {
                for x in 60..260 {
                    let at = (y * crate::app::WIDTH + x) * 4;
                    assert_eq!(&app.screen[at..at + 4], &baseline[at..at + 4]);
                }
            }
        }
    }
}
