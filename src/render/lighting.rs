use super::*;

#[derive(Clone, Copy)]
pub(super) struct Lighting {
    pub sun: Vec3,
    pub direction: Vec3,
    weights: [f32; 3],
}

impl Default for Lighting {
    fn default() -> Self {
        Self::at(0.125, glam::Quat::IDENTITY)
    }
}

impl Lighting {
    pub fn new(flight: &Flight, theme: usize) -> Self {
        let phase = match theme {
            1 => 0.49,
            2 => 0.75,
            _ => 0.125,
        };
        Self::at(
            phase + flight.day_time / crate::flight::DAY_SECONDS,
            flight.sky_rotation,
        )
    }

    fn at(phase: f32, rotation: glam::Quat) -> Self {
        let (s, c) = (phase * std::f32::consts::TAU).sin_cos();
        let sun = vec3(-c * 0.8660254, -c * 0.5, s);
        let direction = rotation.conjugate() * sun;
        Self::from_direction(sun, direction)
    }

    pub fn in_frame(self, rotation: glam::Mat3) -> Self {
        Self::from_direction(self.sun, rotation * self.sun)
    }

    fn from_direction(sun: Vec3, direction: Vec3) -> Self {
        let daylight = ((direction.z + 0.12) / 0.36).clamp(0.0, 1.0);
        let day = ((direction.z - 0.03) / 0.42).clamp(0.0, 1.0);
        // 调色权重量化后复用地形调色板，太阳方向保持连续
        let daylight = (daylight * 128.0).round() / 128.0;
        let day = (day * 128.0).round() / 128.0;
        Self {
            sun,
            direction,
            weights: [daylight * day, daylight * (1.0 - day), 1.0 - daylight],
        }
    }

    pub fn night(self) -> f32 {
        self.weights[2]
    }

    pub fn blend(self, colors: [u32; 3]) -> u32 {
        let mut result = [0.0; 3];
        for (color, weight) in colors.into_iter().zip(self.weights) {
            for (channel, value) in result.iter_mut().zip(color.to_le_bytes()) {
                *channel += value as f32 * weight;
            }
        }
        rgb(result[0] as u8, result[1] as u8, result[2] as u8)
    }

    pub fn color(self, color: u32) -> u32 {
        self.blend([color, grade(color, 1), grade(color, 2)])
    }

    pub fn haze(self, weather: crate::content::weather::WeatherState) -> u32 {
        self.blend(std::array::from_fn(|theme| weather.haze(theme)))
    }

    pub fn diffuse(self, normal: Vec3) -> f32 {
        0.45 + 0.55 * normal.dot(self.direction).max(0.0)
    }
}
