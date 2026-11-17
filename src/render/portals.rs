use super::*;

use crate::world::portal::LENS_RADIUS;

struct Aperture {
    entry: crate::world::portal::Portal,
    center: Vec2,
    depth: f32,
    radius: f32,
    expansion: f32,
    membrane: f32,
    entering: bool,
}

impl Renderer {
    fn aperture(
        &self,
        portal: crate::world::portal::Portal,
        flight: &Flight,
        camera: &Camera,
    ) -> Option<Aperture> {
        let transit = flight.portal_transit.filter(|t| t.matches(portal));
        let entry = transit.map_or_else(|| camera.portal_frame(portal), |t| t.entry);
        let (center, depth, scale) = camera.project(entry.center)?;
        let expansion = transit.map_or(0.0, |t| t.expansion());
        let natural_radius = (entry.radius * scale).max(1.0);
        let corners = [
            Vec2::ZERO,
            vec2(self.w as f32, 0.0),
            vec2(0.0, self.h as f32),
            vec2(self.w as f32, self.h as f32),
        ];
        let cover = corners
            .into_iter()
            .map(|p| p.distance(center))
            .fold(0.0_f32, f32::max)
            / 0.8
            + 4.0;
        let radius = natural_radius + (cover.max(natural_radius) - natural_radius) * expansion;
        let membrane = if transit.is_some_and(|t| t.overhead) {
            1.0 - expansion
        } else if transit.is_some() {
            ((depth + 0.86 / camera.q.max(0.00001)) / (entry.radius * 0.6)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        Some(Aperture {
            entry,
            center,
            depth,
            radius,
            expansion,
            membrane,
            entering: transit.is_some(),
        })
    }

    pub(super) fn portal_mask(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        network: Option<&crate::world::network::Network>,
    ) {
        for (entry, _, _, _) in portal_scenes(world, flight, network) {
            let Some(window) = self.aperture(entry, flight, camera) else {
                continue;
            };
            let Aperture { center, depth, .. } = window;
            if entry.center.distance_squared(camera.position) > 2400.0_f32.powi(2) {
                continue;
            }
            let radius = window.radius * 0.81;
            let x0 = ((center.x - radius).ceil() as i32).max(0);
            let x1 = ((center.x + radius).floor() as i32).min(self.w as i32 - 1);
            for x in x0..=x1 {
                let half = (radius * radius - (x as f32 - center.x).powi(2))
                    .max(0.0)
                    .sqrt();
                let y0 = ((center.y - half).ceil() as i32).max(self.visible[x as usize].0);
                let y1 = ((center.y + half).floor() as i32 + 1).min(self.visible[x as usize].1);
                for y in y0..y1 {
                    let at = x as usize * self.h + y as usize;
                    self.depth[at] = if window.expansion > 0.0 {
                        f32::NEG_INFINITY
                    } else {
                        self.depth[at].min(depth)
                    };
                }
            }
        }
    }
    pub(super) fn portals(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        mission: Option<&Mission>,
        theme: usize,
        network: Option<&crate::world::network::Network>,
    ) {
        if let Some(network) = network.filter(|n| n.active) {
            for route in network.routes(world) {
                if network.get(route.target).is_some() || flight.hides_portal(route.aperture) {
                    continue;
                }
                let Some((p, d, scale)) = camera.project(route.aperture.center) else {
                    continue;
                };
                let r = route.aperture.radius * scale;
                for i in 0..40 {
                    let a = i as f32 * std::f32::consts::TAU / 40.0;
                    let b = (i + 1) as f32 * std::f32::consts::TAU / 40.0;
                    if ((i as f32 + flight.time * 12.0) as usize % 10) < 7 {
                        self.stroke(
                            p + vec2(a.cos(), a.sin()) * r,
                            p + vec2(b.cos(), b.sin()) * r,
                            d,
                            rgb(109, 161, 194),
                        );
                    }
                }
            }
        }
        let mut scenes = portal_scenes(world, flight, network);
        if scenes.is_empty() {
            return;
        }
        scenes.sort_by(|a, b| {
            let depth = |p: &crate::world::portal::Portal| {
                camera.project(p.center).map_or(f32::INFINITY, |p| p.1)
            };
            depth(&b.0).total_cmp(&depth(&a.0))
        });
        let mut remote = self
            .portal_view
            .take()
            .unwrap_or_else(|| Box::new(Self::single()));
        remote.w = self.w;
        remote.full_width = self.w;
        remote.h = self.h;
        remote.pixels.resize(self.w * self.h, 0);
        remote.depth.resize(self.w * self.h, f32::INFINITY);
        remote.mask.resize(self.w * self.h, 0);
        remote.visible.resize(self.w, (0, 0));
        for (original, exit, destination, i) in scenes {
            let Some(window) = self.aperture(original, flight, camera) else {
                continue;
            };
            let Aperture {
                entry,
                center,
                depth,
                radius,
                expansion,
                membrane,
                entering,
            } = window;
            if entry.center.distance_squared(camera.position) > 2400.0_f32.powi(2) {
                continue;
            }
            let extent = radius * LENS_RADIUS;
            let min_x = ((center.x - extent).floor() as i32).max(0);
            let max_x = ((center.x + extent).ceil() as i32).min(self.w as i32 - 1);
            let mut samples = Vec::new();
            remote.visible.fill((self.h as i32, 0));
            for x in min_x..=max_x {
                let nx = (x as f32 - center.x) / radius;
                let half = (LENS_RADIUS * LENS_RADIUS - nx * nx).max(0.0).sqrt() * radius;
                let y0 = ((center.y - half).ceil() as i32).max(self.visible[x as usize].0);
                let y1 = ((center.y + half).floor() as i32 + 1).min(self.visible[x as usize].1);
                for y in y0..y1 {
                    let at = x as usize * self.h + y as usize;
                    if self.occluded.get(at).copied().unwrap_or(false)
                        || (expansion == 0.0 && depth > self.depth[at] + 0.01)
                    {
                        continue;
                    }
                    let ny = (y as f32 - center.y) / radius;
                    let radius2 = nx * nx + ny * ny;
                    samples.push((x as usize, y as usize, depth, radius2));
                    if radius2 < 1.08_f32.powi(2) {
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
            remote.near_depth = if expansion > 0.0 {
                f32::NEG_INFINITY
            } else {
                depth
            };
            if expansion > 0.0 {
                remote.clip_depth.clear();
            }
            remote.column.resize(self.w, self.h as i32);
            for (column, visible) in remote.column.iter_mut().zip(&remote.visible) {
                *column = visible.1;
            }
            if samples.len() > 9000
                && (cfg!(target_os = "linux") || self.w > 640)
                && remote.workers.is_empty()
            {
                remote.workers = (0..3).map(|_| Self::single()).collect();
            }
            let mut remote_flight = flight.clone();
            let remote_theme = if let Some(network) = network.filter(|n| n.active) {
                let environment =
                    network.environment(crate::world::network::MapKey::of(destination));
                remote_flight.weather = environment.weather;
                environment.theme
            } else {
                theme
            };
            remote.background(destination, &remote_flight, &view, remote_theme, 170);
            if !destination.kind.vacuum() {
                remote.precipitation(&remote_flight, &view, remote_theme);
            }
            if let Some(mission) = mission.filter(|_| std::ptr::eq(world, destination)) {
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
            let wave = |phase: f32| {
                waves[((phase * (256.0 / std::f32::consts::TAU)) as i32 & 255) as usize]
            };
            let pixel_scale = camera.scale / Camera::new(flight, 320, 170).scale;
            for (x, y, depth, radius2) in samples {
                let at = x * self.h + y;
                let point = vec2(x as f32, y as f32);
                let uv = (point - center) / radius;
                let r = radius2.sqrt();
                let direction = uv / r.max(0.001);
                let tangent = vec2(-direction.y, direction.x);
                let influence = ((LENS_RADIUS - r) / (LENS_RADIUS - 1.0))
                    .clamp(0.0, 1.0)
                    .powi(2);
                let pull = influence
                    * (radius * 0.35).clamp(5.0 * pixel_scale, 22.0 * pixel_scale)
                    * membrane;
                let swirl =
                    (direction.x * c + direction.y * s) * influence * 4.0 * pixel_scale * membrane;
                let mut color = if r < 1.0 {
                    let ripple = wave(r * 28.0 - phase * 3.0);
                    let cross = wave(uv.x * 13.0 + uv.y * 9.0 + phase * 2.0);
                    let amplitude = (radius * 0.035).min(5.0 * pixel_scale) * membrane;
                    let offset = direction * (ripple * amplitude - pull * 0.15)
                        + tangent * (cross * amplitude + swirl * 0.5);
                    let sample = point + offset;
                    let px = sample.x.clamp(0.0, self.w as f32 - 1.0) as usize;
                    let py = sample.y.clamp(0.0, self.h as f32 - 1.0) as usize;
                    let scene = remote.pixels[px * self.h + py];
                    let veil = (0.14 + ripple * 0.04 + cross * 0.03) * membrane;
                    let shaded = mix(scene, rgb(15, 24, 52), 0.13 * membrane);
                    let tinted = mix(shaded, rim, veil);
                    mix(
                        tinted,
                        rgb(3, 5, 13),
                        ((r - 0.82) / 0.18).clamp(0.0, 1.0) * 0.65 * membrane,
                    )
                } else {
                    let warped = point + direction * pull + tangent * swirl;
                    let sx = warped.x.clamp(0.0, self.w as f32 - 1.0) as usize;
                    let sy = warped.y.clamp(0.0, self.h as f32 - 1.0) as usize;
                    let sample = base[sx * self.h + sy];
                    if sample == 0 { base[at] } else { sample }
                };
                let photon = (1.0 - ((r - 1.035) / 0.045).abs()).max(0.0);
                let halo = (1.0 - ((r - 1.13) / 0.2).abs()).max(0.0);
                let filament =
                    wave(direction.x * 17.0 + direction.y * 11.0 - phase * 3.0) * 0.18 + 0.42;
                color = mix(color, rim, (halo * filament * membrane).clamp(0.0, 0.7));
                color = mix(color, photon_color, photon * 0.95 * membrane);
                self.pixels[at] = color;
                if r < 1.0 && entering {
                    self.depth[at] = remote.depth[at];
                } else if r < 1.06 {
                    self.depth[at] = depth;
                }
            }
        }
        self.portal_view = Some(remote);
    }
}

fn portal_scenes<'a>(
    world: &'a World,
    flight: &Flight,
    network: Option<&'a crate::world::network::Network>,
) -> Vec<(
    crate::world::portal::Portal,
    crate::world::portal::Portal,
    &'a World,
    usize,
)> {
    let mut scenes = vec![];
    if world.portals.len() == 2 && network.is_none_or(|n| !n.active) {
        for i in 0..2 {
            scenes.push((world.portals[i], world.portals[1 - i], world, i));
        }
    }
    if let Some(network) = network.filter(|n| n.active) {
        for route in network.routes(world) {
            if let Some(target) = network.get(route.target) {
                scenes.push((
                    route.aperture,
                    network.exit(target, route.id),
                    target,
                    route.target.kind as usize,
                ));
            }
        }
    }
    scenes.retain(|(entry, _, _, _)| !flight.hides_portal(*entry));
    scenes
}

fn portal_waves() -> &'static [f32; 256] {
    static WAVES: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    WAVES.get_or_init(|| std::array::from_fn(|i| (i as f32 * std::f32::consts::TAU / 256.0).sin()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overhead_expansion_reveals_the_destination_without_a_camera_jump() {
        use crate::world::portal::Portal;
        let mut world = World::generate(crate::world::MapKind::Space, 1999);
        world.scenery.clear();
        let entry = Portal {
            center: vec3(0.0, 0.0, 300.0),
            yaw: std::f32::consts::PI,
            radius: 50.0,
        };
        let exit = Portal {
            center: vec3(1500.0, 1500.0, 400.0),
            yaw: 0.7,
            ..entry
        };
        world.portals = vec![entry, exit];
        let mut flight = Flight::new(&world);
        flight.position = entry.center + Vec3::Y * 10.0;
        flight.follow = flight.position;
        flight.yaw = 0.0;
        flight.camera_yaw = 0.0;
        flight.weights = [1.0, 0.0, 0.0];
        flight.wing_count = 0;
        flight.approach_portal(entry, flight.position + Vec3::Y * 5.0);
        let mut renderer = Renderer::single();
        renderer.render(&world, &flight, 320, 170, 0);
        let camera = Camera::new(&flight, 320, 170);
        let small = renderer.aperture(entry, &flight, &camera).unwrap();
        flight.portal_transit.as_mut().unwrap().elapsed = 0.375;
        let middle = renderer.aperture(entry, &flight, &camera).unwrap();
        assert!(middle.radius > small.radius * 2.0);
        flight.portal_transit.as_mut().unwrap().elapsed = 0.749;
        let final_window = renderer.aperture(entry, &flight, &camera).unwrap();
        for corner in [
            Vec2::ZERO,
            vec2(320.0, 0.0),
            vec2(0.0, 170.0),
            vec2(320.0, 170.0),
        ] {
            assert!(corner.distance(final_window.center) < final_window.radius * 0.82);
        }
        renderer.render(&world, &flight, 320, 170, 0);
        let before = renderer.rgba.clone();
        flight.warp(final_window.entry, exit);
        flight.warp_flash = 0.0;
        renderer.render(&world, &flight, 320, 170, 0);
        let differences = before
            .chunks_exact(4)
            .zip(renderer.rgba.chunks_exact(4))
            .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 5))
            .count();
        assert!(
            differences < 320 * 170 / 100,
            "{differences} pixels differ at the transition"
        );
    }

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
