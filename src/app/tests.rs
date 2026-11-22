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
        std::thread::sleep(Duration::from_millis(1));
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
        if map == crate::world::MapKind::Space {
            for body in crate::world::orbit::Body::ALL {
                assert!(body.altitude(portal.center) >= 2199.0);
            }
        } else {
            let offset = (portal.center - app.flight.position).truncate();
            let forward = app.flight.axes()[1].truncate().normalize();
            assert!(offset.length() >= 679.9);
            assert!(offset.normalize().dot(forward) > 0.999);
            assert!(
                routes[0]
                    .aperture
                    .center
                    .distance(routes[1].aperture.center)
                    >= crate::world::portal::MIN_SPACING
            );
        }
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
fn pitch_gesture_requires_three_seconds_then_two_fresh_presses() {
    use crate::keys::*;
    let world = World::generate(crate::world::MapKind::Moon, 1999);
    let mut app = App::new(world);
    app.start();
    app.auto = false;
    app.flight.position.z = 1800.0;
    let tap = |app: &mut App, code| {
        app.key(code, 1);
        app.key(code, 0);
    };
    for code in [UP, DOWN] {
        for (hold, interval, expected) in
            [(2.99, 0.15, false), (3.01, 0.31, false), (3.01, 0.15, true)]
        {
            app.release_keys();
            app.key(code, 1);
            app.flight.time += hold;
            app.key(code, 2);
            assert!(app.flight.clone().loop_toward(1.0));
            app.key(code, 0);
            app.flight.time += interval * 0.5;
            tap(&mut app, code);
            assert!(app.flight.clone().loop_toward(1.0));
            app.flight.time += interval * 0.5;
            tap(&mut app, code);
            assert_eq!(!app.flight.clone().loop_toward(1.0), expected);
            app.flight = crate::flight::Flight::new(&app.world);
            app.flight.position.z = 1800.0;
        }
    }
    for interrupt in [DOWN, PAUSE, CHART, HELP, SETTINGS] {
        app.page = Page::Playing;
        app.release_keys();
        app.key(UP, 1);
        app.flight.time += 3.01;
        app.key(UP, 0);
        tap(&mut app, interrupt);
        app.page = Page::Playing;
        tap(&mut app, UP);
        tap(&mut app, UP);
        assert!(app.flight.clone().loop_toward(1.0));
    }
    for code in [LEFT, RIGHT] {
        tap(&mut app, code);
        tap(&mut app, code);
        app.step(0.1, None);
        assert_eq!(app.flight.spin, 0.0);
    }
    for (code, sign) in [(ROLL_LEFT, -1.0), (ROLL_RIGHT, 1.0)] {
        tap(&mut app, code);
        app.step(0.1, None);
        assert!(app.flight.spin * sign > 0.0);
        for _ in 0..12 {
            app.step(0.1, None);
        }
    }
}

#[test]
fn camera_keys_leave_aircraft_heading_unchanged_and_return_to_center() {
    use crate::keys::*;
    let mut app = App::new(World::generate(crate::world::MapKind::Moon, 1999));
    app.start();
    app.auto = false;
    app.flight.position.z = 900.0;
    app.flight.follow = app.flight.position;
    let axes = app.flight.axes();
    app.key(LOOK_UP, 1);
    app.key(LOOK_RIGHT, 1);
    for _ in 0..60 {
        app.step(1.0 / 30.0, None);
    }
    assert!(app.flight.camera_look.length() <= 30.0_f32.to_radians() + 0.00001);
    assert!(app.flight.camera_look.length() > 29.9_f32.to_radians());
    for (before, after) in axes.into_iter().zip(app.flight.axes()) {
        assert!(before.distance(after) < 0.0001);
    }
    app.key(LOOK_UP, 0);
    app.key(LOOK_RIGHT, 0);
    for _ in 0..60 {
        app.step(1.0 / 30.0, None);
    }
    assert!(app.flight.camera_look.length() < 0.0001);
}

