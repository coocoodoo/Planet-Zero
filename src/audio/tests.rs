use super::*;

#[test]
fn scores_have_full_arrangements_and_valid_loop_points() {
    for score in scores() {
        assert_eq!(score.steps.len() % 128, 0, "{}", score.title);
        assert_eq!(score.chords.len() * 16, score.steps.len());
        assert!(score.loop_start < score.steps.len());
        assert!(!score.cues.is_empty());
        if score.cues.iter().all(|c| c.rotates()) {
            assert!(score.seconds() > 60.0, "{}", score.title);
        }
        for tone in score.steps.iter().flatten() {
            assert!(tone.hz.is_finite() && tone.hz > 20.0 && tone.hz < 12000.0);
            assert!(tone.hold > 0.0 && tone.gain <= 0.5);
        }
    }
}

#[test]
fn every_scene_and_map_has_matching_music() {
    use crate::world::MapKind;
    for cue in Cue::ALL {
        let list = playlist(cue, MapKind::Islands);
        assert!(!list.is_empty(), "{cue:?}");
        if cue.rotates() && cue != Cue::Flight {
            assert!(list.len() >= 2, "{cue:?} 至少需要两首轮换");
        }
    }
    for map in MapKind::ALL {
        let list = playlist(Cue::Flight, map);
        assert!(!list.is_empty(), "{map:?} 需要地图主题");
        assert!(list.iter().all(|i| scores()[*i].maps.contains(&map)));
    }
    // 太空与地表的巡航曲互不混用
    let space = playlist(Cue::Flight, MapKind::Space);
    assert!(
        playlist(Cue::Flight, MapKind::Desert)
            .iter()
            .all(|i| !space.contains(i))
    );
}

#[test]
fn mixing_is_bounded_and_muting_preserves_the_music_clock() {
    let mut m = Mixer::new(24000);
    m.start_cue(Cue::Boss);
    let mut f = Frame {
        cue: Cue::Boss,
        engine: 1.0,
        boost: 1.0,
        danger: 1.0,
        overdrive: true,
        ..Frame::default()
    };
    let mut buffer = vec![0.0; 4800];
    for _ in 0..30 {
        f.events.push(Sound::Nova);
        f.events.push(Sound::Gun);
        f.events.push(Sound::Explosion(true));
        m.update(f);
        f.events = Events::default();
        m.render(&mut buffer);
        assert!(buffer.iter().all(|v| v.is_finite() && v.abs() <= 0.881));
    }
    let before = m.step();
    f.settings.profile = 3;
    m.update(f);
    for _ in 0..10 {
        m.render(&mut buffer);
    }
    assert!(buffer.iter().all(|v| v.abs() < 0.0001));
    assert!(m.step() > before);
    assert!(m.peak_voices <= 64);
    f.settings.profile = 0;
    m.update(f);
    m.render(&mut buffer);
    assert!(buffer.iter().any(|v| v.abs() > 0.01));
}

