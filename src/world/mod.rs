pub mod airspace;
pub use noise::{hash, index};
pub mod celestial;
mod city;
mod clouds;
mod generation;
pub(crate) mod height_bounds;
mod landmarks;
mod loader;
mod map_data;
pub use map_data::MapData;
pub mod network;
mod noise;
pub mod orbit;
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
pub use clouds::{Cloud, CloudShape};
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
    pub(crate) height_bounds: height_bounds::HeightBounds,
    pub airspace: airspace::Airspace,
    pub heights: MapData,
    pub obstacles: Vec<u8>,
    pub ceiling: u8,
    pub materials: MapData,
    pub clouds: Vec<Cloud>,
    pub cloud_levels: Vec<u16>,
    pub cloud_slopes: Vec<[f32; 4]>,
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
    pub(crate) fn mesh_height(&self, x: i32, y: i32) -> f32 {
        self.height_bounds.vertex(x, y)
    }
    pub(crate) fn height_range(&self, x: i32, y: i32, size: i32) -> [u8; 2] {
        self.height_bounds.range(x, y, size)
    }
    pub fn update_ceiling(&mut self) {
        self.height_bounds = height_bounds::HeightBounds::build(&self.heights);
        self.ceiling = self.heights.iter().copied().max().unwrap_or(0);
        self.prepare_cloud_heights();
        self.airspace = airspace::Airspace::build(self);
        self.heights.compact();
        self.materials.compact();
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
}
