use super::*;
use crate::world::celestial::{Scenery, Structure};

impl Renderer {
    pub(super) fn landscape_structure(&mut self, camera: &Camera, object: Scenery, time: f32) {
        let (s, c) = object.yaw.sin_cos();
        let local = object.to_world();
        let steel = rgb(188, 204, 210);
        match object.kind {
            Structure::Building => {
                let color = if object.scale > 2.3 {
                    rgb(91, 137, 160)
                } else {
                    rgb(119, 135, 153)
                };
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 38.0),
                    vec3(11.0, 14.0, 38.0),
                    color,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 78.0),
                    vec3(6.0, 7.0, 2.0),
                    rgb(58, 74, 95),
                );
                if object.scale > 2.3 {
                    self.world_line(
                        camera,
                        local(vec3(0.0, 0.0, 80.0)),
                        local(vec3(0.0, 0.0, 91.0)),
                        steel,
                    );
                    if let Some((p, d, _)) = camera.project(local(vec3(0.0, 0.0, 91.0))) {
                        self.point(
                            p,
                            d,
                            if (time * 2.0).sin() > 0.0 {
                                rgb(255, 97, 69)
                            } else {
                                rgb(123, 58, 51)
                            },
                            0,
                            0,
                        );
                    }
                }
                let Some((_, _, scale)) =
                    camera.project(object.position + Vec3::Z * 38.0 * object.scale)
                else {
                    return;
                };
                if scale * object.scale < 0.16 {
                    return;
                }
                let (eye, _) = camera.ray(camera.center);
                let delta = (eye - object.position) / object.scale;
                let eye = vec3(
                    c * delta.x + s * delta.y,
                    -s * delta.x + c * delta.y,
                    delta.z,
                );
                let seed = (object.position.x as u32)
                    .wrapping_mul(31)
                    .wrapping_add((object.position.y as u32).wrapping_mul(17));
                if self.light.synthwave > 0.0 && hash(seed) < 100 {
                    let accent = if hash(seed) < 65 {
                        rgb(242, 74, 247)
                    } else {
                        rgb(83, 206, 255)
                    };
                    for (a, b) in [
                        (vec3(-11.1, -14.1, 75.5), vec3(11.1, -14.1, 75.5)),
                        (vec3(11.1, -14.1, 75.5), vec3(11.1, 14.1, 75.5)),
                    ] {
                        self.emissive_line(
                            camera,
                            local(a),
                            local(b),
                            accent,
                            self.light.synthwave,
                        );
                    }
                }
                for floor in 1..12 {
                    let z = floor as f32 * 6.0;
                    let lit = if floor % 3 == 0 {
                        rgb(255, 214, 139)
                    } else {
                        rgb(139, 203, 220)
                    };
                    for side in [-1.0, 1.0] {
                        for window in [-1.0, 0.0, 1.0] {
                            let lit = if self.light.synthwave > 0.0 {
                                let id = hash(
                                    seed.wrapping_add(floor as u32 * 137)
                                        .wrapping_add(((window + 1.0) * 47.0) as u32)
                                        .wrapping_add(((side + 1.0) * 67.0) as u32),
                                );
                                let neon = if id > 80 {
                                    self.light.color(color)
                                } else {
                                    match id % 11 {
                                        0..=1 => rgb(255, 100, 223),
                                        2 => rgb(102, 209, 255),
                                        _ => rgb(255, 192, 119),
                                    }
                                };
                                mix(lit, neon, self.light.synthwave)
                            } else {
                                lit
                            };
                            if eye.y * side > 14.0 {
                                self.emissive_line(
                                    camera,
                                    local(vec3(window * 6.0 - 1.5, side * 14.5, z)),
                                    local(vec3(window * 6.0 + 1.5, side * 14.5, z)),
                                    lit,
                                    1.0,
                                );
                            }
                            if eye.x * side > 11.0 {
                                self.emissive_line(
                                    camera,
                                    local(vec3(side * 11.5, window * 8.0 - 2.0, z)),
                                    local(vec3(side * 11.5, window * 8.0 + 2.0, z)),
                                    lit,
                                    1.0,
                                );
                            }
                        }
                    }
                }
            }
            Structure::Windmill => {
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 23.0),
                    vec3(2.7, 2.7, 23.0),
                    steel,
                );
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 48.0),
                    vec3(4.0, 6.0, 3.0),
                    rgb(227, 235, 232),
                );
                let hub = vec3(0.0, -6.5, 48.0);
                for i in 0..3 {
                    let a = time * 1.1 + i as f32 * std::f32::consts::TAU / 3.0;
                    let d = vec3(a.sin(), 0.0, a.cos());
                    let r = vec3(a.cos(), 0.0, -a.sin());
                    self.triangle(
                        camera,
                        [
                            local(hub - r * 1.8),
                            local(hub + d * 23.0 + r * 0.8),
                            local(hub + d * 6.0 + r * 2.5),
                        ],
                        rgb(239, 244, 231),
                    );
                    self.world_line(
                        camera,
                        local(hub + d * 19.0),
                        local(hub + d * 23.0),
                        rgb(225, 93, 66),
                    );
                }
            }
            Structure::Lighthouse => {
                for band in 0..6 {
                    let width = 6.0 - band as f32 * 0.45;
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(0.0, 0.0, band as f32 * 5.0 + 2.5),
                        vec3(width, width, 2.5),
                        if band % 2 == 0 {
                            rgb(230, 232, 215)
                        } else {
                            rgb(192, 65, 56)
                        },
                    );
                }
                self.box_mesh(
                    camera,
                    &local,
                    vec3(0.0, 0.0, 33.0),
                    vec3(5.0, 5.0, 3.0),
                    rgb(244, 219, 139),
                );
                let a = time * 0.7;
                self.world_line(
                    camera,
                    local(vec3(0.0, 0.0, 34.0)),
                    local(vec3(a.cos() * 65.0, a.sin() * 65.0, 32.0)),
                    rgb(255, 237, 163),
                );
            }
            Structure::RockStack => {
                for level in 0..5 {
                    let width = 10.0 - level as f32 * 0.8;
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(0.0, 0.0, level as f32 * 10.0 + 5.0),
                        vec3(width, width, 5.0),
                        if level % 2 == 0 {
                            rgb(195, 123, 72)
                        } else {
                            rgb(154, 86, 52)
                        },
                    );
                }
            }
            _ => {}
        }
    }
}