#[test]
fn application_routes_results_pause_and_events_to_audio() {
    use crate::{
        app::{App, Page},
        gameplay::Mode,
        world::World,
    };
    let mut app = App::new(World::new(7));
    assert_eq!(app.audio_frame().cue, Cue::Menu);
    app.mode = Mode::Combat;
    app.start();
    app.mission.countdown = 0.0;
    app.mission.sounds.push(Sound::Upgrade);
    let f = app.audio_frame();
    assert_eq!(f.cue, Cue::Combat);
    assert!(f.events.iter().any(|s| s == Sound::Upgrade));
    assert_eq!(app.audio_frame().events.iter().count(), 0);
    app.page = Page::Paused;
    assert!(app.audio_frame().paused);
    app.key(crate::keys::SETTINGS, 1);
    app.key(crate::keys::SETTINGS, 0);
    app.key(crate::keys::ENTER, 1);
    app.key(crate::keys::ENTER, 0);
    app.selection = 3;
    app.key(crate::keys::LEFT, 1);
    app.key(crate::keys::LEFT, 0);
    assert_eq!(app.audio_settings.volume, 6);
    app.key(crate::keys::RIGHT, 1);
    app.key(crate::keys::RIGHT, 0);
    app.key(crate::keys::SETTINGS, 1);
    app.key(crate::keys::SETTINGS, 0);
    assert_eq!(app.audio_settings.volume, 7);
    assert_eq!(app.page, Page::Paused);
    app.page = Page::Results;
    app.mission.outcome = Some(true);
    assert_eq!(app.audio_frame().cue, Cue::Victory);
    app.mission.outcome = Some(false);
    assert_eq!(app.audio_frame().cue, Cue::Defeat);
    app.page = Page::Playing;
    app.mode = Mode::Race;
    assert_eq!(app.audio_frame().cue, Cue::Race);
    app.mode = Mode::Coins;
    assert_eq!(app.audio_frame().cue, Cue::Coins);
    app.mode = Mode::Free;
    app.flight.orbital_thrust = 0.9;
    app.flight.position.z = 400.0;
    let f = app.audio_frame();
    assert_eq!(f.cue, Cue::Flight, "常规飞行高度不叠加升空声部");
    assert_eq!(f.ascent, 0.0);
    assert_eq!(f.riser, 0.0);
    app.flight.position.z = crate::app::ASCENT_MUSIC_ALTITUDE + 50.0;
    let f = app.audio_frame();
    assert_eq!(f.cue, Cue::Flight, "升空在探索配乐上叠加声部，不换曲");
    assert!(f.ascent > 0.8 && f.riser > 0.9);
    assert_eq!(f.map, app.world.kind);
    app.flight.orbital_thrust = 0.0;
    assert_eq!(app.audio_frame().cue, Cue::Flight);
}

#[test]
fn actual_gameplay_emits_weapon_pickup_and_countdown_sounds() {
    use crate::{
        app::App,
        gameplay::{Mode, Pickup, PickupKind, Weapon},
        world::World,
    };
    let mut app = App::new(World::new(7));
    app.mode = Mode::Combat;
    app.start();
    app.mission.countdown = 1.02;
    app.step(0.05, None);
    assert!(
        app.audio_frame()
            .events
            .iter()
            .any(|s| s == Sound::Countdown(1))
    );
    app.mission.countdown = 0.0;
    for expected in [
        Sound::Gun,
        Sound::Missile,
        Sound::Laser,
        Sound::Plasma,
        Sound::Orb,
    ] {
        app.step(0.02, None);
        assert!(
            app.audio_frame().events.iter().any(|s| s == expected),
            "{expected:?}"
        );
        app.mission.cycle_weapon();
        assert!(
            app.audio_frame()
                .events
                .iter()
                .any(|s| matches!(s, Sound::WeaponSwitch(_)))
        );
    }
    for (kind, sound) in [
        (PickupKind::Coin, Sound::Coin(1)),
        (PickupKind::Upgrade(Weapon::Laser), Sound::Upgrade),
        (PickupKind::Support, Sound::Support),
    ] {
        app.mission.pickups.push(Pickup {
            position: app.flight.position,
            kind,
            life: 5.0,
            collected: false,
        });
        app.step(0.01, None);
        assert!(app.audio_frame().events.iter().any(|s| s == sound));
    }
}

#[test]
fn finished_tracks_rotate_to_another_song_without_an_empty_bar() {
    let mut mixer = Mixer::new(8000);
    mixer.start_cue(Cue::Combat);
    let first = mixer.track();
    let blocks = (scores()[first].seconds() * 10.0) as usize + 40;
    let mut block = [0.0; 1600];
    for i in 0..blocks {
        mixer.render(&mut block);
        if i >= blocks - 60 {
            assert!(block.iter().any(|v| v.abs() > 0.01));
        }
    }
    assert_eq!(mixer.rotations, 1);
    assert_ne!(mixer.track(), first);
    assert!(scores()[mixer.track()].cues.contains(&Cue::Combat));
    assert_eq!(mixer.transitions, 0);
}

#[test]
fn results_music_loops_its_tail_instead_of_rotating() {
    let mut mixer = Mixer::new(8000);
    mixer.start_cue(Cue::Victory);
    let track = mixer.track();
    let blocks = (scores()[track].seconds() * 10.0) as usize + 10;
    let mut block = [0.0; 1600];
    for _ in 0..blocks {
        mixer.render(&mut block);
    }
    assert_eq!(mixer.track(), track);
    assert_eq!(mixer.rotations, 0);
    assert!(mixer.step() >= scores()[track].loop_start);
}

