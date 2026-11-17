use super::*;

#[test]
fn orbital_boost_requires_free_flight_altitude_and_respects_pause() {
    use crate::keys::ORBIT;
    let mut app = App::new(World::generate(crate::world::MapKind::Moon, 1999));
    app.start();
    app.auto = false;
    app.flight.position.z = 599.0;
    app.flight.pitch = 0.5;
    app.key(ORBIT, 1);
    app.key(ORBIT, 0);
    assert!(!app.orbit.boost);
    app.flight.position.z = 650.0;
    app.flight.pitch = 0.0;
    assert!(!app.orbit_ready());
    app.flight.pitch = 0.5;
    app.mode = Mode::Combat;
    assert!(!app.orbit_ready());
    app.mode = Mode::Free;
    app.key(ORBIT, 1);
    app.key(ORBIT, 0);
    assert!(app.orbit.boost);
    app.page = Page::Paused;
    let position = app.flight.position;
    app.step(0.1, None);
    assert_eq!(app.flight.position, position);
    app.page = Page::Playing;
    app.step(0.1, None);
    assert!(app.flight.orbital_thrust > 0.0);
    app.key(ORBIT, 1);
    app.key(ORBIT, 0);
    assert!(!app.orbit.boost);
    app.key(ORBIT, 1);
    app.key(ORBIT, 0);
    app.home();
    assert!(!app.orbit.boost);
    assert!(app.orbit.transition.is_none());
}

#[test]
fn orbital_swap_preserves_camera_attitude_and_formation_in_all_views() {
    use crate::world::MapKind;
    use crate::world::network::MapKey;
    use glam::vec3;
    let moon = std::sync::Arc::new(World::generate(MapKind::Moon, MapKind::Moon.seed()));
    for view in 0..3 {
        let mut app = App::new(World::generate(MapKind::Space, MapKind::Space.seed()));
        app.start();
        app.auto = false;
        app.flight.view = view;
        app.flight.weights = std::array::from_fn(|i| f32::from(i == view));
        app.flight.yaw = 0.7;
        app.flight.pitch = -0.4;
        app.flight.bank = 0.2;
        app.flight.follow = app.flight.position + vec3(-8.0, 10.0, 12.0);
        let camera = crate::flight::Camera::new(&app.flight, 320, 170);
        let position = camera
            .project(app.flight.position + app.flight.axes()[1] * 180.0)
            .unwrap()
            .0;
        let wings: Vec<_> = app
            .flight
            .wingmen
            .iter()
            .map(|w| w.position - app.flight.position)
            .collect();
        let axes = app.flight.axes();
        app.network.remember(moon.clone());
        let target = MapKey::of(&moon);
        assert!(app.orbit_arrive(target));
        assert_eq!(app.world.kind, MapKind::Moon);
        assert_eq!(app.flight.axes(), axes);
        let after = crate::flight::Camera::new(&app.flight, 320, 170);
        assert!(
            position.distance(
                after
                    .project(app.flight.position + app.flight.axes()[1] * 180.0)
                    .unwrap()
                    .0
            ) < 0.001
        );
        for (wing, expected) in app.flight.wingmen.iter().zip(wings) {
            assert!((wing.position - app.flight.position - expected).length() < 0.001);
        }
        assert_eq!(app.flight.view, view);
    }
}

