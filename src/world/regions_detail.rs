use super::celestial::{Scenery, Structure};
use super::*;
use glam::{Vec3, vec2, vec3};

#[derive(Clone)]
pub struct Cavern {
    pub center: Vec3,
    pub yaw: f32,
    pub length: f32,
    pub width: f32,
}
impl Cavern {
    pub fn roof(&self, x: f32, y: f32) -> Option<f32> {
        let d = vec2(x - self.center.x, y - self.center.y);
        let (s, c) = self.yaw.sin_cos();
        let local = vec2(c * d.x + s * d.y, -s * d.x + c * d.y);
        if local.y.abs() > self.length || local.x.abs() > self.width {
            return None;
        }
        Some(self.center.z + 24.0 + (1.0 - (local.x / self.width).powi(2)).max(0.0).sqrt() * 50.0)
    }
}
pub fn canyon_center(y: f32) -> f32 {
    2048.0 + (y * std::f32::consts::TAU / 4096.0).sin() * 270.0 + (y * 0.0035).sin() * 95.0
}
impl World {
    pub(super) fn enrich_region(&mut self) {
        if self.kind == MapKind::Canyon {
            self.carve_canyon();
        }
        if !matches!(
            self.kind,
            MapKind::Canyon
                | MapKind::Fjords
                | MapKind::Icebergs
                | MapKind::Highlands
                | MapKind::City
        ) {
            return;
        }
        let mut random = Random(self.seed ^ 0x93adb501);
        for _ in 0..2000 {
            let x = (random.next() * SIZE as f32) as i32;
            let y = (random.next() * SIZE as f32) as i32;
            let at = index(x, y, SIZE);
            let h = self.heights[at] as f32;
            let flat = (self.heights[index(x + 12, y, SIZE)] as f32 - h).abs() < 4.0
                && (self.heights[index(x, y + 12, SIZE)] as f32 - h).abs() < 4.0;
            if self.kind == MapKind::Icebergs {
                if h > 16.0 && random.next() < 0.12 {
                    self.place_landmark(Scenery {
                        position: vec3(x as f32, y as f32, h),
                        yaw: 0.0,
                        scale: 0.3 + random.next(),
                        kind: Structure::IceSpire,
                    });
                }
            } else if flat && (6..=8).contains(&self.materials[at]) {
                let radius = if self.kind == MapKind::City { 10 } else { 18 };
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let p = index(x + dx, y + dy, SIZE);
                        let n = hash(p as u32 ^ self.seed);
                        if dx * dx + dy * dy < radius * radius
                            && (6..=8).contains(&self.materials[p])
                            && n < 155
                        {
                            self.heights[p] = self.heights[p].saturating_add(3 + (n % 4));
                            self.materials[p] = 9 + n % 3;
                        }
                    }
                }
            }
            if flat && h > 4.0 && h < 65.0 && self.kind == MapKind::Fjords && random.next() < 0.08 {
                self.place_landmark(Scenery {
                    position: vec3(x as f32, y as f32, h),
                    yaw: random.next() * std::f32::consts::TAU,
                    scale: 0.7,
                    kind: Structure::House,
                });
            }
            if h > 6.0 && self.kind == MapKind::Highlands && random.next() < 0.04 {
                self.place_landmark(Scenery {
                    position: vec3(x as f32, y as f32, h),
                    yaw: random.next() * std::f32::consts::TAU,
                    scale: 0.4 + random.next() * 0.7,
                    kind: Structure::Boulder,
                });
            }
        }
        if self.kind == MapKind::Highlands {
            for i in 0..4 {
                let p = vec2(1650.0 + i as f32 * 210.0, 2600.0 + i as f32 * 115.0);
                self.paint_road(p - vec2(140.0, 110.0), p + vec2(180.0, 160.0), 2.0);
                for dy in -65..=65 {
                    for dx in -85..=85 {
                        let at = index(p.x as i32 + dx, p.y as i32 + dy, SIZE);
                        if (6..=8).contains(&self.materials[at]) {
                            self.materials[at] = if (dx + dy / 3).rem_euclid(8) < 3 {
                                7
                            } else {
                                8
                            };
                        }
                    }
                }
                let position = vec3(p.x + 100.0, p.y, self.height(p.x + 100.0, p.y));
                self.place_landmark(Scenery {
                    position,
                    yaw: 0.3,
                    scale: 1.0,
                    kind: Structure::House,
                });
            }
        }
    }
    fn carve_canyon(&mut self) {
        for y in 0..SIZE {
            let center = canyon_center(y as f32);
            let width = 54.0 + (y as f32 * 0.002).sin() * 10.0;
            for x in 0..SIZE {
                let at = y * SIZE + x;
                let d = (x as f32 - center).abs();
                let rise = ((d - width) / 44.0).clamp(0.0, 1.0);
                let base = self.heights[at] as f32;
                let h = 5.0 + rise.powf(0.45) * (115.0 + base * 0.45);
                self.heights[at] = h.min(230.0) as u8;
                self.materials[at] = if d < 10.0 {
                    2
                } else if d < width {
                    17 + hash(at as u32) % 2
                } else if rise < 1.0 {
                    19 + (h as usize / 12 % 2) as u8
                } else {
                    18
                };
            }
        }
        self.spawn = (canyon_center(3350.0), 3350.0);
        for y in [2770.0, 1640.0] {
            self.scenery.push(Scenery {
                position: vec3(canyon_center(y), y, 5.0),
                yaw: 0.0,
                scale: 1.0,
                kind: Structure::Arch,
            });
        }
        for y in [2320.0, 1120.0] {
            let center = vec3(canyon_center(y) + 150.0, y, 6.0);
            let cavern = Cavern {
                center,
                yaw: std::f32::consts::FRAC_PI_2,
                length: 110.0,
                width: 48.0,
            };
            for dy in -48..=48 {
                for dx in -190..=190 {
                    let at = index(center.x as i32 + dx, center.y as i32 + dy, SIZE);
                    self.heights[at] = 6;
                    self.materials[at] = if dy.abs() > 40 {
                        12 + hash(at as u32) % 3
                    } else {
                        17 + u8::from(hash(at as u32) < 90)
                    };
                }
            }
            let turn = vec2(center.x + 160.0, y);
            let finish_y = y + 330.0;
            let points = [
                turn,
                vec2(turn.x + 25.0, y + 130.0),
                vec2(turn.x - 20.0, finish_y - 70.0),
                vec2(canyon_center(finish_y), finish_y),
            ];
            for segment in points.windows(2) {
                let length = segment[0].distance(segment[1]);
                for step in 0..=length as usize {
                    let p = segment[0].lerp(segment[1], step as f32 / length);
                    for dy in -46..=46 {
                        for dx in -46..=46 {
                            if dx * dx + dy * dy > 46 * 46 {
                                continue;
                            }
                            let at = index(p.x as i32 + dx, p.y as i32 + dy, SIZE);
                            self.heights[at] = 6;
                            self.materials[at] = 17 + u8::from(hash(at as u32) < 90);
                        }
                    }
                }
            }
            self.scenery.push(Scenery {
                position: center,
                yaw: cavern.yaw,
                scale: 1.0,
                kind: Structure::Cave,
            });
            for i in 0..10 {
                let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                self.scenery.push(Scenery {
                    position: center + vec3((i as f32 - 4.5) * 19.0, side * 34.0, 0.0),
                    yaw: i as f32,
                    scale: 0.3 + (i % 3) as f32 * 0.15,
                    kind: Structure::Boulder,
                });
            }
            self.caverns.push(cavern);
            self.scenery.push(Scenery {
                position: vec3(canyon_center(y) - 62.0, y + 80.0, 12.0),
                yaw: 0.5,
                scale: 1.0,
                kind: Structure::Stairs,
            });
        }
    }
    pub fn flight_height(&self, position: Vec3) -> f32 {
        for cave in &self.caverns {
            if cave
                .roof(position.x, position.y)
                .is_some_and(|roof| position.z < roof - 6.0)
            {
                return self.heights[index(position.x as i32, position.y as i32, SIZE)] as f32;
            }
        }
        self.height(position.x, position.y)
    }
    pub fn flight_ceiling(&self, position: Vec3) -> f32 {
        self.caverns
            .iter()
            .filter_map(|c| c.roof(position.x, position.y))
            .filter(|roof| position.z < *roof - 6.0)
            .map(|roof| roof - 12.0)
            .fold(900.0, f32::min)
    }
}
