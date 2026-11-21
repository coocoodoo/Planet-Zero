use super::*;

impl Renderer {
    pub(super) fn world(&mut self, world: &World, flight: &Flight, camera: &Camera) {
        let started = std::time::Instant::now();
        self.pitched_terrain(world, flight.faceted, camera, flight.weather);
        self.detail[0] = started.elapsed().as_secs_f64() * 1000.0;
    }
    pub(super) fn sky(&mut self, world: &World, camera: &Camera, flight: &Flight) {
        let weather = flight.weather;
        let skies = [
            (rgb(198, 232, 250), rgb(62, 142, 222)),
            (rgb(255, 198, 132), rgb(84, 68, 150)),
            (rgb(36, 56, 110), rgb(7, 10, 34)),
        ];
        let skies: [_; 3] = std::array::from_fn(|i| weather.sky(skies[i].0, skies[i].1, i));
        let low = self.light.blend(skies.map(|s| s.0));
        let high = self.light.blend(skies.map(|s| s.1));
        let night = self.light.night();
        let flash = weather.flash(flight.time);
        let sun_azimuth = self.light.direction.with_z(0.0).normalize_or_zero();
        let twilight = self.light.twilight()
            * (1.0
                - weather.weight(Weather::Fog) * 0.9
                - weather.weight(Weather::Storm) * 0.85
                - weather.weight(Weather::Rain) * 0.55
                - weather.weight(Weather::Cloudy) * 0.28)
            * 0.4;
        // 渐变、晨昏光和闪电只随行变化，先逐行算好
        let rows: Vec<(u32, f32, f32)> = (0..self.h)
            .map(|y| {
                let (_, ray) = camera.ray(vec2(camera.center.x, y as f32));
                let elevation = ray.z / ray.length();
                let gradient = (elevation * 1.4).clamp(0.0, 1.0);
                let glow = (1.0 - elevation.abs() / 0.26).max(0.0).powi(2) * twilight;
                let gradient = if flight.smooth_sky_gradient {
                    gradient
                } else {
                    (gradient * 12.0).floor() / 12.0
                };
                let color = mix(mix(low, high, gradient), rgb(225, 234, 255), flash);
                (color, glow, gradient)
            })
            .collect();
        let stars = night > 0.0 && weather.weight(Weather::Clear) > 0.95;
        let rainbow = weather.weight(Weather::Rainbow);
        let rainbow = if night < 1.0 { rainbow } else { 0.0 };
        let axis = vec3(0.3, -0.95, -0.08).normalize();
        const RAINBOW: [u32; 7] = [
            rgb(248, 99, 88),
            rgb(255, 163, 80),
            rgb(255, 221, 104),
            rgb(126, 210, 142),
            rgb(97, 174, 237),
            rgb(126, 133, 226),
            rgb(185, 132, 221),
        ];
        let yaw_seed = (camera.yaw * 300.0) as i32 as u32;
        // 画布按列存储，逐列遍历可见区间以顺序访问内存
        for x in 0..self.w {
            let column = x * self.h;
            let star_column = (x + self.x_origin).is_multiple_of(3);
            let star_seed = ((x + self.x_origin) as u32).wrapping_add(yaw_seed);
            for y in span(self.visible[x].0, self.visible[x].1, self.h) {
                let at = column + y;
                if self.pixels[at] != 0 || self.depth[at] != f32::INFINITY {
                    continue;
                }
                let (color, glow, gradient) = rows[y];
                let ray = if glow > 0.001 || rainbow > 0.0 {
                    camera.ray(vec2(x as f32, y as f32)).1
                } else {
                    Vec3::ZERO
                };
                let color = if glow > 0.001 {
                    let facing =
                        (ray.dot(sun_azimuth).max(0.0).powi(2) / ray.length_squared()).powi(2);
                    mix(color, rgb(255, 139, 76), glow * facing)
                } else {
                    color
                };
                self.pixels[at] = if stars
                    && gradient > 0.2
                    && hash(star_seed ^ (y as u32 * 8191)) == 255
                    && star_column
                {
                    mix(color, rgb(182, 202, 232), night)
                } else if rainbow > 0.0 {
                    let direction = ray.normalize();
                    let cone = direction.dot(axis);
                    let band = ((cone - 0.728) / 0.027 * 7.0).floor() as i32;
                    if direction.z > 0.015 && (0..7).contains(&band) {
                        let fade = (direction.z * 8.0).min(1.0) * (1.0 - night) * 0.52 * rainbow;
                        mix(color, RAINBOW[band as usize], fade)
                    } else {
                        color
                    }
                } else {
                    color
                };
            }
        }
        if world.kind == crate::world::MapKind::Icebergs && night > 0.0 {
            self.aurora(camera, flight, night);
        }
        self.sun(camera, flight);
    }
}