#[test]
fn orbital_ascent_and_lunar_approach_wait_for_preload_and_switch_once() {
    use crate::world::MapKind;
    use crate::world::celestial::{MOON_RADIUS, moon_center};
    use glam::Vec3;
    let moon = std::sync::Arc::new(World::generate(MapKind::Moon, MapKind::Moon.seed()));
    let mut app = App::new((*moon).clone());
    app.start();
    app.auto = false;
    app.flight.position.z = 1190.0;
    app.flight.follow = app.flight.position;
    app.flight.pitch = 0.5;
    app.network
        .remember(World::generate(MapKind::Space, MapKind::Space.seed()).into());
    app.rendered_map = Some(crate::world::network::MapKey::of(&app.world));
    app.key(crate::keys::ORBIT, 1);
    app.key(crate::keys::ORBIT, 0);
    for _ in 0..180 {
        app.step(1.0 / 30.0, None);
    }
    assert_eq!(app.world.kind, MapKind::Space);
    assert_eq!(app.orbit.transition.as_ref().unwrap().opacity(), 1.0);
    app.page = Page::Paused;
    let position = app.flight.position;
    app.step(0.1, None);
    assert_eq!(app.flight.position, position);
    app.page = Page::Playing;
    app.rendered_map = Some(crate::world::network::MapKey::of(&app.world));
    for _ in 0..90 {
        app.step(1.0 / 30.0, None);
    }
    assert!(app.orbit.transition.is_none());
    assert!(!app.orbit.boost);
    app.network.remember(moon);
    let center = moon_center(app.flight.position);
    app.flight
        .translate(center + Vec3::Y * (MOON_RADIUS + 300.0) - app.flight.position);
    app.flight.yaw = 0.0;
    app.flight.pitch = 0.0;
    app.rendered_map = None;
    for _ in 0..420 {
        app.step(1.0 / 30.0, None);
    }
    assert_eq!(app.world.kind, MapKind::Moon);
    assert!(app.orbit.transition.is_none());
    assert!(!app.orbit.boost);

    app.world = World::generate(MapKind::Space, MapKind::Space.seed()).into();
    app.network = crate::world::network::Network::new(7);
    app.flight.change_world(&app.world);
    let center = moon_center(app.flight.position);
    app.flight
        .translate(center + Vec3::Y * (MOON_RADIUS + 200.0) - app.flight.position);
    app.flight.yaw = 0.0;
    app.flight.pitch = 0.0;
    app.step(1.0 / 30.0, None);
    assert_eq!(app.world.kind, MapKind::Space);
    assert!(app.orbit.transition.is_none());
}

#[test]
fn menu_enter_moves_to_start_without_changing_options() {
    let mut app = App::new(World::new(7));
    for row in 0..START_ROW {
        app.selection = row;
        app.key(28, 1);
        app.key(28, 0);
        assert_eq!(app.selection, START_ROW);
        assert_eq!(app.page, Page::Menu);
        assert_eq!(app.mode, Mode::Free);
        assert_eq!(app.flight.vehicle, Vehicle::Falcon);
        assert!(app.loader.pending.is_none());
    }
    app.key(28, 1);
    assert_eq!(app.page, Page::Playing);
}

#[test]
fn menus_pause_retry_and_hidden_ui_preserve_control() {
    let mut app = App::new(World::new(7));
    app.record_path = None;
    assert_eq!(app.page, Page::Menu);
    assert!(app.flight.demo);
    app.mode = Mode::Combat;
    app.start();
    app.step(1.0, None);
    app.key(25, 1);
    app.key(25, 0);
    assert_eq!(app.page, Page::Paused);
    let before = (app.flight.position, app.mission.countdown);
    app.step(0.1, None);
    assert_eq!((app.flight.position, app.mission.countdown), before);
    app.key(22, 1);
    app.key(22, 0);
    assert!(!app.flight.hud);
    app.key(28, 1);
    app.key(28, 0);
    assert_eq!(app.page, Page::Playing);
    app.mission.score = 3456;
    app.mission.outcome = Some(true);
    app.step(0.01, None);
    assert_eq!(app.page, Page::Results);
    assert_eq!(app.best_score, 3456);
    app.key(19, 1);
    app.key(19, 0);
    assert_eq!(app.page, Page::Playing);
    assert_eq!(app.mission.score, 0);
    assert_eq!(app.mission.health, app.flight.vehicle.shield());
    assert_eq!(app.best_score, 3456);
    assert!(!app.flight.hud);
    app.home();
    app.key(crate::keys::HELP, 1);
    app.key(crate::keys::HELP, 0);
    assert_eq!(app.page, Page::Help);
    app.key(28, 1);
    app.key(28, 0);
    assert_eq!(app.page, Page::Menu);
}

