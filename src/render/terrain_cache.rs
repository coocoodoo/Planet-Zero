use super::*;

#[derive(Default)]
pub(super) struct TerrainCache {
    colors: Option<[u32; MATERIAL_COUNT]>,
    shades: Vec<u32>,
    lod: Vec<u8>,
    side: i32,
    origin: (i32, i32),
}
impl TerrainCache {
    pub fn colors(&mut self, colors: [u32; MATERIAL_COUNT]) {
        if self.colors == Some(colors) {
            return;
        }
        self.shades.clear();
        for color in colors {
            for shade in 0..=256 {
                self.shades
                    .push(mix(rgb(17, 23, 35), color, shade as f32 / 256.0));
            }
        }
        self.colors = Some(colors);
    }
    pub fn shade(&self, material: usize, light: f32) -> u32 {
        self.shades[material * 257 + (light.clamp(0.0, 1.0) * 256.0) as usize]
    }
    pub fn topology(&mut self, leaves: &[(i32, i32, i32)], origin: (i32, i32), side: i32) {
        self.side = side;
        self.origin = origin;
        self.lod.resize((side * side) as usize, 0);
        self.lod.fill(0);
        for &(x, y, size) in leaves {
            let x = x / 8 - origin.0;
            let y = y / 8 - origin.1;
            let span = size / 8;
            let level = size.ilog2() as u8;
            for row in y..y + span {
                let at = (row * side + x) as usize;
                self.lod[at..at + span as usize].fill(level);
            }
        }
    }
    pub fn neighbor(&self, x: i32, y: i32, fallback: i32) -> i32 {
        let (x, y) = (
            x.div_euclid(8) - self.origin.0,
            y.div_euclid(8) - self.origin.1,
        );
        if x < 0 || y < 0 || x >= self.side || y >= self.side {
            return fallback;
        }
        let level = self.lod[(y * self.side + x) as usize];
        if level == 0 { fallback } else { 1 << level }
    }
}
