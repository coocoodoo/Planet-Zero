use super::*;

fn app(kind: MapKind) -> App {
    let mut app = App::new(World::generate(kind, kind.seed()));
    app.start();
    app.auto = false;
    app.record_path = None;
    app.flight.view = 1;
    app.flight.weights = [0.0, 1.0, 0.0];
    app
}

fn ready_ascent(kind: MapKind) -> App {
    let mut app = app(kind);
    app.flight.position.z = Body::of(kind).surface_altitude() + 10.0;
    app.flight.follow = app.flight.position;
    app.flight.pitch = 0.5;
    app.flight.speed = 150.0;
    app.orbit.boost = true;
    assert!(!app.step_orbit(0.03));
    app.network
        .remember(World::generate(MapKind::Space, MapKind::Space.seed()).into());
    assert!(app.step_orbit(0.03));
    app
}

#[cfg(feature = "cardputer")]
#[test]
fn steady_imu_pitch_settles_after_ascent_without_keyboard_input() {
    let mut game = crate::cardputer::Game {
        app: ready_ascent(MapKind::Moon),
        motion: crate::cardputer::imu::Motion {
            pitch: 0.5,
            available: true,
            ready: true,
            ..Default::default()
        },
        imu_enabled: true,
        calibrate_imu: false,
    };
    for _ in 0..1800 {
        game.step(1.0 / 30.0);
        if game.world.kind == MapKind::Space {
            break;
        }
    }
    assert_eq!(game.world.kind, MapKind::Space);
    for _ in 0..300 {
        game.step(1.0 / 30.0);
    }
    let forward = game.flight.axes()[1];
    for _ in 0..300 {
        game.step(1.0 / 30.0);
        assert!(forward.distance(game.flight.axes()[1]) < 0.01);
    }
    assert!(game.flight.pitch > 1.4);
}

#[test]
fn high_altitude_climb_accelerates_with_or_without_shift() {
    let mut app = app(MapKind::Moon);
    app.flight.pitch = 0.5;
    for (height, boost, climb, expected) in [
        (450.0, true, 1.0, false),
        (700.0, false, 1.0, true),
        (700.0, true, 0.0, false),
        (700.0, true, 1.0, true),
    ] {
        app.orbit = Orbit::default();
        app.flight.position.z = height;
        let mut controls = Controls {
            boost,
            climb,
            ..Default::default()
        };
        for frame in 0..7 {
            app.orbit_controls(&mut controls, 0.1);
            assert_eq!(controls.orbital, expected && frame >= 5);
        }
        controls.climb = 0.0;
        app.orbit_controls(&mut controls, 0.1);
        assert!(!controls.orbital);
    }
    app.mode = Mode::Combat;
    let mut controls = Controls {
        boost: true,
        climb: 1.0,
        ..Default::default()
    };
    app.orbit_controls(&mut controls, 1.0);
    assert!(!controls.orbital);
}

