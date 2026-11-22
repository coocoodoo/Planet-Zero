use super::*;
use crate::world::orbit::smooth;

#[derive(Clone, Copy)]
pub(super) struct Lighting {
    pub sun: Vec3,
    pub direction: Vec3,
    weights: [f32; 3],
    pub synthwave: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self::at(0.125, glam::Quat::IDENTITY)
    }
}

impl Lighting {
    pub fn new(flight: &Flight, world: &World, theme: usize) -> Self {
        let phase = match theme {
            1 => 0.735,
            2 => 0.875,
            _ => 0.1875,
        };
        let clock = phase + flight.day_time / crate::flight::DAY_SECONDS;
        let mut light = Self::at(cycle_phase(clock), flight.sky_rotation);
        let cycle = flight.day_cycle.wrapping_add(clock.floor() as u32);
        light.synthwave = synthwave(clock.fract(), cycle, world.kind, world.seed)
            * smooth(-light.direction.z / 0.04);
        light
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
            synthwave: 0.0,
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
        let normal = self.blend([color, grade(color, 1), grade(color, 2)]);
        if self.synthwave == 0.0 {
            return normal;
        }
        let [r, g, b, _] = color.to_le_bytes().map(f32::from);
        let luma = (r * 0.2126 + g * 0.7152 + b * 0.0722) / 255.0;
        let neon = rgb(
            (8.0 + luma * 78.0 + (r - g) * 0.035) as u8,
            (6.0 + luma * 43.0) as u8,
            (24.0 + luma * 112.0 + (b - r) * 0.035) as u8,
        );
        mix(normal, neon, self.synthwave)
    }

    pub fn haze(self, weather: crate::content::weather::WeatherState) -> u32 {
        let normal = self.blend(std::array::from_fn(|theme| weather.haze(theme)));
        mix(normal, weather.shade(rgb(91, 49, 132)), self.synthwave)
    }

    pub fn diffuse(self, normal: Vec3) -> f32 {
        0.45 + 0.55 * normal.dot(self.direction).max(0.0)
    }
}

fn synthwave(clock: f32, cycle: u32, map: crate::world::MapKind, seed: u32) -> f32 {
    use crate::world::MapKind;
    let threshold = match map {
        MapKind::City => 179,
        MapKind::Icebergs | MapKind::Islands => 77,
        _ => return 0.0,
    };
    let night = seed ^ cycle.wrapping_mul(0x9e37_79b9) ^ 0x735a_11ce;
    if hash(night) >= threshold {
        return 0.0;
    }
    // 日落后三秒开始，十二秒渐入，保持二十四秒，再用二十一秒淡出
    smooth((clock - 0.755) / 0.02) * (1.0 - smooth((clock - 0.815) / 0.035))
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
    fn synthwave_chances_depend_on_map_and_stay_fixed_for_each_night() {
        use crate::world::MapKind;
        for map in MapKind::ALL {
            let count = (0..10000)
                .filter(|cycle| synthwave(0.79, *cycle, map, 7109) > 0.0)
                .count();
            let expected = match map {
                MapKind::City => 7000,
                MapKind::Islands | MapKind::Icebergs => 3000,
                _ => 0,
            };
            assert!(count.abs_diff(expected) < 150, "{map:?}: {count}");
            for cycle in 0..100 {
                let active = synthwave(0.79, cycle, map, 7109) > 0.0;
                for clock in [0.756, 0.77, 0.8, 0.83, 0.849] {
                    assert_eq!(synthwave(clock, cycle, map, 7109) > 0.0, active);
                }
                for clock in [0.0, 0.25, 0.70, 0.75, 0.85, 0.9, 0.95, 0.999] {
                    assert_eq!(synthwave(clock, cycle, map, 7109), 0.0);
                }
            }
        }
    }

    #[test]
    fn synthwave_colors_fade_smoothly_and_leave_sunrise_unchanged() {
        use crate::world::MapKind;
        let cycle = (0..100)
            .find(|cycle| synthwave(0.79, *cycle, MapKind::City, 7109) > 0.0)
            .unwrap();
        let mut previous = None;
        for frame in 0..=18000 {
            let clock = frame as f32 / 18000.0;
            let normal = Lighting::at(cycle_phase(clock), glam::Quat::IDENTITY);
            let mut light = normal;
            light.synthwave =
                synthwave(clock, cycle, MapKind::City, 7109) * smooth(-light.direction.z / 0.04);
            let colors =
                [rgb(238, 220, 164), rgb(246, 250, 255), rgb(42, 122, 54)].map(|c| light.color(c));
            if !(0.755..0.85).contains(&clock) {
                assert_eq!(light.synthwave, 0.0);
                assert_eq!(colors[1], normal.color(rgb(246, 250, 255)));
            }
            if let Some(previous) = previous {
                for (a, b) in colors.into_iter().zip(previous) {
                    for (a, b) in a.to_le_bytes().into_iter().zip(u32::to_le_bytes(b)) {
                        assert!(a.abs_diff(b) <= 2, "frame {frame}: {a} -> {b}");
                    }
                }
            }
            previous = Some(colors);
        }
    }

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
