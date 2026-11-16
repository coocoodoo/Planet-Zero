use super::{
    Sound,
    oscillator::{Tone, Wave, frequency},
};

pub fn tones(sound: Sound, mut emit: impl FnMut(Tone)) {
    let mut blip = |wave, from, to, gain, hold, delay| {
        let mut t = Tone::new(wave, from, gain, hold);
        t.end_hz = to;
        t.delay = delay;
        t.priority = 3;
        t.attack = 0.002;
        t.decay = hold * 0.7;
        t.sustain = 0.08;
        t.release = 0.025;
        emit(t);
    };
    match sound {
        Sound::Navigate => blip(Wave::Triangle, 660.0, 880.0, 0.12, 0.04, 0.0),
        Sound::Confirm => {
            blip(Wave::Pulse, 660.0, 660.0, 0.10, 0.06, 0.0);
            blip(Wave::Triangle, 990.0, 990.0, 0.18, 0.12, 0.065);
        }
        Sound::Countdown(n) => {
            let hz = if n == 0 { 1320.0 } else { 660.0 };
            blip(
                Wave::Sine,
                hz,
                hz,
                0.21,
                if n == 0 { 0.28 } else { 0.08 },
                0.0,
            );
        }
        Sound::Warp => {
            blip(Wave::Fm, 90.0, 1600.0, 0.24, 0.22, 0.0);
            blip(Wave::Noise, 700.0, 180.0, 0.12, 0.38, 0.03);
            blip(Wave::Sine, 1200.0, 150.0, 0.22, 0.35, 0.18);
        }
        Sound::Roll => {
            blip(Wave::Noise, 300.0, 1500.0, 0.11, 0.32, 0.0);
            blip(Wave::Triangle, 220.0, 660.0, 0.06, 0.25, 0.0);
        }
        Sound::Gun => {
            blip(Wave::Pulse, 260.0, 80.0, 0.13, 0.035, 0.0);
            blip(Wave::Noise, 1000.0, 1000.0, 0.12, 0.035, 0.0);
        }
        Sound::Missile => {
            blip(Wave::Noise, 700.0, 300.0, 0.20, 0.24, 0.0);
            blip(Wave::Saw, 180.0, 580.0, 0.09, 0.18, 0.02);
        }
        Sound::Laser => blip(Wave::Saw, 1800.0, 220.0, 0.16, 0.18, 0.0),
        Sound::Plasma => {
            blip(Wave::Fm, 580.0, 85.0, 0.23, 0.22, 0.0);
            blip(Wave::Sine, 1100.0, 240.0, 0.07, 0.12, 0.015);
        }
        Sound::Orb => {
            blip(Wave::Fm, 140.0, 420.0, 0.21, 0.30, 0.0);
            blip(Wave::Triangle, 280.0, 840.0, 0.10, 0.25, 0.045);
        }
        Sound::Impact => blip(Wave::Noise, 1000.0, 1000.0, 0.09, 0.045, 0.0),
        Sound::Explosion(big) => {
            blip(
                Wave::Noise,
                160.0,
                70.0,
                if big { 0.40 } else { 0.23 },
                if big { 0.9 } else { 0.28 },
                0.0,
            );
            blip(
                Wave::Sine,
                100.0,
                35.0,
                if big { 0.4 } else { 0.22 },
                if big { 0.65 } else { 0.18 },
                0.0,
            );
        }
        Sound::Damage => {
            blip(Wave::Noise, 900.0, 900.0, 0.25, 0.12, 0.0);
            blip(Wave::Pulse, 320.0, 100.0, 0.14, 0.16, 0.0);
        }
        Sound::Coin(combo) => {
            let note = [76, 79, 83, 86, 88][combo.saturating_sub(1).min(9) as usize % 5];
            blip(
                Wave::Pulse,
                frequency(note),
                frequency(note),
                0.10,
                0.055,
                0.0,
            );
            blip(
                Wave::Sine,
                frequency(note + 12),
                frequency(note + 12),
                0.19,
                0.16,
                0.06,
            );
        }
        Sound::Pickup
        | Sound::Upgrade
        | Sound::Support
        | Sound::Overdrive
        | Sound::Gate(_)
        | Sound::Phase => {
            let notes: &[u8] = match sound {
                Sound::Upgrade => &[64, 67, 71, 76, 83],
                Sound::Support => &[71, 76, 79, 83],
                Sound::Overdrive => &[52, 64, 71, 76, 83, 88],
                Sound::Gate(true) => &[79, 83, 86, 91],
                Sound::Gate(false) => &[76, 83],
                Sound::Phase => &[64, 71, 76, 79, 83, 88],
                _ => &[71, 76, 83],
            };
            for (i, n) in notes.iter().enumerate() {
                blip(
                    Wave::Triangle,
                    frequency(*n),
                    frequency(*n),
                    0.23,
                    0.16,
                    i as f32 * 0.065,
                );
            }
        }
        Sound::Nova => {
            blip(Wave::Fm, 80.0, 1200.0, 0.30, 0.22, 0.0);
            blip(Wave::Noise, 120.0, 120.0, 0.40, 0.75, 0.18);
            blip(Wave::Sine, 140.0, 30.0, 0.38, 0.6, 0.18);
        }
        Sound::Boss => {
            for i in 0..3 {
                blip(Wave::Pulse, 330.0, 220.0, 0.14, 0.16, i as f32 * 0.25);
                blip(Wave::Triangle, 165.0, 110.0, 0.12, 0.16, i as f32 * 0.25);
            }
        }
    }
}
