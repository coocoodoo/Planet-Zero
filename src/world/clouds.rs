use super::*;
use glam::{Vec2, Vec3, vec2, vec3};
use std::sync::Arc;

#[derive(Clone)]
pub struct Cloud {
    pub center: Vec2,
    pub scale: f32,
    pub rank: u8,
    pub shape: Arc<CloudShape>,
}

pub struct CloudShape {
    pub extent: Vec2,
    pub thin: bool,
    pub faces: [Vec<CloudFace>; 3],
}

pub struct CloudFace {
    pub points: [Vec3; 3],
    pub normal: Vec3,
    pub shade: usize,
}

impl Cloud {
    pub const CELL: f32 = 6.0;
    pub fn extent(&self) -> Vec2 {
        self.shape.extent * self.scale
    }
}

impl CloudShape {
    pub fn new(width: usize, density: &[f32], thin: bool) -> Self {
        let height = density.len() / width;
        let extent = vec2((width - 1) as f32, (height - 1) as f32) * Cloud::CELL * 0.5;
        let faces = std::array::from_fn(|level| {
            let step = if level == 0 { 2 } else { 1 };
            let mut faces = Vec::new();
            let point = |x: usize, y: usize| {
                (vec2(x as f32, y as f32) * Cloud::CELL - extent).extend(density[y * width + x])
            };
            for y in (0..height - 1).step_by(step) {
                for x in (0..width - 1).step_by(step) {
                    let x1 = (x + step).min(width - 1);
                    let y1 = (y + step).min(height - 1);
                    let vertices = [point(x, y), point(x1, y), point(x1, y1), point(x, y1)];
                    for face in [[0, 1, 2], [0, 2, 3]] {
                        let input = face.map(|i| vertices[i]);
                        let mut contour = [Vec3::ZERO; 4];
                        let mut count = 0;
                        for edge in 0..3 {
                            let (a, b) = (input[edge], input[(edge + 1) % 3]);
                            if a.z > 0.0 {
                                contour[count] = a;
                                count += 1;
                            }
                            if (a.z > 0.0) != (b.z > 0.0) {
                                contour[count] = a.lerp(b, a.z / (a.z - b.z)).with_z(0.0);
                                count += 1;
                            }
                        }
                        for i in 1..count.saturating_sub(1) {
                            Self::add_face(
                                &mut faces,
                                [contour[0], contour[i], contour[i + 1]],
                                if thin { 0.12 } else { 1.0 },
                                level.saturating_sub(1) as u8,
                            );
                        }
                    }
                }
            }
            faces.shrink_to_fit();
            faces
        });
        Self {
            extent,
            thin,
            faces,
        }
    }

    fn add_face(faces: &mut Vec<CloudFace>, vertices: [Vec3; 3], thickness: f32, refine: u8) {
        if refine > 0 && vertices.iter().any(|p| p.z == 0.0) {
            let [a, b, c] = vertices;
            let (ab, bc, ca) = ((a + b) * 0.5, (b + c) * 0.5, (c + a) * 0.5);
            for triangle in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
                Self::add_face(faces, triangle, thickness, refine - 1);
            }
            return;
        }
        for top in [true, false] {
            let mut points = vertices.map(|p| {
                // 云底内部平坦，边缘向轮廓收口，顶部随密度隆起
                let height = if top {
                    p.z.max(0.0).sqrt()
                } else {
                    -(p.z / 12.25).clamp(0.0, 1.0) * 2.5
                };
                p.with_z(height * thickness)
            });
            if !top {
                points.swap(0, 2);
            }
            let normal = (points[1] - points[0])
                .cross(points[2] - points[0])
                .normalize();
            let shade = (normal.dot(vec3(-0.55, -0.3, 0.78)) * 0.7 + 0.3).clamp(0.0, 1.0);
            faces.push(CloudFace {
                points,
                normal,
                shade: (shade * 15.0) as usize,
            });
        }
    }
}

impl World {
    pub(super) fn generate_clouds(&mut self, random: &mut Random, clusters: usize) {
        for cluster in 0..clusters {
            let thin = cluster % 4 == 3;
            let mut center = vec2(random.next(), random.next()) * CLOUD_SIZE as f32;
            if thin {
                center = self.clouds.last().unwrap().center
                    + (center / CLOUD_SIZE as f32 - Vec2::splat(0.5)) * vec2(150.0, 100.0);
            }
            // 云团共享 64 种形状，通过位置和大小形成变化
            if cluster >= 64 {
                self.clouds.push(Cloud {
                    center,
                    scale: 0.8 + random.next() * 0.4,
                    rank: ((cluster - usize::from(thin)) & 15) as u8,
                    shape: Arc::clone(&self.clouds[cluster % 64].shape),
                });
                continue;
            }
            let scale = 0.6 + random.next() * random.next() * 1.25;
            let angle = 0.35 + (random.next() - 0.5) * 1.3;
            let length = (12.0 + random.next() * 24.0) * scale * if thin { 1.4 } else { 1.0 };
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
                let prominence = hash((cluster * 7919 + puff * 53) as u32) as f32 / 255.0;
                let thickness = (radius * (0.42 + prominence * 0.5)).clamp(3.0, 15.0);
                puffs.push((point, radius, thickness));
            }
            let width = ((high.x - low.x) / Cloud::CELL).ceil() as usize + 1;
            let height = ((high.y - low.y) / Cloud::CELL).ceil() as usize + 1;
            let mut density = vec![0.0; width * height];
            for y in 0..height {
                for x in 0..width {
                    let point = low + vec2(x as f32, y as f32) * Cloud::CELL;
                    let seed = cluster as u32 * 7919;
                    let noise = cloud_noise(point / 12.0, seed) * 0.7
                        + cloud_noise(point / 5.0, seed ^ 0x73a1) * 0.3;
                    density[y * width + x] = puffs
                        .iter()
                        .map(|&(center, radius, thickness)| {
                            let distance = point.distance_squared(center) / (radius * radius);
                            (1.0 - distance + (noise - 0.5) * 0.65) * thickness.powi(2)
                        })
                        .fold(f32::NEG_INFINITY, f32::max);
                }
            }
            self.clouds.push(Cloud {
                center: center
                    + low
                    + vec2((width - 1) as f32, (height - 1) as f32) * Cloud::CELL * 0.5,
                scale: 1.0,
                rank: ((cluster - usize::from(thin)) & 15) as u8,
                shape: Arc::new(CloudShape::new(width, &density, thin)),
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

fn cloud_noise(point: Vec2, seed: u32) -> f32 {
    let cell = point.floor().as_ivec2();
    let fraction = point - point.floor();
    let t = fraction * fraction * (Vec2::splat(3.0) - fraction * 2.0);
    let value = |x: i32, y: i32| {
        hash(seed ^ (x as u32).wrapping_mul(137) ^ (y as u32).wrapping_mul(881)) as f32 / 255.0
    };
    let (a, b, c, d) = (
        value(cell.x, cell.y),
        value(cell.x + 1, cell.y),
        value(cell.x, cell.y + 1),
        value(cell.x + 1, cell.y + 1),
    );
    a + (b - a) * t.x + (c - a + (d - c - b + a) * t.x) * t.y
}
