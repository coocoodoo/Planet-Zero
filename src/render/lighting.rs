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
            1 => 0.735,
            2 => 0.875,
            _ => 0.1875,
        };
        Self::at(
            cycle_phase(phase + flight.day_time / crate::flight::DAY_SECONDS),
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

fn cycle_phase(time: f32) -> f32 {
    let time = time.rem_euclid(1.0);
    if time < 0.75 {
        time * (2.0 / 3.0)
    } else {
        0.5 + (time - 0.75) * 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_lasts_450_seconds_and_night_lasts_150_seconds() {
        let mut day = 0;
        let mut previous = Lighting::at(cycle_phase(0.0), glam::Quat::IDENTITY);
        for frame in 1..=18000 {
            let time = (frame as f32 - 0.5) / (30.0 * crate::flight::DAY_SECONDS);
            let light = Lighting::at(cycle_phase(time), glam::Quat::IDENTITY);
            day += usize::from(light.direction.z > 0.0);
            for (a, b) in previous
                .color(rgb(246, 250, 255))
                .to_le_bytes()
                .into_iter()
                .zip(light.color(rgb(246, 250, 255)).to_le_bytes())
            {
                assert!(a.abs_diff(b) <= 1);
            }
            previous = light;
        }
        assert_eq!(day, 450 * 30);
        assert_eq!(18000 - day, 150 * 30);
        let start = Lighting::at(cycle_phase(0.0), glam::Quat::IDENTITY);
        assert!(start.sun.distance(previous.sun) < 0.001);
    }

    #[test]
    fn dusk_colors_change_by_at_most_one_channel_level_per_frame() {
        let mut previous = Lighting::at(0.45, glam::Quat::IDENTITY).color(rgb(246, 250, 255));
        for frame in 1..=2700 {
            let light = Lighting::at(0.45 + frame as f32 / 27000.0, glam::Quat::IDENTITY);
            let color = light.color(rgb(246, 250, 255));
            for (a, b) in previous.to_le_bytes().into_iter().zip(color.to_le_bytes()) {
                assert!(
                    a.abs_diff(b) <= 1,
                    "frame {frame}: {previous:08x} -> {color:08x}"
                );
            }
            previous = color;
        }
    }
}
