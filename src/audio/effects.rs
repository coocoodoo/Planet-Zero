use super::{
    Sound,
    oscillator::{Tone, Wave, frequency},
};

/// 音效音色参数：起止频率、音量、时长、延迟，噪声的 shape 控制滤波亮度
#[derive(Clone, Copy)]
struct Blip {
    wave: Wave,
    from: f32,
    to: f32,
    gain: f32,
    hold: f32,
    delay: f32,
    shape: f32,
    attack: f32,
    pan: f32,
}
impl Blip {
    fn new(wave: Wave, from: f32, to: f32, gain: f32, hold: f32, delay: f32) -> Self {
        Self {
            wave,
            from,
            to,
            gain,
            hold,
            delay,
            shape: 0.5,
            attack: 0.002,
            pan: 0.0,
        }
    }
    fn shape(mut self, shape: f32) -> Self {
        self.shape = shape;
        self
    }
    fn attack(mut self, attack: f32) -> Self {
        self.attack = attack;
        self
    }
    fn pan(mut self, pan: f32) -> Self {
        self.pan = pan;
        self
    }
    fn tone(self) -> Tone {
        let mut t = Tone::new(self.wave, self.from, self.gain, self.hold);
        t.end_hz = self.to;
        t.delay = self.delay;
        t.priority = 3;
        t.attack = self.attack;
        t.decay = (self.hold * 0.7).max(self.attack);
        t.sustain = 0.08;
        t.release = 0.025;
        t.shape = self.shape;
        t.pan = self.pan;
        t
    }
}

