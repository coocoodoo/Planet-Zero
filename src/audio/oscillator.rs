use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub enum Wave {
    Pulse,
    Saw,
    Triangle,
    Sine,
    Noise,
    Fm,
}

pub struct Tables {
    sine: [f32; 2048],
}
impl Tables {
    pub fn new() -> Self {
        Self {
            sine: std::array::from_fn(|i| (i as f32 * TAU / 2048.0).sin()),
        }
    }
    pub fn sine(&self, phase: f32) -> f32 {
        let p = phase.rem_euclid(1.0) * 2048.0;
        let i = p as usize;
        let t = p - i as f32;
        self.sine[i & 2047] * (1.0 - t) + self.sine[(i + 1) & 2047] * t
    }
}

fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
pub struct Oscillator {
    pub phase: f32,
    random: u32,
    filtered: f32,
}
impl Oscillator {
    pub fn new(seed: u32) -> Self {
        Self {
            phase: 0.0,
            random: seed.max(1),
            filtered: 0.0,
        }
    }
    pub fn sample(&mut self, wave: Wave, step: f32, shape: f32, tables: &Tables) -> f32 {
        let dt = step.clamp(0.00001, 0.45);
        let p = self.phase;
        self.phase = (p + dt).fract();
        match wave {
            Wave::Sine => tables.sine(p),
            Wave::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
            Wave::Saw => 2.0 * p - 1.0 - blep(p, dt),
            Wave::Pulse => {
                let duty = shape.clamp(0.12, 0.88);
                let raw = if p < duty { 1.0 } else { -1.0 };
                raw + blep(p, dt) - blep((p - duty).rem_euclid(1.0), dt) - (2.0 * duty - 1.0)
            }
            Wave::Fm => tables.sine(p + shape * tables.sine(p * 2.0) * 0.23),
            Wave::Noise => {
                self.random ^= self.random << 13;
                self.random ^= self.random >> 17;
                self.random ^= self.random << 5;
                let white = self.random as f32 / u32::MAX as f32 * 2.0 - 1.0;
                self.filtered += (white - self.filtered) * shape.abs().clamp(0.01, 1.0);
                if shape < 0.0 {
                    white - self.filtered
                } else {
                    self.filtered
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Tone {
    pub wave: Wave,
    pub hz: f32,
    pub end_hz: f32,
    pub gain: f32,
    pub pan: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub hold: f32,
    pub release: f32,
    pub shape: f32,
    pub vibrato: f32,
    pub delay: f32,
    pub priority: u8,
    pub send: f32,
}
impl Tone {
    pub fn new(wave: Wave, hz: f32, gain: f32, hold: f32) -> Self {
        Self {
            wave,
            hz,
            end_hz: hz,
            gain,
            pan: 0.0,
            attack: 0.006,
            decay: 0.08,
            sustain: 0.6,
            hold,
            release: 0.05,
            shape: 0.5,
            vibrato: 0.0,
            delay: 0.0,
            priority: 1,
            send: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Voice {
    pub tone: Tone,
    osc: Oscillator,
    age: f32,
    pan: [f32; 2],
    release_at: f32,
}
impl Voice {
    pub fn new(tone: Tone, seed: u32) -> Self {
        let pan = tone.pan.clamp(-1.0, 1.0);
        let release_at = envelope(tone.hold, &tone);
        Self {
            tone,
            osc: Oscillator::new(seed),
            age: -tone.delay,
            pan: [((1.0 - pan) * 0.5).sqrt(), ((1.0 + pan) * 0.5).sqrt()],
            release_at,
        }
    }
    pub fn remaining(&self) -> f32 {
        self.tone.hold + self.tone.release - self.age
    }
    pub fn fade(&mut self, seconds: f32) {
        if self.age >= 0.0 {
            self.release_at = self.level();
            self.tone.hold = self.age;
            self.tone.release = seconds;
        }
    }
    fn level(&self) -> f32 {
        if self.age < self.tone.hold {
            envelope(self.age, &self.tone)
        } else {
            self.release_at
                * (1.0 - (self.age - self.tone.hold) / self.tone.release.max(0.001))
                    .max(0.0)
                    .powi(2)
        }
    }
    pub fn sample(&mut self, dt: f32, tables: &Tables) -> [f32; 2] {
        self.age += dt;
        if self.age < 0.0 {
            return [0.0; 2];
        }
        let t = (self.age / self.tone.hold.max(0.01)).min(1.0);
        let hz = self.tone.hz + (self.tone.end_hz - self.tone.hz) * t;
        let vibrato =
            1.0 + self.tone.vibrato * tables.sine(self.age * 5.4) * (self.age * 6.0).min(1.0);
        let value = self
            .osc
            .sample(self.tone.wave, hz * vibrato * dt, self.tone.shape, tables)
            * self.tone.gain
            * self.level();
        [value * self.pan[0], value * self.pan[1]]
    }
}
fn envelope(age: f32, t: &Tone) -> f32 {
    if age < t.attack {
        (age / t.attack.max(0.001)).max(0.0)
    } else {
        t.sustain
            + (1.0 - t.sustain)
                * (1.0 - (age - t.attack) / t.decay.max(0.001))
                    .max(0.0)
                    .powi(2)
    }
}

pub fn frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}
