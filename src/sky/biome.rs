//! Planet biomes: which engine terrain archetype a planet uses, how its sky, fog and ground are
//! tinted, what environmental hazard it has and what its minerals yield.
use super::inventory::Item;
use super::rng::Rng;
use crate::render::{Grade, rgb};
use crate::world::MapKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Lush,
    Desert,
    Frozen,
    Toxic,
    Volcanic,
    Exotic,
    /// Airless moon (always the engine's Moon body)
    Barren,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hazard {
    Safe,
    Cold,
    Heat,
    Toxic,
    Radiation,
}
impl Hazard {
    pub fn name(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Cold => "COLD",
            Self::Heat => "HEAT",
            Self::Toxic => "TOXIC",
            Self::Radiation => "RADIATION",
        }
    }
}

impl Biome {
    pub const ATMOSPHERIC: [Self; 6] = [
        Self::Lush,
        Self::Desert,
        Self::Frozen,
        Self::Toxic,
        Self::Volcanic,
        Self::Exotic,
    ];
    pub const ALL: [Self; 7] = [
        Self::Lush,
        Self::Desert,
        Self::Frozen,
        Self::Toxic,
        Self::Volcanic,
        Self::Exotic,
        Self::Barren,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Lush => "LUSH",
            Self::Desert => "DESERT",
            Self::Frozen => "FROZEN",
            Self::Toxic => "TOXIC",
            Self::Volcanic => "VOLCANIC",
            Self::Exotic => "EXOTIC",
            Self::Barren => "BARREN MOON",
        }
    }
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(i: usize) -> Self {
        Self::ALL[i.min(6)]
    }
    /// Engine terrain archetype. The planet seed varies the actual terrain.
    pub fn map_kind(self, seed: u32) -> MapKind {
        let choices: &[MapKind] = match self {
            Self::Lush => &[MapKind::Islands, MapKind::Highlands, MapKind::Wetlands],
            Self::Desert => &[MapKind::Desert, MapKind::Canyon],
            Self::Frozen => &[MapKind::Icebergs, MapKind::Alpine, MapKind::Fjords],
            Self::Toxic => &[MapKind::Wetlands, MapKind::Canyon],
            Self::Volcanic => &[MapKind::Volcanic],
            Self::Exotic => &[MapKind::City, MapKind::Highlands],
            Self::Barren => &[MapKind::Moon],
        };
        choices[(seed as usize >> 3) % choices.len()]
    }
    pub fn hazard(self) -> (Hazard, f32) {
        match self {
            Self::Lush => (Hazard::Safe, 0.0),
            Self::Desert => (Hazard::Heat, 0.6),
            Self::Frozen => (Hazard::Cold, 0.7),
            Self::Toxic => (Hazard::Toxic, 0.8),
            Self::Volcanic => (Hazard::Heat, 1.0),
            Self::Exotic => (Hazard::Radiation, 0.7),
            Self::Barren => (Hazard::Radiation, 0.5),
        }
    }
    /// Element that this biome's crystals and deposits yield.
    pub fn mineral(self) -> Item {
        match self {
            Self::Lush | Self::Frozen => Item::Oxygen,
            Self::Desert | Self::Toxic | Self::Volcanic => Item::Sodium,
            Self::Exotic | Self::Barren => Item::Tritium,
        }
    }
    /// Relative weights of flora, rocks and crystals on the surface, and props per chunk.
    pub fn props(self) -> ([u32; 3], u32) {
        match self {
            Self::Lush => ([7, 3, 1], 7),
            Self::Desert => ([2, 6, 2], 4),
            Self::Frozen => ([1, 5, 3], 4),
            Self::Toxic => ([6, 2, 3], 6),
            Self::Volcanic => ([1, 6, 3], 4),
            Self::Exotic => ([3, 2, 5], 5),
            Self::Barren => ([0, 6, 3], 3),
        }
    }
    /// Flora, rock and crystal colours, varied per planet.
    pub fn palette(self, seed: u32) -> [u32; 3] {
        let mut rng = Rng::new(seed as u64 ^ 0x9a1e);
        let v = |rng: &mut Rng, c: [u8; 3]| {
            let k = rng.range(0.82, 1.15);
            rgb(
                (c[0] as f32 * k).min(255.0) as u8,
                (c[1] as f32 * k).min(255.0) as u8,
                (c[2] as f32 * k).min(255.0) as u8,
            )
        };
        let flora = [
            [62, 168, 74],
            [218, 128, 52],
            [120, 196, 220],
            [186, 214, 58],
            [214, 70, 40],
            [210, 90, 230],
            [150, 150, 160],
        ][self.index()];
        let flora = if rng.below(3) == 0 {
            [flora[1], flora[0], flora[2]]
        } else {
            flora
        };
        let rock = [
            [120, 116, 104],
            [178, 132, 86],
            [170, 184, 200],
            [96, 110, 70],
            [62, 52, 56],
            [92, 84, 132],
            [140, 136, 132],
        ][self.index()];
        let crystal = match self.mineral() {
            Item::Oxygen => [255, 96, 90],
            Item::Sodium => [255, 200, 70],
            _ => [90, 240, 255],
        };
        [v(&mut rng, flora), v(&mut rng, rock), v(&mut rng, crystal)]
    }
    /// Colour grade for the 3D view on this planet: sky, fog and ground tint.
    pub fn grade(self, seed: u32) -> Grade {
        let mut rng = Rng::new(seed as u64 ^ 0x6ade);
        let jitter = rng.range(-0.06, 0.06);
        let (matrix, offset) = match self {
            Self::Lush => (
                [
                    [0.98 + jitter, 0.04, 0.0],
                    [0.0, 1.0, 0.02],
                    [0.0, 0.04, 1.0 - jitter],
                ],
                [0.0, 2.0, 0.0],
            ),
            Self::Desert => (
                [
                    [1.06, 0.04, 0.0],
                    [0.02, 0.97, 0.0],
                    [0.0, 0.02, 0.86 + jitter],
                ],
                [6.0, 2.0, 0.0],
            ),
            Self::Frozen => (
                [[0.9, 0.0, 0.06], [0.0, 0.98, 0.06], [0.02, 0.04, 1.08]],
                [0.0, 4.0, 10.0],
            ),
            Self::Toxic => (
                [
                    [0.72 + jitter, 0.2, 0.0],
                    [0.18, 0.96, 0.08],
                    [0.0, 0.12, 0.5],
                ],
                [8.0, 18.0, 0.0],
            ),
            Self::Volcanic => (
                [[1.12, 0.06, 0.0], [0.04, 0.78, 0.0], [0.02, 0.0, 0.72]],
                [12.0, 0.0, 0.0],
            ),
            Self::Exotic => (
                [
                    [0.62, 0.0, 0.42],
                    [0.18, 0.58 + jitter, 0.22],
                    [0.12, 0.44, 0.82],
                ],
                [22.0, 0.0, 34.0],
            ),
            Self::Barren => match rng.below(3) {
                0 => (
                    [[1.08, 0.04, 0.0], [0.0, 0.88, 0.0], [0.0, 0.0, 0.74]],
                    [8.0, 0.0, 0.0],
                ),
                1 => (
                    [[0.86, 0.0, 0.06], [0.0, 0.94, 0.06], [0.0, 0.04, 1.12]],
                    [0.0, 0.0, 8.0],
                ),
                _ => (
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    [0.0; 3],
                ),
            },
        };
        Grade { matrix, offset }
    }
}

