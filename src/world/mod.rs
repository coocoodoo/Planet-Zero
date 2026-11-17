pub mod airspace;
pub use noise::{hash, index};
pub mod celestial;
mod city;
mod clouds;
mod generation;
mod landmarks;
mod loader;
pub mod network;
mod noise;
pub mod portal;
mod regions;
pub mod regions_detail;
#[cfg(test)]
mod tests;
pub use loader::WorldLoader;
use noise::{Random, normalize, octave};
use std::thread::JoinHandle;

pub const SIZE: usize = 4096;
pub const CLOUD_SIZE: usize = 2048;
pub const MAX_HEIGHT: usize = 255;
pub const MATERIAL_COUNT: usize = 35;
const NOISE_SIZE: usize = 1024;
const CELL: usize = SIZE / NOISE_SIZE;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
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
    Fjords,
    Icebergs,
    City,
    Highlands,
}

impl MapKind {
    pub const ALL: [Self; 12] = [
        Self::Islands,
        Self::Alpine,
        Self::Canyon,
        Self::Desert,
        Self::Wetlands,
        Self::Volcanic,
        Self::Moon,
        Self::Space,
        Self::Fjords,
        Self::Icebergs,
        Self::City,
        Self::Highlands,
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

#[derive(Clone)]
pub struct World {
    pub airspace: airspace::Airspace,
    pub heights: Vec<u8>,
    pub obstacles: Vec<u8>,
    pub ceiling: u8,
    pub materials: Vec<u8>,
    pub clouds: Vec<u8>,
    pub cloud_levels: Vec<u16>,
    pub spawn: (f32, f32),
    pub kind: MapKind,
    pub seed: u32,
    pub scenery: Vec<celestial::Scenery>,
    pub caverns: Vec<regions_detail::Cavern>,
    pub portals: Vec<portal::Portal>,
}

impl World {
    pub fn new(seed: u32) -> Self {
        Self::generate(MapKind::Islands, seed)
    }
    pub fn update_ceiling(&mut self) {
        self.ceiling = self.heights.iter().copied().max().unwrap_or(0);
        self.prepare_cloud_heights();
        self.airspace = airspace::Airspace::build(self);
    }
    pub fn height(&self, x: f32, y: f32) -> f32 {
        if self.kind == MapKind::Space {
            return 0.0;
        }
        let ground = self.heights[index(x.floor() as i32, y.floor() as i32, SIZE)];
        let obstacle = if self.obstacles.is_empty() {
            0
        } else {
            self.obstacles[index((x / 16.0).floor() as i32, (y / 16.0).floor() as i32, 256)]
        };
        self.caverns
            .iter()
            .filter_map(|c| c.roof(x, y))
            .map(|h| h + 12.0)
            .fold(ground.max(obstacle) as f32, f32::max)
    }
    pub fn cloud(&self, x: f32, y: f32, time: f32) -> usize {
        self.cloud_cover(x, y, time, 7)
    }
    pub fn cloud_cover(&self, x: f32, y: f32, time: f32, coverage: u8) -> usize {
        if self.kind.vacuum() {
            return 0;
        }
        let cell = self.clouds[index(
            (x - time * 7.0).floor() as i32,
            (y - time * 3.0).floor() as i32,
            CLOUD_SIZE,
        )];
        if cell & 15 == 0 || cell >> 4 >= coverage {
            return 0;
        }
        (cell & 15) as usize
    }
}
