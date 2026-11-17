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
        let [left, right, top, bottom] = [
            camera.center.x,
            self.w as f32 - camera.center.x,
            camera.center.y,
            self.h as f32 - camera.center.y,
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
            let point = |dx, dy| {
                let (wx, wy) = (x + dx, y + dy);
                vec3(
                    wx as f32,
                    wy as f32,
                    world.heights[index(wx, wy, SIZE)] as f32,
                )
            };
            let corners = [
                point(0, 0),
                point(size, 0),
                point(size, size),
                point(0, size),
            ];
            for vertices in [
                [corners[0], corners[1], corners[2]],
                [corners[0], corners[2], corners[3]],
            ] {
                self.ground_triangle(camera, world, vertices, &colors, (fog_start, far, haze));
            }
        }
        if !world.kind.vacuum() && crate::content::maps::definition(world.kind).cloud_clusters > 0 {
            self.tilted_clouds(world, flight, camera, theme, far);
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
        for x in min.x as i32..=max.x as i32 {
            let y0 = (min.y as i32).max(self.visible[x as usize].0);
            let y1 = (max.y as i32 + 1).min(self.visible[x as usize].1);
            for y in y0..y1 {
                let p = vec2(x as f32, y as f32);
                let u = (b - p).perp_dot(c - p) / area;
                let v = (c - p).perp_dot(a - p) / area;
                let w = 1.0 - u - v;
                if u < 0.0 || v < 0.0 || w < 0.0 {
                    continue;
                }
                let weights = vec3(u * inverse[0], v * inverse[1], w * inverse[2]);
                let denominator = weights.element_sum();
                let depth = (1.0 / denominator - 1.0) / camera.q;
                let at = x as usize * self.h + y as usize;
                if depth >= self.depth[at] || self.clip_depth.get(at).is_some_and(|d| depth < *d) {
                    continue;
                }
                let pos =
                    (vertices[0] * weights.x + vertices[1] * weights.y + vertices[2] * weights.z)
                        / denominator;
                let material = world.materials
                    [index(pos.x.floor() as i32, pos.y.floor() as i32, SIZE)]
                    as usize;
                let color = mix(rgb(17, 23, 35), colors[material], shade);
                let amount = ((depth - fog.0) / (fog.1 - fog.0)).clamp(0.0, 1.0);
                self.pixels[at] = mix(color, fog.2, amount);
                self.depth[at] = depth;
            }
        }
    }
    fn tilted_clouds(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
        far: f32,
    ) {
        if camera.q <= 0.00001 {
            return;
        }
        for x in 0..self.w {
            let (origin, mut ray) = camera.ray(vec2(x as f32, self.visible[x].0 as f32));
            let step = camera.up * (-camera.q / camera.scale);
            for y in self.visible[x].0..self.visible[x].1 {
                let direction = ray;
                ray += step;
                if direction.z.abs() < 0.00001 {
                    continue;
                }
                let t = (CLOUD_ALT as f32 + 4.0 - origin.z) / direction.z;
                if t < 0.12 / camera.q || t > far + 1.0 / camera.q {
                    continue;
                }
                let p = origin + direction * t;
                let cloud = world.cloud(p.x, p.y, flight.time);
                if cloud == 0 || (cloud == 1 && (x + y as usize).is_multiple_of(2)) {
                    continue;
                }
                let t = (CLOUD_ALT as f32 + cloud as f32 - origin.z) / direction.z;
                let depth = t - 1.0 / camera.q;
                let at = x * self.h + y as usize;
                if depth >= self.depth[at] || self.clip_depth.get(at).is_some_and(|d| depth < *d) {
                    continue;
                }
                self.pixels[at] = flight.weather.shade(grade(
                    if direction.z < 0.0 {
                        rgb(240, 248, 255)
                    } else {
                        rgb(153, 180, 214)
                    },
                    theme,
                ));
                self.depth[at] = depth;
            }
        }
    }
}