#[test]
fn every_effect_renders_bounded_audio() {
    let all = [
        Sound::BoostStart,
        Sound::OrbitReady,
        Sound::OrbitIgnite,
        Sound::Breakthrough,
        Sound::OrbitArrival,
        Sound::Reentry,
        Sound::Landing,
        Sound::EnemyFire(false),
        Sound::EnemyFire(true),
        Sound::Graze,
        Sound::Flyby,
        Sound::WeaponSwitch(2),
        Sound::Toggle(true),
        Sound::View,
        Sound::Wave,
        Sound::Destroyed,
        Sound::Thunder,
    ];
    let mut slots = std::collections::HashSet::new();
    for sound in all {
        assert!(slots.insert(sound.throttle().0), "{sound:?}");
        let mut m = Mixer::new(24000);
        let mut f = Frame {
            settings: Settings {
                profile: 2,
                volume: 10,
            },
            ..Frame::default()
        };
        f.events.push(sound);
        m.update(f);
        let mut buffer = vec![0.0; 24000];
        m.render(&mut buffer);
        assert!(buffer.iter().any(|v| v.abs() > 0.005), "{sound:?} 没有发声");
        assert!(buffer.iter().all(|v| v.is_finite() && v.abs() <= 0.881));
    }
}

#[test]
fn scene_changes_crossfade_on_the_beat() {
    let mut m = Mixer::new(8000);
    m.start_cue(Cue::Menu);
    seconds(&mut m, 1.0);
    m.update(Frame {
        cue: Cue::Combat,
        ..Frame::default()
    });
    seconds(&mut m, 0.6);
    assert_eq!(m.cue(), Cue::Combat);
    assert_eq!(m.decks_playing(), 2, "旧曲应在淡出中继续演奏");
    assert!(m.step() >= scores()[m.track()].entry, "新曲从主题段进入");
    seconds(&mut m, 3.0);
    assert_eq!(m.decks_playing(), 1);
    assert_eq!(m.transitions, 1);
    m.update(Frame {
        cue: Cue::Victory,
        ..Frame::default()
    });
    seconds(&mut m, 0.6);
    assert_eq!(m.cue(), Cue::Victory);
    assert!(m.step() < 16, "结算曲从开头播放");
}

#[test]
fn map_changes_crossfade_right_away_and_hopping_back_revives_the_old_song() {
    use crate::world::MapKind;
    let mut m = Mixer::new(8000);
    let mut frame = Frame {
        cue: Cue::Flight,
        map: MapKind::Desert,
        ..Frame::default()
    };
    m.update(frame);
    m.start_cue(Cue::Flight);
    let desert = m.track();
    seconds(&mut m, 2.0);
    // 穿过虫洞后立即开始淡出旧曲、淡入新地图主题
    frame.map = MapKind::Space;
    m.update(frame);
    seconds(&mut m, 0.3);
    assert!(playlist(Cue::Flight, MapKind::Space).contains(&m.track()));
    assert_eq!(m.decks_playing(), 2);
    // 淡出途中穿回原地图：原曲直接淡回
    frame.map = MapKind::Desert;
    m.update(frame);
    seconds(&mut m, 0.3);
    assert_eq!(m.track(), desert);
    seconds(&mut m, 2.5);
    assert_eq!(m.decks_playing(), 1);
    assert_eq!(m.track(), desert);
    assert_eq!(m.transitions, 2);
}

#[test]
fn entry_points_skip_quiet_intros() {
    for score in scores() {
        if score.cues.iter().all(|c| c.rotates()) {
            assert!(score.entry < score.steps.len(), "{}", score.title);
            let bar = &score.steps[score.entry..score.entry + 16];
            assert!(bar.iter().flatten().count() > 8, "{}", score.title);
        }
    }
    let hangar = scores()
        .iter()
        .find(|s| s.title == "Hangar Lights")
        .unwrap();
    assert!(hangar.entry >= 128);
}

