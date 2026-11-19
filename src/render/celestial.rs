use super::*;
use crate::world::celestial::{Scenery, Structure};
use std::f32::consts::TAU;
use std::sync::OnceLock;

fn starfield() -> &'static Vec<u32> {
    static SKY: OnceLock<Vec<u32>> = OnceLock::new();
    SKY.get_or_init(|| {
        let mut pixels = Vec::with_capacity(6 * 128 * 128);
        for face in 0..6 {
            for y in 0..128 {
                for x in 0..128 {
                    let d = sky_direction(face, x, y);
                    let band = (1.0 - (d.z * 1.6 + d.x * 0.48 - d.y * 0.22).abs())
                        .max(0.0)
                        .powi(5);
                    let cloud = ((d.x * 14.0 + (d.y * 11.0).sin() * 2.0).sin()
                        * (d.z * 18.0 - d.y * 9.0).sin()
                        * 0.5
                        + 0.5)
                        * band;
                    let dust =
                        ((d.x * 39.0 + d.z * 27.0).sin() * (d.y * 33.0).sin()).max(0.0) * band;
                    let at = face * 16384 + y * 128 + x;
                    let star = hash(at as u32 * 7919) == 255 && hash(at as u32 ^ 931) > 160;
                    pixels.push(if star {
                        let n = hash(at as u32);
                        rgb(100 + n / 2, 120 + n / 2, 175 + n / 4)
                    } else {
                        rgb(
                            (3.0 + cloud * 30.0 + dust * 9.0) as u8,
                            (5.0 + cloud * 17.0 + dust * 9.0) as u8,
                            (13.0 + cloud * 46.0 + dust * 12.0) as u8,
                        )
                    });
                }
            }
        }
        pixels
    })
}

fn aurora_texture() -> &'static Vec<u32> {
    static AURORA: OnceLock<Vec<u32>> = OnceLock::new();
    AURORA.get_or_init(|| {
        let mut pixels = Vec::with_capacity(6 * 128 * 128);
        for face in 0..6 {
            for y in 0..128 {
                for x in 0..128 {
                    let d = sky_direction(face, x, y);
                    let angle = d.y.atan2(d.x);
                    let mut glow_color = Vec3::ZERO;
                    let mut opacity = 0.0_f32;
                    for ribbon in 0..2 {
                        let phase = ribbon as f32 * 1.7;
                        let base = 0.19
                            + ribbon as f32 * 0.25
                            + (angle * 2.0 + phase).sin() * 0.115
                            + (angle * 5.0 - phase).sin() * 0.035;
                        let height = d.z - base;
                        let filaments =
                            0.7 + 0.3 * (angle * 89.0 + (angle * 11.0).sin() * 3.0).sin();
                        let reach = 0.10 + 0.055 * (angle * 7.0 + phase).sin();
                        let curtain = (height / 0.018).clamp(0.0, 1.0)
                            * (-height.max(0.0) / reach).exp()
                            * filaments;
                        let hem = (-(height / 0.022).powi(2)).exp();
                        let amount =
                            (hem * 0.58 + curtain * 0.8).min(1.0) * (d.z * 9.0).clamp(0.0, 1.0);
                        let cyan = vec3(46.0, 228.0, 204.0);
                        let violet = vec3(168.0, 86.0, 231.0);
                        let color = cyan.lerp(violet, (height / 0.14).clamp(0.0, 1.0));
                        glow_color += color * amount;
                        opacity += amount;
                    }
                    let color = glow_color / opacity.max(0.001);
                    pixels.push(u32::from_le_bytes([
                        color.x as u8,
                        color.y as u8,
                        color.z as u8,
                        (opacity.min(1.0) * 195.0) as u8,
                    ]));
                }
            }
        }
        pixels
    })
}

fn sky_direction(face: usize, x: usize, y: usize) -> Vec3 {
    let u = x as f32 / 63.5 - 1.0;
    let v = y as f32 / 63.5 - 1.0;
    match face {
        0 => vec3(1.0, u, v),
        1 => vec3(-1.0, u, v),
        2 => vec3(u, 1.0, v),
        3 => vec3(u, -1.0, v),
        4 => vec3(u, v, 1.0),
        _ => vec3(u, v, -1.0),
    }
    .normalize()
}

