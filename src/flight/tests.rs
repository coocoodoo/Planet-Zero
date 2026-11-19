use super::*;
use crate::world::SIZE;

fn terrain() -> World {
    World {
        height_bounds: Default::default(),
        obstacles: vec![],
        ceiling: 40,
        cloud_levels: vec![],
        cloud_slopes: vec![],
        airspace: Default::default(),
        heights: vec![0; SIZE * SIZE].into(),
        materials: vec![0; SIZE * SIZE].into(),
        clouds: vec![],
        spawn: (256.0, 350.0),
        kind: crate::world::MapKind::Islands,
        seed: 7,
        scenery: vec![],
        caverns: vec![],
        portals: vec![],
    }
}

#[test]
fn warp_preserves_speed_camera_formation_and_cooldown() {
    let world = World::generate(crate::world::MapKind::Space, 1999);
    let [entry, exit] = [world.portals[0], world.portals[1]];
    let mut f = Flight::new(&world);
    f.position = entry.center - entry.normal() * 3.0;
    f.follow = f.position + entry.normal() * 12.0;
    f.speed = 170.0;
    let source = Camera::new(&f, 320, 170).portal_frame(entry);
    f.wingmen[0].position = entry.center;
    f.wingmen[0].follow = entry.center;
    let old_follow = f.follow;
    let old_wing = f.wingmen[0].position;
    let yaw = f.yaw;
    f.warp(source, exit);
    let offset = f.position - source.transform(exit, entry.center - entry.normal() * 3.0);
    assert!(offset.length() < 0.001);
    assert!(
        f.follow
            .distance(source.transform(exit, old_follow) + offset)
            < 0.001
    );
    assert!(
        f.wingmen[0]
            .position
            .distance(source.transform(exit, old_wing) + offset)
            < 0.001
    );
    assert_eq!(f.speed, 170.0);
    assert!((f.yaw - yaw - source.rotation_to(exit)).abs() < 0.001);
    assert!(!f.traverse_portal(&world, exit.center + exit.normal() * 4.0));
    assert!(f.trails.is_empty());
}
#[test]
fn space_flight_can_descend_below_the_surface_reference() {
    let world = World::generate(crate::world::MapKind::Space, 1999);
    let mut f = Flight::new(&world);
    for frame in 0..150 {
        f.step(
            0.033,
            &Controls {
                manual: true,
                climb: if frame < 30 { -1.0 } else { 0.0 },
                ..Default::default()
            },
            &world,
        );
    }
    assert!(f.position.z < 0.0);
    assert!(Camera::new(&f, 320, 170).position.is_finite());
}

#[test]
fn descent_keeps_the_formation_above_terrain() {
    let mut world = terrain();
    for i in 0..220 * SIZE {
        world.heights[i] = 40;
    }
    let mut flight = Flight::new(&world);
    flight.position.z = 60.0;
    flight.yaw = 0.0;
    let controls = Controls {
        climb: -1.0,
        boost: true,
        manual: true,
        ..Default::default()
    };
    for _ in 0..240 {
        let before = flight.position.z;
        flight.step(1.0 / 60.0, &controls, &world);
        assert!(
            flight.position.z - world.height(flight.position.x, flight.position.y) >= MIN_CLEARANCE
        );
        assert!((flight.position.z - before).abs() < 3.0);
        for wing in &flight.wingmen {
            assert!(
                wing.position.z - world.height(wing.position.x, wing.position.y) >= MIN_CLEARANCE
            );
        }
    }
}

#[test]
fn demonstration_completes_a_safe_cycle_and_releases_control() {
    let world = terrain();
    let mut flight = Flight::new(&world);
    flight.set_demo(true);
    let mut seen = std::collections::HashSet::new();
    let mut rolled = false;
    let mut min_altitude = f32::INFINITY;
    let mut max_altitude = 0.0_f32;
    for _ in 0..3600 {
        flight.step(1.0 / 60.0, &Controls::default(), &world);
        seen.insert(flight.demonstration());
        rolled |= flight.spin > 3.0;
        min_altitude = min_altitude.min(flight.position.z);
        max_altitude = max_altitude.max(flight.position.z);
        assert!(flight.position.is_finite() && flight.position.z >= MIN_CLEARANCE);
    }
    assert_eq!(seen.len(), 7);
    assert!(rolled && max_altitude - min_altitude > 100.0);
    flight.set_demo(false);
    let controls = Controls {
        turn: -1.0,
        manual: true,
        ..Default::default()
    };
    for _ in 0..180 {
        flight.step(1.0 / 60.0, &controls, &world);
    }
    assert!(flight.rate < -1.0 && !flight.demo);
}

