use super::*;

#[derive(Default)]
pub(super) struct TerrainCache {
    colors: Option<[u32; MATERIAL_COUNT]>,
    shades: Vec<u32>,
    lod: Vec<u8>,
    dirty_rows: Vec<(usize, usize)>,
    side: i32,
    origin: (i32, i32),
    warp_origin: Vec2,
    warp_x: Vec<Vec3>,
    warp_y: Vec<Vec3>,
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
        let scale = warp.angle_scale();
        for i in 0..=span / 4 {
            let angles = (origin + Vec2::splat((i * 4) as f32) - warp.anchor) * scale;
            self.warp_x
                .push(crate::world::orbit::SurfaceWarp::axis(angles.x));
            self.warp_y
                .push(crate::world::orbit::SurfaceWarp::axis(angles.y));
        }
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
    pub fn topology(&mut self, leaves: &[(i32, i32, i32)], origin: (i32, i32), side: i32) {
        self.origin = origin;
        if self.side != side {
            self.lod.resize((side * side) as usize, 0);
            self.lod.fill(0);
            self.dirty_rows = vec![(side as usize, 0); side as usize];
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
        if x < 0 || y < 0 || x >= self.side || y >= self.side {
            return fallback;
        }
        let level = self.lod[(y * self.side + x) as usize];
        if level == 0 { fallback } else { 1 << level }
    }
}
