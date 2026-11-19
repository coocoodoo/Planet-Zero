use super::*;
use glam::{Vec2, vec2};

#[derive(Clone)]
pub struct Cloud {
    pub center: Vec2,
    pub width: usize,
    pub heights: Vec<u8>,
    pub rank: u8,
}

impl Cloud {
    pub const CELL: f32 = 6.0;
    pub fn extent(&self) -> Vec2 {
        vec2(
            (self.width - 1) as f32,
            (self.heights.len() / self.width - 1) as f32,
        ) * Self::CELL
            * 0.5
    }
}

impl World {
    pub(super) fn generate_clouds(&mut self, random: &mut Random, clusters: usize) {
        for cluster in 0..clusters {
            let center = vec2(random.next(), random.next()) * CLOUD_SIZE as f32;
            let scale = 0.6 + random.next() * random.next() * 1.25;
            let angle = 0.35 + (random.next() - 0.5) * 1.3;
            let length = (12.0 + random.next() * 24.0) * scale;
            let count = 4 + (random.next() * 7.0) as usize;
            let mut puffs = Vec::with_capacity(count);
            let mut low = Vec2::splat(f32::INFINITY);
            let mut high = Vec2::splat(f32::NEG_INFINITY);
            for puff in 0..count {
                let t = puff as f32 / (count - 1) as f32 * 2.0 - 1.0;
                let point = vec2(angle.cos(), angle.sin()) * t * length
                    + vec2(random.next() - 0.5, random.next() - 0.5) * 9.0 * scale;
                let radius = (6.0 + random.next() * 8.0) * scale * (1.0 - 0.45 * t.abs());
                low = low.min(point - Vec2::splat(radius + Cloud::CELL));
                high = high.max(point + Vec2::splat(radius + Cloud::CELL));
                puffs.push((point, radius));
            }
            let width = ((high.x - low.x) / Cloud::CELL).ceil() as usize + 1;
            let height = ((high.y - low.y) / Cloud::CELL).ceil() as usize + 1;
            let mut heights = vec![0; width * height];
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let point = low + vec2(x as f32, y as f32) * Cloud::CELL;
                    let noise = hash((cluster * 7919 + x * 137 + y * 881) as u32) as f32 / 255.0;
                    let thickness = puffs
                        .iter()
                        .map(|&(center, radius)| {
                            let distance = point.distance(center) / radius;
                            let profile =
                                (1.0 - distance * distance + (noise - 0.5) * 0.24).max(0.0);
                            (profile.sqrt() * (radius * 0.5).clamp(2.0, 10.0)
                                + (noise - 0.5) * profile * 1.4)
                                .max(0.0)
                        })
                        .fold(0.0, f32::max);
                    heights[y * width + x] = thickness.round() as u8;
                }
            }
            self.clouds.push(Cloud {
                center: center
                    + low
                    + vec2((width - 1) as f32, (height - 1) as f32) * Cloud::CELL * 0.5,
                width,
                heights,
                rank: (cluster & 15) as u8,
            });
        }
    }
    pub(super) fn prepare_cloud_heights(&mut self) {
        if self.kind.vacuum() {
            return;
        }
        self.cloud_levels = vec![0; 32 * 32];
        for y in 0..32 {
            for x in 0..32 {
                let mut heights = Vec::with_capacity(25);
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        heights
                            .push(self.heights[index(x * 128 + dx * 64, y * 128 + dy * 64, SIZE)]);
                    }
                }
                heights.sort_unstable();
                self.cloud_levels[(y * 32 + x) as usize] = heights[8].min(100) as u16 + 110;
            }
        }
        self.cloud_slopes = (0..32 * 32)
            .map(|at| {
                let (x, y) = (at % 32, at / 32);
                let h = |dx, dy| self.cloud_levels[index(x + dx, y + dy, 32)] as f32;
                let (a, b, c, d) = (h(0, 0), h(1, 0), h(0, 1), h(1, 1));
                [a, b - a, c - a, d - c - b + a]
            })
            .collect();
    }
    pub fn cloud_height(&self, x: f32, y: f32) -> f32 {
        if self.cloud_levels.is_empty() {
            return 110.0;
        }
        let (x, y) = (x / 128.0, y / 128.0);
        let (fx, fy) = (x.floor(), y.floor());
        let (gx, gy) = (fx as i32, fy as i32);
        let (tx, ty) = (x - fx, y - fy);
        let [a, b, c, d] = self.cloud_slopes[index(gx, gy, 32)];
        a + b * tx + (c + d * tx) * ty
    }
}