#[test]
fn hover_key_toggles_and_holds_altitude() {
    use crate::keys::*;
    let mut app = App::new(World::generate(crate::world::MapKind::Moon, 1999));
    app.start();
    app.auto = false;
    app.flight.position.z = 900.0;
    // Alt 进入悬停，速度归零
    app.key(HOVER, 1);
    app.key(HOVER, 0);
    assert!(app.flight.hover);
    for _ in 0..40 {
        app.step(0.1, None);
    }
    assert!(app.flight.speed < 0.01);
    let held = app.flight.position;
    for _ in 0..20 {
        app.step(0.1, None);
    }
    assert!((app.flight.position.z - held.z).abs() < 0.5);
    assert!((app.flight.position - held).truncate().length() < 0.5);
    // 再次按 Alt 退出悬停，速度恢复
    app.key(HOVER, 1);
    app.key(HOVER, 0);
    assert!(!app.flight.hover);
    for _ in 0..20 {
        app.step(0.1, None);
    }
    assert!(app.flight.speed > 50.0);
}

#[test]
fn hover_exits_on_boost_but_keeps_direction_keys() {
    use crate::keys::*;
    let mut app = App::new(World::generate(crate::world::MapKind::Moon, 1999));
    app.start();
    app.auto = false;
    app.flight.position.z = 900.0;
    app.key(HOVER, 1);
    app.key(HOVER, 0);
    assert!(app.flight.hover);
    // 方向键（左右转向、上下俯仰）只调整姿态，保持悬停
    for key in [LEFT, RIGHT, UP, DOWN] {
        app.key(key, 1);
        for _ in 0..10 {
            app.step(0.1, None);
        }
        assert!(app.flight.hover, "方向键不应退出悬停");
        app.key(key, 0);
    }
    // 按加速视为开始移动，自动退出悬停并恢复速度
    app.key(BOOST, 1);
    app.step(0.1, None);
    assert!(!app.flight.hover, "加速应退出悬停");
    app.key(BOOST, 0);
    for _ in 0..20 {
        app.step(0.1, None);
    }
    assert!(app.flight.speed > 50.0, "退出悬停后应恢复速度");
}

