use super::*;

#[test]
fn scores_have_full_arrangements_and_valid_loop_points() {
    for cue in Cue::ALL {
        let score = &scores()[cue as usize];
        assert_eq!(score.steps.len() % 128, 0);
        assert_eq!(score.chords.len() * 16, score.steps.len());
        assert!(score.loop_start < score.steps.len());
        if !matches!(cue, Cue::Victory | Cue::Defeat) {
            assert!(score.seconds() > 100.0);
        }
        for tone in score.steps.iter().flatten() {
            assert!(tone.hz.is_finite() && tone.hz > 20.0 && tone.hz < 12000.0);
            assert!(tone.hold > 0.0 && tone.gain <= 0.5);
        }
    }
}

#[test]
fn transitions_wait_for_bars_and_results_wait_for_beats() {
    let mut m = Mixer::new(48000);
    let mut buffer = vec![0.0; 12000];
    m.render(&mut buffer);
    m.update(Frame {
        cue: Cue::Combat,
        ..Frame::default()
    });
    m.render(&mut buffer);
    assert_eq!(m.cue(), Cue::Menu);
    for _ in 0..14 {
        m.render(&mut buffer);
    }
    assert_eq!(m.cue(), Cue::Combat);
    assert_eq!(m.transitions, 1);
    m.update(Frame {
        cue: Cue::Victory,
        ..Frame::default()
    });
    for _ in 0..4 {
        m.render(&mut buffer);
    }
    assert_eq!(m.cue(), Cue::Victory);
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
    app.key(44, 1);
    app.key(44, 0);
    assert_eq!(app.audio_settings.volume, 6);
    app.key(46, 1);
    app.key(46, 0);
    assert_eq!(app.audio_settings.volume, 7);
    assert_eq!(app.page, Page::Paused);
    app.page = Page::Results;
    app.mission.outcome = Some(true);
    assert_eq!(app.audio_frame().cue, Cue::Victory);
    app.mission.outcome = Some(false);
    assert_eq!(app.audio_frame().cue, Cue::Defeat);
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
fn full_score_wraps_without_an_empty_bar() {
    let mut mixer = Mixer::new(8000);
    mixer.start_cue(Cue::Flight);
    let mut block = [0.0; 1600];
    for i in 0..1075 {
        mixer.render(&mut block);
        if i >= 1040 {
            assert!(block.iter().any(|v| v.abs() > 0.01));
        }
    }
    assert!(mixer.step() < 16);
    assert_eq!(mixer.transitions, 0);
}
