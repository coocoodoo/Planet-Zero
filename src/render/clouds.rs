use super::*;

impl Renderer {
    pub(super) fn draw_clouds(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
    ) {
        if world.kind.vacuum() || crate::content::maps::definition(world.kind).cloud_clusters == 0 {
            return;
        }
        let opacity = (1.0 - self.warp.map_or(0.0, |warp| warp.bend)).powi(4)
            * (1.0 - crate::world::orbit::smooth((flight.position.z - 500.0) / 500.0));
        if opacity < 0.001 {
            return;
        }
        let coverage = match flight.weather {
            Weather::Cloudy | Weather::Storm => 14,
            Weather::Rain | Weather::Snow => 11,
            Weather::Rainbow => 9,
            _ => 7,
        };
        let far = flight.weather.visibility(camera.position.z).1;
        let base = world.cloud_height(camera.position.x, camera.position.y);
        let low = world.cloud_levels.iter().copied().min().unwrap_or(110) as f32 + 4.0;
        let high = world.cloud_levels.iter().copied().max().unwrap_or(210) as f32 + 4.0;
        let near = if camera.q > 0.00001 {
            -0.88 / camera.q
        } else {
            -1e8
        }
        .max(self.near_depth);
        let upper = flight.weather.shade(grade(rgb(240, 248, 255), theme));
        let lower = flight.weather.shade(grade(rgb(153, 180, 214), theme));
        let rows: Vec<_> = (0..self.h)
            .map(|y| {
                let (origin, ray) = camera.ray(vec2(0.0, y as f32));
                let inverse_z = if ray.z.abs() >= 0.0001 {
                    1.0 / ray.z
                } else {
                    0.0
                };
                let initial = (origin - camera.position).dot(camera.forward);
                let step = ray.dot(camera.forward) * inverse_z;
                let a = initial + (low - origin.z) * step;
                let b = initial + (high - origin.z) * step;
                (origin, ray, inverse_z, a.min(b), a.max(b))
            })
            .collect();
        let perspective = camera.q > 0.00001;
        for x in 0..self.w {
            let horizontal = camera.right * (x as f32 / camera.scale);
            let origin_shift = if perspective { Vec3::ZERO } else { horizontal };
            let ray_shift = if perspective {
                horizontal * camera.q
            } else {
                Vec3::ZERO
            };
            for y in self.visible[x].0..self.visible[x].1 {
                let (origin, ray, inverse_z, low_depth, high_depth) = rows[y as usize];
                let at = x * self.h + y as usize;
                if inverse_z == 0.0 || high_depth < near || low_depth >= far.min(self.depth[at]) {
                    continue;
                }
                let origin = origin + origin_shift;
                let ray = ray + ray_shift;
                let mut t = (base + 4.0 - origin.z) * inverse_z;
                for _ in 0..2 {
                    let p = origin + ray * t;
                    t = (world.cloud_height(p.x, p.y) + 4.0 - origin.z) * inverse_z;
                }
                let p = origin + ray * t;
                let depth = (p - camera.position).dot(camera.forward);
                if 1.0 + camera.q * depth < 0.12
                    || depth > far
                    || depth >= self.depth[at]
                    || depth < self.near_depth
                {
                    continue;
                }
                let cloud = world.cloud_cover(p.x, p.y, flight.time, coverage);
                if cloud == 0 || (cloud == 1 && (x + self.x_origin + y as usize).is_multiple_of(2))
                {
                    continue;
                }
                let ceiling = world.height(p.x, p.y);
                if p.z - ceiling < 12.0 {
                    continue;
                }
                let background = if self.pixels[at] == 0 {
                    flight.weather.haze(theme)
                } else {
                    self.pixels[at]
                };
                self.pixels[at] = mix(background, if ray.z < 0.0 { upper } else { lower }, opacity);
                self.depth[at] = depth;
            }
        }
    }
}