#[test]
fn hover_pitch_holds_without_recentering() {
    use crate::keys::*;
    let mut app = App::new(World::generate(crate::world::MapKind::Moon, 1999));
    app.start();
    app.auto = false;
    app.flight.position.z = 2000.0;
    app.key(HOVER, 1);
    app.key(HOVER, 0);
    assert!(app.flight.hover);
    app.key(UP, 1);
    for _ in 0..20 {
        app.step(0.1, None);
    }
    app.key(UP, 0);
    let pitched = app.flight.pitch;
    assert!(pitched > 0.2, "按上应抬头");
    let altitude = app.flight.position.z;
    for _ in 0..40 {
        app.step(0.1, None);
    }
    assert!(
        (app.flight.pitch - pitched).abs() < 0.05,
        "松开后应保持俯仰不回正"
    );
    assert!(
        (app.flight.position.z - altitude).abs() < 0.5,
        "悬停俯仰时高度应保持"
    );
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
    assert_eq!(app.chart.zoom, 6);
    assert_eq!(app.chart.span, 512.0);
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

#[test]
fn perf_key_cycles_layouts_without_resetting_samples_or_conflicting_with_sound() {
    let mut app = App::new(World::new(7));
    let press = |app: &mut App, key| {
        app.key(key, 1);
        app.key(key, 0);
    };
    assert_eq!(app.perf_panel, PerfPanel::Off);
    press(&mut app, crate::keys::PERF);
    assert_eq!(app.perf_panel, PerfPanel::Full);
    app.step(0.3, None);
    let count = app.perf.frames.len();
    press(&mut app, crate::keys::SOUND);
    assert_eq!(app.perf_panel, PerfPanel::Full);
    for panel in [PerfPanel::Half, PerfPanel::Mini, PerfPanel::Off] {
        press(&mut app, crate::keys::PERF);
        assert_eq!(app.perf_panel, panel);
        assert_eq!(app.perf.frames.len(), count);
        assert_eq!(app.perf.frames.back().unwrap().ms, 300.0);
    }
    press(&mut app, crate::keys::PERF);
    assert_eq!(app.perf_panel, PerfPanel::Full);
    assert!(app.perf.frames.is_empty());
    app.key(crate::keys::PERF, 2);
    assert_eq!(app.perf_panel, PerfPanel::Full);
}

#[test]
fn perf_stats_track_frames_and_triangles() {
    let mut app = App::new(World::new(7));
    app.record_path = None;
    app.start();
    app.perf_panel = PerfPanel::Full;
    for _ in 0..70 {
        app.step(1.0 / 30.0, None);
        app.draw();
    }
    assert!(app.renderer.triangles > 0);
    assert!(app.perf.render_ms > 0.0);
    assert!((app.perf.fps - 30.0).abs() < 5.0);
    assert!(!app.perf.frames.is_empty());
}

#[test]
fn overhead_view_controls_only_heading() {
    let mut app = App::new(World::generate(crate::world::MapKind::Desert, 1999));
    app.start();
    app.auto = false;
    app.navigation.active = false;
    app.flight.space = false;
    app.flight.surface = None;
    app.flight.pitch = 0.4;
    app.flight.weights = [1.0, 0.0, 0.0];
    app.flight.position = glam::vec3(0.0, 0.0, 3000.0);
    app.flight.follow = app.flight.position;
    let (z, pitch, yaw) = (app.flight.position.z, app.flight.pitch, app.flight.yaw);
    for _ in 0..40 {
        app.flight.weights = [1.0, 0.0, 0.0];
        app.step(
            1.0 / 30.0,
            Some(Controls {
                manual: true,
                aerobatic: true,
                turn: 0.5,
                climb: 1.0,
                bank: Some(0.5),
                ..Default::default()
            }),
        );
    }
    assert!((app.flight.yaw - yaw).abs() > 0.05, "航向应随方向输入改变");
    assert!(
        (app.flight.position.z - z).abs() < 0.6,
        "俯瞰视角不应改变高度"
    );
    assert!(
        (app.flight.pitch - pitch).abs() < 0.01,
        "俯瞰视角不应改变俯仰"
    );
}

#[test]
fn overhead_navigation_keeps_altitude_control() {
    use crate::world::{MapKind, network::MapKey};
    let mut app = App::new(World::generate(MapKind::Desert, 1999));
    app.start();
    app.auto = false;
    app.record_path = None;
    app.flight.view = 0;
    app.flight.weights = [1.0, 0.0, 0.0];
    app.flight.position = glam::vec3(0.0, 0.0, 500.0);
    app.flight.follow = app.flight.position;
    app.flight.yaw = 0.0;
    app.navigation.target = Some(super::navigation::Waypoint {
        map: MapKey::of(&app.world),
        position: glam::vec3(0.0, -600.0, 700.0),
        portal: None,
        body: None,
    });
    app.navigation.active = true;
    for _ in 0..90 {
        app.step(1.0 / 30.0, None);
    }
    assert!(app.flight.position.z > 600.0);
}

#[test]
fn cloud_setting_survives_world_changes_and_menu_navigation() {
    let mut app = App::new(World::generate(crate::world::MapKind::Islands, 7));
    app.settings_path = None;
    app.open_settings();
    app.selection = 1;
    app.activate_setting();
    app.selection = 1;
    app.flight.dynamic_clouds = true;
    app.key(crate::keys::ENTER, 1);
    app.key(crate::keys::ENTER, 0);
    assert!(!app.flight.dynamic_clouds);
    app.flight.change_world(&app.world);
    assert!(!app.flight.dynamic_clouds);
    app.selection = 0;
    app.key(crate::keys::UP, 1);
    app.key(crate::keys::UP, 0);
    assert_eq!(app.selection, 6);
    app.key(crate::keys::ENTER, 1);
    app.key(crate::keys::ENTER, 0);
    assert_eq!(app.page, Page::Settings);
    assert_eq!(app.settings_section, None);
    assert_eq!(app.selection, 1);
    app.back();
    assert_eq!(app.page, Page::Menu);
    let settings: super::settings::Settings = toml::from_str("dynamic_clouds = false").unwrap();
    assert!(!settings.dynamic_clouds);
}

#[test]
fn cloud_pixel_texture_defaults_on_and_persists_across_maps() {
    let defaults: super::settings::Settings = toml::from_str("").unwrap();
    assert!(defaults.cloud_pixel_texture);
    let mut app = App::new(World::generate(crate::world::MapKind::Islands, 7));
    let path =
        std::env::temp_dir().join(format!("cloud-pixel-settings-{}.toml", std::process::id()));
    app.settings_path = Some(path.clone());
    app.flight.cloud_pixel_texture = true;
    app.open_settings();
    app.selection = 1;
    app.activate_setting();
    app.selection = 2;
    app.activate_setting();
    assert!(!app.flight.cloud_pixel_texture);
    assert!(!super::settings::Settings::load(Some(&path)).cloud_pixel_texture);
    app.flight.change_world(&app.world);
    assert!(!app.flight.cloud_pixel_texture);
    app.activate_setting();
    assert!(super::settings::Settings::load(Some(&path)).cloud_pixel_texture);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn smooth_sky_gradient_defaults_on_and_persists_across_maps() {
    let defaults: super::settings::Settings = toml::from_str("").unwrap();
    assert!(defaults.smooth_sky_gradient);
    let mut app = App::new(World::generate(crate::world::MapKind::Islands, 7));
    let path =
        std::env::temp_dir().join(format!("smooth-sky-settings-{}.toml", std::process::id()));
    app.settings_path = Some(path.clone());
    app.flight.smooth_sky_gradient = true;
    app.open_settings();
    app.selection = 1;
    app.activate_setting();
    app.selection = 4;
    app.activate_setting();
    assert!(!app.flight.smooth_sky_gradient);
    assert!(!super::settings::Settings::load(Some(&path)).smooth_sky_gradient);
    app.flight.change_world(&app.world);
    assert!(!app.flight.smooth_sky_gradient);
    app.activate_setting();
    assert!(super::settings::Settings::load(Some(&path)).smooth_sky_gradient);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn daylight_pauses_freezes_and_keeps_phase_across_maps() {
    let mut app = App::new(World::generate(crate::world::MapKind::Islands, 7));
    app.start();
    app.auto = false;
    app.flight.day_time = 100.0;
    app.flight.day_cycle = 3;
    app.flight.dynamic_lighting = true;
    app.step(0.1, None);
    assert!(app.flight.day_time > 100.0);
    let phase = app.flight.day_time;
    app.page = Page::Paused;
    app.step(0.1, None);
    assert_eq!(app.flight.day_time, phase);
    app.flight.dynamic_lighting = false;
    app.page = Page::Playing;
    app.step(0.1, None);
    assert_eq!(app.flight.day_time, phase);
    app.flight.change_world(&app.world);
    assert_eq!(app.flight.day_time, phase);
    assert_eq!(app.flight.day_cycle, 3);
    assert!(!app.flight.dynamic_lighting);
    app.key(crate::keys::TIME, 1);
    app.key(crate::keys::TIME, 0);
    assert_eq!(app.theme, 1);
    assert_eq!(app.flight.day_time, 0.0);
    app.flight.dynamic_lighting = true;
    app.flight.day_time = crate::flight::DAY_SECONDS - 0.05;
    app.step(0.1, None);
    assert_eq!(app.flight.day_cycle, 4);
    assert!(app.flight.day_time < 0.1);
    let settings: super::settings::Settings = toml::from_str("dynamic_lighting = false").unwrap();
    assert!(!settings.dynamic_lighting);
}

#[test]
fn performance_samples_unclamped_time_only_while_visible() {
    let mut app = App::new(World::generate(crate::world::MapKind::Moon, 7));
    app.step(0.3, None);
    assert!(app.perf.frames.is_empty());
    app.key(crate::keys::PERF, 1);
    app.key(crate::keys::PERF, 0);
    let before = app.flight.time;
    app.step(0.3, None);
    assert_eq!(app.perf.frames.back().unwrap().ms, 300.0);
    assert!((app.flight.time - before - 0.1).abs() < 0.0001);
    for panel in [PerfPanel::Half, PerfPanel::Mini, PerfPanel::Off] {
        app.key(crate::keys::PERF, 1);
        app.key(crate::keys::PERF, 0);
        assert_eq!(app.perf_panel, panel);
        let count = app.perf.frames.len();
        app.step(0.1, None);
        assert_eq!(
            app.perf.frames.len(),
            count + usize::from(panel != PerfPanel::Off)
        );
    }
}

#[test]
fn settings_categories_route_controls_and_persist_portal_preview() {
    use crate::keys::*;
    let tap = |app: &mut App, code| {
        app.key(code, 1);
        app.key(code, 0);
    };
    let mut app = App::new(World::generate(crate::world::MapKind::Space, 7));
    let path = std::env::temp_dir().join(format!("flight-settings-{}.toml", std::process::id()));
    app.settings_path = Some(path.clone());
    app.page = Page::Paused;
    app.selection = 3;
    tap(&mut app, SETTINGS);
    assert_eq!(app.settings_section, None);
    tap(&mut app, LEFT);
    assert_eq!(app.settings_section, None);
    assert!(!path.exists());
    tap(&mut app, DOWN);
    tap(&mut app, ENTER);
    assert_eq!(app.settings_section, Some(SettingsSection::Advanced));
    assert_eq!(app.selection, 0);
    app.selection = 5;
    app.flight.portal_preview = true;
    tap(&mut app, ENTER);
    assert!(!app.flight.portal_preview);
    assert!(!super::settings::Settings::load(Some(&path)).portal_preview);
    app.flight.change_world(&app.world);
    assert!(!app.flight.portal_preview);
    tap(&mut app, ESC);
    assert_eq!(app.page, Page::Settings);
    assert_eq!(app.settings_section, None);
    assert_eq!(app.selection, 1);
    tap(&mut app, UP);
    tap(&mut app, ENTER);
    assert_eq!(app.settings_section, Some(SettingsSection::General));
    let volume = app.audio_settings.volume;
    app.click(192, 100);
    assert_eq!(app.audio_settings.volume, volume.saturating_sub(1));
    app.click(272, 100);
    assert_eq!(app.audio_settings.volume, volume);
    assert!(!app.flight.portal_preview);
    app.click(34, 28);
    assert_eq!(app.settings_section, None);
    assert_eq!(app.selection, 0);
    app.click(120, 83);
    assert_eq!(app.settings_section, Some(SettingsSection::Advanced));
    app.click(120, 115);
    assert_eq!(app.settings_section, None);
    assert_eq!(app.selection, 1);
    tap(&mut app, ESC);
    assert_eq!(app.page, Page::Paused);
    assert_eq!(app.selection, 3);
    tap(&mut app, SETTINGS);
    assert_eq!(app.settings_section, None);
    assert_eq!(app.selection, 0);
    tap(&mut app, ENTER);
    tap(&mut app, SETTINGS);
    assert_eq!(app.page, Page::Paused);
    assert_eq!(app.selection, 3);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn first_portal_preview_loads_alongside_orbital_resources() {
    use crate::world::{
        MapKind,
        network::{MapKey, Network},
    };
    let mut app = App::new(World::generate(MapKind::Moon, MapKind::Moon.seed()));
    app.network = Network::new(7109);
    app.start();
    app.auto = false;
    app.flight.set_demo(false);
    app.network.ensure(&app.world);
    let route = app.network.routes(&app.world)[0];
    app.flight.position = route.aperture.center + route.aperture.normal() * 200.0;
    app.flight.follow = app.flight.position;
    app.flight.yaw = route.aperture.yaw + std::f32::consts::PI;
    app.flight.weights = [0.0, 1.0, 0.0];
    assert!(app.flight.position.z > super::orbit::ARM_ALTITUDE - 100.0);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.network.get(route.target).is_none() && Instant::now() < deadline {
        app.prepare_scene();
        app.network.poll();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(app.network.get(route.target).is_some());
    let space = MapKey {
        kind: MapKind::Space,
        seed: MapKind::Space.seed(),
    };
    for _ in 0..10 {
        app.prepare_scene();
        assert!(app.network.get(space).is_some());
        assert!(app.network.get(route.target).is_some());
        assert!(!app.network.loading());
    }
}

#[test]
fn menu_demo_travel_keeps_the_players_map_and_weather() {
    use crate::content::weather::{Weather, WeatherState};
    use crate::world::{MapKind, network::MapKey};
    let mut app = App::new(World::generate(MapKind::Desert, MapKind::Desert.seed()));
    app.home();
    let chosen = MapKey::of(&app.world);
    app.flight.weather = WeatherState::new(Weather::Storm, chosen.kind, chosen.seed);
    app.network.ensure(&app.world);
    let route = app.network.routes(&app.world)[0];
    app.network
        .remember(World::generate(route.target.kind, route.target.seed).into());
    assert!(app.arrive(route));
    assert_eq!(app.world.kind, route.target.kind);
    // 菜单仍显示玩家的选择，演示所到地图不影响任务
    assert_eq!(app.menu_map(), chosen.kind);
    assert_eq!(app.menu_weather().target, Weather::Storm);
    app.selection = START_ROW;
    app.key(crate::keys::ENTER, 1);
    app.key(crate::keys::ENTER, 0);
    assert_eq!(MapKey::of(&app.world), chosen);
    assert_eq!(app.flight.weather.target, Weather::Storm);
    assert_eq!(app.page, Page::Playing);
}

#[test]
fn records_stay_with_the_starting_map_after_wormhole_travel() {
    let mut app = App::new(World::generate(crate::world::MapKind::City, 2049));
    app.record_path = None;
    app.mode = Mode::Combat;
    app.start();
    let key = app.record_key();
    app.network.ensure(&app.world);
    let route = app.network.routes(&app.world)[0];
    app.network
        .remember(World::generate(route.target.kind, route.target.seed).into());
    assert!(app.arrive(route));
    assert_ne!(app.record_key(), key);
    app.mission.score = 5000;
    app.save_record();
    assert!(app.records.iter().any(|(k, s)| k == &key && *s == 5000));
    assert!(!app.records.iter().any(|(k, _)| k == &app.record_key()));
}

#[test]
fn space_bodies_block_the_aircraft_in_every_mode() {
    use crate::world::{MapKind, orbit::Body};
    for mode in [Mode::Race, Mode::Coins, Mode::Combat] {
        let mut app = App::new(World::generate(MapKind::Space, MapKind::Space.seed()));
        app.mode = mode;
        app.start();
        app.mission.countdown = 0.0;
        let body = Body::Moon;
        app.flight.position = body.center() + glam::Vec3::Z * (body.radius() * 0.5);
        app.step(1.0 / 30.0, None);
        assert!(
            app.flight.position.distance(body.center()) >= body.radius() + 419.0,
            "{mode:?}"
        );
    }
}

#[test]
fn orbital_ascent_climbs_steadily_instead_of_shooting_up() {
    let mut app = App::new(World::generate(crate::world::MapKind::Islands, 7));
    app.start();
    app.key(crate::keys::UP, 1);
    app.key(crate::keys::BOOST, 1);
    let mut fastest: f32 = 0.0;
    for _ in 0..30 * 12 {
        let before = app.flight.position.z;
        app.step(1.0 / 30.0, None);
        if app.flight.orbital_thrust > 0.99 {
            fastest = fastest.max((app.flight.position.z - before) * 30.0);
        }
    }
    // 按住爬升加速 12 秒后仍在大气层内，升空全程约二十多秒
    let z = app.flight.position.z;
    assert!((700.0..1500.0).contains(&z), "altitude {z}");
    assert!(app.flight.speed > 300.0);
    assert!(fastest <= 100.0, "vertical {fastest}");
}

#[test]
fn lightning_in_a_storm_brings_thunder_and_storm_ambience() {
    use crate::content::weather::{Weather, WeatherState};
    let mut app = App::new(World::generate(crate::world::MapKind::Wetlands, 31));
    app.start();
    app.flight.weather = WeatherState::new(Weather::Storm, app.world.kind, app.world.seed);
    let mut thunder = false;
    let mut frame = app.audio_frame();
    for _ in 0..30 * 60 {
        app.step(1.0 / 30.0, None);
        frame = app.audio_frame();
        thunder |= frame
            .events
            .iter()
            .any(|s| s == crate::audio::Sound::Thunder);
    }
    assert!(thunder);
    assert_eq!(frame.weather, Weather::Storm);
    assert!(frame.rain > 0.9 && frame.wind > 0.5);
}

#[test]
fn sound_volume_and_hud_survive_a_restart() {
    let dir = std::env::temp_dir().join(format!("flight-club-settings-{}", std::process::id()));
    let path = dir.join("settings-v1.toml");
    let mut app = App::new(World::new(7));
    app.settings_path = Some(path.clone());
    app.key(crate::keys::SOUND, 1);
    app.key(crate::keys::SOUND, 0);
    app.key(crate::keys::HUD, 1);
    app.key(crate::keys::HUD, 0);
    app.settings_section = Some(SettingsSection::General);
    app.selection = 3;
    app.adjust_setting(-1);
    app.adjust_setting(-1);
    let saved = settings::Settings::load(Some(&path));
    assert_eq!(saved.audio().profile, 1);
    assert_eq!(saved.audio().volume, 5);
    assert!(!saved.hud);
    assert_eq!(saved.portal_labels, app.portal_labels);
    // 原子写入后不留下临时文件
    assert!(!path.with_extension("tmp").exists());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn opening_settings_while_loading_is_not_interrupted() {
    use crate::world::MapKind;
    let mut app = App::new(World::new(7));
    app.loader.start(MapKind::Alpine, MapKind::Alpine.seed());
    app.selection = START_ROW;
    app.key(crate::keys::ENTER, 1);
    app.key(crate::keys::ENTER, 0);
    assert_eq!(app.page, Page::Loading);
    app.key(crate::keys::SETTINGS, 1);
    app.key(crate::keys::SETTINGS, 0);
    assert_eq!(app.page, Page::Settings);
    let deadline = Instant::now() + Duration::from_secs(60);
    while app.loader.pending.is_some() && Instant::now() < deadline {
        app.step(1.0 / 30.0, None);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.world.kind, MapKind::Alpine);
    // 加载完成不打断设置页，关闭后进入游戏而不是卡在加载页
    assert_eq!(app.page, Page::Settings);
    app.close_settings();
    assert_eq!(app.page, Page::Playing);
}

#[test]
fn chart_preloads_the_wormhole_it_focuses() {
    let mut app = App::new(World::generate(crate::world::MapKind::City, 2049));
    app.start();
    app.key(crate::keys::CHART, 1);
    app.key(crate::keys::CHART, 0);
    assert_eq!(app.page, Page::Chart);
    let routes = app.network.routes(&app.world);
    assert!(routes.len() >= 2);
    for expected in [0, 1] {
        app.key(crate::keys::PORTAL, 1);
        app.key(crate::keys::PORTAL, 0);
        let focused = routes[app.chart.selected % routes.len()];
        assert_eq!(app.chart.selected, expected);
        assert_eq!(
            app.chart.focus,
            Some(focused.aperture.center.truncate()),
            "预加载的路线应与聚焦的虫洞一致"
        );
    }
}

#[test]
fn held_keys_come_back_through_autorepeat_after_a_reset() {
    let mut app = App::new(World::new(7));
    app.mode = Mode::Combat;
    app.start();
    app.key(crate::keys::FIRE, 1);
    app.keys.clear();
    app.key(crate::keys::FIRE, 2);
    assert!(app.keys.contains(&crate::keys::FIRE));
    app.key(crate::keys::FIRE, 0);
    assert!(!app.keys.contains(&crate::keys::FIRE));
}

#[test]
fn returning_home_clears_orbit_flight_state() {
    let mut app = App::new(World::new(7));
    app.start();
    app.flight.orbit_flight = true;
    app.flight.orbital_thrust = 0.8;
    app.home();
    assert!(!app.flight.orbit_flight);
    assert_eq!(app.flight.orbital_thrust, 0.0);
}
