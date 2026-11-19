use super::*;
use crate::world::{
    MapKind,
    orbit::{Body, SurfaceMap, SurfaceWarp, UV_SCALE, smooth},
};
use std::sync::OnceLock;

fn earth_texel(uv: Vec2, origin: Vec2) -> u32 {
    static TEXTURE: OnceLock<Vec<u32>> = OnceLock::new();
    let texture = TEXTURE.get_or_init(|| {
        let mut colors = Vec::with_capacity(512 * 256);
        for y in 0..256 {
            let latitude = (y as f32 / 255.0 - 0.5) * std::f32::consts::PI;
            for x in 0..512 {
                let longitude = x as f32 / 512.0 * std::f32::consts::TAU;
                let n = vec3(
                    latitude.cos() * longitude.cos(),
                    latitude.cos() * longitude.sin(),
                    latitude.sin(),
                );
                let continent =
                    (n.x * 7.0 + (n.y * 5.0).sin()).sin() + (n.z * 8.0 + n.x * 3.0).cos() * 0.65;
                let color = if latitude.abs() > 1.24 {
                    rgb(222, 239, 245)
                } else if continent > 0.52 {
                    mix(
                        rgb(43, 116, 65),
                        rgb(176, 154, 93),
                        ((continent - 0.8) * 1.5).max(0.0),
                    )
                } else {
                    mix(
                        rgb(16, 62, 128),
                        rgb(37, 115, 171),
                        (continent + 0.4).max(0.0),
                    )
                };
                let clouds = ((n.x * 19.0 + n.z * 8.0).sin() + (n.y * 23.0 - n.z * 11.0).cos()
                    - 1.0)
                    .max(0.0)
                    * 0.68;
                colors.push(mix(color, rgb(234, 245, 255), clouds));
            }
        }
        colors
    });
    let angles = (uv - origin) / UV_SCALE;
    let x = (angles.x / std::f32::consts::TAU * 512.0).rem_euclid(512.0) as usize;
    let y = ((angles.y / std::f32::consts::PI + 0.5) * 255.0).clamp(0.0, 255.0) as usize;
    texture[y * 512 + x]
}

pub(super) fn earth_color(
    surface: SurfaceWarp,
    uv: Vec2,
    color: u32,
    shade: f32,
    altitude: f32,
) -> u32 {
    if surface.body != Body::Earth {
        return color;
    }
    let amount = smooth((altitude - 1300.0) / 1700.0) * surface.bend;
    if amount <= 0.0 {
        return color;
    }
    let globe = mix(rgb(9, 17, 31), earth_texel(uv, surface.origin), shade);
    mix(color, globe, amount)
}

impl Renderer {
    pub(super) fn orbital_bodies(&mut self, world: &World, camera: &Camera, flight: &Flight) {
        let surface = flight
            .surface
            .or_else(|| {
                (world.kind != MapKind::Space).then(|| SurfaceWarp::ground(world, flight.position))
            })
            .map(|surface| surface.at_surface_position(flight.position));
        for body in Body::ALL {
            if world.kind != MapKind::Space && body == Body::of(world.kind) {
                continue;
            }
            let (center, rotation, offset) = if let Some(surface) = surface {
                let rotation = surface.basis;
                let offset = surface.space_origin() - rotation * surface.anchor.extend(0.0);
                (
                    rotation.transpose() * (body.center() - offset),
                    rotation,
                    offset,
                )
            } else {
                (body.center(), glam::Mat3::IDENTITY, Vec3::ZERO)
            };
            let delta = center - camera.position;
            let radius = body.radius() + 255.0;
            let bounds = [
                (camera.right * camera.scale, camera.center.x),
                (
                    -camera.right * camera.scale,
                    self.w as f32 - camera.center.x,
                ),
                (
                    -camera.up * camera.scale - camera.forward * camera.scale * camera.slope,
                    camera.center.y,
                ),
                (
                    camera.up * camera.scale + camera.forward * camera.scale * camera.slope,
                    self.h as f32 - camera.center.y,
                ),
                (Vec3::ZERO, 0.88),
            ];
            if bounds.into_iter().any(|(axis, offset)| {
                let plane = axis + camera.forward * (offset * camera.q);
                delta.dot(plane) + offset + plane.length() * radius < 0.0
            }) {
                continue;
            }
            let detailed = body.altitude(rotation * camera.position + offset) < 4500.0;
            if let Some(map) = flight.planet_maps[body.index()]
                .as_deref()
                .filter(|_| detailed)
            {
                let anchor = map.uv((camera.position - body.center()).normalize());
                let mut space_camera = *camera;
                space_camera.position = rotation * camera.position + offset;
                space_camera.right = rotation * camera.right;
                space_camera.up = rotation * camera.up;
                space_camera.forward = rotation * camera.forward;
                self.surface_body(&space_camera, flight, body, map, anchor);
            } else {
                self.distant_body(camera, center, body, rotation);
            }
        }
    }
    fn surface_body(
        &mut self,
        camera: &Camera,
        flight: &Flight,
        body: Body,
        map: &SurfaceMap,
        anchor: Vec2,
    ) {
        let warp = SurfaceWarp::new(body, map.origin, anchor, 1.0);
        let rotation = warp.basis.transpose();
        let mut view = *camera;
        view.position = anchor.extend(0.0) + rotation * (camera.position - warp.space_origin());
        view.right = rotation * camera.right;
        view.up = rotation * camera.up;
        view.forward = rotation * camera.forward;
        let previous = self.warp.replace(warp);
        let previous_light = self.light;
        self.light = self.light.in_frame(rotation);
        self.pitched_terrain(&map.world, flight.faceted, &view, map.environment.weather);
        self.warp = previous;
        self.light = previous_light;
    }
    fn distant_body(&mut self, camera: &Camera, center: Vec3, body: Body, rotation: glam::Mat3) {
        let radius = body.radius();
        let Some((point, _, scale)) = camera.project(center) else {
            return;
        };
        let extent = radius * scale * 1.4;
        let min = (point - Vec2::splat(extent)).max(Vec2::ZERO);
        let max = (point + Vec2::splat(extent)).min(vec2(self.w as f32 - 1.0, self.h as f32 - 1.0));
        for x in min.x as i32..=max.x as i32 {
            let y0 = (min.y as i32).max(self.visible[x as usize].0);
            let y1 = (max.y as i32 + 1).min(self.visible[x as usize].1);
            for y in y0..y1 {
                let (origin, ray) = camera.ray(vec2(x as f32, y as f32));
                let delta = origin - center;
                let a = ray.length_squared();
                let b = delta.dot(ray);
                let d = b * b - a * (delta.length_squared() - radius * radius);
                if d < 0.0 {
                    continue;
                }
                let t = (-b - d.sqrt()) / a;
                if t < 0.0 {
                    continue;
                }
                let hit = origin + ray * t;
                let depth = (hit - camera.position).dot(camera.forward);
                let at = x as usize * self.h + y as usize;
                if depth >= self.depth[at] {
                    continue;
                }
                let normal = rotation * (hit - center).normalize();
                let base = if body == Body::Earth {
                    earth_texel(
                        vec2(normal.y.atan2(normal.x), normal.z.asin()) * UV_SCALE,
                        Vec2::ZERO,
                    )
                } else {
                    rgb(164, 167, 174)
                };
                self.pixels[at] = mix(
                    rgb(17, 23, 35),
                    base,
                    normal.dot(self.light.sun).max(0.0) * 0.92 + 0.08,
                );
                self.depth[at] = depth;
            }
        }
    }
}