#[test]
fn night_and_weather_thin_out_exploration_layers() {
    use super::mixer::layer_targets;
    use super::score::{ARP, COUNTER, DRUMS, LEAD, PAD};
    let day = layer_targets(Cue::Flight, &Frame::default());
    assert!(day.iter().all(|g| (*g - 1.0).abs() < 0.001));
    let night = layer_targets(
        Cue::Flight,
        &Frame {
            night: 1.0,
            ..Frame::default()
        },
    );
    assert!(night[DRUMS as usize] < 0.15 && night[ARP as usize] < 0.5);
    assert!(night[LEAD as usize] < 0.7 && night[PAD as usize] > 1.2);
    let snow = layer_targets(
        Cue::Flight,
        &Frame {
            snow: 1.0,
            ..Frame::default()
        },
    );
    assert!(snow[DRUMS as usize] < 0.35);
    // 战斗始终保留主旋律与律动，交火时推满并加入副旋律
    let calm = layer_targets(
        Cue::Combat,
        &Frame {
            intensity: 0.1,
            ..Frame::default()
        },
    );
    let fight = layer_targets(
        Cue::Combat,
        &Frame {
            intensity: 1.0,
            ..Frame::default()
        },
    );
    assert!(
        calm.iter()
            .enumerate()
            .all(|(layer, g)| layer == COUNTER as usize || *g >= 0.55)
    );
    assert!(calm[COUNTER as usize] == 0.0 && calm[DRUMS as usize] < 0.8);
    assert!(fight.iter().all(|g| *g >= 0.999));
}

#[test]
fn combat_layers_rise_quickly_and_release_slowly() {
    use super::score::COUNTER;
    let frame = |intensity| Frame {
        cue: Cue::Combat,
        intensity,
        ..Frame::default()
    };
    let mut m = Mixer::new(24000);
    m.update(frame(0.0));
    m.start_cue(Cue::Combat);
    // 立体声交错：9600 个值为 0.2 秒
    let mut buffer = vec![0.0; 9600];
    let mut run = |m: &mut Mixer, intensity, seconds: usize| {
        for _ in 0..seconds * 5 {
            m.update(frame(intensity));
            m.render(&mut buffer);
        }
    };
    run(&mut m, 0.0, 4);
    let calm = m.layer(COUNTER);
    run(&mut m, 1.0, 3);
    let heated = m.layer(COUNTER);
    assert!(calm < 0.05 && heated > 0.9, "{calm} -> {heated}");
    // 敌机离开一秒后副旋律仍基本保留，不会立即掉下去
    run(&mut m, 0.0, 1);
    assert!(m.layer(COUNTER) > 0.75, "{}", m.layer(COUNTER));
}

#[test]
fn overlays_for_ascent_snow_storm_and_finale_stay_bounded() {
    for (cue, f) in [
        (
            Cue::Flight,
            Frame {
                riser: 1.0,
                altitude: 0.9,
                ascent: 1.0,
                engine: 1.0,
                ..Frame::default()
            },
        ),
        (
            Cue::Flight,
            Frame {
                snow: 1.0,
                storm: 1.0,
                rain: 1.0,
                wind: 1.0,
                night: 1.0,
                ..Frame::default()
            },
        ),
        (
            Cue::Race,
            Frame {
                finale: 1.0,
                intensity: 1.0,
                ..Frame::default()
            },
        ),
    ] {
        let mut m = Mixer::new(24000);
        let f = Frame { cue, ..f };
        m.update(f);
        m.start_cue(cue);
        let mut buffer = vec![0.0; 4800];
        for _ in 0..40 {
            m.update(f);
            m.render(&mut buffer);
            assert!(buffer.iter().all(|v| v.is_finite() && v.abs() <= 0.881));
        }
        assert!(m.peak_voices <= 64);
    }
}

