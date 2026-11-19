use super::*;

impl Renderer {
    pub(super) fn draw_clouds(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
    ) {
        if world.kind.vacuum() || crate::content::maps::definition(world.kind).cloud_clusters == 0 {
            return;
        }
        let opacity = (1.0 - self.warp.map_or(0.0, |warp| warp.bend)).powi(4)
            * (1.0 - crate::world::orbit::thin_air(flight.position.z));
        if opacity < 0.001 {
            return;
        }
        let coverage = match flight.weather {
            Weather::Cloudy | Weather::Storm => 14,
            Weather::Rain | Weather::Snow => 11,
            Weather::Rainbow => 9,
            _ => 7,
        };
        let far = flight.weather.visibility(camera.position.z).1.min(1000.0);
        let low = world.cloud_levels.iter().copied().min().unwrap_or(110) as f32 - 8.0;
        let high = world.cloud_levels.iter().copied().max().unwrap_or(210) as f32 + 16.0;
        let near = if camera.q > 0.00001 {
            -0.88 / camera.q
        } else {
            -far
        }
        .max(self.near_depth);
        let colors = [
            rgb(226, 237, 245),
            rgb(204, 222, 237),
            rgb(244, 249, 250),
            rgb(175, 198, 220),
        ]
        .map(|c| flight.weather.shade(grade(c, theme)));
        let drift = vec2(flight.time * 7.0, flight.time * 3.0);
        const CELL: f32 = 4.0;
        for x in (0..self.w).step_by(2) {
            for y in (self.visible[x].0..self.visible[x].1).step_by(2) {
                let at = x * self.h + y as usize;
                if opacity < 1.0
                    && hash((x + self.x_origin) as u32 * 7919 + y as u32) as f32 / 255.0 > opacity
                {
                    continue;
                }
                let (origin, ray) = camera.ray(vec2(x as f32 + 0.5, y as f32 + 0.5));
                let initial = (origin - camera.position).dot(camera.forward);
                let mut entry = near - initial;
                let mut exit = far.min(self.depth[at]) - initial;
                if ray.z.abs() > 0.00001 {
                    let a = (low - origin.z) / ray.z;
                    let b = (high - origin.z) / ray.z;
                    entry = entry.max(a.min(b));
                    exit = exit.min(a.max(b));
                } else if origin.z < low || origin.z > high {
                    continue;
                }
                if entry >= exit {
                    continue;
                }
                let p = origin + ray * (entry + 0.0001);
                let mut cell = ((p.truncate() - drift) / CELL).floor().as_ivec2();
                let step = glam::ivec2(
                    if ray.x >= 0.0 { 1 } else { -1 },
                    if ray.y >= 0.0 { 1 } else { -1 },
                );
                let advance = vec2(CELL / ray.x.abs(), CELL / ray.y.abs());
                let boundary =
                    (cell.as_vec2() + vec2(f32::from(step.x > 0), f32::from(step.y > 0))) * CELL
                        + drift;
                let mut next = vec2(
                    if ray.x.abs() > 0.00001 {
                        (boundary.x - origin.x) / ray.x
                    } else {
                        f32::INFINITY
                    },
                    if ray.y.abs() > 0.00001 {
                        (boundary.y - origin.y) / ray.y
                    } else {
                        f32::INFINITY
                    },
                );
                let mut side = 0;
                while entry < exit {
                    let end = next.x.min(next.y).min(exit);
                    let center = (cell.as_vec2() + Vec2::splat(0.5)) * CELL + drift;
                    let thickness =
                        world.cloud_cover(center.x, center.y, flight.time, coverage) as f32;
                    if thickness > 0.0 {
                        let base = (world.cloud_height(center.x, center.y) / 2.0).floor() * 2.0;
                        let bottom = base - 2.0 - (thickness / 3.0).floor() * 2.0;
                        let top = base + thickness * 2.0;
                        let (mut hit, mut last, mut face) = (entry, end, side);
                        if ray.z.abs() > 0.00001 {
                            let a = (bottom - origin.z) / ray.z;
                            let b = (top - origin.z) / ray.z;
                            let first = a.min(b);
                            if first > hit {
                                hit = first;
                                face = if ray.z < 0.0 { 2 } else { 3 };
                            }
                            last = last.min(a.max(b));
                        } else if origin.z < bottom || origin.z > top {
                            hit = end;
                        }
                        if hit < last {
                            let depth = initial + hit;
                            let fog = ((depth - far * 0.7) / (far * 0.3)).clamp(0.0, 1.0);
                            let color = mix(colors[face], flight.weather.haze(theme), fog);
                            for xx in x..(x + 2).min(self.w) {
                                for yy in y..(y + 2).min(self.h as i32) {
                                    let at = xx * self.h + yy as usize;
                                    if yy >= self.visible[xx].0
                                        && yy < self.visible[xx].1
                                        && depth < self.depth[at]
                                    {
                                        self.pixels[at] = color;
                                        self.depth[at] = depth;
                                    }
                                }
                            }
                            break;
                        }
                    }
                    if next.x < next.y {
                        entry = next.x;
                        next.x += advance.x;
                        cell.x += step.x;
                        side = 0;
                    } else {
                        entry = next.y;
                        next.y += advance.y;
                        cell.y += step.y;
                        side = 1;
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
    fn level_view_hits_the_side_of_a_cloud_volume() {
        let mut world = World::generate(crate::world::MapKind::Islands, 7);
        world.clouds.fill(0);
        world.cloud_levels.fill(110);
        world.cloud_slopes.fill([110.0, 0.0, 0.0, 0.0]);
        for y in 80..128 {
            for x in 80..128 {
                world.clouds[y * crate::world::CLOUD_SIZE + x] = 6;
            }
        }
        let mut flight = Flight::new(&world);
        flight.position = vec3(100.0, 200.0, 116.0);
        flight.follow = flight.position;
        flight.yaw = 0.0;
        flight.weights = [0.0, 1.0, 0.0];
        let mut camera = Camera::new(&flight, 320, 170);
        camera.slope = 0.0;
        let mut renderer = Renderer::single();
        renderer.w = 320;
        renderer.h = 170;
        renderer.visible = vec![(0, 170); 320];
        renderer.pixels = vec![0; 320 * 170];
        renderer.depth = vec![f32::INFINITY; 320 * 170];
        renderer.draw_clouds(&world, &flight, &camera, 0);
        let at = camera.center.x as usize * 170 + camera.center.y as usize;
        assert_ne!(renderer.pixels[at], 0);
        assert!((0.0..200.0).contains(&renderer.depth[at]));
    }
}
