use super::{
    Cue, Frame, Sound, effects,
    oscillator::{Oscillator, Tables, Tone, Voice, Wave, frequency},
    scores,
};

pub struct Mixer {
    rate: f32,
    tables: Tables,
    voices: [Option<Voice>; 64],
    frame: Frame,
    cue: Cue,
    pending: Option<Cue>,
    step: usize,
    until_step: f64,
    clock: f64,
    last_effect: [f64; 24],
    seed: u32,
    delay: Vec<[f32; 2]>,
    delay_index: usize,
    reverb: Vec<[f32; 2]>,
    reverb_index: usize,
    master: f32,
    music_gain: f32,
    effects_gain: f32,
    duck: f32,
    limiter: f32,
    filtered: [f32; 2],
    dc_input: [f32; 2],
    dc_output: [f32; 2],
    engine: [Oscillator; 3],
    engine_gain: f32,
    boost: f32,
    pub peak_voices: usize,
    pub transitions: u32,
}
impl Mixer {
    pub fn new(rate: u32) -> Self {
        let _ = scores();
        Self {
            rate: rate as f32,
            tables: Tables::new(),
            voices: [None; 64],
            frame: Frame::default(),
            cue: Cue::Menu,
            pending: None,
            step: 0,
            until_step: 0.0,
            clock: 0.0,
            last_effect: [-10.0; 24],
            seed: 173,
            delay: vec![[0.0; 2]; (rate as f32 * 0.3125) as usize],
            delay_index: 0,
            reverb: vec![[0.0; 2]; (rate as f32 * 0.061) as usize],
            reverb_index: 0,
            master: 0.0,
            music_gain: 0.0,
            effects_gain: 0.0,
            duck: 0.0,
            limiter: 1.0,
            filtered: [0.0; 2],
            dc_input: [0.0; 2],
            dc_output: [0.0; 2],
            engine: [
                Oscillator::new(37),
                Oscillator::new(73),
                Oscillator::new(317),
            ],
            engine_gain: 0.0,
            boost: 0.0,
            peak_voices: 0,
            transitions: 0,
        }
    }
    pub fn cue(&self) -> Cue {
        self.cue
    }
    pub fn step(&self) -> usize {
        self.step
    }
    pub fn update(&mut self, frame: Frame) {
        if frame.cue != self.cue {
            self.pending = Some(frame.cue);
        } else {
            self.pending = None;
        }
        if frame.settings.effects() {
            for event in frame.events.iter() {
                self.play(event);
            }
        }
        self.frame = frame;
    }
    pub fn start_cue(&mut self, cue: Cue) {
        self.cue = cue;
        self.pending = None;
        self.step = 0;
        self.until_step = 0.0;
        self.frame.cue = cue;
    }
    fn add(&mut self, tone: Tone) {
        let slot = self.voices.iter().position(Option::is_none).or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.as_ref().is_some_and(|v| v.tone.priority <= tone.priority))
                .min_by(|(_, a), (_, b)| {
                    let a = a.as_ref().unwrap();
                    let b = b.as_ref().unwrap();
                    a.tone
                        .priority
                        .cmp(&b.tone.priority)
                        .then(a.remaining().total_cmp(&b.remaining()))
                })
                .map(|(i, _)| i)
        });
        if let Some(slot) = slot {
            self.seed = self.seed.wrapping_add(7919);
            self.voices[slot] = Some(Voice::new(tone, self.seed));
        }
    }
    fn play(&mut self, sound: Sound) {
        let (index, cooldown) = match sound {
            Sound::Navigate => (0, 0.035),
            Sound::Confirm => (1, 0.07),
            Sound::Countdown(_) => (2, 0.12),
            Sound::Roll => (3, 0.7),
            Sound::Gun => (4, 0.065),
            Sound::Missile => (5, 0.14),
            Sound::Laser => (6, 0.10),
            Sound::Plasma => (7, 0.10),
            Sound::Orb => (8, 0.16),
            Sound::Impact => (9, 0.10),
            Sound::Explosion(false) => (10, 0.10),
            Sound::Explosion(true) => (11, 0.5),
            Sound::Damage => (12, 0.22),
            Sound::Coin(_) => (13, 0.025),
            Sound::Pickup => (14, 0.09),
            Sound::Upgrade => (15, 0.12),
            Sound::Support => (16, 0.2),
            Sound::Gate(_) => (17, 0.12),
            Sound::Nova => (18, 0.3),
            Sound::Overdrive => (19, 0.5),
            Sound::Phase => (20, 0.5),
            Sound::Boss => (21, 1.5),
        };
        if self.clock - self.last_effect[index] < cooldown {
            return;
        }
        self.last_effect[index] = self.clock;
        if matches!(
            sound,
            Sound::Nova | Sound::Boss | Sound::Explosion(true) | Sound::Damage
        ) {
            self.duck = 0.42;
        }
        effects::tones(sound, |tone| self.add(tone));
    }
    fn tick(&mut self) {
        if let Some(next) = self.pending {
            let urgent = matches!(next, Cue::Victory | Cue::Defeat);
            if self.step.is_multiple_of(if urgent { 4 } else { 16 }) {
                for voice in self.voices.iter_mut().flatten() {
                    if voice.tone.priority == 1 {
                        voice.fade(0.15);
                    }
                }
                self.cue = next;
                self.pending = None;
                self.step = 0;
                self.transitions += 1;
                if !urgent {
                    let mut transition = Tone::new(Wave::Noise, 1000.0, 0.07, 0.24);
                    transition.attack = 0.008;
                    transition.decay = 0.22;
                    transition.sustain = 0.0;
                    transition.release = 0.12;
                    self.add(transition);
                }
            }
        }
        let score = &scores()[self.cue as usize];
        if self.step >= score.steps.len() {
            self.step = score.loop_start;
        }
        for tone in &score.steps[self.step] {
            self.add(*tone);
        }
        if matches!(self.cue, Cue::Combat | Cue::Boss) {
            if self.frame.overdrive && self.step % 2 == 1 {
                let chord = score.chords[self.step / 16];
                let mut tone = Tone::new(
                    Wave::Pulse,
                    frequency(chord[self.step % 4] + 24),
                    0.045,
                    0.065,
                );
                tone.pan = if self.step % 4 == 1 { -0.6 } else { 0.6 };
                tone.shape = 0.125;
                self.add(tone);
            }
            if self.frame.danger > 0.2 && (self.step % 16 == 2 || self.step % 16 == 10) {
                let mut tone = Tone::new(Wave::Fm, 140.0, self.frame.danger * 0.12, 0.16);
                tone.end_hz = 65.0;
                tone.decay = 0.12;
                tone.sustain = 0.0;
                self.add(tone);
            }
        }
        self.step += 1;
        self.until_step += self.rate as f64 * 15.0 / score.bpm as f64;
        self.peak_voices = self.peak_voices.max(self.voices.iter().flatten().count());
    }
    pub fn render(&mut self, destination: &mut [f32]) {
        for stereo in destination.chunks_exact_mut(2) {
            let value = self.sample();
            stereo.copy_from_slice(&value);
        }
    }
    pub fn sample(&mut self) -> [f32; 2] {
        if self.until_step <= 0.0 {
            self.tick();
        }
        self.until_step -= 1.0;
        let dt = 1.0 / self.rate;
        self.clock += dt as f64;
        let smoothing = dt * 65.0;
        self.master += (self.frame.settings.volume as f32 / 10.0 - self.master) * smoothing;
        let music_target = if self.frame.settings.music() {
            if self.frame.paused { 0.23 } else { 1.0 }
        } else {
            0.0
        };
        self.music_gain += (music_target - self.music_gain) * dt * 18.0;
        self.effects_gain +=
            (f32::from(self.frame.settings.effects()) - self.effects_gain) * smoothing;
        self.duck = (self.duck - dt * 0.75).max(0.0);
        let mut music = [0.0; 2];
        let mut fx = [0.0; 2];
        let mut send = [0.0; 2];
        for slot in &mut self.voices {
            if let Some(voice) = slot {
                let value = voice.sample(dt, &self.tables);
                let bus = if voice.tone.priority == 1 {
                    &mut music
                } else {
                    &mut fx
                };
                send[0] += value[0] * voice.tone.send;
                send[1] += value[1] * voice.tone.send;
                bus[0] += value[0];
                bus[1] += value[1];
                if voice.remaining() <= 0.0 {
                    *slot = None;
                }
            }
        }
        let echo = self.delay[self.delay_index];
        self.delay[self.delay_index] = [send[0] + echo[1] * 0.34, send[1] + echo[0] * 0.34];
        self.delay_index = (self.delay_index + 1) % self.delay.len();
        let room = self.reverb[self.reverb_index];
        self.reverb[self.reverb_index] = [
            music[0] * 0.055 + room[1] * 0.48,
            music[1] * 0.055 + room[0] * 0.48,
        ];
        self.reverb_index = (self.reverb_index + 1) % self.reverb.len();
        self.engine_gain += (self.frame.engine - self.engine_gain) * dt * 6.0;
        self.boost += (self.frame.boost - self.boost) * dt * 8.0;
        let hz = 65.0 + self.engine_gain * 32.0 + self.boost * 85.0;
        let turbine = self.engine[0].sample(Wave::Saw, hz * dt, 0.5, &self.tables) * 0.012
            + self.engine[1].sample(Wave::Triangle, hz * 2.01 * dt, 0.5, &self.tables) * 0.020
            + self.engine[2].sample(
                Wave::Noise,
                1000.0 * dt,
                0.06 + self.boost * 0.25,
                &self.tables,
            ) * (0.018 + self.boost * 0.065);
        let engine = turbine * self.engine_gain;
        let mut result = [0.0; 2];
        for channel in 0..2 {
            let wet = music[channel] + echo[channel] + room[channel];
            self.filtered[channel] +=
                (wet - self.filtered[channel]) * if self.frame.paused { 0.055 } else { 0.65 };
            let mixed = self.filtered[channel] * self.music_gain * (1.0 - self.duck)
                + (fx[channel] + engine) * self.effects_gain;
            let dc = mixed - self.dc_input[channel] + 0.997 * self.dc_output[channel];
            self.dc_input[channel] = mixed;
            self.dc_output[channel] = dc;
            result[channel] = dc * self.master * 1.55;
        }
        let peak = result[0].abs().max(result[1].abs());
        let required = if peak > 0.88 { 0.88 / peak } else { 1.0 };
        self.limiter = if required < self.limiter {
            required
        } else {
            self.limiter + (required - self.limiter) * dt * 5.0
        };
        result.map(|v| v * self.limiter)
    }
}