#[test]
fn combat_intensity_rises_when_enemies_close_in_and_guns_fire() {
    use crate::{app::App, gameplay::Mode, world::World};
    let mut app = App::new(World::new(7));
    app.mode = Mode::Combat;
    app.start();
    app.mission.countdown = 0.0;
    app.mission.enemies.clear();
    app.mission.shots.clear();
    let calm = app.audio_frame().intensity;
    for id in 0..3 {
        app.mission.enemies.push(crate::gameplay::Enemy {
            id,
            position: app.flight.position + glam::Vec3::X * (100.0 + id as f32 * 30.0),
            yaw: 0.0,
            health: 40.0,
            max_health: 40.0,
            kind: crate::gameplay::EnemyKind::Light,
            phase: 0.0,
            cooldown: 2.0,
            hit: 0.0,
            aim_point: app.flight.position,
        });
    }
    app.keys.insert(crate::keys::FIRE);
    let heated = app.audio_frame().intensity;
    assert!(calm < 0.35 && heated > 0.8, "{calm} {heated}");
}

fn seconds(m: &mut Mixer, seconds: f32) {
    let mut block = vec![0.0; 1600];
    for _ in 0..(seconds * 10.0).round() as usize {
        m.render(&mut block);
    }
}

#[test]
fn every_surface_map_has_its_own_theme() {
    use crate::world::MapKind;
    for map in MapKind::ALL.into_iter().filter(|m| !m.vacuum()) {
        let list = playlist(Cue::Flight, map);
        assert!(
            list.iter().all(|i| scores()[*i].maps == [map]),
            "{map:?} 的主题应为该地图独有"
        );
    }
}

#[test]
fn only_city_visual_events_select_synthwave_music() {
    use crate::{
        app::App,
        world::{MapKind, World},
    };
    let mut app = App::new(World::new(7109));
    app.start();
    app.theme = 1;
    app.flight.day_time = 33.0;
    for map in [MapKind::City, MapKind::Icebergs, MapKind::Islands] {
        std::sync::Arc::make_mut(&mut app.world).kind = map;
        app.flight.day_cycle = (0..100)
            .find(|cycle| {
                app.flight.day_cycle = *cycle;
                crate::render::time_of_day(&app.flight, &app.world, app.theme).2 > 0.9
            })
            .unwrap();
        let frame = app.audio_frame();
        assert_eq!(frame.synthwave > 0.9, map == MapKind::City);
        let list =
            super::score::playlist_for(frame.cue, map, frame.synthwave > 0.0, frame.night > 0.85);
        assert!(
            list.iter()
                .all(|track| scores()[*track].synthwave == (map == MapKind::City))
        );
        if map != MapKind::City {
            assert_eq!(list, playlist(frame.cue, map));
        }
    }
}

#[test]
fn city_music_crossfades_through_synthwave_and_quiet_night() {
    use crate::world::MapKind;
    let mut mixer = Mixer::new(8000);
    let mut frame = Frame {
        cue: Cue::Flight,
        map: MapKind::City,
        ..Frame::default()
    };
    mixer.update(frame);
    mixer.start_cue(Cue::Flight);
    let day_track = mixer.track();
    let mut block = [0.0; 1600];
    for (neon, night) in [(1.0, 1.0), (0.0, 1.0), (0.0, 0.0)] {
        frame.synthwave = neon;
        frame.night = night;
        for index in 0..80 {
            mixer.update(frame);
            mixer.render(&mut block);
            assert!(block.iter().all(|v| v.is_finite() && v.abs() <= 0.881));
            if index == 10 {
                assert_eq!(mixer.decks_playing(), 2);
            }
        }
        let track = &scores()[mixer.track()];
        assert_eq!(track.synthwave, neon > 0.0);
        assert_eq!(track.night, neon == 0.0 && night > 0.0);
        assert_eq!(mixer.decks_playing(), 1);
        assert!(block.iter().any(|v| v.abs() > 0.005));
    }
    assert_eq!(mixer.track(), day_track);
    assert_eq!(mixer.transitions, 3);
    for map in [MapKind::Icebergs, MapKind::Islands] {
        assert_eq!(
            super::score::playlist_for(Cue::Flight, map, true, true),
            playlist(Cue::Flight, map)
        );
    }
    assert!(
        super::score::playlist_for(Cue::Victory, MapKind::City, true, true)
            .iter()
            .all(|track| scores()[*track].cues.contains(&Cue::Victory))
    );
}
