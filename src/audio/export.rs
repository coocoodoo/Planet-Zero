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
    cue: Cue,
    mut control: impl FnMut(f32, &mut Frame),
) -> io::Result<()> {
    let frames = (seconds * 48000.0) as usize;
    let mut file = BufWriter::new(File::create(path)?);
    header(&mut file, frames as u32)?;
    let mut mixer = Mixer::new(48000);
    mixer.start_cue(cue);
    let mut frame = Frame {
        cue,
        ..Frame::default()
    };
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
    for cue in Cue::ALL {
        let score = &scores()[cue as usize];
        wav(
            &directory.join(format!("{}.wav", cue.name())),
            score.seconds(),
            cue,
            |_, _| {},
        )?;
        println!(
            "{} / {} / {:.2} s",
            cue.name(),
            score.title,
            score.seconds()
        );
    }
    let timeline = [
        (0.0, Cue::Menu),
        (8.3, Cue::Flight),
        (20.6, Cue::Race),
        (32.4, Cue::Combat),
        (56.2, Cue::Boss),
        (74.4, Cue::Victory),
        (86.6, Cue::Defeat),
    ];
    let mut last_cue = 0;
    let mut last_effect = 0;
    let effects = [
        (13.0, Sound::Roll),
        (23.0, Sound::Coin(1)),
        (23.7, Sound::Coin(2)),
        (24.4, Sound::Coin(3)),
        (27.0, Sound::Gate(true)),
        (32.4, Sound::Phase),
        (35.0, Sound::Gun),
        (35.14, Sound::Gun),
        (35.28, Sound::Gun),
        (36.0, Sound::Missile),
        (37.0, Sound::Explosion(false)),
        (39.0, Sound::Laser),
        (40.0, Sound::Plasma),
        (41.0, Sound::Orb),
        (42.5, Sound::Upgrade),
        (46.0, Sound::Damage),
        (49.0, Sound::Overdrive),
        (56.2, Sound::Boss),
        (65.0, Sound::Nova),
        (74.0, Sound::Explosion(true)),
    ];
    wav(&directory.join("showreel.wav"), 96.0, Cue::Menu, |t, f| {
        while last_cue + 1 < timeline.len() && t >= timeline[last_cue + 1].0 {
            last_cue += 1;
        }
        f.cue = timeline[last_cue].1;
        f.engine = f32::from((8.3..74.4).contains(&t));
        f.boost = if (14.0..18.0).contains(&t) { 1.0 } else { 0.0 };
        f.danger = if (60.0..74.4).contains(&t) { 0.8 } else { 0.0 };
        f.overdrive = (49.0..57.0).contains(&t);
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
    ];
    let mut index = 0;
    wav(
        &directory.join("effects.wav"),
        all.len() as f32 * 1.5 + 1.0,
        Cue::Menu,
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
