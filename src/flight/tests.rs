use super::*;
use crate::world::{CLOUD_SIZE, SIZE};

fn terrain() -> World {
    World {
        obstacles: vec![],
        ceiling: 40,
        cloud_levels: vec![],
        heights: vec![0; SIZE * SIZE],
        materials: vec![0; SIZE * SIZE],
        clouds: vec![0; CLOUD_SIZE * CLOUD_SIZE],
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
    let source = entry.facing(entry.center + entry.normal() * 4.0);
    let old_follow = f.follow;
    let old_wing = f.wingmen[0].position;
    let yaw = f.yaw;
    assert!(f.traverse_portal(&world, entry.center + entry.normal() * 4.0));
    let offset = f.position - source.transform(exit, entry.center - entry.normal() * 3.0);
    assert!((f.position - exit.center).dot(exit.normal()) > exit.radius * 0.7);
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
    for _ in 0..150 {
        f.step(
            0.033,
            &Controls {
                manual: true,
                climb: -1.0,
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
    world.heights[..220 * SIZE].fill(40);
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
fn held_pitch_completes_upward_and_downward_loops() {
    let world = terrain();
    for direction in [-1.0, 1.0] {
        let mut f = Flight::new(&world);
        f.position.z = f.loop_altitude() + 25.0;
        f.follow = f.position;
        let mut rotation = 0.0;
        for _ in 0..420 {
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
    for _ in 0..600 {
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
    assert!(f.position.z >= MIN_CLEARANCE && f.position.z < MIN_CLEARANCE + 1.0);
    f.position = vec3(256.0, 450.0, 30.0);
    f.pitch = 0.0;
    world.heights[..260 * SIZE].fill(130);
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
    f.position.z = f.loop_altitude() + 25.0;
    assert!(f.roll());
    let mut rotation = 0.0;
    for frame in 0..420 {
        let before = f.pitch;
        let altitude = f.position.z;
        f.step(
            1.0 / 60.0,
            &Controls {
                climb: if frame < 60 { -1.0 } else { 0.0 },
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
