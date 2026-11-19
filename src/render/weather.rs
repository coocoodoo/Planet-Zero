use super::*;

impl Renderer {
    pub(super) fn precipitation(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
    ) {
        let weather = flight.weather;
        let (count, fall, wind) = weather.particles();
        let cloud_top = world.cloud_height(flight.position.x, flight.position.y) + 4.0;
        let density = ((cloud_top + 12.0 - flight.position.z) / 40.0).clamp(0.0, 1.0);
        let count = (count as f32 * density) as usize;
        if count == 0 {
            return;
        }
        let mut low = flight.position.truncate() - glam::Vec2::splat(128.0);
        let mut high = flight.position.truncate() + glam::Vec2::splat(128.0);
        if camera.q <= 0.00001 {
            // 俯瞰视口在降水高度范围内的投影覆盖整个生成区域
            for x in [0.0, self.w as f32] {
                for y in [0.0, self.h as f32] {
                    let (origin, ray) = camera.ray(vec2(x, y));
                    for height in [-80.0, 100.0] {
                        let t = (flight.position.z + height - origin.z) / ray.z;
                        let point = (origin + ray * t).truncate();
                        low = low.min(point);
                        high = high.max(point);
                    }
                }
            }
        }
        let size = high - low;
        let time = flight.time;
        for i in 0..count {
            let seed = i as u32 * 7919 + 31;
            let rx = hash(seed) as f32 / 255.0;
            let ry = hash(seed + 171) as f32 / 255.0;
            let rz = hash(seed + 947) as f32 / 255.0;
            let flutter = if weather == Weather::Snow {
                (time * 1.7 + rx * 20.0).sin() * 4.0
            } else {
                0.0
            };
            let position = vec3(
                (rx * size.x + time * wind + flutter - low.x).rem_euclid(size.x) + low.x,
                (ry * size.y + time * wind * 0.3 - low.y).rem_euclid(size.y) + low.y,
                (rz * 180.0 - time * fall - flight.position.z).rem_euclid(180.0)
                    + flight.position.z
                    - 80.0,
            );
            let ceiling = world.cloud_height(position.x, position.y) + 4.0;
            if position.z > ceiling {
                continue;
            }
            if let Some((a, depth, scale)) = camera.project(position) {
                if a.x < -20.0
                    || a.y < -20.0
                    || a.x > self.w as f32 + 20.0
                    || a.y > self.h as f32 + 20.0
                {
                    continue;
                }
                let base = match weather {
                    Weather::Snow => rgb(239, 247, 250),
                    Weather::Sandstorm => {
                        if i % 3 == 0 {
                            rgb(239, 194, 108)
                        } else {
                            rgb(132, 91, 48)
                        }
                    }
                    _ => rgb(149, 187, 216),
                };
                let color = mix(
                    grade(base, theme),
                    weather.haze(theme),
                    weather.fog(depth, flight.position.z) * 0.5,
                );
                if weather == Weather::Snow {
                    self.point(a, depth, color, 0, 0);
                    if scale > 1.4 && i % 9 == 0 {
                        self.point(a + vec2(1.0, 0.0), depth, color, 0, 0);
                        self.point(a + vec2(0.0, 1.0), depth, color, 0, 0);
                    }
                } else {
                    let mut tail = position - vec3(wind, wind * 0.3, -fall) * 0.035;
                    tail.z = tail.z.min(ceiling);
                    if let Some((b, _, _)) = camera.project(tail) {
                        self.stroke(a, b, depth, color);
                    }
                }
            }
        }
        if weather.flash(time) > 0.06 {
            let (s, c) = camera.yaw.sin_cos();
            let right = vec3(c, -s, 0.0);
            let forward = vec3(-s, -c, 0.0);
            let seed = (time / 11.0) as u32;
            let base = flight.position
                + forward * 420.0
                + right * ((hash(seed + 451) as f32 / 255.0 - 0.5) * 180.0);
            let top = world.cloud_height(base.x, base.y) + 4.0;
            let bottom = world.height(base.x, base.y);
            let mut previous = base.with_z(top);
            for segment in 1..=9 {
                let point = (base
                    + right * ((hash(seed + segment * 71) as f32 / 255.0 - 0.5) * 22.0))
                    .with_z(top + (bottom - top) * segment as f32 / 9.0);
                if let (Some((a, depth, _)), Some((b, _, _))) =
                    (camera.project(previous), camera.project(point))
                {
                    self.stroke(a, b, depth, rgb(241, 244, 255));
                    if (segment == 3 || segment == 5)
                        && let Some((branch, _, _)) =
                            camera.project(point + right * 28.0 - vec3(0.0, 0.0, 30.0))
                    {
                        self.stroke(b, branch, depth, rgb(174, 195, 234));
                    }
                }
                previous = point;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_covers_overhead_edges_and_ends_above_clouds() {
        let world = World::generate(crate::world::MapKind::Islands, 7);
        let mut flight = Flight::new(&world);
        flight.weather = Weather::Rain;
        flight.position.z = 60.0;
        flight.follow = flight.position;
        flight.camera_yaw = 0.0;
        let mut renderer = Renderer::single();
        renderer.w = 320;
        renderer.h = 170;
        renderer.full_width = 320;
        renderer.pixels = vec![0; 320 * 170];
        renderer.depth = vec![f32::INFINITY; 320 * 170];
        renderer.mask = vec![0; 320 * 170];
        for frame in 0..30 {
            flight.time = frame as f32 / 30.0;
            let camera = Camera::new(&flight, 320, 170);
            renderer.precipitation(&world, &flight, &camera, 0);
        }
        let colored = |x: std::ops::Range<usize>, y: std::ops::Range<usize>| {
            x.flat_map(|x| y.clone().map(move |y| x * 170 + y))
                .filter(|&at| renderer.pixels[at] != 0)
                .count()
        };
        assert!(colored(0..16, 0..170) > 20, "left edge");
        assert!(colored(304..320, 0..170) > 20, "right edge");
        assert!(colored(0..320, 0..16) > 20, "top edge");
        assert!(colored(0..320, 154..170) > 20, "bottom edge");
        flight.position.z = world.cloud_height(flight.position.x, flight.position.y) + 50.0;
        flight.follow = flight.position;
        renderer.pixels.fill(0);
        let camera = Camera::new(&flight, 320, 170);
        renderer.precipitation(&world, &flight, &camera, 0);
        assert!(renderer.pixels.iter().all(|&pixel| pixel == 0));
    }
}
