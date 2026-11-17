use super::*;
use glam::vec2;

fn app(kind: MapKind) -> App {
    let mut app = App::new(World::generate(kind, kind.seed()));
    app.start();
    app.auto = false;
    app.record_path = None;
    app
}

#[test]
fn boost_requires_sustained_shift_climb_and_altitude() {
    let mut app = app(MapKind::Moon);
    app.flight.pitch = 0.5;
    for (height, boost, climb, expected) in [
        (450.0, true, 1.0, false),
        (700.0, false, 1.0, false),
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
        controls.boost = false;
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
fn ascent_waits_for_preload_and_can_return_to_both_surfaces() {
    use crate::flight::Camera;
    for kind in [MapKind::Moon, MapKind::Islands] {
        let mut app = app(kind);
        let original = app.world.clone();
        let body = Body::of(kind);
        app.flight.position.z = 1210.0;
        app.flight.follow = app.flight.position;
        app.flight.pitch = 0.5;
        app.flight.speed = 150.0;
        app.orbit.boost = true;
        assert!(!app.step_orbit(0.03));
        assert!(app.orbit.transition.is_none());
        app.network
            .remember(World::generate(MapKind::Space, MapKind::Space.seed()).into());
        assert!(app.step_orbit(0.03));
        assert_eq!(app.flight.surface.unwrap().bend, 0.0);
        let mut switched = 0;
        for _ in 0..100 {
            let previous = app.world.kind;
            app.step(0.1, Some(Controls::default()));
            switched += usize::from(previous != app.world.kind);
        }
        assert_eq!(switched, 1);
        assert_eq!(app.world.kind, MapKind::Space);
        assert!(app.flight.surface.is_none());
        assert!(app.flight.warp_flash <= 0.0);
        assert!(app.flight.position.is_finite());
        let center = body.center();
        let position = center + vec3(0.0, body.radius() + 1000.0, 300.0);
        app.flight.translate(position - app.flight.position);
        app.flight.pitch = -0.2;
        app.flight.yaw = 0.0;
        app.flight.camera_adjustment = None;
        let camera = Camera::new(&app.flight, 320, 170);
        app.network.remember(original);
        app.begin_descent(
            body,
            MapKey {
                kind,
                seed: kind.seed(),
            },
        );
        let warp = app.flight.surface.unwrap();
        let after = Camera::new(&app.flight, 320, 170);
        assert!((warp.basis * after.forward - camera.forward).length() < 0.001);
        assert!(app.flight.warp_flash <= 0.0);
        for _ in 0..100 {
            app.step(0.1, Some(Controls::default()));
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
    }
}

#[test]
fn transition_pauses_and_ignores_aerobatic_shortcuts() {
    let mut app = app(MapKind::Moon);
    app.flight.position.z = 1000.0;
    app.flight.surface = Some(SurfaceWarp::new(
        Body::Moon,
        vec2(2048.0, 2048.0),
        app.flight.position.truncate(),
        0.7,
    ));
    app.begin_transition(
        false,
        MapKey::of(&app.world),
        app.flight.position.with_z(300.0),
    );
    for code in [
        crate::keys::UP,
        crate::keys::DOWN,
        crate::keys::ROLL_LEFT,
        crate::keys::ROLL_RIGHT,
    ] {
        for _ in 0..2 {
            app.key(code, 1);
            app.key(code, 0);
        }
    }
    let mut check = app.flight.clone();
    assert!(check.loop_toward(1.0));
    assert!(check.roll_toward(1.0));
    app.help_return = Page::Playing;
    for page in [Page::Paused, Page::Chart, Page::Help] {
        app.page = page;
        let before = app.flight.position;
        app.step(0.1, None);
        assert_eq!(app.flight.position, before);
        assert_eq!(app.orbit.transition.as_ref().unwrap().elapsed, 0.0);
    }
    app.home();
    assert!(app.orbit.transition.is_none());
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
    app.orbit.reroll_earth = true;
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
        app.flight.position = body.center() + vec3(0.0, body.radius() + 1000.0, 300.0);
        app.flight.follow = app.flight.position;
        app.flight.yaw = 0.0;
        app.flight.pitch = -0.2;
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
