use crate::render::{mix, rgb};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    Snow,
    Storm,
    Fog,
    Sandstorm,
    Cloudy,
    Rainbow,
}

impl Weather {
    pub const ALL: [Self; 8] = [
        Self::Clear,
        Self::Rain,
        Self::Snow,
        Self::Storm,
        Self::Fog,
        Self::Sandstorm,
        Self::Cloudy,
        Self::Rainbow,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Clear => "CLEAR",
            Self::Cloudy => "CLOUDY",
            Self::Rainbow => "RAINBOW",
            Self::Rain => "RAIN",
            Self::Snow => "SNOW",
            Self::Storm => "STORM",
            Self::Fog => "FOG",
            Self::Sandstorm => "SANDSTORM",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|weather| weather.name().eq_ignore_ascii_case(name))
    }
    pub fn visibility(self, altitude: f32) -> (f32, f32) {
        match self {
            Self::Cloudy => (250.0 + altitude, 1400.0 + 2.0 * altitude),
            Self::Rainbow => (280.0 + altitude, 1600.0 + 2.0 * altitude),
            Self::Clear => (300.0 + 2.0 * altitude, 1500.0 + 3.0 * altitude),
            Self::Rain => (100.0, 900.0),
            Self::Snow => (65.0, 700.0),
            Self::Storm => (40.0, 620.0),
            Self::Fog => (5.0, 260.0),
            Self::Sandstorm => (12.0, 340.0),
        }
    }
    pub fn haze(self, theme: usize) -> u32 {
        let color = match self {
            Self::Clear | Self::Rainbow => match theme {
                1 => rgb(250, 182, 134),
                2 => rgb(28, 44, 92),
                _ => rgb(188, 224, 246),
            },
            Self::Cloudy => match theme {
                1 => rgb(220, 178, 160),
                2 => rgb(28, 40, 65),
                _ => rgb(180, 208, 227),
            },
            Self::Rain => rgb(116, 141, 158),
            Self::Snow => rgb(191, 205, 215),
            Self::Storm => rgb(58, 74, 96),
            Self::Fog => rgb(179, 191, 190),
            Self::Sandstorm => rgb(188, 146, 84),
        };
        if self != Self::Clear && theme == 2 {
            mix(color, rgb(8, 15, 30), 0.7)
        } else {
            color
        }
    }
    pub fn shade(self, color: u32) -> u32 {
        match self {
            Self::Clear | Self::Rainbow => color,
            Self::Cloudy => mix(color, rgb(93, 116, 139), 0.12),
            Self::Rain => mix(color, rgb(44, 60, 80), 0.28),
            Self::Snow => mix(color, rgb(153, 171, 184), 0.18),
            Self::Storm => mix(color, rgb(20, 30, 49), 0.5),
            Self::Fog => mix(color, rgb(163, 173, 174), 0.18),
            Self::Sandstorm => mix(color, rgb(173, 121, 56), 0.38),
        }
    }
    pub fn flash(self, time: f32) -> f32 {
        let phase = time.rem_euclid(11.0) - 7.5;
        if self == Self::Storm && (0.0..0.3).contains(&phase) {
            (1.0 - (phase / 0.15 - 1.0).abs()) * 0.55
        } else {
            0.0
        }
    }
    pub fn particles(self) -> (usize, f32, f32) {
        match self {
            Self::Rain => (450, 190.0, 18.0),
            Self::Snow => (270, 18.0, 9.0),
            Self::Storm => (680, 240.0, 55.0),
            Self::Sandstorm => (650, 8.0, 110.0),
            _ => (0, 0.0, 0.0),
        }
    }
}

use crate::world::MapKind;

#[derive(Clone, Copy)]
pub struct WeatherState {
    pub target: Weather,
    weights: [f32; 8],
    from: [f32; 8],
    elapsed: f32,
    remaining: f32,
    random: u32,
    pub fall_distance: f32,
    pub wind_distance: f32,
}

