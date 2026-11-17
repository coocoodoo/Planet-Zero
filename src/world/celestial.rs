use super::*;
use glam::{Vec3, vec3};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Structure {
    Base,
    Satellite,
    Cruiser,
    Asteroid,
    Building,
    Windmill,
    Lighthouse,
    RockStack,
    House,
    Warehouse,
    Arch,
    Cave,
    Stairs,
    IceSpire,
    Boulder,
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
        let profile = crate::content::maps::definition(kind);
        let layout = profile.cosmic.as_ref().expect("月球和太空需要场景配置");
        let mut world = Self {
            heights: if kind == MapKind::Moon {
                vec![0; SIZE * SIZE]
            } else {
                vec![]
            },
            materials: if kind == MapKind::Moon {
                vec![27; SIZE * SIZE]
            } else {
                vec![]
            },
            obstacles: vec![],
            ceiling: 0,
            cloud_levels: vec![],
            cloud_slopes: vec![],
            airspace: Default::default(),
            clouds: vec![],
            spawn: (layout.spawn[0], layout.spawn[1]),
            kind,
            seed,
            scenery: vec![],
            caverns: vec![],
            portals: vec![],
        };
        let mut rng = Random(seed);
        if kind == MapKind::Moon {
            let mut field = vec![0.0; NOISE_SIZE * NOISE_SIZE];
            for &(cell, amplitude) in &profile.elevation_octaves {
                octave(&mut field, cell, amplitude, &mut rng);
            }
            normalize(&mut field);
            for value in &mut field {
                *value = 21.0 + *value * 18.0;
            }
            // 大型环形山和小型撞击坑共用周期高度图
            for i in 0..layout.crater_count {
                let (cx, cy, radius) = if i == 0 {
                    (
                        layout.featured_crater[0] / CELL as f32,
                        layout.featured_crater[1] / CELL as f32,
                        layout.featured_crater[2] / CELL as f32,
                    )
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
            for landmark in &layout.landmarks {
                let [x, y, _] = landmark.position;
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
                    yaw: landmark.yaw,
                    scale: landmark.scale,
                    kind: landmark.kind,
                });
            }
        } else {
            world.portals = layout
                .portals
                .iter()
                .map(|p| portal::Portal {
                    center: Vec3::from_array(p.position),
                    yaw: p.yaw,
                    radius: p.radius,
                })
                .collect();
            world.update_ceiling();
            world
                .scenery
                .extend(layout.landmarks.iter().map(|p| Scenery {
                    position: Vec3::from_array(p.position),
                    yaw: p.yaw,
                    scale: p.scale,
                    kind: p.kind,
                }));
            for i in 0..layout.debris_count {
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
        world.update_ceiling();
        world
    }
}
