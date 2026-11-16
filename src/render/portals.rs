use super::*;

impl Renderer {
    pub(super) fn portals(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        mission: Option<&Mission>,
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
        for (i, entry) in world.portals.iter().enumerate() {
            let exit = world.portals[1 - i];
            let (eye, direction) = camera.ray(camera.center);
            let front = if camera.q > 0.00001 {
                (eye - entry.center).dot(entry.normal()) > 0.0
            } else {
                direction.dot(entry.normal()) < 0.0
            };
            if !front || entry.center.distance_squared(camera.position) > 2000.0_f32.powi(2) {
                continue;
            }
            let mut samples = Vec::new();
            remote.visible.fill((self.h as i32, 0));
            let normal = entry.normal();
            for x in 0..self.w {
                let (origin, mut ray) = camera.ray(vec2(x as f32, self.visible[x].0 as f32));
                for y in self.visible[x].0..self.visible[x].1 {
                    let (origin, direction) = if camera.q > 0.00001 {
                        (origin, ray)
                    } else {
                        camera.ray(vec2(x as f32, y as f32))
                    };
                    let divisor = direction.dot(normal);
                    if divisor.abs() > 0.00001 {
                        let t = (entry.center - origin).dot(normal) / divisor;
                        let hit = origin + direction * t;
                        let radius = (hit - entry.center).length() / entry.radius;
                        if (t > 0.0 || camera.q <= 0.00001) && radius < 1.19 {
                            let depth = camera.project(hit).map_or(f32::INFINITY, |p| p.1);
                            let at = x * self.h + y as usize;
                            if depth < self.depth[at] {
                                samples.push((x, y as usize, depth, radius));
                                remote.visible[x].0 = remote.visible[x].0.min(y);
                                remote.visible[x].1 = remote.visible[x].1.max(y + 1);
                            }
                        }
                    }
                    ray.z -= camera.q / camera.scale;
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
            view.yaw -= entry.rotation_to(exit);
            remote.clip_plane = Some((exit.center, exit.normal()));
            remote.cosmos(world, &view, flight.time);
            remote.scenery(world, &view, flight.time);
            if let Some(mission) = mission {
                remote.mission(mission, flight, &view);
            }
            let center = camera.project(entry.center).map_or(camera.center, |p| p.0);
            let base = self.pixels.clone();
            let rim = if i == 0 {
                rgb(72, 206, 255)
            } else {
                rgb(255, 159, 62)
            };
            for (x, y, depth, radius) in samples {
                let at = x * self.h + y;
                let offset = vec2(x as f32, y as f32) - center;
                let bend = (1.0 - ((radius - 1.0) / 0.19).abs()).max(0.0);
                let swirl = (flight.time * 2.0 + radius * 18.0).sin() * bend;
                let warped = vec2(x as f32, y as f32)
                    + offset.normalize_or_zero() * bend * 5.0
                    + vec2(-offset.y, offset.x).normalize_or_zero() * swirl * 2.0;
                let sx = warped.x.clamp(0.0, self.w as f32 - 1.0) as usize;
                let sy = warped.y.clamp(0.0, self.h as f32 - 1.0) as usize;
                let mut color = if radius < 0.94 {
                    remote.pixels[at]
                } else if radius < 1.0 {
                    mix(remote.pixels[at], rim, bend * 0.65)
                } else {
                    mix(base[sx * self.h + sy], rim, bend * 0.48)
                };
                if (radius - 1.0).abs() < 0.024 {
                    color = mix(rim, rgb(242, 250, 255), 0.65);
                }
                self.pixels[at] = color;
                if radius < 1.025 {
                    self.depth[at] = depth;
                }
            }
        }
        self.portal_view = Some(remote);
    }
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
            virtual_camera.yaw -= entry.rotation_to(exit);
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
