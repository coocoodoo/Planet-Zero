use super::*;

impl Renderer {
    pub(super) fn synthwave_glow(&mut self) {
        let radius = (3 * self.full_width / 320).clamp(2, 7) as i32;
        let mut kernel = Vec::new();
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                let weight = (1.0 - distance / (radius as f32 + 1.0)).max(0.0).powi(2) * 0.16;
                if weight > 0.005 {
                    kernel.push((dx, dy, weight));
                }
            }
        }
        self.bloom.resize(self.pixels.len(), [0; 3]);
        self.bloom.fill([0; 3]);
        for x in 0..self.w {
            for y in span(self.visible[x].0, self.visible[x].1, self.h) {
                let at = x * self.h + y;
                if !self.depth[at].is_finite() {
                    continue;
                }
                let [r, g, b, _] = self.pixels[at].to_le_bytes();
                let peak = r.max(g).max(b);
                if peak < 175 || peak - r.min(g).min(b) < 45 {
                    continue;
                }
                let gain = (peak as f32 - 175.0) / 80.0 * self.light.synthwave;
                for &(dx, dy, weight) in &kernel {
                    let px = x as i32 + dx;
                    let py = y as i32 + dy;
                    if px < 0 || py < 0 || px >= self.w as i32 || py >= self.h as i32 {
                        continue;
                    }
                    let cell = &mut self.bloom[px as usize * self.h + py as usize];
                    for (channel, value) in cell.iter_mut().zip([r, g, b]) {
                        *channel += (value as f32 * weight * gain) as u16;
                    }
                }
            }
        }
        for (pixel, glow) in self.pixels.iter_mut().zip(&self.bloom) {
            let [r, g, b, _] = pixel.to_le_bytes();
            *pixel = rgb(
                (r as u16 + glow[0]).min(255) as u8,
                (g as u16 + glow[1]).min(255) as u8,
                (b as u16 + glow[2]).min(255) as u8,
            );
        }
    }
}
