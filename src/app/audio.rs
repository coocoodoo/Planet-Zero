use super::*;
use crate::audio::{Cue, Frame};

impl App {
    pub fn audio_frame(&mut self) -> Frame {
        let mut events = self.sounds.take();
        for sound in self.mission.sounds.take().iter() {
            events.push(sound);
        }
        let cue = match self.page {
            Page::Menu => Cue::Menu,
            Page::Help if self.help_return == Page::Menu => Cue::Menu,
            Page::Loading => Cue::Flight,
            Page::Results => {
                if self.mission.outcome == Some(true) {
                    Cue::Victory
                } else {
                    Cue::Defeat
                }
            }
            _ => match self.mode {
                Mode::Free => Cue::Flight,
                Mode::Race | Mode::Coins => Cue::Race,
                Mode::Combat if self.mission.countdown > 0.0 => Cue::Flight,
                Mode::Combat if self.mission.phase() == campaign::Phase::Boss => Cue::Boss,
                Mode::Combat => Cue::Combat,
            },
        };
        let flying = self.page == Page::Playing && self.mission.countdown <= 0.0;
        let danger = if self.mode == Mode::Combat {
            (1.0 - self.mission.health / (self.mission.max_health * 0.35))
                .clamp(0.0, 1.0)
                .max(
                    if self
                        .mission
                        .enemies
                        .iter()
                        .any(|e| e.is_boss() && e.health < e.max_health * 0.5)
                    {
                        0.6
                    } else {
                        0.0
                    },
                )
        } else {
            0.0
        };
        Frame {
            cue,
            paused: matches!(self.page, Page::Paused | Page::Help | Page::Settings),
            engine: f32::from(flying)
                * if self.flight.vehicle == crate::content::vehicles::Vehicle::Paper {
                    0.18
                } else {
                    1.0
                },
            boost: self.flight.boost,
            danger,
            overdrive: self.mission.overdrive_time > 0.0,
            settings: self.audio_settings,
            events,
        }
    }
}