#[test]
fn automatic_descent_levels_out_at_ground_protection() {
    let world = terrain();
    let mut flight = Flight::new(&world);
    let controls = Controls {
        climb: -1.0,
        manual: true,
        ..Default::default()
    };
    for _ in 0..600 {
        flight.step(1.0 / 60.0, &controls, &world);
    }
    assert!(flight.position.z >= MIN_CLEARANCE);
    assert!(flight.pitch.abs() < 0.01);
}

#[test]
fn requested_loops_complete_in_both_directions() {
    let world = terrain();
    for direction in [-1.0, 1.0] {
        let mut f = Flight::new(&world);
        f.position.z = f.loop_altitude() + 240.0;
        f.follow = f.position;
        assert!(f.loop_toward(direction));
        let mut rotation = 0.0;
        for _ in 0..660 {
            let before = f.pitch;
            f.step(
                1.0 / 60.0,
                &Controls {
                    climb: direction,
                    manual: true,
                    aerobatic: true,
                    ..Default::default()
                },
                &world,
            );
            rotation += angle(f.pitch - before);
            assert!(f.position.is_finite() && f.position.z > MIN_CLEARANCE);
            for view in [1, 2] {
                f.weights = [0.0; 3];
                f.weights[view] = 1.0;
                let c = Camera::new(&f, 320, 170);
                let (eye, ray) = c.ray(c.center);
                let point = eye + ray * 200.0;
                assert!(c.project(point).unwrap().0.distance(c.center) < 0.002);
            }
        }
        assert!(rotation * direction > TAU);
    }
}

#[test]
fn flat_ground_clearance_stays_low_and_high_terrain_triggers_early_climb() {
    let mut world = terrain();
    let mut f = Flight::new(&world);
    f.yaw = 0.0;
    for _ in 0..900 {
        f.step(
            1.0 / 60.0,
            &Controls {
                climb: -0.2,
                manual: true,
                aerobatic: true,
                ..Default::default()
            },
            &world,
        );
    }
    assert!(
        f.position.z >= MIN_CLEARANCE && f.position.z < MIN_CLEARANCE + 1.0,
        "altitude {}",
        f.position.z
    );
    f.position = vec3(256.0, 450.0, 30.0);
    f.pitch = 0.0;
    for i in 0..260 * SIZE {
        world.heights[i] = 130;
    }
    let mut rose_before_ridge = false;
    for _ in 0..180 {
        f.step(
            1.0 / 60.0,
            &Controls {
                manual: true,
                ..Default::default()
            },
            &world,
        );
        if f.position.y > 300.0 && f.position.z > 45.0 {
            rose_before_ridge = true;
        }
        assert!(f.position.z >= world.height(f.position.x, f.position.y) + MIN_CLEARANCE);
    }
    assert!(rose_before_ridge);
    world.heights.fill(0);
    for i in 0..260 * SIZE {
        world.heights[i] = 5;
    }
    let mut f = Flight::new(&world);
    f.position = vec3(256.0, 450.0, MIN_CLEARANCE);
    f.yaw = 0.0;
    let mut previous_speed = 0.0_f32;
    for _ in 0..240 {
        let before = f.position.z;
        f.step(
            1.0 / 60.0,
            &Controls {
                climb: -1.0,
                manual: true,
                aerobatic: true,
                ..Default::default()
            },
            &world,
        );
        let speed = (f.position.z - before) * 60.0;
        assert!((speed - previous_speed).abs() < 2.0);
        previous_speed = speed;
        assert!(f.position.z >= world.height(f.position.x, f.position.y) + MIN_CLEARANCE);
    }
}