impl Renderer {
    pub(super) fn stars(&mut self, camera: &Camera, opacity: f32, rotation: glam::Quat) {
        self.sky_texture(camera, starfield(), opacity, rotation);
    }

    pub(super) fn aurora(&mut self, camera: &Camera, flight: &Flight, night: f32) {
        let weather = flight.weather;
        let strength = night
            * (1.0
                - weather.weight(Weather::Fog) * 0.95
                - weather.weight(Weather::Storm) * 0.9
                - weather.weight(Weather::Snow) * 0.65);
        let rotation = glam::Quat::from_rotation_z(flight.time * 0.006)
            * glam::Quat::from_rotation_x((flight.time * 0.035).sin() * 0.025);
        self.sky_texture(camera, aurora_texture(), strength, rotation);
    }

    fn sky_texture(
        &mut self,
        camera: &Camera,
        texture: &[u32],
        opacity: f32,
        rotation: glam::Quat,
    ) {
        let mut sky_camera = *camera;
        sky_camera.q = camera.q.max(0.004);
        for x in 0..self.w {
            let (_, mut ray) = sky_camera.ray(vec2(x as f32, self.visible[x].0 as f32));
            ray = rotation * ray;
            let step = rotation * sky_camera.up * (-sky_camera.q / sky_camera.scale);
            for y in self.visible[x].0..self.visible[x].1 {
                let at = x * self.h + y as usize;
                if self.depth[at] == f32::INFINITY {
                    let a = ray.abs();
                    let (face, u, v) = if a.x >= a.y && a.x >= a.z {
                        (usize::from(ray.x < 0.0), ray.y / a.x, ray.z / a.x)
                    } else if a.y >= a.z {
                        (2 + usize::from(ray.y < 0.0), ray.x / a.y, ray.z / a.y)
                    } else {
                        (4 + usize::from(ray.z < 0.0), ray.x / a.z, ray.y / a.z)
                    };
                    let tx = ((u + 1.0) * 63.5).clamp(0.0, 127.0) as usize;
                    let ty = ((v + 1.0) * 63.5).clamp(0.0, 127.0) as usize;
                    let color = texture[face * 16384 + ty * 128 + tx];
                    let opacity = opacity * (color >> 24) as f32 / 255.0;
                    self.pixels[at] = if opacity >= 1.0 {
                        color
                    } else {
                        mix(self.pixels[at], color, opacity)
                    };
                }
                ray += step;
            }
        }
    }
    pub(super) fn cosmos(&mut self, world: &World, camera: &Camera, flight: &Flight) {
        self.stars(camera, 1.0, flight.sky_rotation);
        self.sun(camera, flight);
        self.orbital_bodies(world, camera, flight);
    }

    pub(super) fn sun(&mut self, camera: &Camera, flight: &Flight) {
        if camera.q > 0.0001 {
            self.planet(
                camera,
                camera.position + self.light.direction * 12000.0,
                480.0,
                true,
                flight.time,
            );
        }
    }

    fn planet(&mut self, camera: &Camera, center: Vec3, radius: f32, star: bool, time: f32) {
        let Some((p, depth, scale)) = camera.project(center) else {
            return;
        };
        if depth < 0.0 {
            return;
        }
        let r = (radius * scale).min(self.full_width as f32);
        let extent = r * if star { 1.7 } else { 1.04 };
        let min = (p - Vec2::splat(extent)).max(Vec2::ZERO);
        let max = (p + Vec2::splat(extent)).min(vec2(self.w as f32 - 1.0, self.h as f32 - 1.0));
        for x in min.x as usize..=max.x as usize {
            let y0 = (min.y as i32).max(self.visible[x].0);
            let y1 = (max.y as i32 + 1).min(self.visible[x].1);
            for y in y0.max(0) as usize..y1.max(0) as usize {
                let uv = (vec2(x as f32, y as f32) - p) / r;
                let d = uv.length_squared();
                let at = x * self.h + y;
                if self.depth[at] < depth {
                    continue;
                }
                if d <= 1.0 {
                    let z = (1.0 - d).sqrt();
                    let light = (uv.x * -0.6 - uv.y * 0.4 + z * 0.6).max(0.03);
                    let noise = (uv.x * 15.0 + (uv.y * 18.0).sin() * 2.0).sin()
                        + (uv.y * 12.0 - uv.x * 8.0).cos();
                    let color = if star {
                        mix(
                            rgb(255, 116, 35),
                            rgb(255, 246, 184),
                            (z * 0.6 + noise * 0.07 + (time * 0.5).sin() * 0.02).clamp(0.0, 1.0),
                        )
                    } else {
                        let base = if noise > 0.9 {
                            rgb(93, 156, 144)
                        } else if noise < -1.1 || uv.y.abs() > 0.88 {
                            rgb(219, 236, 242)
                        } else {
                            rgb(28, 94, 180)
                        };
                        mix(rgb(5, 12, 29), base, light.min(1.0))
                    };
                    self.pixels[at] = color;
                } else if star {
                    let glow = ((1.7 - d.sqrt()) / 0.7).max(0.0).powi(3) * 0.4;
                    self.pixels[at] = mix(self.pixels[at], rgb(255, 145, 55), glow);
                } else if d < 1.08 {
                    self.pixels[at] = mix(self.pixels[at], rgb(76, 159, 237), 0.5);
                }
            }
        }
    }