#[test]
fn success_advances_with_reward_and_failure_retries_snapshot() {
    let mut app = App::new(World::new(7));
    app.record_path = None;
    app.mode = Mode::Combat;
    app.start();
    app.mission.outcome = Some(true);
    app.page = Page::Results;
    app.mission.levels[2] = 3;
    app.reward = 1;
    app.result_action();
    assert_eq!(app.level, 2);
    assert_eq!(app.page, Page::Loading);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.page == Page::Loading && Instant::now() < deadline {
        app.step(0.01, None);
        std::thread::yield_now();
    }
    assert_eq!(app.page, Page::Playing);
    assert_eq!(app.world.kind, crate::world::MapKind::Alpine);
    assert_eq!(app.mission.levels[2], 3);
    assert_eq!(app.mission.max_health, 120.0);
    let entry = app.mission.loadout();
    app.mission.levels[0] = 3;
    app.mission.health = 0.0;
    app.mission.outcome = Some(false);
    app.page = Page::Results;
    app.key(28, 1);
    app.key(28, 0);
    assert_eq!(app.level, 2);
    assert_eq!(app.page, Page::Playing);
    assert_eq!(app.mission.loadout(), entry);
}

#[test]
fn teleport_does_not_collect_pickups_between_portal_endpoints() {
    use crate::gameplay::{Pickup, PickupKind};
    use glam::Vec3;
    let mut app = App::new(World::generate(crate::world::MapKind::Space, 1999));
    app.mode = Mode::Coins;
    app.start();
    app.mission.countdown = 0.0;
    let entry = app.world.portals[0];
    let exit = app.world.portals[1];
    app.flight.position = entry.center + entry.normal() * 1.0;
    app.flight.follow = app.flight.position;
    app.flight.yaw = entry.yaw + std::f32::consts::PI;
    let middle = entry.center.lerp(exit.center, 0.5);
    app.mission.pickups = vec![Pickup {
        position: middle,
        kind: PickupKind::Coin,
        collected: false,
        life: 100.0,
    }];
    for _ in 0..40 {
        app.step(
            0.05,
            Some(Controls {
                manual: true,
                ..Default::default()
            }),
        );
        if app.flight.warp_flash > 0.0 {
            break;
        }
    }
    assert!(app.flight.position.distance(exit.center) < 120.0);
    assert_eq!(app.mission.collected, 0);
    assert!(!app.mission.pickups[0].collected);
    assert!(app.sounds.iter().any(|s| s == crate::audio::Sound::Warp));
    assert!(app.flight.position != Vec3::ZERO);
}

#[test]
fn wormhole_hotkey_replaces_one_forward_link_on_every_map_without_pausing() {
    for map in crate::world::MapKind::ALL {
        let mut app = App::new(World::generate(map, map.seed()));
        app.start();
        app.network.ensure(&app.world);
        let before: Vec<_> = app
            .network
            .routes(&app.world)
            .iter()
            .map(|r| r.id)
            .collect();
        app.key(crate::keys::PORTAL, 1);
        app.key(crate::keys::PORTAL, 0);
        assert_eq!(app.page, Page::Playing);
        let routes = app.network.routes(&app.world);
        assert_eq!(routes.len(), 2);
        assert_eq!(routes.iter().filter(|r| !before.contains(&r.id)).count(), 1);
        let portal = routes
            .iter()
            .find(|r| !before.contains(&r.id))
            .unwrap()
            .aperture;
        assert!(
            (680.0..800.1).contains(&(portal.center - app.flight.position).truncate().length())
        );
        let previous: Vec<_> = routes.iter().map(|r| r.id).collect();
        for route in routes {
            assert!(route.aperture.center.is_finite());
            assert_ne!(route.target.kind, map);
            if !map.eq(&crate::world::MapKind::Space) {
                assert!(
                    route.aperture.center.z - route.aperture.radius
                        >= app
                            .world
                            .height(route.aperture.center.x, route.aperture.center.y)
                            + crate::flight::MIN_CLEARANCE
                );
            }
        }
        app.key(crate::keys::PORTAL, 1);
        let routes = app.network.routes(&app.world);
        assert_eq!(routes.len(), 2);
        assert_eq!(
            routes.iter().filter(|r| !previous.contains(&r.id)).count(),
            1
        );
    }
}