#[test]
fn low_altitude_controls_limit_pitch_without_forced_lift() {
    let world = terrain();
    for direction in [-1.0, 1.0] {
        let mut f = Flight::new(&world);
        f.position.z = MIN_CLEARANCE + 2.0;
        assert!(!f.roll());
        let mut previous_speed = 0.0_f32;
        for _ in 0..120 {
            let before = f.position.z;
            f.step(
                1.0 / 60.0,
                &Controls {
                    climb: direction,
                    manual: true,
                    aerobatic: true,
                    ..Default::default()
                },
                &world,
            );
            let speed = (f.position.z - before) * 60.0;
            assert!(
                (speed - previous_speed).abs() < 2.0,
                "vertical acceleration: {}",
                speed - previous_speed
            );
            previous_speed = speed;
            assert!(f.position.z >= MIN_CLEARANCE);
            assert!(f.pitch.abs() <= NORMAL_PITCH + 0.02);
        }
    }
    let mut f = Flight::new(&world);
    f.position.z = f.loop_altitude() + 240.0;
    assert!(f.roll());
    assert!(f.loop_toward(-1.0));
    let mut rotation = 0.0;
    for frame in 0..660 {
        let before = f.pitch;
        let altitude = f.position.z;
        f.step(
            1.0 / 60.0,
            &Controls {
                climb: if frame < 190 { -1.0 } else { 0.0 },
                manual: true,
                aerobatic: true,
                boost: true,
                ..Default::default()
            },
            &world,
        );
        rotation += angle(f.pitch - before);
        assert!((f.position.z - altitude).abs() <= f.speed / 60.0 + 0.01);
        assert!(f.position.z >= MIN_CLEARANCE);
    }
    assert!(rotation < -TAU + 0.03);
    assert!(f.pitch.abs() < 0.03);
}

#[test]
fn portal_capture_matches_the_visible_aperture_in_all_views() {
    use crate::world::portal::Portal;
    let world = terrain();
    for view in 0..3 {
        let mut f = Flight::new(&world);
        f.position = vec3(0.0, 120.0, 200.0);
        f.follow = f.position;
        f.yaw = 0.0;
        f.camera_yaw = 0.0;
        f.weights = [0.0; 3];
        f.weights[view] = 1.0;
        let camera = Camera::new(&f, 320, 170);
        let portal = Portal {
            center: vec3(0.0, 0.0, 200.0),
            yaw: 2.0,
            radius: 50.0,
        };
        let (_, ray) = camera.ray(camera.project(portal.center).unwrap().0);
        let point = portal.center - ray.normalize() * 25.0;
        assert!(camera.enters_portal(portal, point - ray.normalize() * 5.0, point));
        let outside = point + camera.right * 70.0;
        assert!(!camera.enters_portal(portal, outside - ray.normalize() * 5.0, outside));
        let mut near = f.clone();
        near.position = portal.center + Vec3::Y * 12.0;
        near.follow = near.position;
        assert!(Camera::new(&near, 320, 170).enters_portal(
            portal,
            near.position + Vec3::Y * 5.0,
            near.position
        ));
        let entry = camera.portal_frame(portal);
        let exit = Portal {
            center: vec3(1500.0, 900.0, 330.0),
            yaw: -1.1,
            ..portal
        };
        let before = Camera::new(&f, 320, 170);
        let marker = portal.center + vec3(20.0, -70.0, 35.0);
        let projected = before.project(marker).unwrap().0;
        let original = f.position;
        f.warp(entry, exit);
        let offset = f.position - entry.transform(exit, original);
        let after = Camera::new(&f, 320, 170);
        let actual = after
            .project(entry.transform(exit, marker) + offset)
            .unwrap()
            .0;
        assert!(
            actual.distance(projected) < 0.001,
            "view {view}: {actual:?} {projected:?}"
        );
    }
}

