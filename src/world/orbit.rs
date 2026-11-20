use super::{
    MapKind, SIZE, World,
    network::{Environment, MapKey},
};
use glam::{Mat3, Vec2, Vec3, vec2, vec3};
use std::f32::consts::{PI, TAU};

pub const UV_SCALE: f32 = SIZE as f32 / TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Moon,
    Earth,
}
impl Body {
    pub const ALL: [Self; 2] = [Self::Moon, Self::Earth];
    pub fn index(self) -> usize {
        usize::from(self == Self::Earth)
    }
    pub fn radius(self) -> f32 {
        if self == Self::Moon { 640.0 } else { 960.0 }
    }
    pub fn surface_altitude(self) -> f32 {
        self.radius()
    }
    pub fn orbit_altitude(self) -> f32 {
        self.radius() * (8.0 / 3.0)
    }
    pub fn attitude_freedom(self, altitude: f32) -> f32 {
        let high = self.surface_altitude();
        ((altitude - high * 0.5) / (high * 0.5)).clamp(0.0, 1.0)
    }
    pub fn curvature(self, altitude: f32) -> f32 {
        smooth(
            (altitude - self.surface_altitude())
                / (self.orbit_altitude() - self.surface_altitude()),
        )
    }
    pub fn center(self) -> Vec3 {
        if self == Self::Moon {
            vec3(14000.0, -5800.0, -460.0)
        } else {
            vec3(-2800.0, 5000.0, 1400.0)
        }
    }
    pub fn name(self) -> &'static str {
        if self == Self::Moon { "MOON" } else { "EARTH" }
    }
    pub fn of(kind: MapKind) -> Self {
        if kind == MapKind::Moon {
            Self::Moon
        } else {
            Self::Earth
        }
    }
    pub fn altitude(self, position: Vec3) -> f32 {
        position.distance(self.center()) - self.radius()
    }
}

pub struct SurfaceMap {
    pub key: MapKey,
    pub origin: Vec2,
    pub world: std::sync::Arc<World>,
    pub environment: Environment,
}
impl SurfaceMap {
    pub fn new(world: &std::sync::Arc<World>, environment: Environment) -> Self {
        Self {
            key: MapKey::of(world),
            origin: vec2(world.spawn.0, world.spawn.1),
            world: world.clone(),
            environment,
        }
    }
    pub fn uv(&self, normal: Vec3) -> Vec2 {
        self.origin + vec2(normal.y.atan2(normal.x), normal.z.clamp(-1.0, 1.0).asin()) * UV_SCALE
    }
}

#[derive(Clone, Copy)]
pub struct SurfaceWarp {
    pub body: Body,
    pub origin: Vec2,
    pub anchor: Vec2,
    pub basis: Mat3,
    pub bend: f32,
    latitude_axes: (f32, f32),
}
impl SurfaceWarp {
    pub fn new(body: Body, origin: Vec2, anchor: Vec2, bend: f32) -> Self {
        let angles = (anchor - origin) / UV_SCALE;
        let (sl, cl) = angles.x.sin_cos();
        let (sp, cp) = angles.y.sin_cos();
        Self {
            body,
            origin,
            anchor,
            bend,
            latitude_axes: (angles.y * bend).sin_cos(),
            basis: Mat3::from_cols(
                vec3(-sl, cl, 0.0),
                vec3(-sp * cl, -sp * sl, cp),
                vec3(cp * cl, cp * sl, sp),
            ),
        }
    }
    pub fn set_bend(&mut self, bend: f32) {
        self.bend = bend;
        self.latitude_axes = (((self.anchor.y - self.origin.y) / UV_SCALE) * bend).sin_cos();
    }
    pub fn ground(world: &World, position: Vec3) -> Self {
        let body = Body::of(world.kind);
        let other = if body == Body::Earth {
            Body::Moon
        } else {
            Body::Earth
        };
        let direction = (other.center() - body.center()).normalize();
        let axis = direction.cross(Vec3::Z).normalize();
        let n = glam::Quat::from_axis_angle(axis, 55.0_f32.to_radians()) * direction;
        let anchor = position.truncate();
        let origin = anchor - vec2(n.y.atan2(n.x), n.z.asin()) * UV_SCALE;
        Self::new(body, origin, anchor, 0.0)
    }
    pub fn at_surface_position(mut self, position: Vec3) -> Self {
        if self.bend <= 0.0 {
            let offset = position.truncate() - self.anchor;
            self.anchor += offset;
            self.origin += offset;
        }
        self
    }
    pub fn normal(&self, uv: Vec2) -> Vec3 {
        let angles = (uv - self.origin) / UV_SCALE;
        let (sl, cl) = angles.x.sin_cos();
        let (sp, cp) = angles.y.sin_cos();
        vec3(cp * cl, cp * sl, sp)
    }
    pub fn space_origin(&self) -> Vec3 {
        self.body.center() + self.basis.z_axis * self.body.radius()
    }
    pub fn point(&self, p: Vec3) -> Vec3 {
        if self.bend <= 0.0 {
            return p;
        }
        let angles = (p.truncate() - self.anchor) * self.angle_scale();
        self.point_with_axes(p, Self::axis(angles.x), Self::axis(angles.y))
    }
    pub fn angle_scale(&self) -> Vec2 {
        let radius = self.body.radius() / self.bend;
        let cp = self.latitude_axes.1.max(0.000001);
        vec2(
            ((1.0 - self.bend) / cp + self.body.radius() * self.bend / UV_SCALE) / radius,
            (1.0 + (self.body.radius() / UV_SCALE - 1.0) * self.bend) / radius,
        )
    }
    pub fn axis(angle: f32) -> Vec3 {
        let (s, c) = angle.sin_cos();
        vec3(
            s,
            c,
            if angle.abs() < 0.01 {
                angle * angle * 0.5
            } else {
                1.0 - c
            },
        )
    }
    pub fn point_with_axes(&self, p: Vec3, x: Vec3, y: Vec3) -> Vec3 {
        let radius = self.body.radius() / self.bend;
        let (sp, cp) = self.latitude_axes;
        let (sx, dx, sy, cy, dy) = (x.x, x.z, y.x, y.y, y.z);
        let cos_latitude = cp * cy - sp * sy;
        let r = radius + p.z;
        self.anchor.extend(0.0)
            + vec3(
                r * cos_latitude * sx,
                r * (sy + sp * cos_latitude * dx),
                p.z - r * (dy + cp * cos_latitude * dx),
            )
    }
    pub(crate) fn polar_row(&self, y: f32) -> Vec3 {
        // 极区使用方位投影采样地形，45° 到 75° 之间渐变衔接经纬采样
        let latitude = (y - self.origin.y) / UV_SCALE;
        let weight = smooth((latitude.abs() - PI / 4.0) / (PI / 6.0)) * self.bend;
        let radius = (PI * 0.5 - latitude.abs()).max(0.0) * UV_SCALE;
        let pole = self.origin.y + latitude.signum() * PI * 0.5 * UV_SCALE;
        vec3(radius, pole, weight)
    }
    pub(crate) fn polar_sample(&self, uv: Vec2) -> (Vec2, f32) {
        let row = self.polar_row(uv.y);
        let (s, c) = ((uv.x - self.origin.x) / UV_SCALE).sin_cos();
        (vec2(self.origin.x, row.y) + vec2(c, s) * row.x, row.z)
    }
    pub fn contains(&self, p: Vec2, margin: f32) -> bool {
        let expansion = (1.0 - self.bend) * 6000.0 + margin;
        (p.x - self.anchor.x).abs() <= PI * UV_SCALE + expansion
            && (p.y - self.origin.y).abs() <= PI * 0.5 * UV_SCALE + expansion
    }
}