    pub(super) fn scenery(&mut self, world: &World, camera: &Camera, time: f32, scale: f32) {
        if scale < 0.001 {
            return;
        }
        for object in &world.scenery {
            let mut object = *object;
            object.scale *= scale;
            for axis in 0..2 {
                object.position[axis] +=
                    ((camera.position[axis] - object.position[axis]) / SIZE as f32).round()
                        * SIZE as f32;
            }
            if object.position.distance_squared(camera.position) > 2100.0_f32.powi(2) {
                continue;
            }
            if let Some(bounds) = match object.kind {
                Structure::Building => Some(vec3(20.0, 20.0, 47.0)),
                Structure::House => Some(vec3(22.0, 22.0, 10.0)),
                Structure::Warehouse => Some(vec3(40.0, 40.0, 12.0)),
                Structure::IceSpire | Structure::Boulder => Some(vec3(12.0, 12.0, 24.0)),
                kind => kind
                    .landmark_size()
                    .map(|size| vec3(size.x + size.y, size.x + size.y, size.z / 2.0)),
            } {
                let half = bounds * object.scale;
                let delta = object.position + Vec3::Z * half.z - camera.position;
                if let Some((start, inverse_range, _)) = self.scenery_fog
                    && delta.dot(camera.forward) - half.dot(camera.forward.abs())
                        >= start + inverse_range.recip()
                {
                    continue;
                }
                let bounds = [
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
                ];
                if bounds.iter().any(|(axis, offset)| {
                    let plane = *axis + camera.forward * (*offset * camera.q);
                    delta.dot(plane) + offset + plane.abs().dot(half) < 0.0
                }) || 1.0
                    + camera.q * (delta.dot(camera.forward) + camera.forward.abs().dot(half))
                    < 0.12
                {
                    continue;
                }
            }
            self.structure(camera, object, time);
        }
    }

