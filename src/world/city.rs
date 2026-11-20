use super::celestial::{Scenery, Structure};
use super::*;
use glam::{Vec2, vec2, vec3};

impl World {
    pub(super) fn city_layout(&mut self) {
        let mut roads = Vec::new();
        let center = vec2(2380.0, 2040.0);
        for ring in [520.0, 1000.0] {
            for i in 0..64 {
                let point = |i: usize| {
                    let a = i as f32 * std::f32::consts::TAU / 64.0;
                    center
                        + vec2(a.cos() * ring, a.sin() * ring * 0.8)
                        + vec2((a * 3.0).sin() * 45.0, 0.0)
                };
                roads.push((point(i), point(i + 1), 7.0));
            }
        }
        for i in 0..7 {
            let a = i as f32 * std::f32::consts::TAU / 7.0 + 0.12;
            let mut previous = center;
            for segment in 1..=16 {
                let t = segment as f32 / 16.0;
                let p = center
                    + vec2(a.cos(), a.sin()) * 1550.0 * t
                    + vec2(-a.sin(), a.cos()) * (t * 4.0).sin() * 95.0;
                roads.push((previous, p, 8.0));
                previous = p;
            }
        }
        for side in [-1.0, 1.0] {
            for y in (64..4000).step_by(64) {
                roads.push((
                    vec2(river_x(y as f32) + side * 100.0, y as f32),
                    vec2(river_x(y as f32 + 64.0) + side * 100.0, y as f32 + 64.0),
                    5.0,
                ));
            }
        }
        for y in [980.0, 1910.0, 2980.0] {
            roads.push((vec2(900.0, y), vec2(3400.0, y + 230.0), 8.0));
        }
        let districts: [(Vec2, f32); 5] = [
            (vec2(2780.0, 1450.0), 0.24),
            (vec2(3150.0, 2350.0), -0.32),
            (vec2(2530.0, 2950.0), 0.15),
            (vec2(1170.0, 1380.0), -0.22),
            (vec2(1150.0, 2820.0), 0.37),
        ];
        for (c, a) in districts {
            let (s, cosa) = a.sin_cos();
            let local = |x, y| c + vec2(cosa * x - s * y, s * x + cosa * y);
            for row in -3..=3 {
                let y = row as f32 * 90.0;
                roads.push((
                    local(-300.0, y),
                    local(300.0, y + 20.0 * (row as f32).sin()),
                    3.2,
                ));
            }
            roads.push((local(-170.0, -330.0), local(-170.0, 330.0), 4.0));
            roads.push((local(180.0, -330.0), local(180.0, 330.0), 4.0));
        }
        for &(a, b, w) in &roads {
            self.paint_road(a, b, w);
        }
        let mut random = Random(self.seed ^ 0xb37ac91);
        let mut occupied = std::collections::HashSet::new();
        for &(a, b, width) in &roads {
            let direction = (b - a).normalize_or_zero();
            let normal = vec2(-direction.y, direction.x);
            let length = (b - a).length();
            let steps = (length / 40.0).ceil() as usize;
            for n in 0..steps {
                for side in [-1.0, 1.0] {
                    if random.next() < 0.18 {
                        continue;
                    }
                    let t = (n as f32 + random.next() * 0.5) / steps as f32;
                    let p = a.lerp(b, t) + normal * side * (width + 23.0 + random.next() * 9.0);
                    if p.min_element() < 100.0 || p.max_element() > 3990.0 {
                        continue;
                    }
                    let cell = (p.x as i32 / 32, p.y as i32 / 32);
                    if (-1..=1)
                        .any(|dx| (-1..=1).any(|dy| occupied.contains(&(cell.0 + dx, cell.1 + dy))))
                    {
                        continue;
                    }
                    let at = index(p.x as i32, p.y as i32, SIZE);
                    if self.materials[at] < 4 || matches!(self.materials[at], 30..=32) {
                        continue;
                    }
                    let distance = p.distance(center);
                    let (kind, scale) = if distance < 530.0 {
                        (Structure::Building, 0.7 + random.next() * 1.9)
                    } else if distance < 880.0 && random.next() > 0.45 {
                        (Structure::Building, 0.3 + random.next() * 0.7)
                    } else if p.x < 1450.0 && p.y > 2300.0 && random.next() > 0.65 {
                        (Structure::Warehouse, 0.6 + random.next() * 0.4)
                    } else {
                        (Structure::House, 0.65 + random.next() * 0.5)
                    };
                    let z = self.heights[at] as f32;
                    for dy in -18..=18 {
                        for dx in -18..=18 {
                            let at = index(p.x as i32 + dx, p.y as i32 + dy, SIZE);
                            if self.materials[at] >= 4 && !matches!(self.materials[at], 30..=32) {
                                self.heights[at] = z as u8;
                                self.materials[at] = if matches!(kind, Structure::House) {
                                    7
                                } else {
                                    33
                                };
                            }
                        }
                    }
                    self.place_landmark(Scenery {
                        position: vec3(p.x, p.y, z),
                        yaw: direction.y.atan2(direction.x),
                        scale,
                        kind,
                    });
                    occupied.insert(cell);
                }
            }
        }
        self.spawn = (2390.0, 3220.0);
    }
    pub(super) fn paint_road(&mut self, a: Vec2, b: Vec2, width: f32) {
        let length = a.distance(b);
        let d = (b - a) / length;
        let n = vec2(-d.y, d.x);
        let axis = usize::from(d.y.abs() > d.x.abs());
        let other = 1 - axis;
        let radius = width + 3.0;
        let extent = radius / d[axis].abs();
        let first = (a[axis].min(b[axis]) - radius).floor() as i32;
        let last = (a[axis].max(b[axis]) + radius).ceil() as i32;
        for along in first..=last {
            let center = a[other] + (along as f32 - a[axis]) * d[other] / d[axis];
            for across in (center - extent).floor() as i32..=(center + extent).ceil() as i32 {
                let mut p = Vec2::ZERO;
                p[axis] = along as f32;
                p[other] = across as f32;
                let progress = (p - a).dot(d).clamp(0.0, length);
                let distance = p.distance_squared(a + d * progress);
                if distance > radius * radius {
                    continue;
                }
                let at = index(p.x as i32, p.y as i32, SIZE);
                let material = if distance > width * width {
                    32
                } else if (p - a).dot(n).abs() < 0.6 && progress as usize % 24 < 12 {
                    31
                } else {
                    30
                };
                if material == 32 && matches!(self.materials[at], 30 | 31) {
                    continue;
                }
                self.materials[at] = material;
                self.heights[at] = self.heights[at].max(5);
            }
        }
    }
}

