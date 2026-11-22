use super::*;

impl Renderer {
    pub(super) fn trails(&mut self, flight: &Flight, camera: &Camera) {
        for i in (1..flight.trails.len()).rev() {
            for side in 0..2 {
                if let (Some((a, da, _)), Some((b, db, _))) = (
                    camera.project(flight.trails[i - 1][side]),
                    camera.project(flight.trails[i][side]),
                ) {
                    let steps = a.distance(b).ceil().clamp(1.0, self.w.max(100) as f32) as usize;
                    let age = i as f32 / flight.trails.len() as f32;
                    if age > 0.7 && i % 2 == 0 {
                        continue;
                    }
                    let color = mix(
                        flight.weather.shade(self.light.color(mix(
                            rgb(255, 255, 255),
                            rgb(120, 180, 218),
                            age,
                        ))),
                        self.light.haze(flight.weather),
                        if flight.space {
                            0.0
                        } else {
                            flight.weather.fog((da + db) * 0.5, flight.position.z)
                        },
                    );
                    for step in 0..=steps {
                        let t = step as f32 / steps as f32;
                        self.point(a.lerp(b, t), da + (db - da) * t, color, 0, 0);
                    }
                }
            }
        }
    }
    fn voxel_faces(
        &mut self,
        camera: &Camera,
        flight: &Flight,
        voxel: (f32, f32, usize),
        color: u32,
    ) {
        let (a, b, z) = voxel;
        let center = vec3(a, b, (z as f32 - 5.0) * flight.spread);
        let x = (a + 16.5).round() as i32;
        let y = (25.3 - b).round() as i32;
        let occupied = |x: i32, y: i32, z: i32| {
            (0..34).contains(&x)
                && (0..46).contains(&y)
                && (0..13).contains(&z)
                && MODEL[((z * 46 + y) * 34 + x) as usize] != 0
        };
        let (eye, ray) = camera.ray(camera.center);
        for (axis, u, v, delta) in [
            (Vec3::X, Vec3::Y, Vec3::Z, (1, 0, 0)),
            (Vec3::Y, Vec3::X, Vec3::Z, (0, -1, 0)),
            (Vec3::Z, Vec3::X, Vec3::Y, (0, 0, 1)),
        ] {
            for sign in [-1, 1] {
                if occupied(
                    x + sign * delta.0,
                    y + sign * delta.1,
                    z as i32 + sign * delta.2,
                ) && !(delta.2 != 0 && flight.spread > 1.1)
                {
                    continue;
                }
                let c = center + axis * sign as f32 * 0.5;
                let normal = flight.local(c + axis * sign as f32) - flight.local(c);
                let toward = if camera.q > 0.00001 {
                    eye - flight.local(c)
                } else {
                    -ray
                };
                if normal.dot(toward) <= 0.0 {
                    continue;
                }
                let corners = [
                    c - u * 0.5 - v * 0.5,
                    c + u * 0.5 - v * 0.5,
                    c + u * 0.5 + v * 0.5,
                    c - u * 0.5 + v * 0.5,
                ]
                .map(|p| flight.local(p));
                self.surface(
                    camera,
                    [corners[0], corners[1], corners[2]],
                    color,
                    false,
                    None,
                );
                self.surface(
                    camera,
                    [corners[0], corners[2], corners[3]],
                    color,
                    false,
                    None,
                );
            }
        }
    }
    fn paper_plane(&mut self, flight: &Flight, camera: &Camera) {
        let points = [
            vec3(0.0, 32.0, 0.0),
            vec3(-25.0, -19.0, 0.5),
            vec3(-3.0, -16.0, 5.0),
            vec3(0.0, -22.0, -6.0),
            vec3(3.0, -16.0, 5.0),
            vec3(25.0, -19.0, 0.5),
        ]
        .map(|p| flight.local(p));
        for (face, color) in [
            ([0, 1, 2], rgb(249, 245, 222)),
            ([0, 2, 3], rgb(181, 187, 183)),
            ([0, 3, 4], rgb(222, 227, 212)),
            ([0, 4, 5], rgb(255, 252, 238)),
            ([1, 3, 2], rgb(141, 153, 161)),
            ([3, 5, 4], rgb(203, 212, 211)),
        ] {
            self.surface(
                camera,
                face.map(|i| points[i]),
                self.light.color(color),
                false,
                None,
            );
        }
        for (a, b) in [(0, 1), (0, 2), (0, 4), (0, 5), (2, 3), (3, 4)] {
            self.world_line(
                camera,
                points[a],
                points[b],
                self.light.color(rgb(104, 123, 138)),
            );
        }
    }
    pub(super) fn jet(&mut self, flight: &Flight, camera: &Camera) {
        if flight.vehicle == crate::content::vehicles::Vehicle::Paper {
            self.paper_plane(flight, camera);
            return;
        }
        let [right, forward, up] = flight.axes();
        let shape = flight.vehicle.shape();
        let (right, forward, up) = (right * shape.x, forward * shape.y, up * shape.z);
        let (camera_eye, ray) = camera.ray(camera.center);
        let eye = if camera.q < 0.00001 {
            -ray.dot(up).signum() * 1e9
        } else {
            (camera_eye - flight.position).dot(up) / MODEL_SCALE
        };
        let mut order: Vec<usize> = (0..13).collect();
        order.sort_by(|a, b| {
            (((*b as f32 - 5.0) * flight.spread - eye).abs())
                .total_cmp(&((*a as f32 - 5.0) * flight.spread - eye).abs())
        });
        let mut palette = [0_u32; 96];
        for &(i, r, g, b) in JET {
            palette[i] = flight.weather.shade(mix(
                rgb(r, g, b),
                self.light.color(rgb(r, g, b)),
                0.4 + 0.53 * self.light.synthwave,
            ));
        }
        if flight.vehicle != crate::content::vehicles::Vehicle::Falcon {
            let [r, g, b] = flight.vehicle.tint();
            for color in &mut palette {
                *color = mix(*color, rgb(r, g, b), 0.28);
            }
        }
        let jet_depth = camera.project(flight.position).map_or(0.0, |p| p.1);
        let fog = if flight.space {
            0.0
        } else {
            flight.weather.fog(jet_depth, flight.position.z)
        };
        let haze = self.light.haze(flight.weather);
        for color in &mut palette {
            *color = mix(*color, haze, fog);
        }
        self.mask.fill(0);
        let center_scale = camera.project(flight.position).map_or(1.0, |p| p.2);
        let samples = (center_scale * MODEL_SCALE * 1.7).ceil().clamp(2.0, 6.0) as usize;
        let view_axis = |a: glam::Vec3| {
            vec2(
                a.dot(camera.right),
                -(a.dot(camera.up) + camera.slope * a.dot(camera.forward)),
            ) * MODEL_SCALE
        };
        let r = view_axis(right);
        let f = view_axis(forward);
        let mut offsets = [Vec2::ZERO; 36];
        for iy in 0..samples {
            for ix in 0..samples {
                offsets[iy * samples + ix] = r * ((ix as f32 + 0.5) / samples as f32 - 0.5)
                    + f * ((iy as f32 + 0.5) / samples as f32 - 0.5);
            }
        }
        let mut bounds = [self.w as i32, self.h as i32, 0, 0];
        for z in order {
            for voxel in 0..self.voxels[z].len() {
                let (a, b, color, exterior) = self.voxels[z][voxel];
                if !exterior && flight.spread <= 1.1 {
                    continue;
                }
                let center = flight.position
                    + (right * a + forward * b + up * ((z as f32 - 5.0) * flight.spread))
                        * MODEL_SCALE;
                if let Some((p, depth, scale)) = camera.project(center) {
                    bounds[0] = bounds[0].min(p.x as i32 - 4);
                    bounds[1] = bounds[1].min(p.y as i32 - 4);
                    bounds[2] = bounds[2].max(p.x as i32 + 4);
                    bounds[3] = bounds[3].max(p.y as i32 + 4);
                    if center_scale * MODEL_SCALE > 3.0 {
                        self.voxel_faces(camera, flight, (a, b, z), palette[color]);
                    } else {
                        // 同一体素的采样点常落在同一像素上，重复写入同色同深度没有效果，跳过即可
                        let color = self.fogged_scenery(palette[color], depth);
                        let mut drawn = [(i32::MIN, i32::MIN); 36];
                        let mut count = 0;
                        for offset in &offsets[..samples * samples] {
                            let q = p + *offset * scale;
                            let pixel = (q.x.round() as i32, q.y.round() as i32);
                            if drawn[..count].contains(&pixel) {
                                continue;
                            }
                            drawn[count] = pixel;
                            count += 1;
                            self.plot(pixel.0, pixel.1, depth, color, 1);
                        }
                    }
                }
            }
        }
        let center_depth = camera.project(flight.position).map_or(0.0, |p| p.1);
        for y in bounds[1].max(1) as usize..bounds[3].clamp(1, self.h as i32 - 1) as usize {
            for x in bounds[0].max(1) as usize..bounds[2].clamp(1, self.w as i32 - 1) as usize {
                let at = x * self.h + y;
                if self.mask[at] == 0
                    && self.depth[at] >= center_depth - 2.0
                    && [at - 1, at + 1, at - self.h, at + self.h]
                        .iter()
                        .any(|&n| self.mask[n] == 1)
                {
                    self.pixels[at] = rgb(13, 15, 26);
                }
            }
        }
        if flight.vehicle == crate::content::vehicles::Vehicle::Wasp {
            for side in [-1.0, 1.0] {
                let hub = flight.local(vec3(side * 17.0, 0.0, 5.0));
                for blade in 0..3 {
                    let a = flight.time * 42.0 + blade as f32 * std::f32::consts::TAU / 3.0;
                    let tip = hub + (right * a.cos() + forward * a.sin()) * 7.0;
                    if let (Some((p, d, _)), Some((q, _, _))) =
                        (camera.project(hub), camera.project(tip))
                    {
                        self.stroke(p, q, d - 1.0, rgb(202, 225, 187));
                    }
                }
            }
        }
        if flight.spread < 2.5 {
            let length = (9.0 + flight.boost * 19.0) * (0.9 + 0.1 * (flight.time * 47.0).sin());
            for engine in [-1.0, 1.0] {
                for step in (0..(length * 3.0) as usize).rev() {
                    let along = step as f32 / 3.0;
                    let f = along / length;
                    let center = flight.local(vec3(
                        engine * 1.96 - flight.rate * along * along * 0.015,
                        -18.3 - along,
                        -0.9,
                    ));
                    if let Some((p, depth, scale)) = camera.project(center) {
                        let radius = (1.1 * (1.0 - f).sqrt() * scale * MODEL_SCALE).round() as i32;
                        self.point(
                            p,
                            depth,
                            mix(rgb(255, 238, 158), rgb(242, 77, 17), f),
                            radius,
                            2,
                        );
                        if f < 0.55 {
                            self.point(p, depth - 0.1, rgb(255, 253, 224), (radius - 1).max(0), 2);
                        }
                    }
                }
            }
        }
    }
}
