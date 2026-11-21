use super::*;
use crate::world::orbit::smooth;

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
        // 低角度阳光先转暖，落日后的冷色天光继续照亮场景
        let daylight = smooth((direction.z + 0.45) / 0.49);
        let warmth =
            (1.0 - smooth((direction.z + 0.04) / 0.26)) * smooth((direction.z + 0.28) / 0.20);
        Self {
            sun,
            direction,
            weights: [daylight * (1.0 - warmth), daylight * warmth, 1.0 - daylight],
        }
    }

    pub fn night(self) -> f32 {
        self.weights[2]
    }

    pub fn twilight(self) -> f32 {
        self.weights[1]
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
        let night = (time - 0.75) * 4.0;
        0.5 + night / 6.0 + smooth(night) / 3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sunset_warms_before_the_horizon_and_cools_through_blue_hour() {
        let at = |z: f32| Lighting::from_direction(Vec3::Z, vec3(0.0, 0.0, z));
        assert_eq!(at(0.3).weights[0], 1.0);
        assert!(at(0.15).twilight() > 0.1);
        assert_eq!(at(0.15).night(), 0.0);
        assert_eq!(at(0.04).night(), 0.0);
        assert!(at(0.0).twilight() > 0.9);
        assert!(at(-0.08).twilight() > 0.6);
        let blue = at(-0.28);
        assert_eq!(blue.twilight(), 0.0);
        assert!(blue.night() > 0.5 && blue.night() < 0.85);
        let [r, _, b, _] = blue.color(rgb(230, 230, 230)).to_le_bytes();
        assert!(b > r + 30);
        assert_eq!(at(-0.45).night(), 1.0);
        for time in [0.0_f32, 0.75] {
            let before = (cycle_phase(time) - cycle_phase(time - 0.0001)).rem_euclid(1.0);
            let after = (cycle_phase(time + 0.0001) - cycle_phase(time)).rem_euclid(1.0);
            assert!((before - after).abs() < 0.000001);
        }
    }

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
