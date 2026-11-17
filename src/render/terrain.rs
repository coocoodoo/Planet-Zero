use super::*;

impl Renderer {
    pub(super) fn world(&mut self, world: &World, flight: &Flight, camera: &Camera, theme: usize) {
        let started = std::time::Instant::now();
        self.pitched_terrain(world, flight, camera, theme);
        self.detail[0] = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        self.draw_clouds(world, flight, camera, theme);
        self.detail[1] = started.elapsed().as_secs_f64() * 1000.0;
    }
    pub(super) fn sky(&mut self, camera: &Camera, flight: &Flight, theme: usize, _height: usize) {
        let (low, high) = match theme {
            1 => (rgb(255, 198, 132), rgb(84, 68, 150)),
            2 => (rgb(36, 56, 110), rgb(7, 10, 34)),
            _ => (rgb(198, 232, 250), rgb(62, 142, 222)),
        };
        let weather = flight.weather;
        let (low, high) = if matches!(weather, Weather::Clear | Weather::Rainbow) {
            (low, high)
        } else {
            let cover = match weather {
                Weather::Fog => 0.92,
                Weather::Sandstorm => 0.85,
                Weather::Snow => 0.68,
                Weather::Rain => 0.55,
                Weather::Cloudy => 0.3,
                _ => 0.4,
            };
            (
                weather.haze(theme),
                mix(weather.shade(high), weather.haze(theme), cover),
            )
        };
        let flash = weather.flash(flight.time);
        for y in 0..self.h {
            let (_, ray) = camera.ray(vec2(camera.center.x, y as f32));
            let gradient = (ray.z / ray.length() * 1.4).clamp(0.0, 1.0);
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
                    && self.depth[at].is_infinite()
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
                        if weather == Weather::Rainbow && theme != 2 {
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
                                    * if theme == 0 { 0.52 } else { 0.32 };
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
    }
}
