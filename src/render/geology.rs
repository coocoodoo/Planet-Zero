use super::*;
use crate::world::celestial::{Scenery, Structure};
impl Renderer {
    fn rock_box(
        &mut self,
        camera: &Camera,
        local: &impl Fn(Vec3) -> Vec3,
        center: Vec3,
        half: Vec3,
        color: u32,
    ) {
        let p: [Vec3; 8] = std::array::from_fn(|i| {
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
        for face in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            self.rock_triangle(camera, [p[face[0]], p[face[1]], p[face[2]]], color);
            self.rock_triangle(camera, [p[face[0]], p[face[2]], p[face[3]]], color);
        }
    }
    pub(super) fn geological_structure(&mut self, camera: &Camera, object: Scenery) {
        let (s, c) = object.yaw.sin_cos();
        let local = |p: Vec3| {
            object.position + vec3(c * p.x - s * p.y, s * p.x + c * p.y, p.z) * object.scale
        };
        match object.kind {
            Structure::Arch | Structure::Cave => {
                let cave = object.kind == Structure::Cave;
                let (width, height, base, length) = if cave {
                    (48.0, 50.0, 24.0, 110.0)
                } else {
                    (74.0, 74.0, 20.0, 12.0)
                };
                let rock = if cave {
                    rgb(111, 103, 93)
                } else {
                    rgb(186, 130, 83)
                };
                for i in 0..12 {
                    let a = i as f32 * std::f32::consts::PI / 12.0;
                    let b = (i + 1) as f32 * std::f32::consts::PI / 12.0;
                    let inner = |angle: f32, y: f32| {
                        let irregular = (angle * 3.0 + y * 0.027).sin() * 3.0;
                        vec3(
                            angle.cos() * (width + irregular),
                            y,
                            base + angle.sin() * (height + irregular),
                        )
                    };
                    let outer = |angle: f32, y: f32| {
                        vec3(
                            angle.cos() * (width + 12.0),
                            y,
                            base + angle.sin() * (height + 16.0),
                        )
                    };
                    for y in [-length, length] {
                        let p = [
                            local(inner(a, y)),
                            local(inner(b, y)),
                            local(outer(b, y)),
                            local(outer(a, y)),
                        ];
                        self.rock_triangle(camera, [p[0], p[1], p[2]], rock);
                        self.rock_triangle(camera, [p[0], p[2], p[3]], rock);
                        if cave && i % 3 == 0 {
                            self.rock_box(
                                camera,
                                &local,
                                inner(a, y * 0.65) - Vec3::Z * 3.0,
                                vec3(1.0, 1.5, 5.0),
                                mix(rock, rgb(57, 61, 59), 0.3),
                            );
                        }
                    }
                    for (is_inner, shade) in [(true, 0.45), (false, 0.05)] {
                        let point = |angle, y| {
                            if is_inner {
                                inner(angle, y)
                            } else {
                                outer(angle, y)
                            }
                        };
                        let p = [
                            local(point(a, -length)),
                            local(point(b, -length)),
                            local(point(b, length)),
                            local(point(a, length)),
                        ];
                        let color = mix(rock, rgb(23, 29, 35), shade + (i % 3) as f32 * 0.035);
                        self.rock_triangle(camera, [p[0], p[1], p[2]], color);
                        self.rock_triangle(camera, [p[0], p[2], p[3]], color);
                    }
                }
                for side in [-1.0, 1.0] {
                    self.rock_box(
                        camera,
                        &local,
                        vec3(side * (width + 6.0), 0.0, base * 0.5),
                        vec3(6.0, length, base * 0.5),
                        rock,
                    );
                }
            }
            Structure::Stairs => {
                for i in 0..24 {
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(0.0, i as f32 * 4.0, i as f32 * 4.0 + 2.0),
                        vec3(8.0, 2.0, 2.0),
                        rgb(183, 169, 135),
                    );
                }
                for side in [-1.0, 1.0] {
                    self.world_line(
                        camera,
                        local(vec3(side * 8.0, 0.0, 8.0)),
                        local(vec3(side * 8.0, 96.0, 104.0)),
                        rgb(114, 91, 61),
                    );
                }
            }
            Structure::IceSpire | Structure::Boulder => {
                let ice = object.kind == Structure::IceSpire;
                let h = if ice { 45.0 } else { 10.0 };
                for i in 0..6 {
                    let a = i as f32 * std::f32::consts::TAU / 6.0;
                    let b = (i + 1) as f32 * std::f32::consts::TAU / 6.0;
                    let color = if ice {
                        if i % 2 == 0 {
                            rgb(211, 243, 251)
                        } else {
                            rgb(99, 178, 208)
                        }
                    } else {
                        rgb(128, 132, 119)
                    };
                    self.rock_triangle(
                        camera,
                        [
                            local(vec3(a.cos() * 10.0, a.sin() * 8.0, 0.0)),
                            local(vec3(b.cos() * 10.0, b.sin() * 8.0, 0.0)),
                            local(vec3(3.0, -2.0, h)),
                        ],
                        color,
                    );
                }
            }
            _ => {}
        }
    }
}

pub(super) fn stone_texel(p: Vec3) -> usize {
    let sample = |scale: f32| {
        let cell = (p * scale).floor().as_ivec3();
        hash(
            (cell.x as u32).wrapping_mul(73856093)
                ^ (cell.y as u32).wrapping_mul(19349663)
                ^ (cell.z as u32).wrapping_mul(83492791),
        ) as i32
    };
    let broad = sample(0.08);
    let grain = sample(1.4);
    let layer = (p.z * 0.23 + (broad / 64) as f32 * 0.13).rem_euclid(1.0);
    let fissure = (p.x * 0.071 + p.y * 0.057 + (p.z * 0.12).floor() * 0.17).rem_euclid(1.0);
    let mut shade = 5 + broad / 48 + grain / 65;
    if layer < 0.09 {
        shade -= 4;
    }
    if fissure < 0.035 && broad < 155 {
        shade -= 6;
    }
    if p.z.rem_euclid(80.0) < 15.0 && broad < 95 {
        shade -= 2;
    }
    shade.clamp(0, 15) as usize
}