#[test]
fn wingmen_outside_the_aperture_rejoin_from_the_exit() {
    use crate::world::portal::Portal;
    let world = World::generate(crate::world::MapKind::Space, 1999);
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, 0.0, 200.0);
    f.follow = f.position;
    f.yaw = 0.0;
    f.view = 1;
    f.weights = [0.0, 1.0, 0.0];
    let entry = Portal {
        center: f.position,
        yaw: std::f32::consts::PI,
        radius: 50.0,
    };
    let exit = Portal {
        center: vec3(1400.0, 1100.0, 200.0),
        yaw: 0.0,
        ..entry
    };
    f.wingmen[0].position = entry.center + Vec3::X * 120.0;
    f.warp(entry, exit);
    assert!(f.wingmen[0].portal_wait > 0.0);
    assert!(f.wingmen[0].position.distance(exit.center) < exit.radius);
    let mut appeared = false;
    for _ in 0..300 {
        let before = f.wingmen[0].position;
        f.step(
            1.0 / 60.0,
            &Controls {
                manual: true,
                ..Default::default()
            },
            &world,
        );
        let wing = &f.wingmen[0];
        assert!(wing.position.distance(before) <= f.speed * 1.7 / 60.0 + 0.01);
        if !appeared && wing.portal_wait <= 0.0 {
            assert!((wing.position - f.position).dot(f.axes()[1]) < 0.0);
            appeared = true;
        }
    }
    assert!(appeared && !f.wingmen[0].rejoining);
}

#[test]
fn held_pitch_never_starts_a_loop() {
    let world = terrain();
    for orbit_flight in [false, true] {
        for direction in [-1.0, 1.0] {
            let mut f = Flight::new(&world);
            f.position.z = 12000.0;
            f.orbit_flight = orbit_flight;
            for _ in 0..900 {
                f.step(
                    1.0 / 60.0,
                    &Controls {
                        manual: true,
                        aerobatic: true,
                        climb: direction,
                        ..Default::default()
                    },
                    &world,
                );
                assert_eq!(f.loop_remaining, 0.0);
                let limit = if orbit_flight {
                    std::f32::consts::FRAC_PI_2
                } else {
                    NORMAL_PITCH
                };
                assert!(f.pitch.abs() <= limit + 0.02);
            }
            if orbit_flight {
                assert!(
                    f.pitch.abs() > 89.0_f32.to_radians(),
                    "direction {direction}, pitch {}, altitude {}",
                    f.pitch,
                    f.position.z
                );
            }
            f.position.z = MIN_CLEARANCE + 20.0;
            assert!(!f.loop_toward(1.0));
        }
    }
}

#[test]
fn pitch_range_expands_linearly_and_high_altitude_holds_on_release() {
    for kind in [crate::world::MapKind::Moon, crate::world::MapKind::Islands] {
        let world = World::generate(kind, 7);
        let high = crate::world::orbit::Body::of(kind).surface_altitude();
        for fraction in [0.0, 0.5, 1.0] {
            let altitude = high * (0.5 + 0.5 * fraction);
            let limit = NORMAL_PITCH + fraction * (std::f32::consts::FRAC_PI_2 - NORMAL_PITCH);
            for direction in [-1.0, 1.0] {
                let mut f = Flight::new(&world);
                f.orbit_flight = true;
                for _ in 0..180 {
                    f.position.z = altitude;
                    f.step(
                        1.0 / 30.0,
                        &Controls {
                            climb: direction,
                            manual: true,
                            aerobatic: true,
                            ..Default::default()
                        },
                        &world,
                    );
                    assert!(f.pitch.abs() <= limit + 0.002);
                }
                assert!((f.pitch.abs() - limit).abs() < 0.005);
                if fraction == 1.0 {
                    let attitude = f.attitude();
                    for _ in 0..90 {
                        f.position.z = altitude;
                        f.step(
                            1.0 / 30.0,
                            &Controls {
                                manual: true,
                                aerobatic: true,
                                ..Default::default()
                            },
                            &world,
                        );
                        assert!(attitude.dot(f.attitude()).abs() > 0.99999);
                    }
                }
            }
        }
    }
}

