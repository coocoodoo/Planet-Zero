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
        let dark = flight.weather.shade(grade(rgb(182, 201, 219), theme));
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
        let (eye, eye_ray) = camera.ray(camera.center);
        for (number, cloud) in world.clouds.iter().enumerate() {
            let strength = (coverage - cloud.rank as f32).clamp(0.0, 1.0) * opacity;
            if strength < 0.01 {
                continue;
            }
            let scale = strength.sqrt();
            let extent = cloud.extent();
            let mut xy = cloud.center + drift;
            xy += ((camera.position.truncate() - xy) / crate::world::CLOUD_SIZE as f32).round()
                * crate::world::CLOUD_SIZE as f32;
            if xy.distance_squared(camera.position.truncate()) > (far + extent.length()).powi(2) {
                continue;
            }
            let center = xy.extend(world.cloud_height(xy.x, xy.y));
            let half = (extent + Vec2::splat(2.0)).extend(12.0) * scale;
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
            let projected_cell = camera.scale * Cloud::CELL / (1.0 + camera.q * depth).max(0.12);
            let step = if projected_cell < 1.5 { 2 } else { 1 };
            let height = cloud.heights.len() / cloud.width;
            let point = |x: usize, y: usize| {
                let h = cloud.heights[y * cloud.width + x] as f32;
                let seed = (number * 7919 + x * 137 + y * 881) as u32;
                let jitter =
                    vec2(hash(seed) as f32, hash(seed ^ 0x79ab) as f32) / 255.0 - Vec2::splat(0.5);
                let xy = (vec2(x as f32, y as f32) * Cloud::CELL - extent + jitter * 3.0) * scale;
                (
                    center + xy.extend(h * scale),
                    center + xy.extend(-h * 0.45 * scale),
                )
            };
            let fog = ((depth - far * 0.7) / (far * 0.3)).clamp(0.0, 1.0);
            let palette: [u32; 16] =
                std::array::from_fn(|i| mix(mix(dark, light, i as f32 / 15.0), haze, fog));
            for y in (0..height - 1).step_by(step) {
                for x in (0..cloud.width - 1).step_by(step) {
                    let x1 = (x + step).min(cloud.width - 1);
                    let y1 = (y + step).min(height - 1);
                    let cells = [(x, y), (x1, y), (x1, y1), (x, y1)];
                    if cells
                        .iter()
                        .all(|&(x, y)| cloud.heights[y * cloud.width + x] == 0)
                    {
                        continue;
                    }
                    let vertices = cells.map(|(x, y)| point(x, y));
                    for top in [true, false] {
                        let vertices = vertices.map(|p| if top { p.0 } else { p.1 });
                        for face in [[0, 1, 2], [0, 2, 3]] {
                            let points = if top {
                                face.map(|i| vertices[i])
                            } else {
                                [vertices[face[2]], vertices[face[1]], vertices[face[0]]]
                            };
                            let normal = (points[1] - points[0]).cross(points[2] - points[0]);
                            let toward = if camera.q > 0.00001 {
                                eye - points[0]
                            } else {
                                -eye_ray
                            };
                            if normal.dot(toward) <= 0.0 {
                                continue;
                            }
                            let shade = if top {
                                (normal.normalize().dot(vec3(-0.55, -0.3, 0.78)) * 0.5 + 0.5)
                                    .clamp(0.0, 1.0)
                            } else {
                                0.16
                            };
                            self.triangle(camera, points, palette[(shade * 15.0) as usize]);
                        }
                    }
                }
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
        let mut heights = vec![0; 9 * 9];
        for y in 2..7 {
            for x in 2..7 {
                heights[y * 9 + x] = 6;
            }
        }
        world.clouds = vec![Cloud {
            center: vec2(100.0, 100.0),
            width: 9,
            heights,
            rank: 0,
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