pub(super) fn river_x(y: f32) -> f32 {
    1760.0 + (y * std::f32::consts::TAU / 4096.0).sin() * 190.0 + (y * 0.003).sin() * 65.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roads_fill_rotated_pixels_and_keep_junctions_open() {
        let mut world = World::generate(MapKind::Moon, 7);
        world.materials.fill(6);
        let center = vec2(500.0, 500.0);
        for angle in [0.2_f32, 0.6, 1.2, 2.4] {
            let d = vec2(angle.cos(), angle.sin());
            let n = vec2(-d.y, d.x);
            world.paint_road(center - d * 90.0, center + d * 90.0, 4.0);
            for y in 410..590 {
                for x in 410..590 {
                    let offset = vec2(x as f32, y as f32) - center;
                    if offset.dot(d).abs() < 88.0 && offset.dot(n).abs() < 3.5 {
                        assert!(matches!(world.materials[index(x, y, SIZE)], 30 | 31));
                    }
                }
            }
        }
        world.paint_road(center - vec2(100.0, 0.0), center + vec2(100.0, 0.0), 4.0);
        world.paint_road(center, center + vec2(0.0, 100.0), 4.0);
        for x in 410..590 {
            assert!(matches!(world.materials[index(x, 500, SIZE)], 30 | 31));
        }
    }

    #[test]
    fn highland_road_connects_all_four_settlements() {
        let world = World::generate(MapKind::Highlands, MapKind::Highlands.seed());
        for i in 0..=900 {
            let p = vec2(1650.0, 2600.0) + vec2(210.0, 115.0) * (i as f32 / 300.0);
            assert!(matches!(
                world.materials[index(p.x as i32, p.y as i32, SIZE)],
                30 | 31
            ));
        }
    }
}
