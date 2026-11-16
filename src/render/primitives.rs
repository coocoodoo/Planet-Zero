use super::*;

impl Renderer {
    pub(super) fn world_line(&mut self, camera: &Camera, a: Vec3, b: Vec3, color: u32) {
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
                    self.point(p, u * da + v * db + w * dc, color, 0, 1);
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
                    if self.depth[at] >= depth - 1.0 {
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
