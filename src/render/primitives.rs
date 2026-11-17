use super::*;

pub(super) struct TriangleColumns {
    points: [Vec2; 3],
    slopes: [f32; 3],
}
impl TriangleColumns {
    pub fn new(mut points: [Vec2; 3]) -> Self {
        points.sort_by(|a, b| a.x.total_cmp(&b.x));
        let slopes = [(0, 2), (0, 1), (1, 2)].map(|(a, b)| {
            let edge = points[b] - points[a];
            if edge.x.abs() > 0.000001 {
                edge.y / edge.x
            } else {
                0.0
            }
        });
        Self { points, slopes }
    }
    pub fn rows(&self, x: f32) -> (i32, i32) {
        let [a, b, _] = self.points;
        let long = a.y + (x - a.x) * self.slopes[0];
        let short = if x < b.x {
            a.y + (x - a.x) * self.slopes[1]
        } else {
            b.y + (x - b.x) * self.slopes[2]
        };
        (
            long.min(short).floor() as i32 - 1,
            long.max(short).ceil() as i32 + 2,
        )
    }
}

impl Renderer {
    pub(super) fn world_line(&mut self, camera: &Camera, a: Vec3, b: Vec3, color: u32) {
        let (mut a, mut b) = self
            .warp
            .map_or((a, b), |warp| (warp.point(a), warp.point(b)));
        if let Some((p, n)) = self.clip_plane {
            let da = (a - p).dot(n);
            let db = (b - p).dot(n);
            if da < 0.0 && db < 0.0 {
                return;
            }
            if da < 0.0 {
                a = a.lerp(b, da / (da - db));
            } else if db < 0.0 {
                b = a.lerp(b, da / (da - db));
            }
        }
        if let (Some((a, da, _)), Some((b, db, _))) = (camera.project(a), camera.project(b)) {
            if a.distance(b) > self.full_width.max(1200) as f32 * 2.0 {
                return;
            }
            let steps = a
                .distance(b)
                .ceil()
                .clamp(1.0, self.full_width.max(400) as f32 * 2.0) as usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                self.point(a.lerp(b, t), da + (db - da) * t, color, 0, 0);
            }
        }
    }
    pub(super) fn triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32) {
        self.surface(camera, points, color, false);
    }
    pub(super) fn rock_triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32) {
        self.surface(camera, points, color, true);
    }
    fn surface(&mut self, camera: &Camera, points: [Vec3; 3], color: u32, stone: bool) {
        let points = points.map(|point| self.warp.map_or(point, |warp| warp.point(point)));
        let forward = camera.forward;
        let near = camera.position
            + forward
                * if camera.q > 0.00001 {
                    -0.879 / camera.q
                } else {
                    -1e8
                };
        if points.iter().all(|p| {
            (*p - near).dot(forward) >= 0.0
                && self
                    .clip_plane
                    .is_none_or(|(origin, normal)| (*p - origin).dot(normal) >= 0.0)
        }) {
            self.raster_triangle(camera, points, color, stone);
            return;
        }
        let mut polygon = points.to_vec();
        for (origin, normal) in std::iter::once((near, forward)).chain(self.clip_plane) {
            let mut clipped = Vec::with_capacity(5);
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                let da = (a - origin).dot(normal);
                let db = (b - origin).dot(normal);
                if da >= 0.0 {
                    clipped.push(a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    clipped.push(a.lerp(b, da / (da - db)));
                }
            }
            polygon = clipped;
        }
        for i in 1..polygon.len().saturating_sub(1) {
            self.raster_triangle(
                camera,
                [polygon[0], polygon[i], polygon[i + 1]],
                color,
                stone,
            );
        }
    }
    fn raster_triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32, stone: bool) {
        let [Some((a, da, _)), Some((b, db, _)), Some((c, dc, _))] =
            points.map(|p| camera.project(p))
        else {
            return;
        };
        let area = (b - a).perp_dot(c - a);
        if area.abs() < 0.1 {
            return;
        }
        let min = a.min(b).min(c).max(Vec2::ZERO);
        let max = a
            .max(b)
            .max(c)
            .min(vec2(self.w as f32 - 1.0, self.h as f32 - 1.0));
        let inverse = [da, db, dc].map(|d| 1.0 / (1.0 + camera.q * d));
        let inverse_area = 1.0 / area;
        let du = (c.x - b.x) * inverse_area;
        let dv = (a.x - c.x) * inverse_area;
        let inverse_q = if camera.q > 0.00001 {
            1.0 / camera.q
        } else {
            0.0
        };
        let stone_colors: [u32; 16] = if stone {
            std::array::from_fn(|i| {
                mix(
                    color,
                    if i < 8 {
                        rgb(20, 26, 29)
                    } else {
                        rgb(206, 184, 144)
                    },
                    (i as f32 - 7.0).abs() * 0.055,
                )
            })
        } else {
            [color; 16]
        };
        let columns = TriangleColumns::new([a, b, c]);
        for x in min.x as i32..=max.x as i32 {
            let (top, bottom) = columns.rows(x as f32);
            let y0 = (min.y as i32).max(self.visible[x as usize].0).max(top);
            let y1 = (max.y as i32 + 1)
                .min(self.visible[x as usize].1)
                .min(bottom);
            let start = vec2(x as f32, y0 as f32);
            let mut u_next = (b - start).perp_dot(c - start) * inverse_area;
            let mut v_next = (c - start).perp_dot(a - start) * inverse_area;
            for y in y0..y1 {
                let (u, v) = (u_next, v_next);
                u_next += du;
                v_next += dv;
                let w = 1.0 - u - v;
                if u >= 0.0 && v >= 0.0 && w >= 0.0 {
                    let depth = if camera.q > 0.00001 {
                        (1.0 / (u * inverse[0] + v * inverse[1] + w * inverse[2]) - 1.0) * inverse_q
                    } else {
                        u * da + v * db + w * dc
                    };
                    let at = x as usize * self.h + y as usize;
                    if depth > self.depth[at] + 1.0
                        || self
                            .clip_depth
                            .get(at)
                            .is_some_and(|near| depth < *near - 1.0)
                    {
                        continue;
                    }
                    let shaded = if stone {
                        let weights = vec3(u * inverse[0], v * inverse[1], w * inverse[2]);
                        let p =
                            (points[0] * weights.x + points[1] * weights.y + points[2] * weights.z)
                                / weights.element_sum();
                        stone_colors[super::geology::stone_texel(p)]
                    } else {
                        color
                    };
                    self.pixels[at] = shaded;
                    self.mask[at] = 1;
                    self.depth[at] = self.depth[at].min(depth);
                }
            }
        }
    }
    pub(super) fn stroke(&mut self, a: Vec2, b: Vec2, depth: f32, color: u32) {
        let steps = a
            .distance(b)
            .ceil()
            .clamp(1.0, self.full_width.max(48) as f32) as usize;
        for step in 0..=steps {
            self.point(a.lerp(b, step as f32 / steps as f32), depth, color, 0, 0);
        }
    }
    pub(super) fn point(&mut self, p: Vec2, depth: f32, color: u32, radius: i32, mark: u8) {
        let x = p.x.round() as i32;
        let y = p.y.round() as i32;
        for yy in y - radius..=y + radius {
            for xx in x - radius..=x + radius {
                if xx >= 0
                    && yy >= 0
                    && xx < self.w as i32
                    && yy < self.h as i32
                    && (self.clip_depth.is_empty()
                        || (yy >= self.visible[xx as usize].0 && yy < self.visible[xx as usize].1))
                {
                    let at = xx as usize * self.h + yy as usize;
                    if self.depth[at] >= depth - 1.0
                        && self
                            .clip_depth
                            .get(at)
                            .is_none_or(|near| depth >= *near - 1.0)
                    {
                        self.pixels[at] = color;
                        self.mask[at] = mark;
                        if mark == 1 {
                            self.depth[at] = self.depth[at].min(depth);
                        }
                    }
                }
            }
        }
    }
}
