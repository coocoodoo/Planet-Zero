use glam::{Vec3, vec3};

pub(crate) const LENS_RADIUS: f32 = 2.25;
pub(crate) const PREVIEW_DISTANCE: f32 = 1000.0;
pub(crate) const PRELOAD_DISTANCE: f32 = 1200.0;
pub(crate) const MIN_SPACING: f32 = 2600.0;

#[derive(Clone, Copy, Debug)]
pub struct Portal {
    pub center: Vec3,
    pub yaw: f32,
    pub radius: f32,
}

impl Portal {
    pub(super) fn place_height(&mut self, world: &super::World) {
        if world.kind == super::MapKind::Space {
            return;
        }
        let mut floor = 0.0_f32;
        for dx in [-self.radius, 0.0, self.radius] {
            for dy in [-self.radius, 0.0, self.radius] {
                floor = floor.max(world.height(self.center.x + dx, self.center.y + dy));
            }
        }
        let minimum = (floor + self.radius + crate::flight::MIN_CLEARANCE)
            .max(world.airspace.floor(world, self.center) + self.radius);
        self.center.z = self.center.z.max(minimum);
        if let Some((bottom, top)) = world.cloud_layer_range() {
            let clearance = self.radius * 1.08 + crate::flight::MIN_CLEARANCE;
            let below = bottom - clearance;
            let above = top + clearance;
            if self.center.z > below && self.center.z < above {
                self.center.z =
                    if below >= minimum && self.center.z - below <= above - self.center.z {
                        below
                    } else {
                        above
                    };
            }
        }
    }

    pub fn normal(self) -> Vec3 {
        vec3(self.yaw.sin(), -self.yaw.cos(), 0.0)
    }
    pub fn right(self) -> Vec3 {
        vec3(self.yaw.cos(), self.yaw.sin(), 0.0)
    }
    pub fn rotation_to(self, exit: Self) -> f32 {
        crate::flight::angle(exit.yaw - self.yaw + std::f32::consts::PI)
    }
    pub fn transform(self, exit: Self, point: Vec3) -> Vec3 {
        let delta = point - self.center;
        let (s, c) = self.rotation_to(exit).sin_cos();
        exit.center
            + vec3(
                c * delta.x - s * delta.y,
                s * delta.x + c * delta.y,
                delta.z,
            )
    }
    pub fn facing(self, observer: Vec3) -> Self {
        let d = observer - self.center;
        Self {
            yaw: if d.truncate().length_squared() > 0.001 {
                d.x.atan2(-d.y)
            } else {
                self.yaw
            },
            ..self
        }
    }
    pub fn crossed(self, from: Vec3, to: Vec3) -> bool {
        let delta = to - from;
        let offset = from - self.center;
        if delta.length_squared() < 0.000001 || offset.dot(delta) >= 0.0 {
            return false;
        }
        let t = (-offset.dot(delta) / delta.length_squared()).clamp(0.0, 1.0);
        (from + delta * t - self.center).length_squared() < (self.radius * 0.7).powi(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portal_height_uses_the_nearest_clear_side_of_the_cloud_sea() {
        let mut world = super::super::World::new(7);
        world.heights.fill(0);
        world.obstacles.clear();
        world.scenery.clear();
        world.airspace = Default::default();
        let (bottom, top) = world.cloud_layer_range().unwrap();
        let mut portal = Portal {
            center: vec3(100.0, 100.0, bottom),
            yaw: 0.0,
            radius: 48.0,
        };
        let clearance = portal.radius * 1.08 + crate::flight::MIN_CLEARANCE;
        let below = bottom - clearance;
        let above = top + clearance;
        for (requested, expected) in [
            (bottom, below),
            (top, above),
            (below - 10.0, below - 10.0),
            (above + 10.0, above + 10.0),
        ] {
            portal.center.z = requested;
            portal.place_height(&world);
            assert!((portal.center.z - expected).abs() < 0.001);
        }
        world.heights.fill(180);
        portal.center.z = bottom;
        portal.place_height(&world);
        assert!((portal.center.z - above).abs() < 0.001);
        world.clouds.clear();
        portal.center.z = top;
        portal.place_height(&world);
        assert_eq!(portal.center.z, top);
    }

    #[test]
    fn portal_mapping_preserves_distances_and_reverses_the_surface_normal() {
        let a = Portal {
            center: vec3(5.0, 12.0, 30.0),
            yaw: 0.7,
            radius: 58.0,
        };
        let b = Portal {
            center: vec3(-400.0, 800.0, 200.0),
            yaw: -1.4,
            radius: 58.0,
        };
        let point = a.center + a.right() * 12.0 + Vec3::Z * 5.0 + a.normal() * 28.0;
        let out = a.transform(b, point);
        assert!(
            out.distance(b.center - b.right() * 12.0 + Vec3::Z * 5.0 - b.normal() * 28.0) < 0.001
        );
        assert!(b.transform(a, out).distance(point) < 0.001);
        assert!((point.distance(a.center) - out.distance(b.center)).abs() < 0.001);
    }
    #[test]
    fn high_speed_crossing_accepts_every_approach_and_rejects_the_outer_rim() {
        let p = Portal {
            center: Vec3::ZERO,
            yaw: 0.0,
            radius: 58.0,
        };
        assert!(p.crossed(vec3(0.0, -300.0, 20.0), vec3(0.0, 300.0, 20.0)));
        assert!(!p.crossed(vec3(55.0, -300.0, 0.0), vec3(55.0, 300.0, 0.0)));
        assert!(p.crossed(vec3(0.0, 300.0, 0.0), vec3(0.0, -300.0, 0.0)));
        assert!(!p.crossed(vec3(0.0, -300.0, 0.0), vec3(0.0, -80.0, 0.0)));
    }
}
