pub use noise::{hash, index};
pub mod celestial;
mod generation;
mod loader;
mod noise;
pub mod portal;
#[cfg(test)]
mod tests;
pub use loader::WorldLoader;
use noise::{Random, normalize, octave};
use std::thread::JoinHandle;

pub const SIZE: usize = 4096;
pub const CLOUD_SIZE: usize = 2048;
pub const CLOUD_ALT: usize = 80;
pub const MAX_HEIGHT: usize = 96;
pub const MATERIAL_COUNT: usize = 30;
const NOISE_SIZE: usize = 1024;
const CELL: usize = SIZE / NOISE_SIZE;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MapKind {
    #[default]
    Islands,
    Alpine,
    Canyon,
    Desert,
    Wetlands,
    Volcanic,
    Moon,
    Space,
}

impl MapKind {
    pub const ALL: [Self; 8] = [
        Self::Islands,
        Self::Alpine,
        Self::Canyon,
        Self::Desert,
        Self::Wetlands,
        Self::Volcanic,
        Self::Moon,
        Self::Space,
    ];
    pub fn vacuum(self) -> bool {
        matches!(self, Self::Moon | Self::Space)
    }
    pub fn name(self) -> &'static str {
        &crate::content::maps::definition(self).name
    }
    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }
    pub fn seed(self) -> u32 {
        crate::content::maps::definition(self).seed
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name().eq_ignore_ascii_case(name))
    }
}

pub struct World {
    pub heights: Vec<u8>,
    pub materials: Vec<u8>,
    pub clouds: Vec<u8>,
    pub spawn: (f32, f32),
    pub kind: MapKind,
    pub seed: u32,
    pub scenery: Vec<celestial::Scenery>,
    pub portals: Vec<portal::Portal>,
}

impl World {
    pub fn new(seed: u32) -> Self {
        Self::generate(MapKind::Islands, seed)
    }
    pub fn height(&self, x: f32, y: f32) -> f32 {
        self.heights[index(x.floor() as i32, y.floor() as i32, SIZE)] as f32
    }
    pub fn cloud(&self, x: f32, y: f32, time: f32) -> usize {
        let cell = self.clouds[index(
            (x - time * 7.0).floor() as i32,
            (y - time * 3.0).floor() as i32,
            CLOUD_SIZE,
        )];
        if cell >> 4 < 7 {
            (cell & 15) as usize
        } else {
            0
        }
    }
}
