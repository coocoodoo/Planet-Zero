use super::*;
use glam::{Vec3, vec3};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Structure {
    Base,
    Satellite,
    Cruiser,
    Asteroid,
}

#[derive(Clone, Copy)]
pub struct Scenery {
    pub position: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub kind: Structure,
}

impl World {
    pub(super) fn cosmic(kind: MapKind, seed: u32) -> Self {
        let mut world = Self {
            heights: vec![0; SIZE * SIZE],
            materials: vec![27; SIZE * SIZE],
            clouds: vec![0; CLOUD_SIZE * CLOUD_SIZE],
            spawn: (2048.0, 2900.0),
            kind,
            seed,
            scenery: vec![],
            portals: vec![],
        };
        let mut rng = Random(seed);
        if kind == MapKind::Moon {
            let mut field = vec![0.0; NOISE_SIZE * NOISE_SIZE];
            for &(cell, amplitude) in &crate::content::maps::definition(kind).elevation_octaves {
                octave(&mut field, cell, amplitude, &mut rng);
            }
            normalize(&mut field);
            for value in &mut field {
                *value = 21.0 + *value * 18.0;
            }
            // 大型环形山和小型撞击坑共用周期高度图
            for i in 0..210 {
                let (cx, cy, radius) = if i == 0 {
                    (578.0, 600.0, 105.0)
                } else {
                    (
                        rng.next() * 1024.0,
                        rng.next() * 1024.0,
                        5.0 + rng.next().powi(3) * 75.0,
                    )
                };
                let extent = (radius * 1.3).ceil() as i32;
                for y in -extent..=extent {
                    for x in -extent..=extent {
                        let r = ((x * x + y * y) as f32).sqrt() / radius;
                        if r > 1.3 {
                            continue;
                        }
                        let rim = (1.0 - ((r - 0.92) / 0.19).abs()).max(0.0);
                        let bowl = (1.0 - (r / 0.83).powi(2)).max(0.0);
                        let peak = if radius > 45.0 {
                            (1.0 - r / 0.2).max(0.0) * 8.0
                        } else {
                            0.0
                        };
                        let at = index(cx as i32 + x, cy as i32 + y, NOISE_SIZE);
                        field[at] += rim * (radius * 0.34).min(30.0)
                            - bowl * (radius * 0.3).min(23.0)
                            + peak;
                    }
                }
            }
            for y in 0..SIZE {
                for x in 0..SIZE {
                    let gx = x as i32 / CELL as i32;
                    let gy = y as i32 / CELL as i32;
                    let tx = (x % CELL) as f32 / CELL as f32;
                    let ty = (y % CELL) as f32 / CELL as f32;
                    let a = field[index(gx, gy, NOISE_SIZE)];
                    let b = field[index(gx + 1, gy, NOISE_SIZE)];
                    let c = field[index(gx, gy + 1, NOISE_SIZE)];
                    let d = field[index(gx + 1, gy + 1, NOISE_SIZE)];
                    let h = a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty;
                    let at = y * SIZE + x;
                    world.heights[at] = h.clamp(3.0, 79.0) as u8;
                    world.materials[at] = if b - a + c - a > 0.3 {
                        28
                    } else if hash(at as u32 ^ seed) < 18 {
                        29
                    } else {
                        27
                    };
                }
            }
            for (x, y) in [
                (2210.0, 2630.0),
                (1100.0, 900.0),
                (3250.0, 1100.0),
                (700.0, 3200.0),
            ] {
                let z = world.height(x, y);
                for dy in -78..=78 {
                    for dx in -110..=110 {
                        let at = index(x as i32 + dx, y as i32 + dy, SIZE);
                        world.heights[at] = z as u8;
                        if dx.abs() < 103 && dy.abs() < 70 {
                            world.materials[at] = 29;
                        }
                    }
                }
                world.scenery.push(Scenery {
                    position: vec3(x, y, z),
                    yaw: 0.0,
                    scale: 1.0,
                    kind: Structure::Base,
                });
            }
        } else {
            let forward = vec3(0.55_f32.sin(), -0.55_f32.cos(), 0.0);
            world.portals = vec![
                portal::Portal {
                    center: vec3(2048.0, 2900.0, 140.0) + forward * 650.0,
                    yaw: 0.55 + PI,
                    radius: 58.0,
                },
                portal::Portal {
                    center: vec3(800.0, 900.0, 190.0),
                    yaw: -1.1,
                    radius: 58.0,
                },
            ];
            for (position, yaw, scale, kind) in [
                (vec3(2350.0, 2510.0, 120.0), -0.3, 1.4, Structure::Cruiser),
                (vec3(2110.0, 2620.0, 185.0), 0.4, 1.0, Structure::Satellite),
                (vec3(480.0, 560.0, 230.0), 1.8, 1.8, Structure::Cruiser),
                (vec3(540.0, 850.0, 220.0), 0.0, 1.3, Structure::Satellite),
            ] {
                world.scenery.push(Scenery {
                    position,
                    yaw,
                    scale,
                    kind,
                });
            }
            for i in 0..110 {
                world.scenery.push(Scenery {
                    position: vec3(
                        rng.next() * SIZE as f32,
                        rng.next() * SIZE as f32,
                        -100.0 + rng.next() * 520.0,
                    ),
                    yaw: rng.next() * TAU,
                    scale: 0.5 + rng.next() * 1.5,
                    kind: match i % 14 {
                        0 => Structure::Cruiser,
                        1..=3 => Structure::Satellite,
                        _ => Structure::Asteroid,
                    },
                });
            }
        }
        world
    }
}
