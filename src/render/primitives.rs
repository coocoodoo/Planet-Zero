use super::*;

impl Renderer {
    pub(super) fn world_line(&mut self, camera: &Camera, a: Vec3, b: Vec3, color: u32) {
        let (mut a, mut b) = (a, b);
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
            if a.distance(b) > 1200.0 {
                return;
            }
            let steps = a.distance(b).ceil().clamp(1.0, 400.0) as usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                self.point(a.lerp(b, t), da + (db - da) * t, color, 0, 0);
            }
        }
    }
    pub(super) fn triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32) {
        let (s, c) = camera.yaw.sin_cos();
        let forward = vec3(-s, -c, 0.0);
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
            self.raster_triangle(camera, points, color);
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
            self.raster_triangle(camera, [polygon[0], polygon[i], polygon[i + 1]], color);
        }
    }
    fn raster_triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32) {
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
        for y in min.y as i32..=max.y as i32 {
            for x in min.x as i32..=max.x as i32 {
                let p = vec2(x as f32, y as f32);
                let u = (b - p).perp_dot(c - p) / area;
                let v = (c - p).perp_dot(a - p) / area;
                let w = 1.0 - u - v;
                if u >= 0.0 && v >= 0.0 && w >= 0.0 {
                    let depth = if camera.q > 0.00001 {
                        (1.0 / (u / (1.0 + camera.q * da)
                            + v / (1.0 + camera.q * db)
                            + w / (1.0 + camera.q * dc))
                            - 1.0)
                            / camera.q
                    } else {
                        u * da + v * db + w * dc
                    };
                    self.point(p, depth, color, 0, 1);
                }
            }
        }
    }
    pub(super) fn stroke(&mut self, a: Vec2, b: Vec2, depth: f32, color: u32) {
        let steps = a.distance(b).ceil().clamp(1.0, 48.0) as usize;
        for step in 0..=steps {
            self.point(a.lerp(b, step as f32 / steps as f32), depth, color, 0, 0);
        }
    }
    pub(super) fn point(&mut self, p: Vec2, depth: f32, color: u32, radius: i32, mark: u8) {
        let x = p.x.round() as i32;
        let y = p.y.round() as i32;
        for yy in y - radius..=y + radius {
            for xx in x - radius..=x + radius {
                if xx >= 0 && yy >= 0 && xx < self.w as i32 && yy < self.h as i32 {
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
