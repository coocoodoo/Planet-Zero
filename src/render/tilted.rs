use super::*;

impl Renderer {
    pub(super) fn pitched_terrain(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
    ) {
        let weather = if world.kind.vacuum() {
            Weather::Clear
        } else {
            flight.weather
        };
        let (fog_start, far) = if world.kind.vacuum() {
            (2200.0, 2600.0)
        } else {
            weather.visibility(camera.position.z)
        };
        let colors = TOP.map(|c| weather.shade(grade(rgb(c[0], c[1], c[2]), theme)));
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
            let center = vec3(x as f32 + half.x, y as f32 + half.y, half.z);
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
            let footprint =
                camera.scale * size as f32 / (1.0 + camera.q * delta.dot(camera.forward)).max(0.15);
            if size > 8 && footprint > 10.0 {
                let h = size / 2;
                cells.extend([(x, y, h), (x + h, y, h), (x, y + h, h), (x + h, y + h, h)]);
                continue;
            }
            leaves.push((x, y, size));
        }
        let lookup: std::collections::HashSet<_> = leaves.iter().copied().collect();
        for (x, y, size) in leaves {
            let sample = |x, y| vec3(x as f32, y as f32, world.heights[index(x, y, SIZE)] as f32);
            let center = sample(x + size / 2, y + size / 2);
            let mut boundary = Vec::with_capacity(8);
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
                    let neighbor = [8, 16, 32, 64, 128, 256, 512]
                        .into_iter()
                        .find(|&n| {
                            lookup.contains(&(nx.div_euclid(n) * n, ny.div_euclid(n) * n, n))
                        })
                        .unwrap_or(size);
                    offset += (neighbor - coordinate.rem_euclid(neighbor)).min(size - offset);
                }
            }
            if boundary.len() == 4 {
                for indices in [[0, 1, 2], [0, 2, 3]] {
                    self.ground_triangle(
                        camera,
                        world,
                        indices.map(|i| boundary[i]),
                        &colors,
                        (fog_start, far, haze),
                    );
                }
                continue;
            }
            for i in 0..boundary.len() {
                self.ground_triangle(
                    camera,
                    world,
                    [center, boundary[i], boundary[(i + 1) % boundary.len()]],
                    &colors,
                    (fog_start, far, haze),
                );
            }
        }
    }
    fn ground_triangle(
        &mut self,
        camera: &Camera,
        world: &World,
        vertices: [Vec3; 3],
        colors: &[u32; MATERIAL_COUNT],
        fog: (f32, f32, u32),
    ) {
        if vertices
            .iter()
            .any(|p| 1.0 + camera.q * (*p - camera.position).dot(camera.forward) < 0.121)
        {
            let near = camera.position - camera.forward * (0.878 / camera.q);
            let mut clipped = Vec::with_capacity(4);
            for i in 0..3 {
                let a = vertices[i];
                let b = vertices[(i + 1) % 3];
                let da = (a - near).dot(camera.forward);
                let db = (b - near).dot(camera.forward);
                if da >= 0.0 {
                    clipped.push(a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    clipped.push(a.lerp(b, da / (da - db)));
                }
            }
            for i in 1..clipped.len().saturating_sub(1) {
                self.ground_triangle(
                    camera,
                    world,
                    [clipped[0], clipped[i], clipped[i + 1]],
                    colors,
                    fog,
                );
            }
            return;
        }
        let [Some((a, da, _)), Some((b, db, _)), Some((c, dc, _))] =
            vertices.map(|p| camera.project(p))
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
        let inverse = [
            1.0 / (1.0 + camera.q * da),
            1.0 / (1.0 + camera.q * db),
            1.0 / (1.0 + camera.q * dc),
        ];
        let normal = (vertices[1] - vertices[0])
            .cross(vertices[2] - vertices[0])
            .normalize_or_zero();
        let shade = (normal.x * 0.3 + normal.y * 0.2 + normal.z.abs() * 0.9).clamp(0.45, 1.0);
        let inverse_area = 1.0 / area;
        let du = (c.x - b.x) * inverse_area;
        let dv = (a.x - c.x) * inverse_area;
        let inverse_q = if camera.q > 0.00001 {
            1.0 / camera.q
        } else {
            0.0
        };
        for x in min.x as i32..=max.x as i32 {
            let y0 = (min.y as i32).max(self.visible[x as usize].0);
            let y1 = (max.y as i32 + 1).min(self.visible[x as usize].1);
            let start = vec2(x as f32, y0 as f32);
            let mut u_next = (b - start).perp_dot(c - start) * inverse_area;
            let mut v_next = (c - start).perp_dot(a - start) * inverse_area;
            for y in y0..y1 {
                let (u, v) = (u_next, v_next);
                u_next += du;
                v_next += dv;
                let w = 1.0 - u - v;
                if u < -0.00001 || v < -0.00001 || w < -0.00001 {
                    continue;
                }
                let weights = vec3(u * inverse[0], v * inverse[1], w * inverse[2]);
                let denominator = weights.element_sum();
                let reciprocal = 1.0 / denominator;
                let depth = (reciprocal - 1.0) * inverse_q;
                let at = x as usize * self.h + y as usize;
                if depth >= self.depth[at] || self.clip_depth.get(at).is_some_and(|d| depth < *d) {
                    continue;
                }
                let pos =
                    (vertices[0] * weights.x + vertices[1] * weights.y + vertices[2] * weights.z)
                        * reciprocal;
                let material = world.materials
                    [index(pos.x.floor() as i32, pos.y.floor() as i32, SIZE)]
                    as usize;
                let detail = if matches!(material, 12..=14 | 19..=20) {
                    0.65 + super::geology::stone_texel(pos) as f32 * 0.035
                } else {
                    1.0
                };
                let color = mix(rgb(17, 23, 35), colors[material], (shade * detail).min(1.0));
                let amount = ((depth - fog.0) / (fog.1 - fog.0)).clamp(0.0, 1.0);
                self.pixels[at] = mix(color, fog.2, amount);
                self.depth[at] = depth;
            }
        }
    }
}
