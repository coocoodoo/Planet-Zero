use super::*;

impl Renderer {
    pub(super) fn portals(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        mission: Option<&Mission>,
        theme: usize,
    ) {
        if world.portals.len() != 2 {
            return;
        }
        let mut remote = self
            .portal_view
            .take()
            .unwrap_or_else(|| Box::new(Self::single()));
        remote.w = self.w;
        remote.h = self.h;
        remote.pixels.resize(self.w * self.h, 0);
        remote.depth.resize(self.w * self.h, f32::INFINITY);
        remote.mask.resize(self.w * self.h, 0);
        remote.visible.resize(self.w, (0, 0));
        for (i, original) in world.portals.iter().enumerate() {
            let exit = world.portals[1 - i];
            let (eye, _) = camera.ray(camera.center);
            let entry = original.facing(eye);
            let Some((center, depth, scale)) = camera.project(entry.center) else {
                continue;
            };
            if entry.center.distance_squared(camera.position) > 2400.0_f32.powi(2) {
                continue;
            }
            let radius = (entry.radius * scale).max(1.0);
            let extent = radius * 1.6;
            let min_x = ((center.x - extent).floor() as i32).max(0);
            let max_x = ((center.x + extent).ceil() as i32).min(self.w as i32 - 1);
            let mut samples = Vec::new();
            remote.visible.fill((self.h as i32, 0));
            for x in min_x..=max_x {
                let nx = (x as f32 - center.x) / radius;
                let half = (2.56 - nx * nx).max(0.0).sqrt() * radius;
                let y0 = ((center.y - half).ceil() as i32).max(self.visible[x as usize].0);
                let y1 = ((center.y + half).floor() as i32 + 1).min(self.visible[x as usize].1);
                for y in y0..y1 {
                    let at = x as usize * self.h + y as usize;
                    if depth >= self.depth[at] {
                        continue;
                    }
                    let ny = (y as f32 - center.y) / radius;
                    let radius2 = nx * nx + ny * ny;
                    samples.push((x as usize, y as usize, depth, radius2));
                    if radius2 < 1.04_f32.powi(2) {
                        remote.visible[x as usize].0 = remote.visible[x as usize].0.min(y);
                        remote.visible[x as usize].1 = remote.visible[x as usize].1.max(y + 1);
                    }
                }
            }
            if samples.is_empty() {
                continue;
            }
            remote.clip_depth.resize(self.w * self.h, f32::NEG_INFINITY);
            remote.clip_depth.fill(f32::NEG_INFINITY);
            for &(x, y, depth, _) in &samples {
                remote.clip_depth[x * self.h + y] = depth;
            }
            remote.pixels.fill(0);
            remote.depth.fill(f32::INFINITY);
            remote.mask.fill(0);
            let mut view = *camera;
            view.position = entry.transform(exit, camera.position);
            view.orient(view.yaw - entry.rotation_to(exit), view.pitch);
            remote.clip_plane = None;
            remote.near_depth = depth;
            remote.column.resize(self.w, self.h as i32);
            for (column, visible) in remote.column.iter_mut().zip(&remote.visible) {
                *column = visible.1;
            }
            remote.background(world, flight, &view, theme, 170);
            remote.scenery(world, &view, flight.time);
            if let Some(mission) = mission {
                remote.mission(mission, flight, &view);
            }
            let base = self.pixels.clone();
            let rim = if i == 0 {
                rgb(90, 201, 255)
            } else {
                rgb(255, 155, 70)
            };
            let phase = flight.time * 1.5;
            let (s, c) = phase.sin_cos();
            let photon_color = mix(rim, rgb(255, 245, 216), 0.72);
            let waves = portal_waves();
            for (x, y, depth, radius2) in samples {
                let at = x * self.h + y;
                if radius2 < 0.82_f32.powi(2) {
                    self.pixels[at] = remote.pixels[at];
                    self.depth[at] = depth;
                    continue;
                }
                let r = radius2.sqrt();
                let direction = (vec2(x as f32, y as f32) - center) / (radius * r);
                let tangent = vec2(-direction.y, direction.x);
                let influence = ((1.6 - r) / 0.6).clamp(0.0, 1.0).powi(2);
                let pull = influence * (radius * 0.35).clamp(5.0, 22.0);
                let swirl = (direction.x * c + direction.y * s) * influence * 4.0;
                let warped = vec2(x as f32, y as f32) + direction * pull + tangent * swirl;
                let sx = warped.x.clamp(0.0, self.w as f32 - 1.0) as usize;
                let sy = warped.y.clamp(0.0, self.h as f32 - 1.0) as usize;
                let mut color = if r < 1.0 {
                    let sample =
                        vec2(x as f32, y as f32) - direction * pull * 0.25 + tangent * swirl;
                    let px = sample.x.clamp(0.0, self.w as f32 - 1.0) as usize;
                    let py = sample.y.clamp(0.0, self.h as f32 - 1.0) as usize;
                    mix(
                        remote.pixels[px * self.h + py],
                        rgb(3, 5, 13),
                        ((r - 0.82) / 0.18).clamp(0.0, 1.0) * 0.65,
                    )
                } else {
                    base[sx * self.h + sy]
                };
                let photon = (1.0 - ((r - 1.035) / 0.045).abs()).max(0.0);
                let halo = (1.0 - ((r - 1.13) / 0.16).abs()).max(0.0);
                let wave = ((direction.x * 17.0 + direction.y * 11.0 - phase * 3.0)
                    * (256.0 / std::f32::consts::TAU)) as i32;
                let filament = waves[(wave & 255) as usize];
                color = mix(color, rim, (halo * filament).clamp(0.0, 0.7));
                color = mix(color, photon_color, photon * 0.95);
                self.pixels[at] = color;
                if r < 1.06 {
                    self.depth[at] = depth;
                }
            }
        }
        self.portal_view = Some(remote);
    }
}

