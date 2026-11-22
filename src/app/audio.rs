use super::*;
use crate::audio::{Cue, Frame, Sound};
use crate::content::weather::Weather;
use crate::world::{MapKind, orbit::Body};

/// 火箭轰鸣达到完整强度的高度
const ASCENT_RUMBLE_FULL_ALTITUDE: f32 = 1000.0;
/// 火箭轰鸣从这一高度开始逐渐增强
const ASCENT_RUMBLE_ALTITUDE: f32 = 600.0;

/// 连续飞行状态的上一帧取值，用于触发一次性音效
#[derive(Default)]
pub(super) struct SoundState {
    boosting: bool,
    armed: bool,
    lightning: bool,
}

impl App {
    /// 竞速、金币与战斗的激烈程度，以及冲刺段强度
    fn intensity(&self) -> (f32, f32) {
        let m = &self.mission;
        if m.countdown > 0.0 {
            return (0.2, 0.0);
        }
        match self.mode {
            Mode::Race => {
                let progress = m.gate_index as f32 / m.gates.len().max(1) as f32;
                let finale = ((progress - 0.75) / 0.1).clamp(0.0, 1.0);
                (0.35 + 0.65 * progress + 0.1 * self.flight.boost, finale)
            }
            Mode::Coins => {
                let finale = ((15.0 - m.remaining) / 3.0).clamp(0.0, 1.0);
                (0.4 + m.combo as f32 * 0.06, finale)
            }
            Mode::Combat => {
                if m.phase() == campaign::Phase::Boss {
                    return (1.0, 0.0);
                }
                // 巡逻时只保留律动，敌机逼近、弹幕密集或开火时逐层加满
                let near = m
                    .enemies
                    .iter()
                    .filter(|e| e.position.distance(self.flight.position) < 420.0)
                    .count() as f32;
                let fire = m.shots.iter().filter(|s| s.hostile).count() as f32;
                // 自动开火默认开启，只有附近有敌机时才算交火
                let firing =
                    f32::from(near > 0.0 && (self.keys.contains(&crate::keys::FIRE) || m.autofire));
                let danger = 1.0 - m.health / m.max_health.max(1.0);
                (
                    0.2 + near * 0.14 + fire * 0.012 + firing * 0.2 + danger * 0.3,
                    0.0,
                )
            }
            Mode::Free => (1.0, 0.0),
        }
    }
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
        // 闪电亮起时补一声雷鸣，雷声本身带有延迟
        let lightning =
            !self.world.kind.vacuum() && self.flight.weather.flash(self.flight.time) > 0.06;
        if lightning && !self.sound_state.lightning {
            self.sounds.push(Sound::Thunder);
        }
        self.sound_state.lightning = lightning;
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
        // 下降段仍在太空配乐中，落地后切回地表曲目
        let descending = self.orbit.transition.is_some() && self.orbit.descending;
        let ascending = self.mode == Mode::Free
            && self.world.kind != MapKind::Space
            && !descending
            && (self.orbit.boost || ascent > 0.3 || self.orbit.transition.is_some());
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
        let weather = self.flight.weather;
        let w = |kind| weather.weight(kind).min(1.0);
        let air = f32::from(!self.world.kind.vacuum() && !descending);
        let (twilight, night, synthwave) = if air > 0.0 {
            crate::render::time_of_day(&self.flight, &self.world, self.theme)
        } else {
            (0.0, 0.0, 0.0)
        };
        let (intensity, finale) = self.intensity();
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
                    / (ASCENT_RUMBLE_FULL_ALTITUDE - ASCENT_RUMBLE_ALTITUDE))
                    .clamp(0.0, 1.0),
            altitude,
            vacuum: self.world.kind.vacuum() || descending || altitude > 0.85,
            weather: weather.target,
            rain: air * (w(Weather::Rain) + w(Weather::Storm)),
            wind: air * (w(Weather::Sandstorm) + w(Weather::Storm) * 0.6 + w(Weather::Snow) * 0.3),
            haze: air * (w(Weather::Fog) + w(Weather::Sandstorm) * 0.6 + w(Weather::Snow) * 0.4),
            snow: air * w(Weather::Snow),
            storm: air * w(Weather::Storm),
            twilight,
            night,
            synthwave: if map == MapKind::City { synthwave } else { 0.0 },
            intensity: intensity.clamp(0.0, 1.0),
            finale,
            music_ducked: ascending && cue == Cue::Flight,
        }
    }
}
