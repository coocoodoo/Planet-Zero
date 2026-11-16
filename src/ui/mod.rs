mod canvas;
mod combat;
mod hud;
mod menus;
use crate::app::{App, Page};
use crate::flight::angle;
use crate::gameplay::Mode;
use canvas::*;
use combat::combat_hud;

pub fn draw(app: &mut App) {
    if app.page == Page::Playing {
        hud::draw(app);
    } else {
        menus::draw(app);
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
