use super::*;

impl Renderer {
    pub(super) fn pitched_terrain(
        &mut self,
        world: &World,
        faceted: bool,
        camera: &Camera,
        weather: crate::content::weather::WeatherState,
    ) {
        let ground_height =
            world.heights[index(camera.position.x as i32, camera.position.y as i32, SIZE)] as f32;
        self.terrain_cache.altitude_fade =
            ((camera.position.z - ground_height - 500.0) / 700.0).clamp(0.0, 1.0);
        let weather = if world.kind.vacuum() {
            crate::content::weather::WeatherState::new(Weather::Clear, world.kind, world.seed)
        } else {
            weather
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
        fog_start += (650.0 - fog_start) * self.light.synthwave;
        fog_start += bend * 10000.0;
        let fog_end = far + bend * 10000.0;
        let colors = TOP.map(|c| weather.shade(self.light.color(rgb(c[0], c[1], c[2]))));
        self.terrain_cache.colors(colors);
        let haze = if world.kind.vacuum() {
            rgb(8, 10, 18)
        } else {
            self.light.haze(weather)
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
            let [low, high] = if self.warp.is_some() {
                [0, world.ceiling]
            } else {
                world.height_range(x, y, size)
            };
            let half = vec3(
                size as f32 * 0.5,
                size as f32 * 0.5,
                (high - low) as f32 * 0.5,
            );
            let mut center = vec3(x as f32 + half.x, y as f32 + half.y, low as f32 + half.z);
            let mut half = half;
            let mut footprint_span = size as f32 * footprint_scale;
            if let Some(warp) = self.warp {
                if !warp.contains(center.truncate(), size as f32 * 0.6) {
                    continue;
                }
                let latitude = ((center.y - warp.origin.y).abs() - half.y).max(0.0)
                    / crate::world::orbit::UV_SCALE;
                let compression = latitude
                    .min(std::f32::consts::FRAC_PI_2)
                    .cos()
                    .max(0.0)
                    .sqrt();
                footprint_span *= 1.0 + (compression - 1.0) * warp.polar_row(center.y).z;
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
            let lod_center = if self.warp.is_some() {
                center
            } else {
                center.with_z(world.ceiling as f32 * 0.5)
            };
            let pixel_scale = camera.scale
                / (1.0 + camera.q * (lod_center - camera.position).dot(camera.forward)).max(0.15);
            let footprint = pixel_scale * footprint_span;
            // 球面弦高的投影误差限制为四分之一像素
            let curvature_error = self.warp.map_or(0.0, |warp| {
                (size as f32 * footprint_scale).powi(2) * warp.bend * pixel_scale
                    / (8.0 * warp.body.radius())
            });
            if size > 8 && (footprint > 10.0 || curvature_error > 0.25) {
                let h = size / 2;
                cells.extend([(x, y, h), (x + h, y, h), (x, y + h, h), (x + h, y + h, h)]);
                continue;
            }
            leaves.push((x, y, size));
        }
        self.terrain_cache.topology(&leaves);
        let mut boundary = Vec::with_capacity(16);
        for (x, y, size) in leaves {
            let sample = |x, y| {
                let uv = vec2(x as f32, y as f32);
                let height = self.warp.map_or_else(
                    || world.mesh_height(x, y),
                    |warp| self.terrain_cache.terrain_height(warp, world, uv),
                );
                uv.extend(height)
            };
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
        if let Some(warp) = self.warp.filter(|warp| warp.bend == 1.0) {
            let half = std::f32::consts::FRAC_PI_2 * crate::world::orbit::UV_SCALE;
            if let Some(outside) = vertices.iter().find(|p| (p.y - warp.origin.y).abs() > half) {
                let sign = (outside.y - warp.origin.y).signum();
                let boundary = warp.origin.y + sign * half;
                let mut clipped = [Vec3::ZERO; 4];
                let mut count = 0;
                for i in 0..3 {
                    let a = vertices[i];
                    let b = vertices[(i + 1) % 3];
                    let da = (boundary - a.y) * sign;
                    let db = (boundary - b.y) * sign;
                    if da >= 0.0 {
                        clipped[count] = a;
                        count += 1;
                    }
                    if (da >= 0.0) != (db >= 0.0) {
                        let mut edge = a.lerp(b, da / (da - db));
                        edge.y = boundary;
                        edge.z = self
                            .terrain_cache
                            .terrain_height(warp, world, edge.truncate());
                        clipped[count] = edge;
                        count += 1;
                    }
                }
                for i in 1..count.saturating_sub(1) {
                    let vertices = [clipped[0], clipped[i], clipped[i + 1]];
                    self.ground_polygon(
                        camera,
                        world,
                        vertices,
                        vertices.map(|p| warp.point(p)),
                        faceted,
                        fog,
                    );
                }
                return;
            }
        }
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
        {
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
            let mut clipped = [(Vec3::ZERO, Vec3::ZERO); 4];
            let mut count = 0;
            for i in 0..3 {
                let a = points[i];
                let b = points[(i + 1) % 3];
                let da = (a - near).dot(camera.forward);
                let db = (b - near).dot(camera.forward);
                if da >= 0.0 {
                    clipped[count] = (a, vertices[i]);
                    count += 1;
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let t = da / (da - db);
                    clipped[count] = (a.lerp(b, t), vertices[i].lerp(vertices[(i + 1) % 3], t));
                    count += 1;
                }
            }
            for i in 1..count.saturating_sub(1) {
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
        let altitude_fade = self.terrain_cache.altitude_fade;

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
        let shade = self.light.diffuse(normal);
        let inverse_area = 1.0 / area as f32;
        let columns = super::primitives::TriangleColumns::new([a, b, c]);
        let weighted = std::array::from_fn::<_, 3, _>(|i| vertices[i] * inverse[i]);
        let polar = self.warp.and_then(|warp| {
            let samples = std::array::from_fn::<_, 3, _>(|i| {
                self.terrain_cache
                    .polar_sample(warp, vertices[i].truncate())
                    * inverse[i]
            });
            samples.iter().any(|p| p.z > 0.0).then_some(samples)
        });
        let vertex_shade = if self.warp.is_some() || (faceted && altitude_fade == 0.0) {
            [shade; 3]
        } else {
            vertices.map(|p| {
                let h = |x, y| world.heights[index(x, y, SIZE)] as f32;
                let x = p.x as i32;
                let y = p.y as i32;
                // 采样跨度随视距增大，远处坡面明暗收敛为更粗的斑块
                let step = if altitude_fade > 0.0 {
                    let coarse = (p.distance(camera.position) / 48.0).clamp(8.0, 96.0);
                    (8.0 + (coarse - 8.0) * altitude_fade) as i32
                } else {
                    8
                };
                let normal = vec3(
                    h(x - step, y) - h(x + step, y),
                    h(x, y - step) - h(x, y + step),
                    2.0 * step as f32,
                )
                .normalize();
                self.light.diffuse(normal)
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
                let view_distance = if altitude_fade > 0.0 {
                    (pos - camera.position).length()
                } else {
                    0.0
                };
                let sample_step = if altitude_fade > 0.0 {
                    ((view_distance / 100.0).clamp(0.5, 16.0) * altitude_fade).max(0.15)
                } else {
                    0.15
                };
                let shade = if faceted && altitude_fade == 0.0 {
                    shade
                } else {
                    let u = u_edge as f32 * inverse_area;
                    let v = v_edge as f32 * inverse_area;
                    let w = w_edge as f32 * inverse_area;
                    let smooth_shade = (vertex_shade[0] * inverse[0] * u
                        + vertex_shade[1] * inverse[1] * v
                        + vertex_shade[2] * inverse[2] * w)
                        * reciprocal;
                    // 远处淡出面片明暗，收敛 LOD 边界的硬接缝
                    if faceted {
                        let smooth =
                            ((view_distance - 500.0) / 900.0).clamp(0.0, 1.0) * altitude_fade;
                        shade * (1.0 - smooth) + smooth_shade * smooth
                    } else {
                        smooth_shade
                    }
                };
                let cap = polar.map(|samples| {
                    let u = u_edge as f32 * inverse_area;
                    let v = v_edge as f32 * inverse_area;
                    (samples[2] + (samples[0] - samples[2]) * u + (samples[1] - samples[2]) * v)
                        * reciprocal
                });
                let uv = cap.filter(|p| p.z >= 0.9999).unwrap_or(pos);
                let cell = sample_step;
                let (sx, sy) = (uv.x / cell, uv.y / cell);
                let gx = sx.floor();
                let gy = sy.floor();
                let x0 = (gx * cell) as i32;
                let x1 = ((gx + 1.0) * cell) as i32;
                let y0 = (gy * cell) as i32;
                let y1 = ((gy + 1.0) * cell) as i32;
                let a = world.materials[index(x0, y0, SIZE)] as usize;
                let b = if x0 == x1 {
                    a
                } else {
                    world.materials[index(x1, y0, SIZE)] as usize
                };
                let c = if y0 == y1 {
                    a
                } else {
                    world.materials[index(x0, y1, SIZE)] as usize
                };
                let d = if x0 == x1 {
                    c
                } else if y0 == y1 {
                    b
                } else {
                    world.materials[index(x1, y1, SIZE)] as usize
                };
                let materials = [a, b, c, d];
                let rocky = |material| {
                    matches!(material, 12..=14 | 19..=20)
                        || (world.kind == crate::world::MapKind::Canyon
                            && matches!(material, 17..=20))
                };
                let mut color = if !rocky(materials[0])
                    && materials.iter().all(|&material| material == materials[0])
                {
                    self.terrain_cache.shade(materials[0], shade)
                } else {
                    // 慢路径才需要细节层级与四个采样权重
                    let (fx, fy) = (sx - gx, sy - gy);
                    let samples = [
                        (gx, gy, (1.0 - fx) * (1.0 - fy)),
                        (gx + 1.0, gy, fx * (1.0 - fy)),
                        (gx, gy + 1.0, (1.0 - fx) * fy),
                        (gx + 1.0, gy + 1.0, fx * fy),
                    ];
                    let lod = if altitude_fade > 0.0 {
                        (view_distance.max(1.0) / 180.0).log2() * altitude_fade
                    } else {
                        0.0
                    };
                    let (mut red, mut green, mut blue) = (0.0_f32, 0.0_f32, 0.0_f32);
                    let details = if materials.iter().any(|&m| rocky(m)) {
                        super::geology::stone_texels_lod(
                            samples.map(|(x, y, _)| vec3(x * cell, y * cell, pos.z)),
                            lod,
                        )
                    } else {
                        [0.0; 4]
                    };
                    for (((_, _, weight), material), detail) in
                        samples.into_iter().zip(materials).zip(details)
                    {
                        let detail = if rocky(material) {
                            0.65 + detail * 0.035
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
                    rgb(red as u8, green as u8, blue as u8)
                };
                if let Some(sample) = cap.filter(|p| p.z < 0.9999) {
                    let material =
                        world.materials[index(sample.x as i32, sample.y as i32, SIZE)] as usize;
                    color = mix(color, self.terrain_cache.shade(material, shade), sample.z);
                }
                if let Some(warp) = self.warp {
                    color = super::orbit::earth_color(
                        warp,
                        pos.truncate(),
                        color,
                        shade,
                        camera.position.z,
                    );
                }
                let amount = (depth - fog.0) * inverse_fog;
                self.pixels[at] = if amount <= 0.0 {
                    color
                } else {
                    mix(color, fog.2, amount.min(1.0))
                };
                self.depth[at] = depth;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spherical_terrain_covers_both_poles_with_off_grid_origins() {
        use crate::world::orbit::{Body, SurfaceWarp, UV_SCALE};
        let mut world = World::generate(crate::world::MapKind::Moon, 7);
        world.heights.fill(60);
        world.materials.fill(27);
        world.update_ceiling();
        let origin = vec2(2073.25, 1919.5);
        let mut renderer = Renderer::single();
        renderer.w = 320;
        renderer.h = 170;
        renderer.visible = vec![(0, 170); 320];
        renderer.depth = vec![f32::INFINITY; 320 * 170];
        renderer.pixels = vec![0; 320 * 170];
        for body in Body::ALL {
            for latitude in [-90.0_f32, -82.0, 82.0, 90.0] {
                let anchor = origin + vec2(0.7, latitude.to_radians()) * UV_SCALE;
                renderer.warp = Some(SurfaceWarp::new(body, origin, anchor, 1.0));
                renderer.depth.fill(f32::INFINITY);
                let camera = Camera {
                    position: anchor.extend(1500.0),
                    yaw: 0.0,
                    pitch: -std::f32::consts::FRAC_PI_2,
                    forward: -Vec3::Z,
                    right: Vec3::X,
                    up: Vec3::Y,
                    q: 1.0 / 80.0,
                    scale: 3.0,
                    slope: 0.0,
                    center: vec2(160.0, 85.0),
                };
                let weather =
                    crate::content::weather::WeatherState::new(Weather::Clear, world.kind, 7);
                renderer.pitched_terrain(&world, false, &camera, weather);
                for x in 125..195 {
                    for y in 50..120 {
                        assert!(
                            renderer.depth[x * 170 + y].is_finite(),
                            "{body:?} {latitude}: {x}/{y}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn terrain_covers_shared_edges_at_every_camera_attitude() {
        let mut world = World::new(7);
        world.heights.fill(60);
        world.materials.fill(15);
        world.clouds.clear();
        world.scenery.clear();
        world.portals.clear();
        world.update_ceiling();
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