pub fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 高空大气稀薄度，0 表示海平面密度，1 表示真空
pub fn thin_air(altitude: f32) -> f32 {
    smooth((altitude - 1000.0 / 3.0) / (1400.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn polar_terrain_has_one_height_and_material_location_at_each_pole() {
        let world = World::generate(MapKind::Moon, 7);
        let origin = vec2(2073.25, 1919.5);
        for body in Body::ALL {
            let mut warp = SurfaceWarp::new(body, origin, origin, 1.0);
            for sign in [-1.0, 1.0] {
                let pole = origin + vec2(0.0, sign * PI * 0.5 * UV_SCALE);
                let height = world.mesh_height(pole.x as i32, pole.y as i32);
                for longitude in [-PI, -2.0, -0.7, 0.0, 1.2, PI] {
                    let uv = pole + vec2(longitude * UV_SCALE, 0.0);
                    let (sample, weight) = warp.polar_sample(uv);
                    assert!(sample.distance(pole) < 0.001);
                    assert_eq!(weight, 1.0);
                    assert_eq!(world.mesh_height(sample.x as i32, sample.y as i32), height);
                    let p = warp.point(uv.extend(height));
                    assert!(p.distance(warp.point(pole.extend(height))) < 0.002);
                }
            }
            let equator = origin + vec2(800.0, 200.0);
            assert_eq!(warp.polar_sample(equator).1, 0.0);
            warp.set_bend(0.0);
            let uv = origin + vec2(900.0, 1024.0);
            assert_eq!(warp.polar_sample(uv).1, 0.0);
        }
    }
    #[test]
    fn surface_sky_keeps_the_other_body_distant_and_above_the_horizon() {
        for kind in [MapKind::Moon, MapKind::Islands] {
            let world = World::generate(kind, kind.seed());
            let position = vec3(world.spawn.0, world.spawn.1, 120.0);
            let surface = SurfaceWarp::ground(&world, position);
            let other = if kind == MapKind::Moon {
                Body::Earth
            } else {
                Body::Moon
            };
            let offset = |warp: SurfaceWarp, player: Vec3| {
                warp.anchor.extend(0.0)
                    + warp.basis.transpose() * (other.center() - warp.space_origin())
                    - player
            };
            let original = offset(surface, position);
            assert!(original.length() > 18000.0);
            assert!((0.3..0.8).contains(&original.normalize().z));
            let moved = position + vec3(30000.0, -16000.0, 0.0);
            assert!(original.distance(offset(surface.at_surface_position(moved), moved)) < 0.01);
        }
    }
    #[test]
    fn curvature_matches_the_sphere_and_converges_to_the_plane() {
        let origin = vec2(2048.0, 2048.0);
        for body in Body::ALL {
            for latitude in [-PI * 0.5, -1.45, -0.8, 0.0, 0.9, 1.45, PI * 0.5] {
                let anchor = origin + vec2(0.7, latitude) * UV_SCALE;
                let mut warp = SurfaceWarp::new(body, origin, anchor, 1.0);
                for offset in [
                    Vec3::ZERO,
                    vec3(170.0, -190.0, 100.0),
                    vec3(-600.0, 300.0, 80.0),
                ] {
                    let p = anchor.extend(0.0) + offset;
                    let expected =
                        body.center() + warp.normal(p.truncate()) * (body.radius() + p.z);
                    let actual =
                        warp.space_origin() + warp.basis * (warp.point(p) - anchor.extend(0.0));
                    assert!(expected.distance(actual) < 0.003);
                    warp.set_bend(0.000001);
                    assert!(warp.point(p).distance(p) < 0.03);
                    warp.set_bend(0.0);
                    assert_eq!(warp.point(p), p);
                    warp.set_bend(1.0);
                }
            }
        }
    }
}
