use super::{MapKind, SIZE, World, index};
use crate::flight::MIN_CLEARANCE;
use glam::Vec3;

const GRID: usize = 256;
const CELL: f32 = 16.0;
pub const SLOPE: f32 = 0.18;

#[derive(Clone, Default)]
pub struct Airspace {
    pub baseline: f32,
    floors: Vec<f32>,
}

impl Airspace {
    pub(super) fn build(world: &World) -> Self {
        if world.kind == MapKind::Space {
            return Self::default();
        }
        let mut histogram = [0_usize; 256];
        let mut floors = vec![0.0_f32; GRID * GRID];
        for y in 0..SIZE {
            for x in 0..GRID {
                let at = (y / 16) * GRID + x;
                let start = y * SIZE + x * 16;
                let highest = (0..16).map(|dx| world.heights[start + dx]).max().unwrap();
                floors[at] = floors[at].max(highest as f32);
                if y.is_multiple_of(4) {
                    for dx in (0..16).step_by(4) {
                        histogram[world.heights[start + dx] as usize] += 1;
                    }
                }
            }
        }
        let mut count = 0;
        let ground = histogram
            .iter()
            .position(|&n| {
                count += n;
                count >= world.heights.len() / 16 * 3 / 5
            })
            .unwrap_or(0) as f32;
        let mut structures = (0.0, 0);
        for object in &world.scenery {
            use super::celestial::Structure;
            // 只统计普通房屋与楼宇，地标和自然构件不代表街区高度
            if !matches!(
                object.kind,
                Structure::House | Structure::Warehouse | Structure::Building
            ) {
                continue;
            }
            let height = object.kind.footprint().map_or(0.0, |size| size.z) * object.scale;
            if height <= 48.0 {
                structures.0 += height;
                structures.1 += 1;
            }
        }
        let average = if structures.1 > 0 {
            structures.0 / structures.1 as f32
        } else {
            0.0
        };
        let baseline = ground + MIN_CLEARANCE + average;
        for (i, floor) in floors.iter_mut().enumerate() {
            *floor = (*floor + MIN_CLEARANCE).max(baseline);
            if let Some(&obstacle) = world.obstacles.get(i) {
                *floor = floor.max(obstacle as f32 + MIN_CLEARANCE);
            }
        }
        // 高度包络按固定坡度向周围扩展，网格边界按地图周期连接
        for reverse in [false, true, false, true] {
            for cell in 0..GRID * GRID {
                let at = if reverse {
                    GRID * GRID - 1 - cell
                } else {
                    cell
                };
                let (x, y) = ((at % GRID) as i32, (at / GRID) as i32);
                let direction = if reverse { 1 } else { -1 };
                for (dx, dy) in [
                    (direction, 0),
                    (0, direction),
                    (direction, direction),
                    (-direction, direction),
                ] {
                    let cost = CELL
                        * SLOPE
                        * if dx != 0 && dy != 0 {
                            std::f32::consts::SQRT_2
                        } else {
                            1.0
                        };
                    floors[at] = floors[at].max(floors[index(x + dx, y + dy, GRID)] - cost);
                }
            }
        }
        Self { baseline, floors }
    }

    pub fn floor(&self, world: &World, position: Vec3) -> f32 {
        if world.kind == MapKind::Space {
            return -240.0;
        }
        let ground = world.flight_height(position);
        if self.floors.is_empty()
            || (world.kind == MapKind::Canyon && ground + 40.0 < self.baseline)
            || world.flight_ceiling(position) < 900.0
        {
            return ground + MIN_CLEARANCE;
        }
        let x = position.x / CELL - 0.5;
        let y = position.y / CELL - 0.5;
        let (gx, gy) = (x.floor() as i32, y.floor() as i32);
        let (tx, ty) = (x - x.floor(), y - y.floor());
        let h = |dx, dy| self.floors[index(gx + dx, gy + dy, GRID)];
        let (a, b, c, d) = (h(0, 0), h(1, 0), h(0, 1), h(1, 1));
        a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::celestial::{Scenery, Structure};
    use glam::vec3;
    #[test]
    fn ordinary_ground_is_uniform_and_towers_have_wide_approaches() {
        let mut world = World::generate(MapKind::City, 2049);
        world.heights.fill(10);
        world.obstacles.clear();
        world.scenery.clear();
        world.place_landmark(Scenery {
            position: vec3(500.0, 500.0, 10.0),
            yaw: 0.0,
            scale: 1.0,
            kind: Structure::House,
        });
        world.place_landmark(Scenery {
            position: vec3(2000.0, 2000.0, 10.0),
            yaw: 0.0,
            scale: 2.0,
            kind: Structure::Building,
        });
        world.update_ceiling();
        assert_eq!(world.airspace.baseline, 10.0 + MIN_CLEARANCE + 16.0);
        let sample = |x, y| world.airspace.floor(&world, vec3(x, y, 200.0));
        assert!((sample(500.0, 500.0) - sample(800.0, 500.0)).abs() < 0.001);
        assert!(sample(1500.0, 2000.0) > world.airspace.baseline + 40.0);
        for x in 1000..2100 {
            assert!(
                (sample(x as f32 + 1.0, 2000.0) - sample(x as f32, 2000.0)).abs() <= SLOPE + 0.001
            );
        }
        let canyon = World::generate(MapKind::Canyon, MapKind::Canyon.seed());
        let p = vec3(
            crate::world::regions_detail::canyon_center(3000.0),
            3000.0,
            50.0,
        );
        assert!(canyon.airspace.floor(&canyon, p) < canyon.airspace.baseline - 80.0);
        assert!(
            (canyon.airspace.floor(&canyon, p) - canyon.flight_height(p) - MIN_CLEARANCE).abs()
                < 0.001
        );
    }
}