#[test]
fn interplanetary_crossing_preserves_loadout_and_has_a_return_route() {
    let mut app = App::new(World::generate(crate::world::MapKind::City, 2049));
    app.start();
    app.network.ensure(&app.world);
    app.network.active = true;
    let source = app.world.clone();
    let route = app.network.routes(&source)[0];
    app.network
        .remember(World::generate(route.target.kind, route.target.seed).into());
    app.flight.position = route.aperture.center + route.aperture.normal() * 12.0;
    app.flight.speed = 145.0;
    app.mission.bombs = 4;
    app.mission.score = 321;
    app.flight.follow = app.flight.position;
    app.flight.yaw = route.aperture.yaw + std::f32::consts::PI;
    app.flight.camera_yaw = -app.flight.yaw;
    app.flight.view = 0;
    app.flight.weights = [1.0, 0.0, 0.0];
    assert!(!app.cross_world(route.aperture.center + route.aperture.normal() * 30.0));
    assert_eq!(app.world.kind, source.kind);
    for _ in 0..60 {
        let before = app.flight.position;
        app.flight.step(
            1.0 / 60.0,
            &Controls {
                manual: true,
                ..Default::default()
            },
            &source,
        );
        if app.cross_world(before) {
            break;
        }
    }
    assert_eq!(app.world.kind, route.target.kind);
    assert_eq!(app.mission.bombs, 4);
    assert_eq!(app.mission.score, 321);
    assert_eq!(app.flight.speed, app.flight.vehicle.speed());
    let back = app
        .network
        .routes(&app.world)
        .into_iter()
        .find(|r| r.id == route.id)
        .unwrap();
    assert_eq!(back.target.kind, source.kind);
    assert!(app.arrive(back));
    assert_eq!(app.world.kind, source.kind);
    assert!(std::sync::Arc::ptr_eq(&app.world, &source));
}

#[test]
fn direction_double_taps_require_release_and_ignore_hold_repeat() {
    use crate::keys::*;
    let mut app = App::new(World::generate(crate::world::MapKind::Space, 1999));
    app.start();
    app.auto = false;
    app.flight.position.z = 700.0;
    app.key(UP, 1);
    for _ in 0..180 {
        app.key(UP, 2);
        app.step(1.0 / 60.0, None);
    }
    assert!(app.flight.pitch.abs() < 0.6);
    app.key(UP, 0);
    app.direction_tap = Some((UP, Instant::now() - Duration::from_secs(1)));
    app.key(UP, 1);
    app.key(UP, 0);
    app.key(UP, 1);
    app.key(UP, 0);
    let mut rotation = 0.0;
    for _ in 0..360 {
        let before = app.flight.pitch;
        app.step(1.0 / 60.0, None);
        rotation += crate::flight::angle(app.flight.pitch - before);
    }
    assert!(rotation > 5.5);
    app.key(LEFT, 1);
    app.key(LEFT, 0);
    app.key(RIGHT, 1);
    app.key(RIGHT, 0);
    app.step(0.1, None);
    assert_eq!(app.flight.spin, 0.0);
    app.key(RIGHT, 1);
    app.key(RIGHT, 0);
    app.step(0.1, None);
    assert!(app.flight.spin > 0.0);
}

#[test]
fn chart_zooms_pans_marks_and_navigates_without_changing_the_view() {
    use crate::keys::*;
    let mut app = App::new(World::generate(crate::world::MapKind::Space, 1999));
    app.start();
    app.auto = false;
    let press = |app: &mut App, key| {
        app.key(key, 1);
        app.key(key, 0);
    };
    press(&mut app, CHART);
    let before = app.flight.position;
    let view = app.flight.view;
    for _ in 0..12 {
        press(&mut app, WEATHER);
    }
    app.chart.prepare(&app.world, app.flight.position);
    assert_eq!(app.chart.zoom, 9);
    assert_eq!(app.chart.span, 8.0);
    let center = app.chart.center;
    app.key(RIGHT, 1);
    app.step(0.1, None);
    app.key(RIGHT, 0);
    app.chart.prepare(&app.world, app.flight.position);
    assert!(app.chart.center.x > center.x);
    assert_eq!(app.flight.position, before);
    app.chart.focus = Some(app.flight.position.truncate() + glam::vec2(0.0, -600.0));
    press(&mut app, VIEW);
    assert_eq!(app.flight.view, view);
    let target = app.navigation.target.unwrap();
    assert!(target.position.y < app.flight.position.y);
    press(&mut app, ENTER);
    assert_eq!(app.page, Page::Playing);
    assert!(app.navigation.active);
    app.flight.yaw = 0.0;
    let distance = app.flight.position.distance(target.position);
    for _ in 0..120 {
        app.step(1.0 / 60.0, None);
    }
    assert!(app.flight.position.distance(target.position) < distance);
    press(&mut app, LEFT);
    assert!(!app.navigation.active);
    assert!(app.navigation.target.is_some());
}