#[test]
fn chase_waits_for_the_camera_and_warps_without_an_exit_push() {
    use crate::world::portal::Portal;
    let world = World::generate(crate::world::MapKind::Space, 1999);
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, 30.0, 300.0);
    f.follow = f.position + Vec3::Y * 18.0;
    f.yaw = 0.0;
    f.camera_yaw = 0.0;
    f.view = 1;
    f.weights = [0.0, 1.0, 0.0];
    let entry = Portal {
        center: vec3(0.0, 0.0, 300.0),
        yaw: std::f32::consts::PI,
        radius: 50.0,
    };
    let exit = Portal {
        center: vec3(1700.0, 500.0, 500.0),
        yaw: 1.1,
        ..entry
    };
    assert!(
        f.approach_portal(entry, f.position + Vec3::Y * 5.0)
            .is_none()
    );
    let mut finished = false;
    for frame in 0..120 {
        let previous = f.position;
        f.step(
            1.0 / 60.0,
            &Controls {
                manual: true,
                ..Default::default()
            },
            &world,
        );
        let before = f.clone();
        let camera = Camera::new(&f, 320, 170);
        if let Some(source) = f.approach_portal(entry, previous) {
            assert!(frame > 45);
            let marker = f.position + f.axes()[1] * 200.0;
            let projected = camera.project(marker).unwrap().0;
            let plane = camera.project(f.position).unwrap().0;
            let trail = f.trails[0];
            f.warp(source, exit);
            assert!(f.trails[0][0].distance(source.transform(exit, trail[0])) < 0.001);
            assert!(f.trails[0][1].distance(source.transform(exit, trail[1])) < 0.001);
            let actual = Camera::new(&f, 320, 170);
            assert!(f.position.distance(source.transform(exit, before.position)) < 0.001);
            assert!(f.follow.distance(source.transform(exit, before.follow)) < 0.001);
            assert!(actual.project(f.position).unwrap().0.distance(plane) < 0.002);
            assert!(
                actual
                    .project(source.transform(exit, marker))
                    .unwrap()
                    .0
                    .distance(projected)
                    < 0.002
            );
            finished = true;
            break;
        }
        assert!(f.position.distance(before.position) < 0.001);
    }
    assert!(finished);
}

#[test]
fn overhead_preserves_wingmen_and_hides_the_exit_until_its_halo_leaves_view() {
    use crate::world::portal::{LENS_RADIUS, Portal};
    let world = World::generate(crate::world::MapKind::Space, 1999);
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, -50.0, 300.0);
    f.follow = f.position;
    f.yaw = 0.0;
    f.camera_yaw = 0.0;
    f.weights = [1.0, 0.0, 0.0];
    let entry = Portal {
        center: vec3(0.0, 0.0, 300.0),
        yaw: std::f32::consts::PI,
        radius: 50.0,
    };
    let exit = Portal {
        center: vec3(1600.0, 1500.0, 300.0),
        yaw: 0.0,
        ..entry
    };
    f.wingmen[0].position = f.position + vec3(100.0, 0.0, 15.0);
    f.wingmen[1].position = f.position + vec3(-85.0, 70.0, 10.0);
    let camera = Camera::new(&f, 320, 170);
    let positions: Vec<_> = f
        .wingmen
        .iter()
        .map(|wing| (wing.position, camera.project(wing.position).unwrap().0))
        .collect();
    f.warp(entry, exit);
    let camera = Camera::new(&f, 320, 170);
    for (wing, (position, pixel)) in f.wingmen.iter().zip(positions) {
        assert!(wing.position.distance(entry.transform(exit, position)) < 0.001);
        assert!(camera.project(wing.position).unwrap().0.distance(pixel) < 0.001);
        assert_eq!(wing.portal_wait, 0.0);
        assert!(!wing.rejoining);
    }
    f.portal_cooldown = 0.0;
    f.update_portal_visibility();
    assert!(f.hides_portal(exit));
    assert!(
        f.approach_portal(exit, f.position + Vec3::Y * 5.0)
            .is_none()
    );
    let origin = f.position;
    f.position.x += (160.0 + 60.0) / camera.scale;
    f.follow = f.position;
    let (center, _, scale) = Camera::new(&f, 320, 170).project(exit.center).unwrap();
    assert!(center.x + exit.radius * scale < 0.0);
    assert!(center.x + exit.radius * scale * LENS_RADIUS > 0.0);
    f.update_portal_visibility();
    assert!(f.hides_portal(exit));
    f.position.x += 120.0;
    f.follow = f.position;
    f.update_portal_visibility();
    assert!(!f.hides_portal(exit));
    f.position = origin;
    f.follow = origin;
    f.portal_cooldown = 0.4;
    f.update_portal_visibility();
    assert!(!f.hides_portal(exit));
}