#[test]
fn ascent_and_descent_follow_manual_flight_and_preserve_velocity() {
    for kind in [MapKind::Moon, MapKind::Islands] {
        let mut app = ready_ascent(kind);
        let original = app.world.clone();
        let body = Body::of(kind);
        let controls = Controls {
            manual: true,
            aerobatic: true,
            climb: 1.0,
            ..Default::default()
        };
        for _ in 0..1800 {
            app.step(1.0 / 60.0, Some(Controls { ..controls }));
            if app.world.kind == MapKind::Space {
                break;
            }
        }
        assert_eq!(app.world.kind, MapKind::Space);
        assert!((body.altitude(app.flight.position) - body.orbit_altitude()).abs() < 100.0);
        assert_eq!(app.orbit.arrival_notice, 5.0);
        assert!(app.flight.speed > app.flight.vehicle.speed() * 3.0);
        if body == Body::Earth {
            assert_ne!(app.orbit.earth.unwrap().kind, kind);
        }
        assert!(app.flight.surface.is_none());
        assert!(app.flight.warp_flash <= 0.0);
        let position = body.center() + Vec3::Z * (body.radius() + body.orbit_altitude() - 10.0);
        app.flight.translate(position - app.flight.position);
        app.flight.space_attitude = Some(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
        app.flight.camera_adjustment = None;
        app.flight.camera_look = glam::vec2(0.2, 0.1);
        let camera = crate::flight::Camera::new(&app.flight, 320, 170);
        let velocity = app.flight.velocity();
        app.network.remember(original);
        app.begin_descent(
            body,
            MapKey {
                kind,
                seed: kind.seed(),
            },
        );
        let warp = app.flight.surface.unwrap();
        let after = crate::flight::Camera::new(&app.flight, 320, 170);
        assert!((warp.basis * after.forward - camera.forward).length() < 0.001);
        assert!((warp.basis * app.flight.velocity() - velocity).length() < 0.001);
        for _ in 0..1800 {
            app.step(
                1.0 / 60.0,
                Some(Controls {
                    climb: -1.0,
                    ..controls
                }),
            );
            if app.orbit.transition.is_none() {
                break;
            }
        }
        assert_eq!(app.world.kind, kind);
        assert_eq!(app.flight.surface.unwrap().bend, 0.0);
        assert!(app.orbit.transition.is_none());
        assert!(
            app.flight.position.z
                > app
                    .world
                    .height(app.flight.position.x, app.flight.position.y)
        );
        assert!(app.flight.warp_flash <= 0.0);
    }
}

#[test]
fn transition_holds_altitude_accepts_turns_reverses_and_pauses() {
    let mut app = ready_ascent(MapKind::Moon);
    app.flight.position.z = 1800.0;
    app.flight.pitch = 0.0;
    let controls = Controls {
        manual: true,
        aerobatic: true,
        turn: 0.3,
        ..Default::default()
    };
    let yaw = app.flight.yaw;
    for _ in 0..900 {
        app.step(1.0 / 60.0, Some(Controls { ..controls }));
    }
    assert!(app.orbit.transition.is_some());
    assert!((app.flight.position.z - 1800.0).abs() < 0.01);
    assert!((app.flight.yaw - yaw).abs() > 1.0);
    app.help_return = Page::Playing;
    for page in [Page::Paused, Page::Chart, Page::Help] {
        app.page = page;
        let before = app.flight.position;
        let bend = app.flight.surface.unwrap().bend;
        app.step(0.1, None);
        assert_eq!(app.flight.position, before);
        assert_eq!(app.flight.surface.unwrap().bend, bend);
    }
    app.page = Page::Playing;
    for _ in 0..1800 {
        app.step(
            1.0 / 60.0,
            Some(Controls {
                climb: -1.0,
                turn: 0.0,
                ..controls
            }),
        );
        if app.orbit.transition.is_none() {
            break;
        }
    }
    assert!(app.orbit.transition.is_none());
    assert_eq!(app.world.kind, MapKind::Moon);
    assert_eq!(app.flight.surface.unwrap().bend, 0.0);
    app.home();
    assert!(app.flight.surface.is_none());
    assert!(app.flight.camera_adjustment.is_none());
}

#[test]
fn earth_destination_is_stable_while_visible_and_uses_surface_maps() {
    let mut app = app(MapKind::Space);
    app.network = crate::world::network::Network::new(7109);
    let first = app.network.random_earth();
    app.orbit.earth = Some(first);
    app.flight
        .translate(Body::Earth.center() + Vec3::Y * 4000.0 - app.flight.position);
    app.flight.follow = app.flight.position;
    app.flight.yaw = 0.0;
    app.flight.view = 1;
    app.flight.weights = [0.0, 1.0, 0.0];
    app.orbit.departure = Some(MapKind::Islands);
    for _ in 0..10 {
        app.prepare_orbit();
        assert_eq!(app.orbit.earth, Some(first));
    }
    let mut kinds = std::collections::HashSet::new();
    for _ in 0..120 {
        let key = app.network.random_earth();
        assert!(!key.kind.vacuum());
        kinds.insert(key.kind);
    }
    assert_eq!(kinds.len(), 10);
    app.network
        .park_portals(&app.world, app.flight.position, app.flight.axes()[1]);
    for route in app.network.routes(&app.world) {
        assert!((route.aperture.center - app.flight.position).dot(app.flight.axes()[1]) < -3000.0);
        for body in Body::ALL {
            assert!(body.altitude(route.aperture.center) >= 2199.0);
        }
    }
}

#[test]
fn surface_handoff_preserves_the_rendered_globe() {
    for kind in [MapKind::Moon, MapKind::Canyon] {
        let mut app = app(MapKind::Space);
        let body = Body::of(kind);
        let destination = Arc::new(World::generate(kind, kind.seed()));
        let key = MapKey::of(&destination);
        let environment = app.network.environment(key);
        app.flight.planet_maps[body.index()] =
            Some(Arc::new(SurfaceMap::new(&destination, environment)));
        app.orbit.earth = if body == Body::Earth { Some(key) } else { None };
        app.network.remember(destination);
        app.flight.position =
            body.center() + vec3(0.0, body.radius() + body.orbit_altitude() - 10.0, 300.0);
        app.flight.follow = app.flight.position;
        app.flight.yaw = 0.0;
        app.flight.pitch = -0.2;
        app.flight.space_attitude = Some(Quat::from_rotation_x(0.2) * Quat::from_rotation_y(0.3));
        app.flight.weights = [0.0, 1.0, 0.0];
        app.flight.wing_count = 0;
        app.flight.orbit_quiet = true;
        app.renderer.render(&app.world, &app.flight, 320, 170, 0);
        let before = app.renderer.rgba.clone();
        app.begin_descent(body, key);
        app.renderer
            .render(&app.world, &app.flight, 320, 170, app.theme);
        let error: u64 = before
            .iter()
            .zip(&app.renderer.rgba)
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum();
        assert!(
            error as f32 / (320.0 * 170.0 * 4.0) < 3.0,
            "{}: pixel error {}",
            kind.name(),
            error as f32 / (320.0 * 170.0 * 4.0)
        );
    }
}

#[test]
fn ascent_handoff_keeps_motion_and_manual_aerobatics() {
    let mut app = ready_ascent(MapKind::Moon);
    app.flight.position.z = Body::Moon.orbit_altitude();
    app.flight.pitch = 0.9;
    app.flight.camera_look = glam::vec2(-0.2, 0.3);
    assert!(app.flight.loop_toward(1.0));
    let before = app.flight.velocity();
    let rotation = app.flight.surface.unwrap().basis;
    app.step_orbit(0.03);
    assert!(app.flight.velocity().distance(rotation * before) < 0.001);
    assert!(!app.flight.clone().loop_toward(1.0));
    let before = app.flight.velocity();
    app.step(
        1.0 / 60.0,
        Some(Controls {
            manual: true,
            ..Default::default()
        }),
    );
    assert!(app.flight.velocity().distance(before) < 15.0);
}

#[test]
fn planetary_collision_protection_checks_bodies_outside_the_view() {
    let mut app = app(MapKind::Space);
    app.orbit.cooldown = 2.0;
    for body in Body::ALL {
        app.flight.position = body.center() + Vec3::Y * (body.radius() + 100.0);
        app.flight.follow = app.flight.position;
        app.flight.space_attitude = Some(Quat::from_rotation_z(std::f32::consts::PI));
        assert!(!app.step_orbit(0.03));
        assert!(body.altitude(app.flight.position) >= 419.99);
    }
}

#[test]
fn ascent_handoff_preserves_space_scenery() {
    for kind in [MapKind::Moon, MapKind::Islands] {
        let mut app = app(kind);
        let destination = Arc::new(World::generate(MapKind::Space, MapKind::Space.seed()));
        let origin = app.flight.position.truncate();
        let body = Body::of(kind);
        let mut map = SurfaceMap::new(&app.world, app.network.environment(MapKey::of(&app.world)));
        map.origin = origin;
        app.flight.planet_maps[body.index()] = Some(Arc::new(map));
        app.flight.position.z = 2300.0;
        app.flight.follow = app.flight.position;
        app.flight.pitch = 0.9;
        app.flight.weights = [1.0, 0.0, 0.0];
        app.flight.wing_count = 0;
        let surface = SurfaceWarp::new(body, origin, origin, 1.0);
        app.flight.surface = Some(surface);
        app.flight.sky_rotation = Quat::from_mat3(&surface.basis);
        app.flight.orbital_scenery = Some(destination.clone());
        app.flight.orbit_quiet = true;
        app.renderer
            .render(&app.world, &app.flight, 320, 170, app.theme);
        let before = app.renderer.rgba.clone();
        app.finish_ascent(destination);
        app.renderer
            .render(&app.world, &app.flight, 320, 170, app.theme);
        let error: u64 = before
            .iter()
            .zip(&app.renderer.rgba)
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum();
        assert!(
            error as f32 / (320.0 * 170.0 * 4.0) < 3.0,
            "{}: pixel error {}",
            kind.name(),
            error as f32 / (320.0 * 170.0 * 4.0)
        );
    }
}

#[test]
fn descent_can_be_reversed_or_finished_by_navigation() {
    for automatic in [false, true] {
        let mut app = app(MapKind::Space);
        let destination = Arc::new(World::generate(MapKind::Moon, MapKind::Moon.seed()));
        let key = MapKey::of(&destination);
        app.network.remember(destination);
        app.flight.position = Body::Moon.center()
            + Vec3::Z * (Body::Moon.radius() + Body::Moon.orbit_altitude() - 10.0);
        app.flight.follow = app.flight.position;
        app.flight.space_attitude = Some(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
        app.navigation.active = automatic;
        app.begin_descent(Body::Moon, key);
        for _ in 0..5400 {
            app.step(
                1.0 / 60.0,
                Some(Controls {
                    manual: true,
                    aerobatic: true,
                    climb: 1.0,
                    ..Default::default()
                }),
            );
            if app.world.kind == MapKind::Space || app.navigation.arrived {
                break;
            }
        }
        if automatic {
            assert_eq!(app.world.kind, MapKind::Moon);
            assert!(
                app.navigation.arrived,
                "position {:?}, target {:?}",
                app.flight.position,
                app.navigation.target.unwrap().position
            );
            assert!(app.orbit.transition.is_none());
        } else {
            assert_eq!(app.world.kind, MapKind::Space);
            assert!(
                app.flight.axes()[1].dot((app.flight.position - Body::Moon.center()).normalize())
                    > 0.9
            );
        }
    }
}

#[test]
fn descent_flattens_terrain_and_tilts_camera_down() {
    let kind = MapKind::Moon;
    let mut app = ready_ascent(kind);
    let body = Body::of(kind);
    let original = app.world.clone();
    let position = body.center() + Vec3::Z * (body.radius() + body.orbit_altitude() - 10.0);
    app.flight.translate(position - app.flight.position);
    app.flight.space_attitude = Some(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    app.flight.camera_adjustment = None;
    app.network.remember(original);
    app.begin_descent(
        body,
        MapKey {
            kind,
            seed: kind.seed(),
        },
    );
    app.flight
        .translate(Vec3::Z * (body.surface_altitude() * 0.5 - app.flight.position.z));
    app.flight.follow = app.flight.position;
    app.flight.pitch = 0.0;
    for _ in 0..120 {
        app.step(
            1.0 / 60.0,
            Some(Controls {
                manual: true,
                aerobatic: true,
                ..Default::default()
            }),
        );
    }
    let bend = app.flight.surface.unwrap().bend;
    assert!(bend < 0.3, "bend {bend}");
    let camera = crate::flight::Camera::new(&app.flight, 320, 170);
    assert!(camera.pitch < -0.08, "camera pitch {}", camera.pitch);
}

#[test]
fn descent_keeps_camera_scale_and_heading_stable() {
    let kind = MapKind::Moon;
    let mut app = ready_ascent(kind);
    let body = Body::of(kind);
    let original = app.world.clone();
    let position = body.center() + Vec3::Z * (body.radius() + body.orbit_altitude() - 10.0);
    app.flight.translate(position - app.flight.position);
    app.flight.space_attitude = Some(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    app.network.remember(original);
    app.begin_descent(
        body,
        MapKey {
            kind,
            seed: kind.seed(),
        },
    );
    let mut previous: Option<(f32, Vec3)> = None;
    for _ in 0..30 {
        app.step(
            1.0 / 30.0,
            Some(Controls {
                manual: true,
                aerobatic: true,
                throttle: Some(1.0),
                ..Default::default()
            }),
        );
        let camera = crate::flight::Camera::new(&app.flight, 320, 170);
        if let Some((scale, forward)) = previous {
            assert!(
                (camera.scale - scale).abs() < 0.05,
                "camera scale jumped {} -> {}",
                scale,
                camera.scale
            );
            assert!(camera.forward.dot(forward) > 0.98, "camera heading jumped");
        }
        previous = Some((camera.scale, camera.forward));
    }
}

#[test]
fn curvature_depends_on_altitude_through_reversal() {
    let mut app = ready_ascent(MapKind::Moon);
    for height in [1300.0, 1800.0, 2400.0, 1800.0, 1300.0] {
        app.flight.position.z = height;
        let expected = Body::Moon.curvature(height);
        for pitch in [-0.6, 0.6, 0.0, -0.6] {
            app.flight.pitch = pitch;
            app.step_orbit(0.03);
            app.step_orbit(0.03);
            assert!((app.flight.surface.unwrap().bend - expected).abs() < 0.00001);
        }
    }
}

#[test]
fn level_surface_flight_keeps_altitude_and_heading_across_a_pole() {
    let mut app = ready_ascent(MapKind::Moon);
    let anchor = glam::vec2(
        1000.0,
        std::f32::consts::FRAC_PI_2 * crate::world::orbit::UV_SCALE - 10.0,
    );
    let bend = Body::Moon.curvature(1800.0);
    app.flight.surface = Some(SurfaceWarp::new(Body::Moon, glam::Vec2::ZERO, anchor, bend));
    app.flight
        .translate(anchor.extend(1800.0) - app.flight.position);
    app.flight.follow = app.flight.position;
    app.flight.yaw = std::f32::consts::PI;
    app.flight.pitch = 0.0;
    let mut previous = None;
    for _ in 0..600 {
        app.step(
            1.0 / 60.0,
            Some(Controls {
                manual: true,
                aerobatic: true,
                ..Default::default()
            }),
        );
        let surface = app.flight.surface.unwrap();
        assert!(
            surface
                .point(app.flight.position)
                .distance(app.flight.position)
                < 0.01
        );
        assert!((app.flight.position.z - 1800.0).abs() < 0.01);
        let forward = surface.basis * app.flight.axes()[1];
        if let Some(before) = previous {
            assert!(forward.dot(before) > 0.99);
        }
        previous = Some(forward);
    }
}
