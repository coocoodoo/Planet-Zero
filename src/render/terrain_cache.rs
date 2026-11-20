use super::*;

#[derive(Default)]
pub(super) struct TerrainCache {
    pub altitude_fade: f32,
    colors: Option<[u32; MATERIAL_COUNT]>,
    shades: Vec<u32>,
    lod: Vec<u8>,
    dirty_rows: Vec<(usize, usize)>,
    side: i32,
    origin: (i32, i32),
    warp_origin: Vec2,
    warp_x: Vec<Vec3>,
    warp_y: Vec<Vec3>,
    polar_x: Vec<Vec2>,
    polar_y: Vec<Vec3>,
}

impl TerrainCache {
    pub fn prepare_warp(
        &mut self,
        warp: crate::world::orbit::SurfaceWarp,
        origin: Vec2,
        span: i32,
    ) {
        self.warp_origin = origin;
        self.warp_x.clear();
        self.warp_y.clear();
        self.polar_x.clear();
        self.polar_y.clear();
        let scale = warp.angle_scale();
        for i in 0..=span / 4 {
            let angles = (origin + Vec2::splat((i * 4) as f32) - warp.anchor) * scale;
            self.warp_x
                .push(crate::world::orbit::SurfaceWarp::axis(angles.x));
            self.warp_y
                .push(crate::world::orbit::SurfaceWarp::axis(angles.y));
            let uv = origin + Vec2::splat((i * 4) as f32);
            let longitude = (uv.x - warp.origin.x) / crate::world::orbit::UV_SCALE;
            let (s, c) = longitude.sin_cos();
            self.polar_x.push(vec2(c, s));
            self.polar_y.push(warp.polar_row(uv.y));
        }
    }
    pub fn polar_sample(&self, warp: crate::world::orbit::SurfaceWarp, uv: Vec2) -> Vec3 {
        let at = (uv - self.warp_origin) * 0.25;
        if at.x.fract() != 0.0 || at.y.fract() != 0.0 {
            let (sample, weight) = warp.polar_sample(uv);
            return sample.extend(weight);
        }
        let row = self.polar_y[at.y as usize];
        let sample = vec2(warp.origin.x, row.y) + self.polar_x[at.x as usize] * row.x;
        sample.extend(row.z)
    }
    pub fn terrain_height(
        &self,
        warp: crate::world::orbit::SurfaceWarp,
        world: &World,
        uv: Vec2,
    ) -> f32 {
        let height = world.mesh_height(uv.x as i32, uv.y as i32);
        let sample = self.polar_sample(warp, uv);
        if sample.z == 0.0 {
            return height;
        }
        let cap = world.mesh_height(sample.x as i32, sample.y as i32);
        height + (cap - height) * sample.z
    }
    pub fn warped_point(&self, warp: crate::world::orbit::SurfaceWarp, point: Vec3) -> Vec3 {
        let at = ((point.truncate() - self.warp_origin) * 0.25).as_ivec2();
        warp.point_with_axes(
            point,
            self.warp_x[at.x as usize],
            self.warp_y[at.y as usize],
        )
    }
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
    pub fn topology(&mut self, leaves: &[(i32, i32, i32)]) {
        let Some(&(x, y, size)) = leaves.first() else {
            return;
        };
        let bounds = leaves
            .iter()
            .fold([x, y, x + size, y + size], |b, &(x, y, size)| {
                [
                    b[0].min(x),
                    b[1].min(y),
                    b[2].max(x + size),
                    b[3].max(y + size),
                ]
            });
        let origin = (bounds[0] / 8, bounds[1] / 8);
        let side = (bounds[2] - bounds[0]) / 8;
        let height = (bounds[3] - bounds[1]) as usize / 8;
        self.origin = origin;
        if self.side != side || self.dirty_rows.len() != height {
            self.lod.resize(side as usize * height, 0);
            self.lod.fill(0);
            self.dirty_rows = vec![(side as usize, 0); height];
        } else {
            for (row, (start, end)) in self.dirty_rows.iter_mut().enumerate() {
                if *start < *end {
                    let at = row * side as usize;
                    self.lod[at + *start..at + *end].fill(0);
                }
                (*start, *end) = (side as usize, 0);
            }
        }
        self.side = side;
        for &(x, y, size) in leaves {
            let x = x / 8 - origin.0;
            let y = y / 8 - origin.1;
            let span = size / 8;
            let level = size.ilog2() as u8;
            for row in y..y + span {
                let at = (row * side + x) as usize;
                self.lod[at..at + span as usize].fill(level);
                let dirty = &mut self.dirty_rows[row as usize];
                dirty.0 = dirty.0.min(x as usize);
                dirty.1 = dirty.1.max((x + span) as usize);
            }
        }
    }
    pub fn neighbor(&self, x: i32, y: i32, fallback: i32) -> i32 {
        let (x, y) = (
            x.div_euclid(8) - self.origin.0,
            y.div_euclid(8) - self.origin.1,
        );
        if x < 0 || y < 0 || x >= self.side || y >= self.dirty_rows.len() as i32 {
            return fallback;
        }
        let level = self.lod[(y * self.side + x) as usize];
        if level == 0 { fallback } else { 1 << level }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::orbit::{Body, SurfaceWarp};

    #[test]
    fn polar_cache_matches_projection_at_grid_and_clipped_vertices() {
        let origin = vec2(2073.25, 1919.5);
        let anchor = origin + vec2(300.0, 980.0);
        let mut cache = TerrainCache::default();
        for body in Body::ALL {
            for bend in [0.0, 0.4, 1.0] {
                let warp = SurfaceWarp::new(body, origin, anchor, bend);
                cache.prepare_warp(warp, Vec2::ZERO, SIZE as i32);
                for x in [0.0, 104.0, 2073.25, 4096.0] {
                    for y in [0.0, 896.0, 1919.5 + 1024.0, 3072.0, 4096.0] {
                        let uv = vec2(x, y);
                        let (sample, weight) = warp.polar_sample(uv);
                        assert_eq!(cache.polar_sample(warp, uv), sample.extend(weight));
                    }
                }
            }
        }
    }
}
