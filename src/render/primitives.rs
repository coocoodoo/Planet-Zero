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
    pub(super) fn fogged_scenery(&self, color: u32, depth: f32) -> u32 {
        self.scenery_fog
            .map_or(color, |(start, inverse_range, haze)| {
                mix(color, haze, (depth - start) * inverse_range)
            })
    }

    pub(super) fn world_line(&mut self, camera: &Camera, a: Vec3, b: Vec3, color: u32) {
        let (a, b) = self
            .warp
            .map_or((a, b), |warp| (warp.point(a), warp.point(b)));
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
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        let color = mix(
            self.light.color(rgb(17, 23, 35)),
            self.light.color(color),
            self.light.diffuse(normal),
        );
        self.surface(camera, points, color, false, None);
    }
    pub(super) fn rock_triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32) {
        let color = self.light.color(color);
        self.surface(camera, points, color, true, None);
    }
    pub(super) fn surface(
        &mut self,
        camera: &Camera,
        points: [Vec3; 3],
        color: u32,
        stone: bool,
        cloud: Option<super::clouds::CloudSurface<'_>>,
    ) {
        let points = points.map(|point| self.warp.map_or(point, |warp| warp.point(point)));
        let forward = camera.forward;
        let near_depth = if camera.q > 0.00001 {
            -0.879 / camera.q
        } else {
            -1e8
        }
        .max(self.near_depth);
        let near = camera.position + forward * near_depth;
        if points.iter().all(|p| (*p - near).dot(forward) >= 0.0) {
            self.raster_triangle(camera, points, color, stone, cloud);
            return;
        }
        let mut polygon = [Vec3::ZERO; 4];
        let mut amounts = [glam::Vec4::ZERO; 4];
        let attributes = cloud.map_or([glam::Vec4::ZERO; 3], |cloud| cloud.attributes);
        let mut count = 0;
        for i in 0..3 {
            let a = points[i];
            let b = points[(i + 1) % 3];
            let da = (a - near).dot(forward);
            let db = (b - near).dot(forward);
            if da >= 0.0 {
                polygon[count] = a;
                amounts[count] = attributes[i];
                count += 1;
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                polygon[count] = a.lerp(b, t);
                amounts[count] = attributes[i] + (attributes[(i + 1) % 3] - attributes[i]) * t;
                count += 1;
            }
        }
        for i in 1..count.saturating_sub(1) {
            self.raster_triangle(
                camera,
                [polygon[0], polygon[i], polygon[i + 1]],
                color,
                stone,
                cloud.map(|cloud| super::clouds::CloudSurface {
                    attributes: [amounts[0], amounts[i], amounts[i + 1]],
                    ..cloud
                }),
            );
        }
    }
    fn raster_triangle(
        &mut self,
        camera: &Camera,
        points: [Vec3; 3],
        color: u32,
        stone: bool,
        cloud: Option<super::clouds::CloudSurface<'_>>,
    ) {
        self.triangles = self.triangles.wrapping_add(1);
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
                    if depth > self.depth[at] + 1.0 {
                        continue;
                    }
                    if let Some(cloud) = cloud {
                        let sample = &mut self.cloud_pixels[at];
                        if depth < sample.depth {
                            if sample.depth == f32::INFINITY {
                                self.cloud_touched.push(at);
                            }
                            let weights = vec3(u * inverse[0], v * inverse[1], w * inverse[2]);
                            let attributes = (cloud.attributes[0] * weights.x
                                + cloud.attributes[1] * weights.y
                                + cloud.attributes[2] * weights.z)
                                / weights.element_sum();
                            let density = (attributes.x / 8.0).clamp(0.0, 1.0);
                            let mut shade = attributes.y;
                            if !cloud.relief.is_empty() {
                                let tx = (attributes.z as usize).min(63);
                                let ty = (attributes.w as usize).min(63);
                                let relief = glam::Vec4::from_array(
                                    cloud.relief[ty * 64 + tx].map(|v| v as f32),
                                );
                                shade +=
                                    relief.dot(cloud.relief_light) * (attributes.x / 20.0).min(1.0);
                            }
                            let dither = [0.0, 0.5, 0.75, 0.25]
                                [((y as usize & 1) << 1) | ((x as usize + self.x_origin) & 1)];
                            let offset = if cloud.pixel_texture {
                                dither * 0.5
                            } else {
                                0.1875
                            };
                            let color =
                                cloud.palette[(shade * 7.0 + offset).clamp(0.0, 7.0) as usize];
                            *sample = super::clouds::CloudPixel {
                                depth,
                                color,
                                opacity: density * (2.0 - density),
                            };
                        }
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
                    self.pixels[at] = self.fogged_scenery(shaded, depth);
                    self.mask[at] = self.mark;
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
    /// 圆形像素点，用于弹体等需要与方形拾取物区分的目标
    pub(super) fn disc(&mut self, p: Vec2, depth: f32, color: u32, radius: i32) {
        if radius <= 1 {
            self.point(p, depth, color, radius, 0);
            return;
        }
        let limit = radius * radius + radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= limit {
                    self.point(p + vec2(dx as f32, dy as f32), depth, color, 0, 0);
                }
            }
        }
    }
    pub(super) fn point(&mut self, p: Vec2, depth: f32, color: u32, radius: i32, mark: u8) {
        let color = self.fogged_scenery(color, depth);
        let x = p.x.round() as i32;
        let y = p.y.round() as i32;
        for yy in y - radius..=y + radius {
            for xx in x - radius..=x + radius {
                self.plot(xx, yy, depth, color, mark);
            }
        }
    }
    /// 写入单个像素（已含雾色），带画布、可见区与深度检查
    #[inline]
    pub(super) fn plot(&mut self, x: i32, y: i32, depth: f32, color: u32, mark: u8) {
        if x >= 0
            && y >= 0
            && x < self.w as i32
            && y < self.h as i32
            && (self.near_depth == f32::NEG_INFINITY
                || (y >= self.visible[x as usize].0 && y < self.visible[x as usize].1))
        {
            let at = x as usize * self.h + y as usize;
            if self.depth[at] >= depth - 1.0 && depth >= self.near_depth - 1.0 {
                self.pixels[at] = color;
                self.mask[at] = mark;
                if mark == 1 {
                    self.depth[at] = self.depth[at].min(depth);
                }
            }
        }
    }
}