/// Multiply two grades: `outer` is applied after `inner`.
pub fn compose(outer: &Grade, inner: &Grade) -> Grade {
    let mut out = Grade::IDENTITY;
    for r in 0..3 {
        for c in 0..3 {
            out.matrix[r][c] = (0..3)
                .map(|k| outer.matrix[r][k] * inner.matrix[k][c])
                .sum();
        }
        out.offset[r] = (0..3)
            .map(|k| outer.matrix[r][k] * inner.offset[k])
            .sum::<f32>()
            + outer.offset[r];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn biomes_map_to_valid_engine_maps_and_only_moons_are_airless() {
        for biome in Biome::ALL {
            for seed in 0..64 {
                let kind = biome.map_kind(seed);
                assert_eq!(kind.vacuum(), biome == Biome::Barren, "{biome:?}");
                assert_ne!(kind, MapKind::Space);
            }
        }
    }
    #[test]
    fn composed_grade_equals_applying_both() {
        let a = Biome::Toxic.grade(3);
        let b = Biome::Exotic.grade(9);
        let c = rgb(120, 80, 200);
        let both = compose(&b, &a).apply(c);
        let step = b.apply(a.apply(c));
        let [r1, g1, b1, _] = both.to_le_bytes();
        let [r2, g2, b2, _] = step.to_le_bytes();
        assert!(r1.abs_diff(r2) <= 2 && g1.abs_diff(g2) <= 2 && b1.abs_diff(b2) <= 2);
    }
}
