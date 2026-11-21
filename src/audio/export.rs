use super::{Cue, Events, Frame, Mixer, Sound, scores};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::Path,
    time::Instant,
};

fn header(out: &mut impl Write, frames: u32) -> io::Result<()> {
    let bytes = frames * 4;
    out.write_all(b"RIFF")?;
    out.write_all(&(bytes + 36).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16_u32.to_le_bytes())?;
    out.write_all(&1_u16.to_le_bytes())?;
    out.write_all(&2_u16.to_le_bytes())?;
    out.write_all(&48000_u32.to_le_bytes())?;
    out.write_all(&192000_u32.to_le_bytes())?;
    out.write_all(&4_u16.to_le_bytes())?;
    out.write_all(&16_u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&bytes.to_le_bytes())
}
fn wav(
    path: &Path,
    seconds: f32,
    track: usize,
    mut control: impl FnMut(f32, &mut Frame),
) -> io::Result<()> {
    let frames = (seconds * 48000.0) as usize;
    let mut file = BufWriter::new(File::create(path)?);
    header(&mut file, frames as u32)?;
    let mut mixer = Mixer::new(48000);
    let score = &scores()[track];
    let mut frame = Frame {
        cue: score.cues[0],
        map: score
            .maps
            .first()
            .copied()
            .unwrap_or(crate::world::MapKind::Islands),
        ..Frame::default()
    };
    mixer.update(frame);
    mixer.start_track(track);
    let mut samples = [0.0; 960];
    for offset in (0..frames).step_by(480) {
        let time = offset as f32 / 48000.0;
        frame.events = Events::default();
        control(time, &mut frame);
        if seconds - time < 0.8 {
            frame.settings.volume = ((seconds - time) / 0.8 * 7.0) as u8;
        }
        mixer.update(frame);
        let count = (frames - offset).min(480) * 2;
        mixer.render(&mut samples[..count]);
        for value in &samples[..count] {
            file.write_all(&((*value * 32767.0) as i16).to_le_bytes())?;
        }
    }
    file.flush()
}

