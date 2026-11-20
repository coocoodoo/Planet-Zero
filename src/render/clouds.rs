use super::*;
use crate::world::Cloud;
use glam::Vec4;

#[derive(Clone, Copy)]
pub(super) struct CloudSurface<'a> {
    // 厚度、顶点亮度和细节纹理坐标
    pub attributes: [Vec4; 3],
    pub palette: &'a [u32; 8],
    pub pixel_texture: bool,
    pub relief: &'a [[i8; 4]],
    pub relief_light: Vec4,
}

#[derive(Clone, Copy)]
pub(super) struct CloudPixel {
    pub depth: f32,
    pub color: u32,
    pub opacity: f32,
}

impl Default for CloudPixel {
    fn default() -> Self {
        Self {
            depth: f32::INFINITY,
            color: 0,
            opacity: 0.0,
        }
    }
}

impl Renderer {
    pub(super) fn draw_clouds(&mut self, world: &World, flight: &Flight, camera: &Camera) {
        let opacity = (1.0 - self.warp.map_or(0.0, |warp| warp.bend)).powi(4)
            * (1.0 - crate::world::orbit::thin_air(flight.position.z));
        if world.clouds.is_empty() || opacity < 0.001 {
            return;
        }
        self.cloud_pixels
            .resize(self.pixels.len(), CloudPixel::default());
        let layer_strength = flight.weather.weight(Weather::Cloudy)
            + flight.weather.weight(Weather::Storm)
            + flight.weather.weight(Weather::Rain) * 0.7
            + flight.weather.weight(Weather::Snow) * 0.6;
        let coverage = flight.weather.coverage() - layer_strength * 5.0;
        let mut immersion = 0.0_f32;
        let far = flight.weather.visibility(camera.position.z).1.min(1000.0);
        let drift = vec2(flight.time * 7.0, flight.time * 3.0);
        let haze = self.light.haze(flight.weather);
        let dark = flight.weather.shade(self.light.color(rgb(145, 172, 198)));
        let light = flight.weather.shade(self.light.color(rgb(246, 249, 250)));
        let planes = [
            (camera.right * camera.scale, camera.center.x),
            (
                -camera.right * camera.scale,
                self.w as f32 - camera.center.x,
            ),
            (
                -camera.up * camera.scale - camera.forward * camera.scale * camera.slope,
                camera.center.y,
            ),
            (
                camera.up * camera.scale + camera.forward * camera.scale * camera.slope,
                self.h as f32 - camera.center.y,
            ),
        ]
        .map(|(axis, offset)| (axis + camera.forward * offset * camera.q, offset));
        let (eye, ray) = camera.ray(camera.center);
        let mut visible = Vec::new();
        for cloud in &world.clouds {
            let strength = if cloud.shape.layer {
                layer_strength * opacity
            } else {
                (coverage - cloud.rank as f32).clamp(0.0, 1.0) * opacity
            };
            if strength < 0.01 {
                continue;
            }
            let scale = if cloud.shape.layer {
                cloud.scale
            } else {
                strength.sqrt() * cloud.scale
            };
            let extent = cloud.extent();
            let mut xy = cloud.center + drift;
            xy += ((camera.position.truncate() - xy) / crate::world::CLOUD_SIZE as f32).round()
                * crate::world::CLOUD_SIZE as f32;
            if xy.distance_squared(camera.position.truncate()) > (far + extent.length()).powi(2) {
                continue;
            }
            let thickness = if cloud.shape.thin { 0.12 } else { 1.0 };
            let center = xy.extend(if cloud.shape.layer {
                230.0
            } else {
                world.cloud_height(xy.x, xy.y) - if cloud.shape.thin { 9.0 } else { 0.0 }
            });
            if cloud.shape.layer && flight.dynamic_clouds {
                let local = eye - center;
                if let Some((bottom, top)) = cloud.shape.vertical_bounds(local.truncate()) {
                    let edge = (local.z - bottom).min(top - local.z);
                    immersion = immersion.max((edge / 6.0).clamp(0.0, 1.0) * strength);
                }
            }
            let half = (cloud.shape.extent + Vec2::splat(2.0)).extend(if cloud.shape.layer {
                50.0
            } else {
                18.0 * thickness
            }) * scale;
            let delta = center - camera.position;
            if planes
                .iter()
                .any(|(normal, offset)| delta.dot(*normal) + offset + normal.abs().dot(half) < 0.0)
            {
                continue;
            }
            let depth = delta.dot(camera.forward);
            if depth - half.dot(camera.forward.abs()) > far
                || 1.0 + camera.q * (depth + half.dot(camera.forward.abs())) < 0.12
            {
                continue;
            }
            let projected_cell =
                camera.scale * Cloud::CELL * scale / (1.0 + camera.q * depth).max(0.12);
            let level = if !flight.dynamic_clouds {
                0
            } else if cloud.shape.layer {
                1
            } else if projected_cell > 4.0 {
                2
            } else if projected_cell > 1.5 {
                1
            } else {
                0
            };
            visible.push((cloud, center, scale, level, depth));
        }
        visible.sort_unstable_by(|a, b| b.4.total_cmp(&a.4));
        for (cloud, center, scale, level, depth) in visible {
            let fog = ((depth - far * 0.7) / (far * 0.3)).clamp(0.0, 1.0);
            let palette: [u32; 8] =
                std::array::from_fn(|i| mix(mix(dark, light, i as f32 / 7.0), haze, fog));
            let local_eye = (eye - center) / scale;
            for face in &cloud.shape.faces[level] {
                let toward = if camera.q > 0.00001 {
                    local_eye - face.points[0]
                } else {
                    -ray
                };
                if if flight.dynamic_clouds {
                    face.normal.dot(toward) <= 0.0
                } else {
                    face.normal.z <= 0.0
                } {
                    continue;
                }
                self.surface(
                    camera,
                    face.points.map(|p| {
                        center
                            + if flight.dynamic_clouds {
                                p
                            } else {
                                p.with_z(0.0)
                            } * scale
                    }),
                    0,
                    false,
                    Some(CloudSurface {
                        attributes: std::array::from_fn(|i| {
                            let normal =
                                Vec3::from_array(face.shade_normals[i].map(|v| v as f32 / 127.0));
                            let shade =
                                (normal.dot(self.light.direction) * 0.7 + 0.3).clamp(0.0, 1.0);
                            let thickness = face.thickness[i]
                                * scale
                                * if cloud.shape.layer {
                                    layer_strength * opacity
                                } else {
                                    1.0
                                };
                            let texture = (face.points[i].truncate() + cloud.shape.extent) / 4.0;
                            Vec4::new(thickness, shade, texture.x, texture.y)
                        }),
                        palette: &palette,
                        pixel_texture: flight.cloud_pixel_texture,
                        relief: if flight.dynamic_clouds && face.normal.z > 0.0 {
                            &cloud.shape.relief
                        } else {
                            &[]
                        },
                        relief_light: (self.light.direction * 0.7).extend(-0.18) / 127.0,
                    }),
                );
            }
            for at in self.cloud_touched.drain(..) {
                let sample = &mut self.cloud_pixels[at];
                self.pixels[at] = mix(self.pixels[at], sample.color, sample.opacity);
                sample.depth = f32::INFINITY;
            }
        }
        if immersion > 0.0 {
            let color = mix(dark, light, 0.45);
            for (x, &(y0, y1)) in self.visible.iter().enumerate() {
                for y in y0..y1 {
                    let at = x * self.h + y as usize;
                    let distance =
                        self.depth[at] + if camera.q > 0.0 { 1.0 / camera.q } else { 0.0 };
                    let fog = (distance / 90.0).clamp(0.0, 0.94) * immersion;
                    self.pixels[at] = mix(self.pixels[at], color, fog);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cloud_sides_transmit_background_and_respect_foreground_depth() {
        let mut world = World::generate(crate::world::MapKind::Islands, 7);
        let mut density = vec![-9.0; 9 * 9];
        for y in 2..7 {
            for x in 2..7 {
                density[y * 9 + x] = 36.0;
            }
        }
        world.clouds = vec![Cloud {
            center: vec2(100.0, 100.0),
            scale: 1.0,
            rank: 0,
            shape: std::sync::Arc::new(crate::world::CloudShape::new(9, &density, false)),
        }];
        world.cloud_levels.fill(110);
        world.cloud_slopes.fill([110.0, 0.0, 0.0, 0.0]);
        let mut flight = Flight::new(&world);
        flight.position = vec3(100.0, 200.0, 114.0);
        flight.follow = flight.position;
        flight.yaw = 0.0;
        flight.weights = [0.0, 1.0, 0.0];
        let mut camera = Camera::new(&flight, 320, 170);
        camera.slope = 0.0;
        let mut renderer = Renderer::single();
        renderer.w = 320;
        renderer.h = 170;
        renderer.full_width = 320;
        renderer.visible = vec![(0, 170); 320];
        renderer.pixels = vec![0; 320 * 170];
        renderer.mask = vec![0; 320 * 170];
        renderer.depth = vec![f32::INFINITY; 320 * 170];
        renderer.draw_clouds(&world, &flight, &camera);
        let at = camera.center.x as usize * 170 + camera.center.y as usize;
        let on_black = renderer.pixels[at];
        assert_ne!(on_black, 0);
        assert_eq!(renderer.depth[at], f32::INFINITY);
        let background = rgb(40, 80, 160);
        renderer.pixels.fill(background);
        renderer.draw_clouds(&world, &flight, &camera);
        assert_ne!(renderer.pixels[at], on_black);
        assert_ne!(renderer.pixels[at], background);
        renderer.pixels.fill(background);
        renderer.depth.fill(0.0);
        renderer.draw_clouds(&world, &flight, &camera);
        assert!(renderer.pixels.iter().all(|&pixel| pixel == background));
    }
}
