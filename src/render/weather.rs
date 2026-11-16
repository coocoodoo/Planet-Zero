use super::*;

impl Renderer {
    pub(super) fn precipitation(&mut self, flight: &Flight, camera: &Camera, theme: usize) {
        let weather = flight.weather;
        let (count, fall, wind) = weather.particles();
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
                (rx * 256.0 + time * wind + flutter - flight.position.x).rem_euclid(256.0)
                    + flight.position.x
                    - 128.0,
                (ry * 256.0 + time * wind * 0.3 - flight.position.y).rem_euclid(256.0)
                    + flight.position.y
                    - 128.0,
                (rz * 180.0 - time * fall - flight.position.z).rem_euclid(180.0)
                    + flight.position.z
                    - 80.0,
            );
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
                    let tail = position - vec3(wind, wind * 0.3, -fall) * 0.035;
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
            let mut previous = base + vec3(0.0, 0.0, 160.0);
            for segment in 1..=9 {
                let point = base
                    + right * ((hash(seed + segment * 71) as f32 / 255.0 - 0.5) * 22.0)
                    + vec3(0.0, 0.0, 160.0 - segment as f32 * 25.0);
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