impl Weather {
    fn weight(self, map: MapKind) -> u16 {
        use MapKind::*;
        // 权重顺序为晴、雨、雪、暴雨、雾、沙尘暴、多云、彩虹
        let weights = match map {
            Moon | Space => [100, 0, 0, 0, 0, 0, 0, 0],
            Desert => [65, 2, 0, 1, 0, 18, 14, 3],
            Canyon => [55, 8, 0, 3, 3, 0, 31, 8],
            Alpine | Icebergs => [28, 0, 34, 0, 12, 0, 26, 0],
            Wetlands => [20, 28, 0, 10, 18, 0, 24, 12],
            Fjords | Highlands => [28, 22, 8, 5, 14, 0, 23, 10],
            Islands => [40, 22, 0, 9, 4, 0, 25, 12],
            City => [42, 17, 3, 5, 8, 0, 25, 10],
            Volcanic => [45, 12, 0, 5, 8, 0, 30, 6],
        };
        weights[self as usize]
    }
    fn successors(self) -> &'static [Self] {
        use Weather::*;
        match self {
            Clear => &[Cloudy, Sandstorm],
            Cloudy => &[Clear, Rain, Snow, Fog],
            Rain => &[Cloudy, Storm, Rainbow, Clear],
            Storm => &[Rain],
            Snow => &[Cloudy],
            Fog => &[Cloudy, Clear],
            Sandstorm => &[Clear, Cloudy],
            Rainbow => &[Clear],
        }
    }
    pub fn choose(map: MapKind, value: f32) -> Self {
        Self::weighted(
            &[
                Self::Clear,
                Self::Cloudy,
                Self::Rain,
                Self::Snow,
                Self::Storm,
                Self::Fog,
                Self::Sandstorm,
            ],
            map,
            value,
        )
    }
    fn weighted(choices: &[Self], map: MapKind, value: f32) -> Self {
        let total: u16 = choices.iter().map(|w| w.weight(map)).sum();
        let mut remaining = value * total as f32;
        for &weather in choices {
            remaining -= weather.weight(map) as f32;
            if remaining < 0.0 {
                return weather;
            }
        }
        Self::Clear
    }
}

