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
        for y in 0..self.h {
            let (_, ray) = camera.ray(vec2(camera.center.x, y as f32));
            let elevation = ray.z / ray.length();
            let gradient = (elevation * 1.4).clamp(0.0, 1.0);
            let glow = (1.0 - elevation.abs() / 0.26).max(0.0).powi(2) * twilight;
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
                    && self.depth[at] == f32::INFINITY
                {
                    let color = if glow > 0.001 {
                        let (_, ray) = camera.ray(vec2(x as f32, y as f32));
                        let facing =
                            (ray.dot(sun_azimuth).max(0.0).powi(2) / ray.length_squared()).powi(2);
                        mix(color, rgb(255, 139, 76), glow * facing)
                    } else {
                        color
                    };
                    self.pixels[at] = if night > 0.0
                        && weather.weight(Weather::Clear) > 0.95
                        && gradient > 0.2
                        && hash(
                            ((x + self.x_origin) as u32)
                                .wrapping_add((camera.yaw * 300.0) as i32 as u32)
                                ^ (y as u32 * 8191),
                        ) == 255
                        && (x + self.x_origin).is_multiple_of(3)
                    {
                        mix(color, rgb(182, 202, 232), night)
                    } else {
                        if weather.weight(Weather::Rainbow) > 0.0 && night < 1.0 {
                            let (_, ray) = camera.ray(vec2(x as f32, y as f32));
                            let direction = ray.normalize();
                            let axis = vec3(0.3, -0.95, -0.08).normalize();
                            let cone = direction.dot(axis);
                            let band = ((cone - 0.728) / 0.027 * 7.0).floor() as i32;
                            if direction.z > 0.015 && (0..7).contains(&band) {
                                let rainbow = [
                                    rgb(248, 99, 88),
                                    rgb(255, 163, 80),
                                    rgb(255, 221, 104),
                                    rgb(126, 210, 142),
                                    rgb(97, 174, 237),
                                    rgb(126, 133, 226),
                                    rgb(185, 132, 221),
                                ];
                                let fade = (direction.z * 8.0).min(1.0)
                                    * (1.0 - night)
                                    * 0.52
                                    * weather.weight(Weather::Rainbow);
                                mix(color, rainbow[band as usize], fade)
                            } else {
                                color
                            }
                        } else {
                            color
                        }
                    };
                }
            }
        }
        if world.kind == crate::world::MapKind::Icebergs && night > 0.0 {
            self.aurora(camera, flight, night);
        }
        self.sun(camera, flight);
    }
}