pub fn soundtrack(directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    for (index, score) in scores().iter().enumerate() {
        let slug: String = score
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        wav(
            &directory.join(format!(
                "{:02}-{}-{slug}.wav",
                index + 1,
                score.cues[0].name()
            )),
            score.seconds(),
            index,
            |_, _| {},
        )?;
        let cues: Vec<_> = score.cues.iter().map(|c| c.name()).collect();
        println!(
            "{} / {} / {} BPM / {:.2} s",
            cues.join(","),
            score.title,
            score.bpm,
            score.seconds()
        );
    }
    use crate::world::MapKind;
    // 菜单 → 沙漠巡航 → 升空 → 太空 → 竞速 → 空战 → Boss → 通关 → 失败
    let timeline = [
        (0.0, Cue::Menu, MapKind::Islands),
        (8.0, Cue::Flight, MapKind::Desert),
        (20.0, Cue::Flight, MapKind::Desert),
        (30.0, Cue::Flight, MapKind::Space),
        (40.0, Cue::Race, MapKind::City),
        (52.0, Cue::Combat, MapKind::Canyon),
        (72.0, Cue::Boss, MapKind::Volcanic),
        (90.0, Cue::Victory, MapKind::Volcanic),
        (100.0, Cue::Defeat, MapKind::Volcanic),
    ];
    let mut last_cue = 0;
    let mut last_effect = 0;
    let effects = [
        (12.0, Sound::BoostStart),
        (15.0, Sound::Roll),
        (18.5, Sound::OrbitReady),
        (20.0, Sound::OrbitIgnite),
        (26.0, Sound::Breakthrough),
        (30.0, Sound::OrbitArrival),
        (42.0, Sound::Coin(1)),
        (42.7, Sound::Coin(2)),
        (43.4, Sound::Coin(3)),
        (46.0, Sound::Gate(true)),
        (52.0, Sound::Wave),
        (55.0, Sound::Gun),
        (55.14, Sound::Gun),
        (55.28, Sound::Gun),
        (56.0, Sound::EnemyFire(false)),
        (56.5, Sound::WeaponSwitch(1)),
        (57.0, Sound::Missile),
        (58.0, Sound::Explosion(false)),
        (59.0, Sound::Flyby),
        (60.0, Sound::Graze),
        (61.0, Sound::Laser),
        (62.0, Sound::Plasma),
        (63.0, Sound::Orb),
        (64.5, Sound::Upgrade),
        (66.0, Sound::Damage),
        (68.0, Sound::Overdrive),
        (72.0, Sound::Boss),
        (75.0, Sound::EnemyFire(true)),
        (80.0, Sound::Nova),
        (89.5, Sound::Explosion(true)),
        (99.5, Sound::Destroyed),
    ];
    wav(&directory.join("showreel.wav"), 110.0, 0, |t, f| {
        while last_cue + 1 < timeline.len() && t >= timeline[last_cue + 1].0 {
            last_cue += 1;
        }
        (_, f.cue, f.map) = timeline[last_cue];
        f.engine = f32::from((8.0..90.0).contains(&t));
        f.boost = if (12.0..16.0).contains(&t) { 1.0 } else { 0.0 };
        f.ascent = if (20.0..30.0).contains(&t) { 1.0 } else { 0.0 };
        f.riser = f.ascent;
        f.altitude = ((t - 18.0) / 12.0).clamp(0.0, 1.0);
        f.vacuum = (28.0..40.0).contains(&t);
        f.vehicle = if t < 40.0 { 0 } else { 2 };
        f.danger = if (80.0..90.0).contains(&t) { 0.8 } else { 0.0 };
        f.overdrive = (68.0..76.0).contains(&t);
        while last_effect < effects.len() && t >= effects[last_effect].0 {
            f.events.push(effects[last_effect].1);
            last_effect += 1;
        }
    })?;
    let all = [
        Sound::Navigate,
        Sound::Confirm,
        Sound::Countdown(3),
        Sound::Countdown(0),
        Sound::Roll,
        Sound::Gun,
        Sound::Missile,
        Sound::Laser,
        Sound::Plasma,
        Sound::Orb,
        Sound::Impact,
        Sound::Explosion(false),
        Sound::Explosion(true),
        Sound::Damage,
        Sound::Coin(1),
        Sound::Coin(5),
        Sound::Pickup,
        Sound::Upgrade,
        Sound::Support,
        Sound::Gate(false),
        Sound::Gate(true),
        Sound::Nova,
        Sound::Overdrive,
        Sound::Phase,
        Sound::Boss,
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
        Sound::WeaponSwitch(0),
        Sound::WeaponSwitch(3),
        Sound::Toggle(true),
        Sound::Toggle(false),
        Sound::View,
        Sound::Wave,
        Sound::Destroyed,
        Sound::Thunder,
    ];
    let mut index = 0;
    wav(
        &directory.join("effects.wav"),
        all.len() as f32 * 1.5 + 1.0,
        0,
        |t, f| {
            f.settings.profile = 2;
            if index < all.len() && t >= index as f32 * 1.5 {
                f.events.push(all[index]);
                index += 1;
            }
        },
    )
}

pub fn benchmark(seconds: f32) {
    let mut mixer = Mixer::new(48000);
    mixer.start_cue(Cue::Boss);
    let mut frame = Frame {
        cue: Cue::Boss,
        engine: 1.0,
        boost: 1.0,
        danger: 1.0,
        overdrive: true,
        ..Frame::default()
    };
    let mut samples = [0.0; 960];
    let mut peak = 0.0_f32;
    let mut square = 0.0_f64;
    let began = Instant::now();
    let blocks = (seconds * 100.0).ceil() as usize;
    for i in 0..blocks {
        frame.events = Events::default();
        if i % 10 == 0 {
            frame.events.push(Sound::Gun);
        }
        if i % 50 == 0 {
            frame.events.push(Sound::Explosion(false));
        }
        if i % 100 == 0 {
            frame.events.push(Sound::Nova);
        }
        mixer.update(frame);
        mixer.render(&mut samples);
        for value in samples {
            peak = peak.max(value.abs());
            square += (value * value) as f64;
        }
    }
    let cpu = began.elapsed().as_secs_f64();
    println!(
        "{{\"audio_seconds\":{:.2},\"compute_seconds\":{cpu:.4},\"core_percent\":{:.2},\"peak\":{peak:.5},\"rms\":{:.5},\"peak_voices\":{}}}",
        blocks as f64 / 100.0,
        cpu / (blocks as f64 / 100.0) * 100.0,
        (square / (blocks * 960) as f64).sqrt(),
        mixer.peak_voices
    );
}
