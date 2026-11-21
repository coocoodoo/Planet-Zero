use super::*;
use crate::audio::{Cue, Frame, Sound};
use crate::world::{MapKind, orbit::Body};

/// 升空配乐切入高度：冲出低空、接近大气层边缘时才进入升空段落
pub const ASCENT_MUSIC_ALTITUDE: f32 = 1000.0;
/// 火箭轰鸣从这一高度开始逐渐增强
const ASCENT_RUMBLE_ALTITUDE: f32 = 600.0;

/// 连续飞行状态的上一帧取值，用于触发一次性音效
#[derive(Default)]
pub(super) struct SoundState {
    boosting: bool,
    armed: bool,
}

impl App {
    /// 大气层内的轨道推进强度，太空中的加速不算升空
    fn ascent(&self) -> f32 {
        if self.mode != Mode::Free || self.world.kind == MapKind::Space {
            0.0
        } else {
            self.flight.orbital_thrust
        }
    }
    /// 根据连续飞行状态的变化补充音效：加力点火与入轨高度提示
    pub(super) fn flight_sounds(&mut self) {
        let boost = self.flight.boost;
        if !self.sound_state.boosting && boost > 0.45 && self.ascent() < 0.1 {
            self.sound_state.boosting = true;
            self.sounds.push(Sound::BoostStart);
        } else if self.sound_state.boosting && boost < 0.15 {
            self.sound_state.boosting = false;
        }
        let altitude = self.flight.position.z;
        if self.flight.orbit_flight
            && self.world.kind != MapKind::Space
            && self.orbit.transition.is_none()
        {
            if !self.sound_state.armed && altitude >= orbit::ARM_ALTITUDE {
                self.sound_state.armed = true;
                self.sounds.push(Sound::OrbitReady);
            } else if altitude < orbit::ARM_ALTITUDE - 40.0 {
                self.sound_state.armed = false;
            }
        }
    }
    pub fn audio_frame(&mut self) -> Frame {
        let mut events = self.sounds.take();
        for sound in self.mission.sounds.take().iter() {
            events.push(sound);
        }
        let ascent = self.ascent();
        let altitude_m = self.flight.position.z;
        let ascending = ascent > 0.3
            && (altitude_m >= ASCENT_MUSIC_ALTITUDE || self.orbit.transition.is_some());
        // 下降段仍在太空配乐中，落地后切回地表曲目
        let descending = self.orbit.transition.is_some() && ascent <= 0.3;
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
                Mode::Free if ascending => Cue::Ascent,
                Mode::Free => Cue::Flight,
                Mode::Race => Cue::Race,
                Mode::Coins => Cue::Coins,
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
        let map = if descending {
            MapKind::Space
        } else {
            self.world.kind
        };
        let altitude = if self.world.kind == MapKind::Space {
            1.0
        } else {
            let body = Body::of(self.world.kind);
            (self.flight.position.z / body.orbit_altitude()).clamp(0.0, 1.0)
        };
        Frame {
            cue,
            paused: matches!(
                self.page,
                Page::Paused | Page::Help | Page::Settings | Page::Chart
            ),
            engine: f32::from(flying)
                * if self.flight.vehicle == crate::content::vehicles::Vehicle::Paper {
                    0.18
                } else {
                    1.0
                },
            boost: self.flight.boost.max(self.flight.orbital_thrust),
            danger,
            overdrive: self.mission.overdrive_time > 0.0,
            settings: self.audio_settings,
            events,
            map,
            vehicle: self.flight.vehicle as u8,
            ascent: ascent
                * f32::from(flying)
                * ((altitude_m - ASCENT_RUMBLE_ALTITUDE)
                    / (ASCENT_MUSIC_ALTITUDE - ASCENT_RUMBLE_ALTITUDE))
                    .clamp(0.0, 1.0),
            altitude,
            vacuum: self.world.kind.vacuum() || descending || altitude > 0.85,
        }
    }
}