impl WeatherState {
    pub fn new(weather: Weather, map: MapKind, seed: u32) -> Self {
        let target = if weather.weight(map) > 0 {
            weather
        } else {
            Weather::Clear
        };
        let weights = std::array::from_fn(|i| f32::from(i == target as usize));
        let mut state = Self {
            target,
            weights,
            from: weights,
            elapsed: 12.0,
            remaining: 0.0,
            random: seed,
            fall_distance: 0.0,
            wind_distance: 0.0,
        };
        state.remaining = 55.0 + state.random() * 40.0;
        state
    }
    fn random(&mut self) -> f32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 8) as f32 / 16777216.0
    }
    pub fn name(self) -> &'static str {
        self.target.name()
    }
    pub fn weight(self, weather: Weather) -> f32 {
        self.weights[weather as usize]
    }
    pub fn change(&mut self, target: Weather) {
        self.from = self.weights;
        self.target = target;
        self.elapsed = 0.0;
        self.remaining = if target == Weather::Rainbow {
            25.0
        } else {
            55.0 + self.random() * 40.0
        };
    }
    pub fn cycle(&mut self, map: MapKind, direction: i32) {
        let mut at = self.target as i32;
        for _ in 0..Weather::ALL.len() {
            at = (at + direction).rem_euclid(8);
            let target = Weather::ALL[at as usize];
            if target.weight(map) > 0 {
                self.change(target);
                return;
            }
        }
    }
    pub fn step(&mut self, dt: f32, map: MapKind) {
        if map.vacuum() {
            return;
        }
        self.elapsed = (self.elapsed + dt).min(12.0);
        let t = self.elapsed / 12.0;
        self.weights = std::array::from_fn(|i| {
            self.from[i] + (f32::from(i == self.target as usize) - self.from[i]) * t
        });
        let (_, _, fall, wind) = self.precipitation();
        self.fall_distance += fall * dt;
        self.wind_distance += wind * dt;
        self.remaining -= dt;
        if self.remaining <= 0.0 {
            let choices = self.target.successors();
            let random = self.random();
            self.change(Weather::weighted(choices, map, random));
        }
    }
    fn scalar(self, f: impl Fn(Weather) -> f32) -> f32 {
        Weather::ALL
            .into_iter()
            .zip(self.weights)
            .filter(|(_, w)| *w > 0.0)
            .map(|(kind, w)| f(kind) * w)
            .sum()
    }
    pub(crate) fn color(self, f: impl Fn(Weather) -> u32) -> u32 {
        let mut channels = [0.0; 3];
        for (kind, weight) in Weather::ALL.into_iter().zip(self.weights) {
            if weight == 0.0 {
                continue;
            }
            let c = f(kind).to_le_bytes();
            for i in 0..3 {
                channels[i] += c[i] as f32 * weight;
            }
        }
        rgb(
            channels[0].round() as u8,
            channels[1].round() as u8,
            channels[2].round() as u8,
        )
    }
    pub fn haze(self, theme: usize) -> u32 {
        self.color(|w| w.haze(theme))
    }
    pub fn shade(self, color: u32) -> u32 {
        self.color(|w| w.shade(color))
    }
    pub fn visibility(self, altitude: f32) -> (f32, f32) {
        (
            self.scalar(|w| w.visibility(altitude).0),
            self.scalar(|w| w.visibility(altitude).1),
        )
    }
    pub fn fog(self, distance: f32, altitude: f32) -> f32 {
        let (start, end) = self.visibility(altitude);
        ((distance - start) / (end - start)).clamp(0.0, 1.0).sqrt()
    }
    pub fn flash(self, time: f32) -> f32 {
        Weather::Storm.flash(time) * self.weight(Weather::Storm)
    }
    pub fn coverage(self) -> f32 {
        self.scalar(|w| match w {
            Weather::Cloudy | Weather::Storm => 14.0,
            Weather::Rain | Weather::Snow => 11.0,
            Weather::Rainbow => 9.0,
            _ => 7.0,
        })
    }
    pub fn sky(self, low: u32, high: u32, theme: usize) -> (u32, u32) {
        let colors = |w: Weather| {
            let cover = match w {
                Weather::Clear | Weather::Rainbow => return (low, high),
                Weather::Fog => 0.92,
                Weather::Sandstorm => 0.85,
                Weather::Snow => 0.68,
                Weather::Rain => 0.55,
                Weather::Cloudy => 0.3,
                _ => 0.4,
            };
            (w.haze(theme), mix(w.shade(high), w.haze(theme), cover))
        };
        (self.color(|w| colors(w).0), self.color(|w| colors(w).1))
    }
    pub fn precipitation(self) -> (Weather, f32, f32, f32) {
        let rain = self.weight(Weather::Rain) + self.weight(Weather::Storm);
        let kind = if self.weight(Weather::Snow) > rain
            && self.weight(Weather::Snow) > self.weight(Weather::Sandstorm)
        {
            Weather::Snow
        } else if self.weight(Weather::Sandstorm) > rain {
            Weather::Sandstorm
        } else {
            Weather::Rain
        };
        let amount = if kind == Weather::Rain {
            rain
        } else {
            self.weight(kind)
        };
        if amount == 0.0 {
            return (kind, 0.0, 0.0, 0.0);
        }
        let values = |index| {
            self.scalar(|w| {
                let (n, fall, wind) = w.particles();
                if w == kind || (kind == Weather::Rain && w == Weather::Storm) {
                    [n as f32, fall, wind][index]
                } else {
                    0.0
                }
            })
        };
        (kind, values(0), values(1) / amount, values(2) / amount)
    }
}

#[cfg(test)]
mod dynamic_tests {
    use super::*;
    #[test]
    fn transitions_follow_climate_and_rain_sequences() {
        for map in MapKind::ALL {
            let mut state = WeatherState::new(Weather::Clear, map, 19);
            for _ in 0..500 {
                let before = state.target;
                state.step(100.0, map);
                assert!(state.target.weight(map) > 0);
                if !map.vacuum() {
                    assert!(before.successors().contains(&state.target));
                }
            }
        }
        assert_eq!(Weather::Storm.successors(), &[Weather::Rain]);
        assert_eq!(Weather::Rainbow.successors(), &[Weather::Clear]);
    }
    #[test]
    fn changing_weather_keeps_current_colors_and_fades_rain() {
        let mut state = WeatherState::new(Weather::Storm, MapKind::Islands, 7);
        let old = state.haze(0);
        state.change(Weather::Rain);
        assert_eq!(old, state.haze(0));
        state.step(6.0, MapKind::Islands);
        assert_eq!(state.weight(Weather::Storm), 0.5);
        assert!(state.precipitation().1 > 450.0);
        let midpoint = state.haze(0);
        state.change(Weather::Clear);
        assert_eq!(midpoint, state.haze(0));
        state.step(12.0, MapKind::Islands);
        assert_eq!(state.precipitation().1, 0.0);
        assert_eq!(state.weight(Weather::Clear), 1.0);
    }
}
