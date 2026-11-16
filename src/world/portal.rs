use glam::{Vec3, vec3};

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
        exit.yaw - self.yaw + std::f32::consts::PI
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
    pub fn crossed(self, from: Vec3, to: Vec3) -> bool {
        let a = (from - self.center).dot(self.normal());
        let b = (to - self.center).dot(self.normal());
        if a <= 0.0 || b > 0.0 {
            return false;
        }
        let hit = from.lerp(to, a / (a - b)) - self.center;
        hit.length_squared() < (self.radius - 4.0).powi(2)
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
    fn high_speed_crossing_requires_front_face_and_clear_aperture() {
        let p = Portal {
            center: Vec3::ZERO,
            yaw: 0.0,
            radius: 58.0,
        };
        assert!(p.crossed(vec3(0.0, -300.0, 20.0), vec3(0.0, 300.0, 20.0)));
        assert!(!p.crossed(vec3(55.0, -300.0, 0.0), vec3(55.0, 300.0, 0.0)));
        assert!(!p.crossed(vec3(0.0, 300.0, 0.0), vec3(0.0, -300.0, 0.0)));
        assert!(!p.crossed(vec3(0.0, -300.0, 0.0), vec3(0.0, -1.0, 0.0)));
    }
}