    fn structure(&mut self, camera: &Camera, object: Scenery, time: f32) {
        let (s, c) = object.yaw.sin_cos();
        let local = |p: Vec3| {
            object.position + vec3(c * p.x - s * p.y, s * p.x + c * p.y, p.z) * object.scale
        };
        let hull = rgb(169, 190, 208);
        let dark = rgb(48, 66, 89);
        let panel = rgb(37, 73, 137);
        let light = rgb(118, 232, 248);
        match object.kind {
            Structure::Pyramid
            | Structure::Eiffel
            | Structure::Torii
            | Structure::Pavilion
            | Structure::StaveChurch
            | Structure::Castle
            | Structure::Temple
            | Structure::SplitGate
            | Structure::PolarStation => self.monument(camera, object),
            Structure::Fuji => {}
            Structure::House | Structure::Warehouse => self.settlement(camera, object),
            Structure::Arch
            | Structure::Cave
            | Structure::Stairs
            | Structure::IceSpire
            | Structure::Boulder => self.geological_structure(camera, object),
            Structure::Building
            | Structure::Windmill
            | Structure::Lighthouse
            | Structure::RockStack => self.landscape_structure(camera, object, time),
            Structure::Base => {
                for x in [-51.0, 0.0, 51.0] {
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(x, 0.0, 7.0),
                        vec3(20.0, 14.0, 7.0),
                        hull,
                    );
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(x, 0.0, 16.0),
                        vec3(16.0, 11.0, 3.0),
                        rgb(211, 221, 224),
                    );
                    for win in [-10.0, 0.0, 10.0] {
                        self.box_mesh(
                            camera,
                            &local,
                            vec3(x + win, 14.5, 9.0),
                            vec3(2.5, 0.7, 2.0),
                            light,
                        );
                    }
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 5.0),
                    vec3(76.0, 3.0, 3.0),
                    dark,
                );
                for x in [-57.0, 0.0, 57.0] {
                    self.solar(
                        camera,
                        &local,
                        vec3(x, -44.0, 9.0),
                        vec3(21.0, 13.0, 0.0),
                        panel,
                    );
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(82.0, 28.0, 17.0),
                    vec3(4.0, 4.0, 17.0),
                    hull,
                );
                let dish = vec3(82.0, 28.0, 38.0);
                for i in 0..12 {
                    let a = i as f32 * TAU / 12.0;
                    let b = (i + 1) as f32 * TAU / 12.0;
                    self.triangle(
                        camera,
                        [
                            local(dish - Vec3::Z * 6.0),
                            local(dish + vec3(a.cos() * 13.0, a.sin() * 13.0, 0.0)),
                            local(dish + vec3(b.cos() * 13.0, b.sin() * 13.0, 0.0)),
                        ],
                        if i < 6 { hull } else { dark },
                    );
                }
                for i in 0..24 {
                    let a = i as f32 * TAU / 24.0;
                    let b = (i + 1) as f32 * TAU / 24.0;
                    self.world_line(
                        camera,
                        local(vec3(-68.0 + a.cos() * 21.0, 47.0 + a.sin() * 21.0, 0.8)),
                        local(vec3(-68.0 + b.cos() * 21.0, 47.0 + b.sin() * 21.0, 0.8)),
                        rgb(232, 189, 88),
                    );
                }
                for x in [-75.0, -61.0] {
                    self.world_line(
                        camera,
                        local(vec3(x, 38.0, 1.0)),
                        local(vec3(x, 56.0, 1.0)),
                        hull,
                    );
                }
                self.world_line(
                    camera,
                    local(vec3(-75.0, 47.0, 1.0)),
                    local(vec3(-61.0, 47.0, 1.0)),
                    hull,
                );
            }
            Structure::Satellite => {
                self.box_mesh(
                    camera,
                    &local,
                    Vec3::ZERO,
                    vec3(7.0, 10.0, 8.0),
                    rgb(220, 183, 99),
                );
                self.box_mesh(camera, &local, Vec3::ZERO, vec3(39.0, 1.4, 1.4), hull);
                for side in [-1.0, 1.0] {
                    self.solar(
                        camera,
                        &local,
                        vec3(side * 29.0, 0.0, 2.0),
                        vec3(17.0, 21.0, 0.0),
                        panel,
                    );
                }
                self.world_line(
                    camera,
                    local(vec3(0.0, 0.0, 8.0)),
                    local(vec3(0.0, 0.0, 24.0)),
                    hull,
                );
                self.triangle(
                    camera,
                    [
                        local(vec3(-9.0, 0.0, 17.0)),
                        local(vec3(9.0, 0.0, 17.0)),
                        local(vec3(0.0, 6.0, 10.0)),
                    ],
                    hull,
                );
            }
            Structure::Cruiser => {
                self.box_mesh(camera, &local, Vec3::ZERO, vec3(14.0, 60.0, 9.0), hull);
                self.triangle(
                    camera,
                    [
                        local(vec3(-14.0, -60.0, 9.0)),
                        local(vec3(14.0, -60.0, 9.0)),
                        local(vec3(0.0, -92.0, 0.0)),
                    ],
                    rgb(213, 221, 226),
                );
                self.triangle(
                    camera,
                    [
                        local(vec3(-14.0, -60.0, -9.0)),
                        local(vec3(14.0, -60.0, -9.0)),
                        local(vec3(0.0, -92.0, 0.0)),
                    ],
                    dark,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 25.0, 15.0),
                    vec3(10.0, 16.0, 7.0),
                    dark,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 9.0, 17.0),
                    vec3(9.0, 1.0, 2.0),
                    light,
                );
                for side in [-1.0, 1.0] {
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(side * 20.0, 14.0, 0.0),
                        vec3(14.0, 8.0, 3.0),
                        dark,
                    );
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(side * 32.0, 20.0, 0.0),
                        vec3(7.0, 38.0, 7.0),
                        hull,
                    );
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(side * 32.0, 59.0, 0.0),
                        vec3(5.0, 1.0, 5.0),
                        light,
                    );
                    let flame = 12.0 + (time * 12.0).sin() * 3.0;
                    self.triangle(
                        camera,
                        [
                            local(vec3(side * 32.0 - 4.0, 60.0, 0.0)),
                            local(vec3(side * 32.0 + 4.0, 60.0, 0.0)),
                            local(vec3(side * 32.0, 60.0 + flame, 0.0)),
                        ],
                        rgb(81, 157, 250),
                    );
                    for y in [-40.0, -20.0, 0.0, 20.0] {
                        self.world_line(
                            camera,
                            local(vec3(side * 14.5, y, -1.0)),
                            local(vec3(side * 14.5, y + 6.0, -1.0)),
                            light,
                        );
                    }
                }
            }
            Structure::Asteroid => {
                let points = [
                    vec3(-12.0, 0.0, 0.0),
                    vec3(0.0, -10.0, 0.0),
                    vec3(14.0, 1.0, 0.0),
                    vec3(1.0, 13.0, 0.0),
                ];
                for i in 0..4 {
                    self.triangle(
                        camera,
                        [
                            local(points[i]),
                            local(points[(i + 1) % 4]),
                            local(vec3(-3.0, 1.0, 11.0)),
                        ],
                        rgb(117 + i as u8 * 13, 110 + i as u8 * 12, 113 + i as u8 * 12),
                    );
                    self.triangle(
                        camera,
                        [
                            local(points[i]),
                            local(points[(i + 1) % 4]),
                            local(vec3(2.0, -2.0, -9.0)),
                        ],
                        dark,
                    );
                }
            }
        }
    }

    pub(super) fn box_mesh(
        &mut self,
        camera: &Camera,
        local: &impl Fn(Vec3) -> Vec3,
        center: Vec3,
        half: Vec3,
        color: u32,
    ) {
        let vertices: [Vec3; 8] = std::array::from_fn(|i| {
            local(
                center
                    + half
                        * vec3(
                            if i & 1 == 0 { -1.0 } else { 1.0 },
                            if i & 2 == 0 { -1.0 } else { 1.0 },
                            if i & 4 == 0 { -1.0 } else { 1.0 },
                        ),
            )
        });
        for (i, face) in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ]
        .into_iter()
        .enumerate()
        {
            let (eye, ray) = camera.ray(camera.center);
            let normal = (vertices[face[1]] - vertices[face[0]])
                .cross(vertices[face[2]] - vertices[face[0]]);
            let toward = if camera.q > 0.00001 {
                eye - vertices[face[0]]
            } else {
                -ray
            };
            if normal.dot(toward) <= 0.0 {
                continue;
            }
            let shade = mix(
                color,
                rgb(13, 22, 40),
                [0.65, 0.0, 0.24, 0.42, 0.48, 0.15][i],
            );
            self.triangle(
                camera,
                [vertices[face[0]], vertices[face[1]], vertices[face[2]]],
                shade,
            );
            self.triangle(
                camera,
                [vertices[face[0]], vertices[face[2]], vertices[face[3]]],
                shade,
            );
        }
    }
    fn solar(
        &mut self,
        camera: &Camera,
        local: &impl Fn(Vec3) -> Vec3,
        center: Vec3,
        half: Vec3,
        color: u32,
    ) {
        self.box_mesh(camera, local, center, half + Vec3::Z * 0.7, color);
        for i in -3..=3 {
            let dx = half.x * i as f32 / 3.0;
            self.world_line(
                camera,
                local(center + vec3(dx, -half.y, 1.0)),
                local(center + vec3(dx, half.y, 1.0)),
                rgb(96, 142, 191),
            );
        }
        for i in -2..=2 {
            let dy = half.y * i as f32 / 2.0;
            self.world_line(
                camera,
                local(center + vec3(-half.x, dy, 1.0)),
                local(center + vec3(half.x, dy, 1.0)),
                rgb(96, 142, 191),
            );
        }
    }
}
