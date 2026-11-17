use super::*;

impl World {
    pub(super) fn generate_clouds(&mut self, random: &mut Random, clusters: usize) {
        for cluster in 0..clusters {
            let rank = (cluster & 15) as u8;
            let cx = random.next() * CLOUD_SIZE as f32;
            let cy = random.next() * CLOUD_SIZE as f32;
            let scale = 0.6 + random.next() * random.next() * 1.25;
            let angle = 0.35 + (random.next() - 0.5) * 1.3;
            let length = (12.0 + random.next() * 24.0) * scale;
            let puffs = 4 + (random.next() * 7.0) as i32;
            for puff in 0..puffs {
                let t = puff as f32 / (puffs - 1) as f32 * 2.0 - 1.0;
                let px = cx + angle.cos() * t * length + (random.next() - 0.5) * 9.0 * scale;
                let py = cy + angle.sin() * t * length + (random.next() - 0.5) * 9.0 * scale;
                let r = (6.0 + random.next() * 8.0) * scale * (1.0 - 0.45 * t.abs());
                let thickness = (r * 0.6).round().clamp(2.0, 8.0);
                let extent = r.ceil() as i32;
                for dy in -extent..=extent {
                    for dx in -extent..=extent {
                        let distance = (dx * dx + dy * dy) as f32 / (r * r);
                        if distance > 1.0 {
                            continue;
                        }
                        let height = ((1.0 - distance).sqrt() * thickness).round().max(1.0) as u8;
                        let at = index(px.round() as i32 + dx, py.round() as i32 + dy, CLOUD_SIZE);
                        if height > self.clouds[at] & 15 {
                            self.clouds[at] = (rank << 4) | height;
                        }
                    }
                }
            }
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
    }
    pub fn cloud_height(&self, x: f32, y: f32) -> f32 {
        if self.cloud_levels.is_empty() {
            return 110.0;
        }
        let (gx, gy) = ((x / 128.0).floor() as i32, (y / 128.0).floor() as i32);
        let (tx, ty) = ((x / 128.0).rem_euclid(1.0), (y / 128.0).rem_euclid(1.0));
        let h = |dx, dy| self.cloud_levels[index(gx + dx, gy + dy, 32)] as f32;
        let (a, b, c, d) = (h(0, 0), h(1, 0), h(0, 1), h(1, 1));
        a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty
    }
}
