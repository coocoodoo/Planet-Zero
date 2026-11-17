use glam::{Vec3, vec3};

pub(crate) const LENS_RADIUS: f32 = 2.25;

#[derive(Clone, Copy, Debug)]
pub struct Portal {
    pub center: Vec3,
    pub yaw: f32,
    pub radius: f32,
}

impl Portal {
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

impl super::World {
    pub fn spawn_portals(&mut self, position: Vec3, forward: Vec3) {
        let heading = forward.x.atan2(-forward.y);
        let direction = vec3(heading.sin(), -heading.cos(), 0.0);
        let right = vec3(direction.y, -direction.x, 0.0);
        let radius = 48.0;
        let safe = |mut center: Vec3| {
            if self.kind != super::MapKind::Space {
                let mut floor = 0.0_f32;
                for dx in [-radius, 0.0, radius] {
                    for dy in [-radius, 0.0, radius] {
                        floor = floor.max(self.height(center.x + dx, center.y + dy));
                    }
                }
                center.z = center.z.max(floor + radius + crate::flight::MIN_CLEARANCE);
            }
            center
        };
        self.portals = vec![
            Portal {
                center: safe(position + forward * 240.0),
                yaw: heading + std::f32::consts::PI,
                radius,
            },
            Portal {
                center: safe(position + direction * 1500.0 + right * 850.0),
                yaw: heading + 0.9,
                radius,
            },
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
