use super::*;

impl Renderer {
    pub(super) fn pitched_terrain(
        &mut self,
        world: &World,
        faceted: bool,
        camera: &Camera,
        environment: crate::world::network::Environment,
    ) {
        let theme = environment.theme;
        let weather = if world.kind.vacuum() {
            Weather::Clear
        } else {
            environment.weather
        };
        let (mut fog_start, far) = if world.kind.vacuum() {
            (2200.0, 2600.0)
        } else {
            {
                let (start, end) = weather.visibility(camera.position.z);
                (start.min(3900.0), end.min(4200.0))
            }
        };
        let bend = self.warp.map_or(0.0, |warp| warp.bend);
        let footprint_scale = self.warp.map_or(1.0, |warp| {
            1.0 + (warp.body.radius() / crate::world::orbit::UV_SCALE - 1.0).max(0.0) * bend
        });
        fog_start += bend * 10000.0;
        let fog_end = far + bend * 10000.0;
        let colors = TOP.map(|c| weather.shade(grade(rgb(c[0], c[1], c[2]), theme)));
        self.terrain_cache.colors(colors);
        let haze = if world.kind.vacuum() {
            rgb(8, 10, 18)
        } else {
            weather.haze(theme)
        };
        let mut extent = [self.w as i32, 0, self.h as i32, 0];
        for (x, &(y0, y1)) in self.visible.iter().enumerate() {
            if y0 < y1 {
                extent[0] = extent[0].min(x as i32);
                extent[1] = extent[1].max(x as i32 + 1);
                extent[2] = extent[2].min(y0);
                extent[3] = extent[3].max(y1);
            }
        }
        if extent[0] >= extent[1] {
            return;
        }
        let [left, right, top, bottom] = [
            camera.center.x - extent[0] as f32,
            extent[1] as f32 - camera.center.x,
            camera.center.y - extent[2] as f32,
            extent[3] as f32 - camera.center.y,
        ];
        let planes = [
            (
                camera.right * camera.scale + camera.forward * (left * camera.q),
                left,
            ),
            (
                -camera.right * camera.scale + camera.forward * (right * camera.q),
                right,
            ),
            (
                -camera.up * camera.scale
                    + camera.forward * (top * camera.q - camera.scale * camera.slope),
                top,
            ),
            (
                camera.up * camera.scale
                    + camera.forward * (bottom * camera.q + camera.scale * camera.slope),
                bottom,
            ),
            (camera.forward * camera.q, 0.88),
        ];
        let cx = (camera.position.x / 512.0).floor() as i32;
        let cy = (camera.position.y / 512.0).floor() as i32;
        let reach = (far / 512.0).ceil() as i32;
        if let Some(warp) = self.warp {
            self.terrain_cache.prepare_warp(
                warp,
                vec2(((cx - reach) * 512) as f32, ((cy - reach) * 512) as f32),
                (reach * 2 + 1) * 512,
            );
        }
        let mut cells = Vec::with_capacity(200);
        for y in -reach..=reach {
            for x in -reach..=reach {
                cells.push(((cx + x) * 512, (cy + y) * 512, 512));
            }
        }
        let mut leaves = Vec::new();
        while let Some((x, y, size)) = cells.pop() {
            let half = vec3(
                size as f32 * 0.5,
                size as f32 * 0.5,
                world.ceiling as f32 * 0.5,
            );
            let mut center = vec3(x as f32 + half.x, y as f32 + half.y, half.z);
            let mut half = half;
            if let Some(warp) = self.warp {
                if !warp.contains(center.truncate(), size as f32 * 0.6) {
                    continue;
                }
                let mut low = Vec3::splat(f32::INFINITY);
                let mut high = Vec3::splat(f32::NEG_INFINITY);
                for dx in [-1.0, 0.0, 1.0] {
                    for dy in [-1.0, 0.0, 1.0] {
                        for dz in [-1.0, 1.0] {
                            let point = self
                                .terrain_cache
                                .warped_point(warp, center + half * vec3(dx, dy, dz));
                            low = low.min(point);
                            high = high.max(point);
                        }
                    }
                }
                center = (low + high) * 0.5;
                half = (high - low) * 0.5;
            }
            let delta = center - camera.position;
            if delta.truncate().length() > far + size as f32 {
                continue;
            }
            if planes
                .iter()
                .any(|(n, d)| delta.dot(*n) + d + n.abs().dot(half) < 0.0)
            {
                continue;
            }
            let footprint = camera.scale * size as f32 * footprint_scale
                / (1.0 + camera.q * delta.dot(camera.forward)).max(0.15);
            if size > 8 && footprint > 10.0 {
                let h = size / 2;
                cells.extend([(x, y, h), (x + h, y, h), (x, y + h, h), (x + h, y + h, h)]);
                continue;
            }
            leaves.push((x, y, size));
        }
        self.terrain_cache.topology(
            &leaves,
            ((cx - reach) * 64, (cy - reach) * 64),
            (reach * 2 + 1) * 64,
        );
        let mut boundary = Vec::with_capacity(16);
        for (x, y, size) in leaves {
            let sample = |x, y| vec3(x as f32, y as f32, world.heights[index(x, y, SIZE)] as f32);
            let center = sample(x + size / 2, y + size / 2);
            boundary.clear();
            for edge in 0..4 {
                let mut offset = 0;
                while offset < size {
                    let (px, py, nx, ny, coordinate) = match edge {
                        0 => (x + offset, y, x + offset, y - 1, x + offset),
                        1 => (x + size, y + offset, x + size, y + offset, y + offset),
                        2 => (
                            x + size - offset,
                            y + size,
                            x + size - offset - 1,
                            y + size,
                            -(x + size - offset),
                        ),
                        _ => (
                            x,
                            y + size - offset,
                            x - 1,
                            y + size - offset - 1,
                            -(y + size - offset),
                        ),
                    };
                    boundary.push(sample(px, py));
                    let neighbor = self.terrain_cache.neighbor(nx, ny, size);
                    offset += (neighbor - coordinate.rem_euclid(neighbor)).min(size - offset);
                }
            }
            if boundary.len() == 4 {
                for indices in [[0, 1, 2], [0, 2, 3]] {
                    self.ground_triangle(
                        camera,
                        world,
                        indices.map(|i| boundary[i]),
                        faceted,
                        (fog_start, fog_end, haze),
                    );
                }
                continue;
            }
            for i in 0..boundary.len() {
                self.ground_triangle(
                    camera,
                    world,
                    [center, boundary[i], boundary[(i + 1) % boundary.len()]],
                    faceted,
                    (fog_start, fog_end, haze),
                );
            }
        }
    }
    fn ground_triangle(
        &mut self,
        camera: &Camera,
        world: &World,
        vertices: [Vec3; 3],
        faceted: bool,
        fog: (f32, f32, u32),
    ) {
        let points = vertices.map(|p| {
            self.warp
                .map_or(p, |warp| self.terrain_cache.warped_point(warp, p))
        });
        self.ground_polygon(camera, world, vertices, points, faceted, fog);
    }
    fn ground_polygon(
        &mut self,
        camera: &Camera,
        world: &World,
        vertices: [Vec3; 3],
        points: [Vec3; 3],
        faceted: bool,
        fog: (f32, f32, u32),
    ) {
        if self.warp.is_some() {
            let normal = (points[1] - points[0]).cross(points[2] - points[0]);
            let (eye, ray) = camera.ray(camera.center);
            let towards = if camera.q > 0.00001 {
                eye - points[0]
            } else {
                -ray
            };
            if normal.dot(towards) <= 0.0 {
                return;
            }
        }
        if points
            .iter()
            .any(|p| 1.0 + camera.q * (*p - camera.position).dot(camera.forward) < 0.121)
        {
            let near = camera.position - camera.forward * (0.878 / camera.q);
            let mut clipped = Vec::with_capacity(4);
            for i in 0..3 {
                let a = points[i];
                let b = points[(i + 1) % 3];
                let da = (a - near).dot(camera.forward);
                let db = (b - near).dot(camera.forward);
                if da >= 0.0 {
                    clipped.push((a, vertices[i]));
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let t = da / (da - db);
                    clipped.push((a.lerp(b, t), vertices[i].lerp(vertices[(i + 1) % 3], t)));
                }
            }
            for i in 1..clipped.len().saturating_sub(1) {
                self.ground_polygon(
                    camera,
                    world,
                    [clipped[0].1, clipped[i].1, clipped[i + 1].1],
                    [clipped[0].0, clipped[i].0, clipped[i + 1].0],
                    faceted,
                    fog,
                );
            }
            return;
        }
        // 低空保持原始采样精度，海拔升高后才逐步放大采样跨度
        let ground_height =
            world.heights[index(camera.position.x as i32, camera.position.y as i32, SIZE)] as f32;
        let altitude_fade = ((camera.position.z - ground_height - 500.0) / 700.0).clamp(0.0, 1.0);

        self.triangles = self.triangles.wrapping_add(1);
        let [Some((a, da, _)), Some((b, db, _)), Some((c, dc, _))] =
            points.map(|p| camera.project(p))
        else {
            return;
        };
        let snapped = [a, b, c].map(|p| (p * 256.0).round());
        let [a, b, c] = snapped.map(|p| p / 256.0);
        let edge = |a: Vec2, b: Vec2, p: Vec2| -> i64 {
            (b.x as i64 - a.x as i64) * (p.y as i64 - a.y as i64)
                - (b.y as i64 - a.y as i64) * (p.x as i64 - a.x as i64)
        };
        let area = edge(snapped[0], snapped[1], snapped[2]);
        if area == 0 {
            return;
        }
        let min = a.min(b).min(c).max(Vec2::ZERO);
        let max = a
            .max(b)
            .max(c)
            .min(vec2(self.w as f32 - 1.0, self.h as f32 - 1.0));
        let inverse = [
            1.0 / (1.0 + camera.q * da),
            1.0 / (1.0 + camera.q * db),
            1.0 / (1.0 + camera.q * dc),
        ];
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        let light = self.warp.map_or(vec3(0.3, 0.2, 0.9), |warp| {
            vec3(0.3, 0.2, 0.9).lerp(warp.basis.transpose() * vec3(0.3, 0.2, 0.9), warp.bend)
        });
        let shade = if self.warp.is_some() {
            normal.dot(light).abs().clamp(0.45, 1.0)
        } else {
            (normal.x * 0.3 + normal.y * 0.2 + normal.z.abs() * 0.9).clamp(0.45, 1.0)
        };
        let inverse_area = 1.0 / area as f32;
        let columns = super::primitives::TriangleColumns::new([a, b, c]);
        let weighted = std::array::from_fn::<_, 3, _>(|i| vertices[i] * inverse[i]);
        let vertex_shade = if self.warp.is_some() {
            [shade; 3]
        } else {
            vertices.map(|p| {
                let h = |x, y| world.heights[index(x, y, SIZE)] as f32;
                let x = p.x as i32;
                let y = p.y as i32;
                // 采样跨度随视距增大，远处坡面明暗收敛为更粗的斑块
                let coarse = ((p.distance(camera.position)) / 48.0).clamp(8.0, 96.0);
                let step = (8.0 + (coarse - 8.0) * altitude_fade) as i32;
                let normal = vec3(
                    h(x - step, y) - h(x + step, y),
                    h(x, y - step) - h(x, y + step),
                    2.0 * step as f32,
                )
                .normalize();
                (normal.x * 0.3 + normal.y * 0.2 + normal.z * 0.9).clamp(0.45, 1.0)
            })
        };
        let sign = area.signum();
        let du = (snapped[2].x as i64 - snapped[1].x as i64) * 256;
        let dv = (snapped[0].x as i64 - snapped[2].x as i64) * 256;
        let u_step = du as f32 * inverse_area;
        let v_step = dv as f32 * inverse_area;
        let position_step =
            (weighted[0] - weighted[2]) * u_step + (weighted[1] - weighted[2]) * v_step;
        let denominator_step =
            (inverse[0] - inverse[2]) * u_step + (inverse[1] - inverse[2]) * v_step;
        let linear_depth_step = (da - dc) * u_step + (db - dc) * v_step;
        let inverse_q = if camera.q > 0.00001 {
            1.0 / camera.q
        } else {
            0.0
        };
        let inverse_fog = 1.0 / (fog.1 - fog.0);
        for x in min.x as i32..=max.x as i32 {
            let (top, bottom) = columns.rows(x as f32);
            let y0 = (min.y as i32).max(self.visible[x as usize].0).max(top);
            let y1 = (max.y as i32 + 1)
                .min(self.visible[x as usize].1)
                .min(bottom);
            let start = vec2(x as f32 * 256.0, y0 as f32 * 256.0);
            let mut eu = edge(snapped[1], snapped[2], start);
            let mut ev = edge(snapped[2], snapped[0], start);
            let u = eu as f32 * inverse_area;
            let v = ev as f32 * inverse_area;
            let mut position_next =
                weighted[2] + (weighted[0] - weighted[2]) * u + (weighted[1] - weighted[2]) * v;
            let mut denominator_next =
                inverse[2] + (inverse[0] - inverse[2]) * u + (inverse[1] - inverse[2]) * v;
            let mut linear_depth = dc + (da - dc) * u + (db - dc) * v;
            for y in y0..y1 {
                let (numerator, denominator, orthographic_depth) =
                    (position_next, denominator_next, linear_depth);
                position_next += position_step;
                denominator_next += denominator_step;
                linear_depth += linear_depth_step;
                let (u_edge, v_edge) = (eu, ev);
                eu += du;
                ev += dv;
                let w_edge = area - u_edge - v_edge;
                if u_edge * sign < 0 || v_edge * sign < 0 || w_edge * sign < 0 {
                    continue;
                }
                let at = x as usize * self.h + y as usize;
                if self.depth[at] == f32::NEG_INFINITY {
                    continue;
                }
                let u = u_edge as f32 * inverse_area;
                let v = v_edge as f32 * inverse_area;
                let w = w_edge as f32 * inverse_area;
                let reciprocal = 1.0 / denominator;
                let pos = numerator * reciprocal;
                let depth = if camera.q > 0.00001 {
                    (reciprocal - 1.0) * inverse_q
                } else {
                    orthographic_depth
                };
                if depth >= self.depth[at] || depth < self.near_depth {
                    continue;
                }
                // 视距越远按越大跨度做双线性采样，细碎纹样平均成较缓的明暗
                let view_distance = (pos - camera.position).length();
                let coarse = (view_distance / 100.0).clamp(0.5, 16.0);
                let sample_step = (coarse * altitude_fade).max(0.15);
                let smooth_shade = (vertex_shade[0] * inverse[0] * u
                    + vertex_shade[1] * inverse[1] * v
                    + vertex_shade[2] * inverse[2] * w)
                    * reciprocal;
                // 远处淡出面片明暗，收敛 LOD 边界的硬接缝
                let smooth = ((view_distance - 500.0) / 900.0).clamp(0.0, 1.0) * altitude_fade;
                let shade = if faceted {
                    shade * (1.0 - smooth) + smooth_shade * smooth
                } else {
                    smooth_shade
                };
                let cell = sample_step;
                let gx = (pos.x / cell).floor();
                let gy = (pos.y / cell).floor();
                let fx = pos.x / cell - gx;
                let fy = pos.y / cell - gy;
                let (mut red, mut green, mut blue) = (0.0f32, 0.0f32, 0.0f32);
                for (ix, iy, weight) in [
                    (gx, gy, (1.0 - fx) * (1.0 - fy)),
                    (gx + 1.0, gy, fx * (1.0 - fy)),
                    (gx, gy + 1.0, (1.0 - fx) * fy),
                    (gx + 1.0, gy + 1.0, fx * fy),
                ] {
                    let sample = vec3(ix * cell, iy * cell, pos.z);
                    let material =
                        world.materials[index(sample.x as i32, sample.y as i32, SIZE)] as usize;
                    let detail = if matches!(material, 12..=14 | 19..=20)
                        || (world.kind == crate::world::MapKind::Canyon
                            && matches!(material, 17..=20))
                    {
                        0.65 + super::geology::stone_texel_lod(
                            sample,
                            (view_distance.max(1.0) / 180.0).log2() * altitude_fade,
                        ) * 0.035
                    } else {
                        1.0
                    };
                    let [r, g, b, _] = self
                        .terrain_cache
                        .shade(material, shade * detail)
                        .to_le_bytes();
                    red += r as f32 * weight;
                    green += g as f32 * weight;
                    blue += b as f32 * weight;
                }
                let mut color = rgb(red as u8, green as u8, blue as u8);
                if let Some(warp) = self.warp {
                    color = super::orbit::earth_color(
                        warp,
                        pos.truncate(),
                        color,
                        shade,
                        camera.position.z,
                    );
                }
                let amount = ((depth - fog.0) * inverse_fog).clamp(0.0, 1.0);
                self.pixels[at] = mix(color, fog.2, amount);
                self.depth[at] = depth;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terrain_covers_shared_edges_at_every_camera_attitude() {
        let mut world = World::new(7);
        world.heights.fill(60);
        world.materials.fill(15);
        world.clouds.fill(0);
        world.scenery.clear();
        world.portals.clear();
        world.ceiling = 60;
        let mut flight = Flight::new(&world);
        flight.position.z = 180.0;
        flight.follow = flight.position;
        flight.wing_count = 0;
        for view in [0, 1] {
            flight.weights = [0.0; 3];
            flight.weights[view] = 1.0;
            for pitch in [0.0, -0.55, 0.35, -2.8] {
                flight.pitch = pitch;
                let camera = Camera::new(&flight, 320, 170);
                let mut renderer = Renderer::single();
                renderer.render(&world, &flight, 320, 170, 0);
                for x in 0..320 {
                    for y in 0..170 {
                        let (eye, ray) = camera.ray(vec2(x as f32, y as f32));
                        let point = eye + ray * ((60.0 - eye.z) / ray.z);
                        let depth = (point - camera.position).dot(camera.forward);
                        if 1.0 + camera.q * depth > 0.13 && point.distance(camera.position) < 900.0
                        {
                            assert!(
                                renderer.depth[x * 170 + y].is_finite(),
                                "view {view} pitch {pitch} at {x}/{y}"
                            );
                        }
                    }
                }
            }
        }
    }
}
