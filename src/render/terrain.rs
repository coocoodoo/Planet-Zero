use super::*;

impl Renderer {
    pub(super) fn span(
        &mut self,
        x: usize,
        y0: i32,
        y1: i32,
        color: u32,
        depth: f32,
        checker: bool,
    ) {
        for y in y0.max(self.visible[x].0)..y1.min(self.visible[x].1) {
            let i = x * self.h + y as usize;
            if self.pixels[i] == 0 && !(checker && (x + self.x_origin + y as usize) & 1 == 0) {
                self.pixels[i] = color;
                self.depth[i] = depth;
            }
        }
    }
    pub(super) fn world(&mut self, world: &World, flight: &Flight, camera: &Camera, theme: usize) {
        let (s, c) = camera.yaw.sin_cos();
        let q = camera.q;
        let scale = camera.scale;
        let slope = camera.slope;
        let altitude = camera.position.z;
        let cx = camera.center.x;
        let cy = camera.center.y;
        let weather = if world.kind.vacuum() {
            Weather::Clear
        } else {
            flight.weather
        };
        let flash = weather.flash(flight.time);
        let lighting = |color| {
            mix(
                weather.shade(grade(color, theme)),
                rgb(225, 234, 255),
                flash,
            )
        };
        let top = TOP.map(|v| lighting(rgb(v[0], v[1], v[2])));
        let sides = SIDE.map(|v| lighting(rgb(v[0], v[1], v[2])));
        let shadow_color = rgb(13, 26, 56);
        let haze = if world.kind.vacuum() {
            rgb(8, 10, 18)
        } else {
            mix(weather.haze(theme), rgb(225, 234, 255), flash)
        };
        let mut colors = [[[[0_u32; 2]; MATERIAL_COUNT + 2]; 2]; 11];
        let mut clouds = [[0_u32; 9]; 11];
        for fog in 0..=10 {
            let amount = fog as f32 / 10.0;
            for (shade, materials) in colors[fog].iter_mut().enumerate() {
                for (material, faces) in materials.iter_mut().enumerate() {
                    let color = match material {
                        MATERIAL_COUNT => lighting(rgb(240, 252, 255)),
                        m if m == MATERIAL_COUNT + 1 => lighting(rgb(160, 211, 244)),
                        _ => top[material],
                    };
                    let side = if !(4..MATERIAL_COUNT).contains(&material) {
                        color
                    } else {
                        sides[material]
                    };
                    for (face, base) in [color, side].into_iter().enumerate() {
                        faces[face] = mix(
                            if shade == 1 {
                                mix(base, shadow_color, 0.33)
                            } else {
                                base
                            },
                            haze,
                            amount,
                        );
                    }
                }
            }
            for (k, cloud_color) in clouds[fog].iter_mut().enumerate() {
                let f = k as f32 / 7.0;
                let base = match k {
                    8 => rgb(255, 255, 255),
                    7 => rgb(138, 160, 200),
                    _ => rgb(
                        (158.0 + 44.0 * f) as u8,
                        (182.0 + 36.0 * f) as u8,
                        (222.0 + 18.0 * f) as u8,
                    ),
                };
                *cloud_color = mix(lighting(base), haze, amount);
            }
        }
        let below = self.h as f32 - cy;
        let nearest = if q > 0.00001 { -0.85 / q } else { -1e8 };
        let camera_height = if q > 0.00001 {
            altitude + slope / q
        } else {
            1e8
        };
        let dn = scale * slope + q * below;
        let mut distance = if camera_height > MAX_HEIGHT as f32 + 1.0 && dn > 0.00001 {
            ((scale * (altitude - MAX_HEIGHT as f32) - below) / dn - 2.0).max(nearest)
        } else {
            nearest
        };
        let (fog_start, view_distance) = if world.kind.vacuum() {
            (2400.0, 2600.0)
        } else {
            weather.visibility(altitude)
        };
        let above = cy + 2.0;
        let end = if scale * slope > q * above {
            view_distance.min((scale * altitude + above) / (scale * slope - q * above))
        } else {
            view_distance
        };
        let mut rows = [0_i32; MAX_HEIGHT + 1];
        for (z, row) in rows.iter_mut().enumerate() {
            *row = (cy - scale / (1.0 + q * distance) * (slope * distance - altitude + z as f32))
                .floor() as i32;
        }
        let shadows: Vec<_> = std::iter::once(flight)
            .chain(flight.wingmen.iter().take(flight.wing_count))
            .map(|f| {
                let (js, jc) = f.yaw.sin_cos();
                (
                    f.position + vec3(0.5, 0.32, 0.0) * f.position.z,
                    js,
                    jc,
                    1.0 / (MODEL_SCALE
                        * f.vehicle.shape().x
                        * (f.bank + f.spin).cos().abs().max(0.3)),
                    1.0 / (MODEL_SCALE * f.vehicle.shape().y),
                )
            })
            .collect();
        for _ in 0..4000 {
            if distance >= end {
                break;
            }
            let denominator = 1.0 + q * distance;
            let step = (denominator * denominator / (scale * (slope + q * altitude)))
                .min(2.5 * denominator / scale)
                .max(0.5);
            distance += step;
            let denominator = 1.0 + q * distance;
            let m = scale / denominator;
            let previous = rows;
            for (z, row) in rows.iter_mut().enumerate() {
                *row = (cy - m * (slope * distance - altitude + z as f32) + 0.00001).floor() as i32;
            }
            let inverse = denominator / scale;
            let ax = c * inverse;
            let ay = -s * inverse;
            let mut wx = camera.position.x - cx * ax - distance * s;
            let mut wy = camera.position.y - cx * ay - distance * c;
            let fog = if distance > fog_start {
                (((distance - fog_start) / (view_distance - fog_start)).sqrt() * 10.0).round()
                    / 10.0
            } else {
                0.0
            };
            let fog_index = ((fog * 10.0) as usize).min(10);
            for x in 0..self.w {
                if self.visible[x].0 >= self.visible[x].1 || self.column[x] <= self.visible[x].0 {
                    wx += ax;
                    wy += ay;
                    continue;
                }
                let cloud = world.cloud(wx, wy, flight.time);
                if cloud > 0 {
                    let z = CLOUD_ALT + cloud - 1;
                    let color = clouds[fog_index][8];
                    self.span(x, rows[z], previous[z], color, distance, cloud == 1);
                    self.span(
                        x,
                        previous[CLOUD_ALT],
                        rows[CLOUD_ALT],
                        clouds[fog_index][7],
                        distance,
                        cloud == 1,
                    );
                    for k in (0..cloud - 1).rev() {
                        self.span(
                            x,
                            previous[CLOUD_ALT + k + 1],
                            previous[CLOUD_ALT + k],
                            clouds[fog_index][k],
                            distance,
                            false,
                        );
                    }
                }
                let gx = wx.floor() as i32;
                let gy = wy.floor() as i32;
                let at = index(gx, gy, SIZE);
                let h = world.heights[at] as usize;
                let mut highest = self.column[x];
                if rows[h] < highest || previous[h] < highest {
                    let mut material = world.materials[at] as usize;
                    let mut shade = world.cloud(wx + 20.0, wy + 12.8, flight.time) > 1;
                    for &(shadow, js, jc, bank_scale, length_scale) in &shadows {
                        let dx = wx - shadow.x + 0.5 * h as f32;
                        let dy = wy - shadow.y + 0.32 * h as f32;
                        if dx.abs() < 35.0 && dy.abs() < 35.0 {
                            let a = (dx * jc + dy * js) * bank_scale;
                            let b = (dx * js - dy * jc) * length_scale;
                            let ix = (a + 17.0).floor() as i32;
                            let iy = (25.8 - b).floor() as i32;
                            if (0..34).contains(&ix) && (0..46).contains(&iy) {
                                shade |= self.silhouette[iy as usize * 34 + ix as usize];
                            }
                        }
                    }
                    if material == 3
                        && hash((at as u32).wrapping_add((flight.time * 36.0) as u32)) < 120
                    {
                        material = MATERIAL_COUNT;
                    } else if material < 3
                        && inverse < 1.8
                        && hash(index((gx + (flight.time * 6.0) as i32) >> 2, gy, 256) as u32) > 247
                    {
                        material = MATERIAL_COUNT + 1;
                    }
                    let [color, side] = colors[fog_index][usize::from(shade)][material];
                    if previous[h] < highest {
                        self.span(x, previous[h], highest, side, distance, false);
                        highest = previous[h].max(self.visible[x].0);
                    }
                    if rows[h] < highest {
                        self.span(x, rows[h], highest, color, distance, false);
                        highest = rows[h];
                    }
                    self.column[x] = highest;
                }
                wx += ax;
                wy += ay;
            }
        }
    }
    pub(super) fn sky(&mut self, camera: &Camera, flight: &Flight, theme: usize, height: usize) {
        let horizon = if camera.q > 0.00001 {
            camera.center.y - camera.scale * camera.slope / camera.q
        } else {
            -1e8
        };
        let (low, high) = match theme {
            1 => (rgb(255, 198, 132), rgb(84, 68, 150)),
            2 => (rgb(36, 56, 110), rgb(7, 10, 34)),
            _ => (rgb(198, 232, 250), rgb(62, 142, 222)),
        };
        let weather = flight.weather;
        let (low, high) = if weather == Weather::Clear {
            (low, high)
        } else {
            let cover = match weather {
                Weather::Fog => 0.92,
                Weather::Sandstorm => 0.85,
                Weather::Snow => 0.68,
                Weather::Rain => 0.55,
                _ => 0.4,
            };
            (
                weather.haze(theme),
                mix(weather.shade(high), weather.haze(theme), cover),
            )
        };
        let flash = weather.flash(flight.time);
        for y in 0..self.h {
            let gradient = ((horizon - y as f32) / (height as f32 * 0.75)).clamp(0.0, 1.0);
            let color = mix(
                mix(low, high, (gradient * 12.0).floor() / 12.0),
                rgb(225, 234, 255),
                flash,
            );
            for x in 0..self.w {
                let at = x * self.h + y;
                if (y as i32) >= self.visible[x].0
                    && (y as i32) < self.visible[x].1
                    && self.pixels[at] == 0
                {
                    self.pixels[at] = if theme == 2
                        && weather == Weather::Clear
                        && gradient > 0.2
                        && hash(
                            ((x + self.x_origin) as u32)
                                .wrapping_add((camera.yaw * 300.0) as i32 as u32)
                                ^ (y as u32 * 8191),
                        ) == 255
                        && (x + self.x_origin).is_multiple_of(3)
                    {
                        rgb(182, 202, 232)
                    } else {
                        color
                    };
                }
            }
        }
    }
}
