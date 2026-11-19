use super::*;
use crate::world::Cloud;

impl Renderer {
    pub(super) fn draw_clouds(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
    ) {
        let opacity = (1.0 - self.warp.map_or(0.0, |warp| warp.bend)).powi(4)
            * (1.0 - crate::world::orbit::thin_air(flight.position.z));
        if world.clouds.is_empty() || opacity < 0.001 {
            return;
        }
        let coverage = flight.weather.coverage();
        let far = flight.weather.visibility(camera.position.z).1.min(1000.0);
        let drift = vec2(flight.time * 7.0, flight.time * 3.0);
        let haze = flight.weather.haze(theme);
        let dark = flight.weather.shade(grade(rgb(145, 172, 198), theme));
        let light = flight.weather.shade(grade(rgb(246, 249, 250), theme));
        let planes = [
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
        ]
        .map(|(axis, offset)| (axis + camera.forward * offset * camera.q, offset));
        let (eye, ray) = camera.ray(camera.center);
        for cloud in &world.clouds {
            let strength = (coverage - cloud.rank as f32).clamp(0.0, 1.0) * opacity;
            if strength < 0.01 {
                continue;
            }
            let scale = strength.sqrt() * cloud.scale;
            let extent = cloud.extent();
            let mut xy = cloud.center + drift;
            xy += ((camera.position.truncate() - xy) / crate::world::CLOUD_SIZE as f32).round()
                * crate::world::CLOUD_SIZE as f32;
            if xy.distance_squared(camera.position.truncate()) > (far + extent.length()).powi(2) {
                continue;
            }
            let thickness = if cloud.shape.thin { 0.12 } else { 1.0 };
            let center = xy
                .extend(world.cloud_height(xy.x, xy.y) - if cloud.shape.thin { 9.0 } else { 0.0 });
            let half = (cloud.shape.extent + Vec2::splat(2.0)).extend(18.0 * thickness) * scale;
            let delta = center - camera.position;
            if planes
                .iter()
                .any(|(normal, offset)| delta.dot(*normal) + offset + normal.abs().dot(half) < 0.0)
            {
                continue;
            }
            let depth = delta.dot(camera.forward);
            if depth - half.dot(camera.forward.abs()) > far
                || 1.0 + camera.q * (depth + half.dot(camera.forward.abs())) < 0.12
            {
                continue;
            }
            let projected_cell =
                camera.scale * Cloud::CELL * scale / (1.0 + camera.q * depth).max(0.12);
            let level = if projected_cell > 4.0 {
                2
            } else if projected_cell > 1.5 {
                1
            } else {
                0
            };
            let fog = ((depth - far * 0.7) / (far * 0.3)).clamp(0.0, 1.0);
            let fog = if cloud.shape.thin {
                0.35 + fog * 0.65
            } else {
                fog
            };
            let palette: [u32; 16] =
                std::array::from_fn(|i| mix(mix(dark, light, i as f32 / 15.0), haze, fog));
            let local_eye = (eye - center) / scale;
            for face in &cloud.shape.faces[level] {
                let toward = if camera.q > 0.00001 {
                    local_eye - face.points[0]
                } else {
                    -ray
                };
                if face.normal.dot(toward) <= 0.0 {
                    continue;
                }
                self.triangle(
                    camera,
                    face.points.map(|p| center + p * scale),
                    palette[face.shade],
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn level_view_hits_sloped_cloud_sides() {
        let mut world = World::generate(crate::world::MapKind::Islands, 7);
        let mut density = vec![-9.0; 9 * 9];
        for y in 2..7 {
            for x in 2..7 {
                density[y * 9 + x] = 36.0;
            }
        }
        world.clouds = vec![Cloud {
            center: vec2(100.0, 100.0),
            scale: 1.0,
            rank: 0,
            shape: std::sync::Arc::new(crate::world::CloudShape::new(9, &density, false)),
        }];
        world.cloud_levels.fill(110);
        world.cloud_slopes.fill([110.0, 0.0, 0.0, 0.0]);
        let mut flight = Flight::new(&world);
        flight.position = vec3(100.0, 200.0, 114.0);
        flight.follow = flight.position;
        flight.yaw = 0.0;
        flight.weights = [0.0, 1.0, 0.0];
        let mut camera = Camera::new(&flight, 320, 170);
        camera.slope = 0.0;
        let mut renderer = Renderer::single();
        renderer.w = 320;
        renderer.h = 170;
        renderer.full_width = 320;
        renderer.visible = vec![(0, 170); 320];
        renderer.pixels = vec![0; 320 * 170];
        renderer.mask = vec![0; 320 * 170];
        renderer.depth = vec![f32::INFINITY; 320 * 170];
        renderer.draw_clouds(&world, &flight, &camera, 0);
        let at = camera.center.x as usize * 170 + camera.center.y as usize;
        assert_ne!(renderer.pixels[at], 0);
        assert!((0.0..200.0).contains(&renderer.depth[at]));
    }
}
