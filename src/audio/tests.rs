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
        if cue.rotates() && cue != Cue::Ascent {
            assert!(list.len() >= 2, "{cue:?} 至少需要两首轮换");
        }
    }
    for map in MapKind::ALL {
        let list = playlist(Cue::Flight, map);
        assert!(list.len() >= 2, "{map:?} 需要至少两首地图配乐");
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
    let f = app.audio_frame();
    assert_eq!(f.cue, Cue::Ascent);
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
fn ascent_layer_and_vacuum_stay_bounded() {
    let mut m = Mixer::new(24000);
    m.start_cue(Cue::Ascent);
    let mut f = Frame {
        cue: Cue::Ascent,
        engine: 1.0,
        boost: 1.0,
        ascent: 1.0,
        vehicle: 2,
        ..Frame::default()
    };
    let mut buffer = vec![0.0; 4800];
    for i in 0..40 {
        f.altitude = i as f32 / 40.0;
        f.vacuum = i > 30;
        m.update(f);
        m.render(&mut buffer);
        assert!(buffer.iter().all(|v| v.is_finite() && v.abs() <= 0.881));
    }
}

fn seconds(m: &mut Mixer, seconds: f32) {
    let mut block = vec![0.0; 1600];
    for _ in 0..(seconds * 10.0).round() as usize {
        m.render(&mut block);
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
fn map_music_waits_for_the_new_map_to_settle() {
    use crate::world::MapKind;
    let mut m = Mixer::new(8000);
    let mut frame = Frame {
        cue: Cue::Flight,
        map: MapKind::Desert,
        ..Frame::default()
    };
    m.update(frame);
    m.start_cue(Cue::Flight);
    let first = m.track();
    // 每 3 秒穿越一次虫洞，曲目保持不变
    for hop in 0..10 {
        frame.map = [MapKind::Space, MapKind::City, MapKind::Icebergs][hop % 3];
        m.update(frame);
        seconds(&mut m, 3.0);
        frame.map = MapKind::Desert;
        m.update(frame);
        seconds(&mut m, 0.2);
    }
    assert_eq!(m.track(), first);
    assert_eq!(m.transitions, 0);
    // 在新地图停留足够久后换成该地图的配乐
    frame.map = MapKind::Space;
    m.update(frame);
    for _ in 0..12 {
        seconds(&mut m, 1.0);
        m.update(frame);
    }
    seconds(&mut m, 1.0);
    assert!(playlist(Cue::Flight, MapKind::Space).contains(&m.track()));
    assert_eq!(m.cue(), Cue::Flight);
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
