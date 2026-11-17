use super::*;

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
    app.key(35, 1);
    app.key(35, 0);
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
    app.step(
        0.05,
        Some(Controls {
            manual: true,
            ..Default::default()
        }),
    );
    assert!(app.flight.position.distance(exit.center) < 100.0);
    assert_eq!(app.mission.collected, 0);
    assert!(!app.mission.pickups[0].collected);
    assert!(app.sounds.iter().any(|s| s == crate::audio::Sound::Warp));
    assert!(app.flight.position != Vec3::ZERO);
}

#[test]
fn wormhole_hotkey_replaces_two_links_on_every_map_without_pausing() {
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
        app.key(35, 1);
        app.key(35, 0);
        assert_eq!(app.page, Page::Playing);
        let routes = app.network.routes(&app.world);
        assert_eq!(routes.len(), 2);
        assert!(routes.iter().all(|r| !before.contains(&r.id)));
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
        app.key(35, 1);
        let routes = app.network.routes(&app.world);
        assert_eq!(routes.len(), 2);
        assert!(routes.iter().all(|r| !previous.contains(&r.id)));
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
    assert!(app.cross_world(route.aperture.center + route.aperture.normal() * 30.0));
    assert_eq!(app.world.kind, route.target.kind);
    assert_eq!(app.mission.bombs, 4);
    assert_eq!(app.mission.score, 321);
    assert_eq!(app.flight.speed, 145.0);
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