#[test]
fn terrain_protection_keeps_pitch_level() {
    let mut world = World::generate(
        crate::world::MapKind::Alpine,
        crate::world::MapKind::Alpine.seed(),
    );
    world.heights.fill(250);
    world.scenery.clear();
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, 0.0, 120.0);
    f.follow = f.position;
    f.yaw = 0.0;
    f.speed = 92.0;
    let controls = Controls {
        throttle: Some(1.0),
        manual: false,
        aerobatic: false,
        ..Default::default()
    };
    let mut previous = f.pitch;
    let mut worst = 0.0_f32;
    for _ in 0..120 {
        f.step(1.0 / 30.0, &controls, &world);
        worst = worst.max((f.pitch - previous).abs());
        previous = f.pitch;
    }
    assert!(
        f.position.z > 250.0,
        "did not clear terrain {}",
        f.position.z
    );
    assert!(worst < 0.05, "pitch jumped {worst}");
}

#[test]
fn camera_view_altitude_smooths_abrupt_lift() {
    let world = World::generate(
        crate::world::MapKind::Moon,
        crate::world::MapKind::Moon.seed(),
    );
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, 0.0, 320.0);
    f.follow = f.position;
    f.view_altitude = 1200.0;
    let controls = Controls {
        throttle: Some(1.0),
        manual: true,
        aerobatic: true,
        ..Default::default()
    };
    f.step(1.0 / 30.0, &controls, &world);
    assert!(
        f.view_altitude > 800.0,
        "view altitude snapped {}",
        f.view_altitude
    );
}

#[test]
fn hover_holds_altitude_when_entered_from_descent() {
    let world = World::generate(
        crate::world::MapKind::Moon,
        crate::world::MapKind::Moon.seed(),
    );
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, 0.0, 900.0);
    f.follow = f.position;
    f.pitch = -0.35;
    f.speed = 92.0;
    f.hover = true;
    let controls = Controls {
        throttle: Some(0.0),
        manual: true,
        aerobatic: true,
        ..Default::default()
    };
    let start = f.position.z;
    let mut worst = 0.0_f32;
    for _ in 0..60 {
        f.step(1.0 / 30.0, &controls, &world);
        worst = worst.max((f.position.z - start).abs());
    }
    assert!(f.speed < 0.01, "speed {}", f.speed);
    assert!(worst < 5.0, "altitude drifted {worst}");
}

#[test]
fn safety_lift_rises_without_a_jump() {
    let mut world = World::generate(
        crate::world::MapKind::Alpine,
        crate::world::MapKind::Alpine.seed(),
    );
    world.heights.fill(250);
    world.scenery.clear();
    let mut f = Flight::new(&world);
    f.position = vec3(0.0, 0.0, 120.0);
    f.follow = f.position;
    f.speed = 92.0;
    let controls = Controls {
        throttle: Some(1.0),
        manual: true,
        aerobatic: true,
        ..Default::default()
    };
    let before = f.position.z;
    f.step(1.0 / 30.0, &controls, &world);
    let first = f.position.z - before;
    assert!(first <= 2.8, "first lift frame jumped {first}");
    for _ in 0..180 {
        f.step(1.0 / 30.0, &controls, &world);
    }
    assert!(
        f.position.z > 250.0,
        "did not clear terrain {}",
        f.position.z
    );
}

#[test]
fn overhead_blocks_climb_and_keeps_pitch() {
    use crate::world::World;
    let world = World::generate(crate::world::MapKind::Desert, 1999);
    let mut flight = Flight::new(&world);
    flight.space = false;
    flight.surface = None;
    flight.pitch = 0.4;
    flight.vertical_speed = -40.0;
    flight.position = glam::vec3(0.0, 0.0, 3000.0);
    flight.follow = flight.position;
    let controls = Controls {
        manual: true,
        aerobatic: true,
        heading_only: true,
        turn: 0.5,
        climb: 1.0,
        ..Default::default()
    };
    let (z, pitch) = (flight.position.z, flight.pitch);
    for _ in 0..40 {
        flight.step(1.0 / 30.0, &controls, &world);
    }
    assert!((flight.position.z - z).abs() < 0.6, "俯瞰视角不应改变高度");
    assert!((flight.pitch - pitch).abs() < 0.01, "俯瞰视角不应改变俯仰");
}
