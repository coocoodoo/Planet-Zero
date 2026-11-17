mod canvas;
mod chart;
mod combat;
mod hud;
mod menus;
use crate::app::{App, Page};
use crate::flight::angle;
use crate::gameplay::Mode;
use canvas::*;
use combat::combat_hud;

pub(crate) const HELP_PAGE_COUNT: usize = menus::HELP_PAGES.len();

pub fn draw(app: &mut App) {
    if app.page == Page::Chart {
        chart::draw(app);
    } else if app.page == Page::Playing {
        hud::draw(app);
        if app.flight.hud && app.portal_labels {
            portal_labels(app);
        }
    } else {
        menus::draw(app);
    }
}

fn portal_labels(app: &mut App) {
    if !app.network.active {
        return;
    }
    let camera = crate::flight::Camera::new(&app.flight, 320, 170);
    let pivot = glam::vec2(160.0, 170.0 * 0.43);
    let (s, c) = (-(app.flight.bank + app.flight.spin) * app.flight.weights[2]).sin_cos();
    let routes = app.network.routes(&app.world);
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
        let x = p.x - label.len() as f32 * 3.0;
        if x < 3.0 || x + label.len() as f32 * 6.0 > 317.0 || !(18.0..148.0).contains(&y) {
            continue;
        }
        canvas.text(
            x as i32,
            y as i32,
            label,
            if app.network.get(route.target).is_some() {
                CYAN
            } else {
                MUTED
            },
        );
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
        app.mission.announce("WEAPON UPGRADE");
        app.mission.announce("WARNING / BOSS APPROACH");
        app.mission.announce("SHIELD HIT / ROLL TO EVADE");
        app.screen.fill(88);
        super::draw(&mut app);
        for y in 0..crate::app::HEIGHT {
            for x in 30..290 {
                let at = (y * crate::app::WIDTH + x) * 4;
                assert_eq!(&app.screen[at..at + 3], &[88; 3]);
            }
        }
    }
}
