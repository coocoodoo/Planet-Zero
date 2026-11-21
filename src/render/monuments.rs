use super::*;
use crate::world::celestial::{Scenery, Structure};

impl Renderer {
    pub(super) fn monument(&mut self, camera: &Camera, object: Scenery) {
        let local = object.to_world();
        let stone = rgb(176, 165, 143);
        let red = rgb(186, 54, 38);
        let roof = rgb(42, 65, 70);
        match object.kind {
            Structure::Pyramid => {
                self.frustum(
                    camera,
                    &local,
                    (Vec3::ZERO, Vec2::splat(65.0)),
                    (vec3(0.0, 0.0, 90.0), Vec2::ZERO),
                    rgb(220, 177, 108),
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, -59.0, 5.0),
                    vec3(4.0, 1.0, 5.0),
                    rgb(63, 48, 33),
                );
            }
            Structure::Eiffel => {
                let iron = rgb(146, 116, 84);
                for x in [-1.0, 1.0] {
                    for y in [-1.0, 1.0] {
                        for (z0, w0, z1, w1, thickness) in [
                            (0.0, 42.0, 42.0, 20.0, 4.0),
                            (42.0, 20.0, 85.0, 9.0, 2.8),
                            (85.0, 9.0, 137.0, 2.0, 1.8),
                        ] {
                            self.frustum(
                                camera,
                                &local,
                                (vec3(x * w0, y * w0, z0), Vec2::splat(thickness)),
                                (vec3(x * w1, y * w1, z1), Vec2::splat(thickness * 0.7)),
                                iron,
                            );
                            for n in 0..3 {
                                let a = n as f32 / 3.0;
                                let b = (n + 1) as f32 / 3.0;
                                let wa = w0 + (w1 - w0) * a;
                                let wb = w0 + (w1 - w0) * b;
                                self.world_line(
                                    camera,
                                    local(vec3(x * wa, y * wa, z0 + (z1 - z0) * a)),
                                    local(vec3(-x * wb, y * wb, z0 + (z1 - z0) * b)),
                                    iron,
                                );
                                self.world_line(
                                    camera,
                                    local(vec3(x * wa, y * wa, z0 + (z1 - z0) * a)),
                                    local(vec3(x * wb, -y * wb, z0 + (z1 - z0) * b)),
                                    iron,
                                );
                            }
                        }
                    }
                }
                for (z, w) in [(42.0, 24.0), (85.0, 12.0), (137.0, 5.0)] {
                    self.box_mesh(camera, &local, vec3(0.0, 0.0, z), vec3(w, w, 2.0), iron);
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 146.0),
                    vec3(1.2, 1.2, 9.0),
                    iron,
                );
            }
            Structure::Torii => {
                for x in [-22.0, 22.0] {
                    self.frustum(
                        camera,
                        &local,
                        (vec3(x, 0.0, 0.0), vec2(3.6, 3.6)),
                        (vec3(x * 0.88, 0.0, 43.0), vec2(2.8, 2.8)),
                        red,
                    );
                    self.box_mesh(camera, &local, vec3(x, 0.0, 3.0), vec3(4.0, 4.0, 3.0), roof);
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 32.0),
                    vec3(29.0, 2.5, 2.5),
                    red,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 43.0),
                    vec3(34.0, 4.0, 2.5),
                    red,
                );
                self.frustum(
                    camera,
                    &local,
                    (vec3(0.0, 0.0, 45.5), vec2(34.0, 4.0)),
                    (vec3(0.0, 0.0, 48.0), vec2(29.0, 3.0)),
                    roof,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 38.0),
                    vec3(2.0, 2.0, 5.0),
                    red,
                );
            }
            Structure::Pavilion => {
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 3.0),
                    vec3(28.0, 28.0, 3.0),
                    stone,
                );
                for x in [-18.0, 18.0] {
                    for y in [-18.0, 18.0] {
                        self.box_mesh(camera, &local, vec3(x, y, 18.0), vec3(2.5, 2.5, 12.0), red);
                    }
                }
                for (z, w) in [(30.0, 30.0), (44.0, 19.0)] {
                    self.frustum(
                        camera,
                        &local,
                        (vec3(0.0, 0.0, z), Vec2::splat(w)),
                        (vec3(0.0, 0.0, z + 10.0), vec2(4.0, 4.0)),
                        roof,
                    );
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(0.0, 0.0, z + 10.5),
                        vec3(5.0, 5.0, 0.5),
                        rgb(210, 172, 76),
                    );
                }
            }
            Structure::StaveChurch => {
                let wood = rgb(97, 65, 43);
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 13.0),
                    vec3(18.0, 28.0, 13.0),
                    wood,
                );
                for (z, w, d) in [(26.0, 24.0, 34.0), (43.0, 16.0, 22.0), (60.0, 9.0, 12.0)] {
                    self.frustum(
                        camera,
                        &local,
                        (vec3(0.0, 0.0, z), vec2(w, d)),
                        (vec3(0.0, 0.0, z + 18.0), vec2(0.0, d * 0.65)),
                        roof,
                    );
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, -28.5, 9.0),
                    vec3(4.0, 0.5, 9.0),
                    rgb(34, 29, 29),
                );
            }
            Structure::Castle => {
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 23.0),
                    vec3(34.0, 24.0, 23.0),
                    stone,
                );
                for x in [-38.0, 38.0] {
                    for y in [-28.0, 28.0] {
                        self.box_mesh(
                            camera,
                            &local,
                            vec3(x, y, 28.0),
                            vec3(10.0, 10.0, 28.0),
                            stone,
                        );
                        for dx in [-7.0, 7.0] {
                            for dy in [-7.0, 7.0] {
                                self.box_mesh(
                                    camera,
                                    &local,
                                    vec3(x + dx, y + dy, 60.5),
                                    vec3(3.0, 3.0, 4.5),
                                    stone,
                                );
                            }
                        }
                        self.box_mesh(
                            camera,
                            &local,
                            vec3(x, y - 10.5, 37.0),
                            vec3(1.5, 0.5, 5.0),
                            rgb(40, 44, 47),
                        );
                    }
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, -24.5, 12.0),
                    vec3(7.0, 0.5, 12.0),
                    rgb(51, 45, 36),
                );
            }
            Structure::Temple => {
                let sand = rgb(195, 125, 92);
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 5.0, 24.0),
                    vec3(40.0, 19.0, 24.0),
                    sand,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, -14.5, 15.0),
                    vec3(8.0, 0.5, 15.0),
                    rgb(69, 40, 34),
                );
                for x in [-33.0, -19.0, 19.0, 33.0] {
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(x, -20.0, 22.0),
                        vec3(3.0, 3.0, 22.0),
                        rgb(222, 151, 111),
                    );
                }
                self.frustum(
                    camera,
                    &local,
                    (vec3(0.0, 0.0, 46.0), vec2(44.0, 24.0)),
                    (vec3(0.0, 0.0, 61.0), vec2(0.0, 24.0)),
                    sand,
                );
            }
            Structure::SplitGate => {
                for side in [-1.0, 1.0] {
                    for level in 0..6 {
                        let w = 12.0 - level as f32 * 1.4;
                        self.box_mesh(
                            camera,
                            &local,
                            vec3(side * (8.0 + w), 0.0, level as f32 * 10.0 + 5.0),
                            vec3(w, 12.0 - level as f32, 5.0),
                            rgb(99, 96, 89),
                        );
                        self.box_mesh(
                            camera,
                            &local,
                            vec3(side * (8.0 + w), 0.0, level as f32 * 10.0 + 10.0),
                            vec3(w + 2.0, 13.0 - level as f32, 1.0),
                            stone,
                        );
                    }
                }
            }
            Structure::PolarStation => {
                for x in [-28.0, 28.0] {
                    for y in [-14.0, 14.0] {
                        self.box_mesh(camera, &local, vec3(x, y, 4.0), vec3(3.0, 3.0, 4.0), stone);
                    }
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(x, 0.0, 15.0),
                        vec3(24.0, 20.0, 7.0),
                        red,
                    );
                    self.frustum(
                        camera,
                        &local,
                        (vec3(x, 0.0, 22.0), vec2(24.0, 22.0)),
                        (vec3(x, 0.0, 26.0), vec2(24.0, 0.0)),
                        rgb(227, 235, 238),
                    );
                    for dx in [-13.0, 0.0, 13.0] {
                        self.box_mesh(
                            camera,
                            &local,
                            vec3(x + dx, -20.5, 16.0),
                            vec3(3.0, 0.5, 3.0),
                            rgb(108, 189, 211),
                        );
                    }
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 12.0),
                    vec3(6.0, 7.0, 4.0),
                    stone,
                );
            }
            _ => {}
        }
    }
    fn frustum(
        &mut self,
        camera: &Camera,
        local: &impl Fn(Vec3) -> Vec3,
        bottom: (Vec3, Vec2),
        top: (Vec3, Vec2),
        color: u32,
    ) {
        let vertices: [Vec3; 8] = std::array::from_fn(|i| {
            let (center, half) = if i < 4 { bottom } else { top };
            local(
                center
                    + vec3(
                        if i & 1 == 0 { -half.x } else { half.x },
                        if i & 2 == 0 { -half.y } else { half.y },
                        0.0,
                    ),
            )
        });
        let (eye, ray) = camera.ray(camera.center);
        for (i, face) in [
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ]
        .into_iter()
        .enumerate()
        {
            let toward = if camera.q > 0.00001 {
                eye - vertices[face[0]]
            } else {
                -ray
            };
            let normal = (vertices[face[1]] - vertices[face[0]])
                .cross(vertices[face[2]] - vertices[face[0]]);
            // 锥顶三角形使用非重合顶点计算法线
            let normal = if normal.length_squared() == 0.0 {
                (vertices[face[2]] - vertices[face[0]]).cross(vertices[face[3]] - vertices[face[0]])
            } else {
                normal
            };
            if normal.dot(toward) <= 0.0 {
                continue;
            }
            let shade = mix(color, rgb(20, 25, 35), [0.0, 0.15, 0.32, 0.38, 0.08][i]);
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
}
