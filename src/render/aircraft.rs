use super::*;

impl Renderer {
    pub(super) fn trails(&mut self, flight: &Flight, camera: &Camera, theme: usize) {
        for i in (1..flight.trails.len()).rev() {
            for side in 0..2 {
                if let (Some((a, da, _)), Some((b, db, _))) = (
                    camera.project(flight.trails[i - 1][side]),
                    camera.project(flight.trails[i][side]),
                ) {
                    let steps = a.distance(b).ceil().clamp(1.0, 100.0) as usize;
                    let age = i as f32 / flight.trails.len() as f32;
                    if age > 0.7 && i % 2 == 0 {
                        continue;
                    }
                    let color = mix(
                        flight.weather.shade(grade(
                            mix(rgb(255, 255, 255), rgb(120, 180, 218), age),
                            theme,
                        )),
                        flight.weather.haze(theme),
                        flight.weather.fog((da + db) * 0.5, flight.position.z),
                    );
                    for step in 0..=steps {
                        let t = step as f32 / steps as f32;
                        self.point(a.lerp(b, t), da + (db - da) * t, color, 0, 0);
                    }
                }
            }
        }
    }
    pub(super) fn jet(&mut self, flight: &Flight, camera: &Camera, theme: usize) {
        let [right, forward, up] = flight.axes();
        let shape = flight.vehicle.shape();
        let (right, forward, up) = (right * shape.x, forward * shape.y, up * shape.z);
        let (s, c) = camera.yaw.sin_cos();
        let up_depth = -(up.x * s + up.y * c);
        let eye = if camera.q < 0.00001 {
            if camera.slope * up.z - up_depth >= 0.0 {
                1e9
            } else {
                -1e9
            }
        } else {
            let eye = camera.position + vec3(s / camera.q, c / camera.q, camera.slope / camera.q);
            (eye - flight.position).dot(up) / MODEL_SCALE
        };
        let mut order: Vec<usize> = (0..13).collect();
        order.sort_by(|a, b| {
            (((*b as f32 - 5.0) * flight.spread - eye).abs())
                .total_cmp(&((*a as f32 - 5.0) * flight.spread - eye).abs())
        });
        let mut palette = [0_u32; 96];
        for &(i, r, g, b) in JET {
            palette[i] = flight
                .weather
                .shade(mix(rgb(r, g, b), grade(rgb(r, g, b), theme), 0.4));
        }
        if flight.vehicle != crate::content::vehicles::Vehicle::Falcon {
            let [r, g, b] = flight.vehicle.tint();
            for color in &mut palette {
                *color = mix(*color, rgb(r, g, b), 0.28);
            }
        }
        let jet_depth = camera.project(flight.position).map_or(0.0, |p| p.1);
        let fog = flight.weather.fog(jet_depth, flight.position.z);
        for color in &mut palette {
            *color = mix(*color, flight.weather.haze(theme), fog);
        }
        self.mask.fill(0);
        let center_scale = camera.project(flight.position).map_or(1.0, |p| p.2);
        let samples = (center_scale * MODEL_SCALE * 1.7).ceil().clamp(2.0, 6.0) as usize;
        let view_axis = |a: glam::Vec3| {
            vec2(a.x * c - a.y * s, -a.z + camera.slope * (a.x * s + a.y * c)) * MODEL_SCALE
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
                let (a, b, color) = self.voxels[z][voxel];
                let center = flight.position
                    + (right * a + forward * b + up * ((z as f32 - 5.0) * flight.spread))
                        * MODEL_SCALE;
                if let Some((p, depth, scale)) = camera.project(center) {
                    bounds[0] = bounds[0].min(p.x as i32 - 4);
                    bounds[1] = bounds[1].min(p.y as i32 - 4);
                    bounds[2] = bounds[2].max(p.x as i32 + 4);
                    bounds[3] = bounds[3].max(p.y as i32 + 4);
                    for offset in &offsets[..samples * samples] {
                        self.point(p + *offset * scale, depth, palette[color], 0, 1);
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