pub fn tones(sound: Sound, mut emit: impl FnMut(Tone)) {
    let mut put = |b: Blip| emit(b.tone());
    let blip = Blip::new;
    match sound {
        Sound::Navigate => put(blip(Wave::Triangle, 660.0, 880.0, 0.12, 0.04, 0.0)),
        Sound::Confirm => {
            put(blip(Wave::Pulse, 660.0, 660.0, 0.10, 0.06, 0.0));
            put(blip(Wave::Triangle, 990.0, 990.0, 0.18, 0.12, 0.065));
        }
        Sound::Countdown(n) => {
            let hz = if n == 0 { 1320.0 } else { 660.0 };
            put(blip(
                Wave::Sine,
                hz,
                hz,
                0.21,
                if n == 0 { 0.28 } else { 0.08 },
                0.0,
            ));
        }
        Sound::Warp => {
            put(blip(Wave::Fm, 90.0, 1600.0, 0.24, 0.22, 0.0));
            put(blip(Wave::Noise, 700.0, 180.0, 0.12, 0.38, 0.03));
            put(blip(Wave::Sine, 1200.0, 150.0, 0.22, 0.35, 0.18));
        }
        Sound::Roll => {
            put(blip(Wave::Noise, 300.0, 1500.0, 0.11, 0.32, 0.0).shape(0.3));
            put(blip(Wave::Triangle, 220.0, 660.0, 0.06, 0.25, 0.0));
        }
        Sound::Gun => {
            put(blip(Wave::Pulse, 260.0, 80.0, 0.13, 0.035, 0.0));
            put(blip(Wave::Noise, 1000.0, 1000.0, 0.12, 0.035, 0.0));
        }
        Sound::Missile => {
            put(blip(Wave::Noise, 700.0, 300.0, 0.20, 0.24, 0.0));
            put(blip(Wave::Saw, 180.0, 580.0, 0.09, 0.18, 0.02));
        }
        Sound::Laser => put(blip(Wave::Saw, 1800.0, 220.0, 0.16, 0.18, 0.0)),
        Sound::Plasma => {
            put(blip(Wave::Fm, 580.0, 85.0, 0.23, 0.22, 0.0));
            put(blip(Wave::Sine, 1100.0, 240.0, 0.07, 0.12, 0.015));
        }
        Sound::Orb => {
            put(blip(Wave::Fm, 140.0, 420.0, 0.21, 0.30, 0.0));
            put(blip(Wave::Triangle, 280.0, 840.0, 0.10, 0.25, 0.045));
        }
        Sound::Impact => put(blip(Wave::Noise, 1000.0, 1000.0, 0.09, 0.045, 0.0)),
        Sound::Explosion(big) => {
            put(blip(
                Wave::Noise,
                160.0,
                70.0,
                if big { 0.40 } else { 0.23 },
                if big { 0.9 } else { 0.28 },
                0.0,
            ));
            put(blip(
                Wave::Sine,
                100.0,
                35.0,
                if big { 0.4 } else { 0.22 },
                if big { 0.65 } else { 0.18 },
                0.0,
            ));
            if big {
                // 大爆炸的二次碎裂与低频余震
                put(blip(Wave::Noise, 0.0, 0.0, 0.16, 0.5, 0.22).shape(0.12));
                put(blip(Wave::Fm, 70.0, 28.0, 0.18, 0.8, 0.12));
            }
        }
        Sound::Damage => {
            put(blip(Wave::Noise, 900.0, 900.0, 0.25, 0.12, 0.0));
            put(blip(Wave::Pulse, 320.0, 100.0, 0.14, 0.16, 0.0));
        }
        Sound::Coin(combo) => {
            let note = [76, 79, 83, 86, 88][combo.saturating_sub(1).min(9) as usize % 5];
            put(blip(
                Wave::Pulse,
                frequency(note),
                frequency(note),
                0.10,
                0.055,
                0.0,
            ));
            put(blip(
                Wave::Sine,
                frequency(note + 12),
                frequency(note + 12),
                0.19,
                0.16,
                0.06,
            ));
        }
        Sound::Pickup
        | Sound::Upgrade
        | Sound::Support
        | Sound::Overdrive
        | Sound::Gate(_)
        | Sound::Phase
        | Sound::OrbitArrival
        | Sound::Landing => {
            let notes: &[u8] = match sound {
                Sound::Upgrade => &[64, 67, 71, 76, 83],
                Sound::Support => &[71, 76, 79, 83],
                Sound::Overdrive => &[52, 64, 71, 76, 83, 88],
                Sound::Gate(true) => &[79, 83, 86, 91],
                Sound::Gate(false) => &[76, 83],
                Sound::Phase => &[64, 71, 76, 79, 83, 88],
                Sound::OrbitArrival => &[60, 67, 72, 76, 79, 84, 88, 91],
                Sound::Landing => &[67, 72, 76, 79],
                _ => &[71, 76, 83],
            };
            let (wave, spacing, hold) = match sound {
                Sound::OrbitArrival => (Wave::Sine, 0.09, 0.55),
                Sound::Landing => (Wave::Triangle, 0.11, 0.4),
                _ => (Wave::Triangle, 0.065, 0.16),
            };
            for (i, n) in notes.iter().enumerate() {
                put(blip(
                    wave,
                    frequency(*n),
                    frequency(*n),
                    0.23,
                    hold,
                    i as f32 * spacing,
                )
                .pan(if i % 2 == 0 { -0.3 } else { 0.3 }));
            }
            if sound == Sound::OrbitArrival {
                // 入轨时的星光余韵
                put(blip(Wave::Fm, 1046.0, 2093.0, 0.07, 1.2, 0.5).shape(0.6));
            }
        }
        Sound::Nova => {
            put(blip(Wave::Fm, 80.0, 1200.0, 0.30, 0.22, 0.0));
            put(blip(Wave::Noise, 120.0, 120.0, 0.40, 0.75, 0.18));
            put(blip(Wave::Sine, 140.0, 30.0, 0.38, 0.6, 0.18));
        }
        Sound::Boss => {
            for i in 0..3 {
                put(blip(Wave::Pulse, 330.0, 220.0, 0.14, 0.16, i as f32 * 0.25));
                put(blip(
                    Wave::Triangle,
                    165.0,
                    110.0,
                    0.12,
                    0.16,
                    i as f32 * 0.25,
                ));
            }
        }
        Sound::BoostStart => {
            // 加力点火：低频闷响后接上扬的气流
            put(blip(Wave::Sine, 55.0, 110.0, 0.20, 0.22, 0.0));
            put(blip(Wave::Noise, 0.0, 0.0, 0.16, 0.5, 0.0)
                .shape(0.35)
                .attack(0.06));
            put(blip(Wave::Saw, 90.0, 260.0, 0.06, 0.42, 0.02));
        }
        Sound::OrbitReady => {
            // 达到入轨高度：两声确认与高音提示
            put(blip(Wave::Sine, 880.0, 880.0, 0.16, 0.09, 0.0));
            put(blip(Wave::Sine, 1175.0, 1175.0, 0.16, 0.09, 0.12));
            put(blip(Wave::Triangle, 1760.0, 1760.0, 0.14, 0.22, 0.24));
        }
        Sound::OrbitIgnite => {
            // 轨道推进点火：火箭轰鸣与上升音
            put(blip(Wave::Sine, 70.0, 40.0, 0.34, 0.3, 0.0));
            put(blip(Wave::Noise, 0.0, 0.0, 0.34, 1.3, 0.0)
                .shape(0.05)
                .attack(0.08));
            put(blip(Wave::Noise, 0.0, 0.0, 0.10, 0.9, 0.05).shape(0.4));
            put(blip(Wave::Fm, 60.0, 260.0, 0.13, 1.0, 0.1));
            put(blip(Wave::Saw, 40.0, 72.0, 0.10, 1.1, 0.0));
        }
        Sound::Breakthrough => {
            // 冲出大气层：音爆后风声迅速远去
            put(blip(Wave::Noise, 0.0, 0.0, 0.32, 0.07, 0.0).shape(-0.6));
            put(blip(Wave::Sine, 90.0, 30.0, 0.32, 0.35, 0.0));
            put(blip(Wave::Noise, 0.0, 0.0, 0.18, 1.2, 0.05)
                .shape(0.22)
                .attack(0.02));
            put(blip(Wave::Sine, 1400.0, 300.0, 0.08, 0.9, 0.1));
        }
        Sound::Reentry => {
            // 再入大气：低频摩擦与下滑的热浪
            put(blip(Wave::Noise, 0.0, 0.0, 0.28, 1.6, 0.0)
                .shape(0.07)
                .attack(0.4));
            put(blip(Wave::Fm, 320.0, 80.0, 0.11, 1.4, 0.0).attack(0.2));
            put(blip(Wave::Noise, 0.0, 0.0, 0.08, 1.2, 0.3).shape(0.5));
        }
        Sound::EnemyFire(heavy) => {
            if heavy {
                put(blip(Wave::Fm, 260.0, 90.0, 0.12, 0.18, 0.0).pan(0.2));
                put(blip(Wave::Noise, 0.0, 0.0, 0.07, 0.12, 0.0).shape(0.3));
                put(blip(Wave::Pulse, 520.0, 180.0, 0.05, 0.14, 0.04).pan(-0.2));
            } else {
                put(blip(Wave::Pulse, 900.0, 380.0, 0.06, 0.06, 0.0)
                    .shape(0.2)
                    .pan(0.25));
            }
        }
        Sound::Graze => {
            put(blip(Wave::Sine, 2400.0, 3200.0, 0.07, 0.04, 0.0));
            put(blip(Wave::Noise, 0.0, 0.0, 0.05, 0.06, 0.0).shape(-0.4));
        }
        Sound::Flyby => {
            // 敌机贴身掠过：风压渐强后多普勒下滑
            put(blip(Wave::Noise, 0.0, 0.0, 0.15, 0.55, 0.0)
                .shape(0.16)
                .attack(0.18));
            put(blip(Wave::Saw, 240.0, 120.0, 0.045, 0.5, 0.05).attack(0.1));
        }
        Sound::WeaponSwitch(weapon) => {
            let note = [64, 67, 71, 74, 76][weapon as usize % 5];
            put(blip(Wave::Noise, 0.0, 0.0, 0.10, 0.02, 0.0).shape(0.7));
            put(blip(
                Wave::Pulse,
                frequency(note),
                frequency(note),
                0.09,
                0.05,
                0.03,
            ));
            put(blip(
                Wave::Pulse,
                frequency(note + 7),
                frequency(note + 7),
                0.09,
                0.07,
                0.08,
            ));
        }
        Sound::Toggle(enabled) => {
            let (from, to) = if enabled {
                (520.0, 780.0)
            } else {
                (780.0, 520.0)
            };
            put(blip(Wave::Triangle, from, to, 0.13, 0.07, 0.0));
            put(blip(Wave::Noise, 0.0, 0.0, 0.05, 0.02, 0.0).shape(0.7));
        }
        Sound::View => {
            put(blip(Wave::Pulse, 1400.0, 1400.0, 0.05, 0.025, 0.0));
            put(blip(Wave::Noise, 0.0, 0.0, 0.06, 0.03, 0.0).shape(-0.3));
        }
        Sound::Wave => {
            for i in 0..2 {
                let at = i as f32 * 0.34;
                put(blip(Wave::Pulse, 740.0, 740.0, 0.09, 0.14, at));
                put(blip(Wave::Pulse, 587.0, 587.0, 0.09, 0.14, at + 0.16));
            }
        }
        Sound::Destroyed => {
            put(blip(Wave::Noise, 0.0, 0.0, 0.42, 1.4, 0.0).shape(0.1));
            put(blip(Wave::Sine, 110.0, 30.0, 0.4, 0.9, 0.0));
            put(blip(Wave::Fm, 420.0, 40.0, 0.14, 1.3, 0.05));
            put(blip(Wave::Noise, 0.0, 0.0, 0.18, 0.6, 0.35).shape(0.4));
        }
    }
}