fn portal_waves() -> &'static [f32; 256] {
    static WAVES: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    WAVES.get_or_init(|| {
        std::array::from_fn(|i| (i as f32 * std::f32::consts::TAU / 256.0).sin() * 0.18 + 0.42)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn virtual_camera_keeps_portal_points_at_identical_pixels() {
        let world = World::generate(crate::world::MapKind::Space, 1999);
        let entry = world.portals[0];
        let exit = world.portals[1];
        let mut f = Flight::new(&world);
        f.position = entry.center + entry.normal() * 130.0;
        f.follow = f.position;
        f.yaw = entry.yaw + std::f32::consts::PI;
        f.camera_yaw = -f.yaw;
        for weights in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
            f.weights = weights;
            let camera = Camera::new(&f, 320, 170);
            let mut virtual_camera = camera;
            virtual_camera.position = entry.transform(exit, camera.position);
            virtual_camera.orient(
                virtual_camera.yaw - entry.rotation_to(exit),
                virtual_camera.pitch,
            );
            for point in [
                entry.center,
                entry.center + entry.right() * 20.0 + Vec3::Z * 15.0,
            ] {
                let a = camera.project(point).unwrap();
                let b = virtual_camera
                    .project(entry.transform(exit, point))
                    .unwrap();
                assert!(a.0.distance(b.0) < 0.002);
                assert!((a.1 - b.1).abs() < 0.002);
                let (origin, ray) = camera.ray(a.0);
                let t = (point - origin).dot(entry.normal()) / ray.dot(entry.normal());
                assert!((origin + ray * t).distance(point) < 0.002);
            }
        }
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;
    use crate::world::celestial::{Scenery, Structure};
    #[test]
    fn window_renders_changes_at_the_remote_exit() {
        let mut world = World::generate(crate::world::MapKind::Space, 1999);
        let entry = world.portals[0];
        let exit = world.portals[1];
        world.scenery = vec![Scenery {
            position: exit.center + exit.normal() * 160.0,
            yaw: 0.0,
            scale: 1.5,
            kind: Structure::Satellite,
        }];
        let mut flight = Flight::new(&world);
        flight.position = entry.center + entry.normal() * 140.0;
        flight.follow = flight.position;
        flight.yaw = entry.yaw + std::f32::consts::PI;
        flight.camera_yaw = -flight.yaw;
        flight.weights = [0.0, 1.0, 0.0];
        flight.wing_count = 0;
        let mut renderer = Renderer::single();
        renderer.render(&world, &flight, 320, 170, 0);
        let before = renderer.rgba.clone();
        world.scenery[0].yaw = 1.5;
        renderer.render(&world, &flight, 320, 170, 0);
        let changed: Vec<_> = before
            .chunks_exact(4)
            .zip(renderer.rgba.chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| (i % 320, i / 320))
            .collect();
        assert!(changed.len() > 20);
        assert!(
            changed
                .iter()
                .all(|(x, y)| (100..220).contains(x) && (30..155).contains(y))
        );
    }
}
