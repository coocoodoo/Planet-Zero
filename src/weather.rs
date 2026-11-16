use crate::render::{mix, rgb};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    Snow,
    Storm,
    Fog,
    Sandstorm,
}

impl Weather {
    pub const ALL: [Self; 6] = [
        Self::Clear,
        Self::Rain,
        Self::Snow,
        Self::Storm,
        Self::Fog,
        Self::Sandstorm,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Clear => "CLEAR",
            Self::Rain => "RAIN",
            Self::Snow => "SNOW",
            Self::Storm => "STORM",
            Self::Fog => "FOG",
            Self::Sandstorm => "SANDSTORM",
        }
    }
    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|weather| weather.name().eq_ignore_ascii_case(name))
    }
    pub fn visibility(self, altitude: f32) -> (f32, f32) {
        match self {
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
            Self::Clear => match theme {
                1 => rgb(250, 182, 134),
                2 => rgb(28, 44, 92),
                _ => rgb(188, 224, 246),
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
            Self::Clear => color,
            Self::Rain => mix(color, rgb(44, 60, 80), 0.28),
            Self::Snow => mix(color, rgb(153, 171, 184), 0.18),
            Self::Storm => mix(color, rgb(20, 30, 49), 0.5),
            Self::Fog => mix(color, rgb(163, 173, 174), 0.18),
            Self::Sandstorm => mix(color, rgb(173, 121, 56), 0.38),
        }
    }
    pub fn fog(self, distance: f32, altitude: f32) -> f32 {
        let (start, end) = self.visibility(altitude);
        ((distance - start) / (end - start)).clamp(0.0, 1.0).sqrt()
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
