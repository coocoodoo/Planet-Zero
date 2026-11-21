use super::{
    Cue, Frame, LAYERS, Sound, THROTTLE_SLOTS, effects,
    oscillator::{Oscillator, Tables, Tone, Voice, Wave, frequency},
    playlist, score, scores,
};

fn ramp(value: f32, from: f32, to: f32) -> f32 {
    ((value - from) / (to - from)).clamp(0.0, 1.0)
}

/// 各编配层的目标音量：探索随时间段与天气变化，竞速和战斗随激烈程度逐层加入
pub(super) fn layer_targets(cue: Cue, frame: &Frame) -> [f32; LAYERS] {
    let mut g = [1.0; LAYERS];
    match cue {
        Cue::Flight => {
            let (dusk, night) = (frame.twilight, frame.night);
            g[score::DRUMS as usize] = (1.0 - 0.45 * dusk - 0.9 * night) * (1.0 - 0.7 * frame.snow);
            g[score::HATS as usize] = (1.0 - 0.3 * dusk - 0.55 * night) * (1.0 - 0.4 * frame.snow);
            g[score::ARP as usize] = 1.0 - 0.2 * dusk - 0.6 * night;
            g[score::LEAD as usize] = (1.0 - 0.35 * night) * (1.0 - 0.25 * frame.haze);
            g[score::BASS as usize] = 1.0 - 0.25 * night;
            g[score::PAD as usize] = 1.0 + 0.35 * night + 0.25 * frame.rain;
        }
        Cue::Race | Cue::Coins | Cue::Combat | Cue::Boss => {
            let i = frame.intensity;
            g[score::BASS as usize] = 0.6 + 0.4 * ramp(i, 0.0, 0.2);
            g[score::HATS as usize] = 0.5 + 0.5 * ramp(i, 0.05, 0.3);
            g[score::DRUMS as usize] = 0.3 + 0.7 * ramp(i, 0.25, 0.5);
            g[score::ARP as usize] = 0.15 + 0.85 * ramp(i, 0.4, 0.65);
            g[score::LEAD as usize] = ramp(i, 0.55, 0.8);
            g[score::COUNTER as usize] = ramp(i, 0.7, 0.95);
        }
        _ => {}
    }
    g
}

/// 机型引擎音色：基频、锯齿、三角、风噪、噪声亮度
const ENGINES: [(f32, f32, f32, f32, f32); 5] = [
    (65.0, 0.012, 0.020, 0.018, 0.06),
    (82.0, 0.016, 0.014, 0.016, 0.09),
    (48.0, 0.010, 0.026, 0.022, 0.04),
    (98.0, 0.019, 0.010, 0.014, 0.12),
    (60.0, 0.002, 0.004, 0.030, 0.05),
];

/// 同一场景内换地图或天气后，新环境需保持的秒数才换曲
const SCENE_DWELL: f64 = 8.0;
/// 因环境变化换曲前，当前曲目至少播放的秒数
const MINIMUM_PLAY: f64 = 25.0;

/// 一条配乐播放轨；换曲时旧轨淡出、新轨淡入，两轨同时发声
#[derive(Clone, Copy)]
struct Deck {
    track: usize,
    step: usize,
    until: f64,
    gain: f32,
    fade: f32,
    playing: bool,
    started: f64,
}
impl Deck {
    fn new(track: usize, step: usize, started: f64) -> Self {
        Self {
            track,
            step,
            until: 0.0,
            gain: 1.0,
            fade: 0.0,
            playing: true,
            started,
        }
    }
}

pub struct Mixer {
    rate: f32,
    tables: Tables,
    voices: [Option<Voice>; 64],
    frame: Frame,
    cue: Cue,
    decks: [Deck; 2],
    live: usize,
    shuffle: u32,
    pending: Option<Cue>,
    unfit_since: Option<f64>,
    clock: f64,
    last_effect: [f64; THROTTLE_SLOTS],
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
    engine: [Oscillator; 7],
    rain: f32,
    wind: f32,
    haze: f32,
    night: f32,
    layers: [f32; LAYERS],
    engine_gain: f32,
    boost: f32,
    ascent: f32,
    vacuum: f32,
    pub peak_voices: usize,
    pub transitions: u32,
    pub rotations: u32,
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
            decks: [
                Deck::new(
                    playlist(Cue::Menu, crate::world::MapKind::Islands)[0],
                    0,
                    0.0,
                ),
                Deck {
                    playing: false,
                    ..Deck::new(0, 0, 0.0)
                },
            ],
            live: 0,
            shuffle: 0x9e37_79b9,
            pending: None,
            unfit_since: None,
            clock: 0.0,
            last_effect: [-10.0; THROTTLE_SLOTS],
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
                Oscillator::new(911),
                Oscillator::new(1291),
                Oscillator::new(1777),
                Oscillator::new(2333),
            ],
            rain: 0.0,
            wind: 0.0,
            haze: 0.0,
            night: 0.0,
            layers: [1.0; LAYERS],
            engine_gain: 0.0,
            boost: 0.0,
            ascent: 0.0,
            vacuum: 0.0,
            peak_voices: 0,
            transitions: 0,
            rotations: 0,
        }
    }
    /// 设定选曲随机种子；实时输出用时间播种，测试与导出保持确定
    pub fn reseed(&mut self, seed: u32) {
        self.shuffle = seed | 1;
    }
    pub fn cue(&self) -> Cue {
        self.cue
    }
    pub fn track(&self) -> usize {
        self.decks[self.live].track
    }
    /// 正在发声的播放轨数量，交叉淡化期间为 2
    pub fn decks_playing(&self) -> usize {
        self.decks.iter().filter(|d| d.playing).count()
    }
    fn random(&mut self) -> u32 {
        self.shuffle ^= self.shuffle << 13;
        self.shuffle ^= self.shuffle >> 17;
        self.shuffle ^= self.shuffle << 5;
        self.shuffle
    }
    /// 从场景曲库随机选曲，曲库多于一首时避开正在播放的曲目
    fn choose(&mut self, cue: Cue, exclude: Option<usize>) -> usize {
        let mut list = playlist(cue, self.frame.map);
        if list.len() > 1
            && let Some(current) = exclude
        {
            list.retain(|i| *i != current);
        }
        list[self.random() as usize % list.len()]
    }
    pub fn step(&self) -> usize {
        self.decks[self.live].step
    }
    pub fn update(&mut self, frame: Frame) {
        let fits = frame.cue == self.cue && playlist(frame.cue, frame.map).contains(&self.track());
        self.pending = if fits {
            self.unfit_since = None;
            None
        } else if frame.cue != self.cue {
            // 场景变化在下一拍交叉淡化
            self.unfit_since = None;
            Some(frame.cue)
        } else {
            // 同一场景换地图或天气：新环境稳定且当前曲目播放够久才换，快速穿梭虫洞时保持原曲
            let since = *self.unfit_since.get_or_insert(self.clock);
            let played = self.clock - self.decks[self.live].started;
            (self.clock - since >= SCENE_DWELL && played >= MINIMUM_PLAY).then_some(frame.cue)
        };
        if frame.settings.effects() {
            for event in frame.events.iter() {
                self.play(event);
            }
        }
        self.frame = frame;
    }
    pub fn start_cue(&mut self, cue: Cue) {
        let track = self.choose(cue, None);
        self.start(cue, track);
    }
    /// 直接播放指定曲目，用于导出与试听
    pub fn start_track(&mut self, track: usize) {
        self.start(scores()[track].cues[0], track);
    }
    fn start(&mut self, cue: Cue, track: usize) {
        self.silence(0);
        self.silence(1);
        self.live = 0;
        self.decks[0] = Deck::new(track, 0, self.clock);
        self.decks[1].playing = false;
        self.cue = cue;
        self.pending = None;
        self.unfit_since = None;
        self.frame.cue = cue;
    }
    /// 立即移除一条播放轨的全部配乐声部
    fn silence(&mut self, deck: usize) {
        for slot in &mut self.voices {
            if slot
                .as_ref()
                .is_some_and(|v| v.tone.priority == 1 && v.deck as usize == deck)
            {
                *slot = None;
            }
        }
    }
    /// 旧曲在原有节拍上继续演奏并淡出，新曲从主题段淡入
    fn crossfade(&mut self, next: Cue) {
        let (fade_out, fade_in) = if next.urgent() {
            (0.9, 0.25)
        } else {
            (2.6, 1.4)
        };
        let old = self.live;
        let new = 1 - old;
        if self.decks[new].playing {
            self.silence(new);
        }
        let exclude = (next == self.cue).then_some(self.track());
        let track = self.choose(next, exclude);
        let entry = if next.rotates() {
            scores()[track].entry
        } else {
            0
        };
        self.decks[old].fade = -1.0 / fade_out;
        self.decks[new] = Deck {
            gain: 0.0,
            fade: 1.0 / fade_in,
            ..Deck::new(track, entry, self.clock)
        };
        self.live = new;
        self.cue = next;
        self.pending = None;
        self.unfit_since = None;
        self.transitions += 1;
    }
    fn add(&mut self, tone: Tone) {
        self.add_to(tone, 0);
    }
    fn add_to(&mut self, tone: Tone, deck: usize) {
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
            let mut voice = Voice::new(tone, self.seed);
            voice.deck = deck as u8;
            self.voices[slot] = Some(voice);
        }
    }
    fn play(&mut self, sound: Sound) {
        let (index, cooldown) = sound.throttle();
        if self.clock - self.last_effect[index] < cooldown {
            return;
        }
        self.last_effect[index] = self.clock;
        if matches!(
            sound,
            Sound::Nova
                | Sound::Boss
                | Sound::Explosion(true)
                | Sound::Damage
                | Sound::Breakthrough
                | Sound::Destroyed
        ) {
            self.duck = 0.42;
        }
        effects::tones(sound, |tone| self.add(tone));
    }
    fn tick(&mut self, d: usize) {
        if d == self.live
            && let Some(next) = self.pending
            && self.decks[d].step.is_multiple_of(4)
        {
            self.crossfade(next);
        }
        let live = d == self.live;
        if self.decks[d].step >= scores()[self.decks[d].track].steps.len() {
            // 整首播完后换一首同场景曲目，结算曲循环尾段
            if live && self.cue.rotates() && playlist(self.cue, self.frame.map).len() > 1 {
                let track = self.choose(self.cue, Some(self.decks[d].track));
                self.decks[d].track = track;
                self.decks[d].started = self.clock;
                self.rotations += 1;
                self.decks[d].step = 0;
            } else {
                self.decks[d].step = scores()[self.decks[d].track].loop_start;
            }
        }
        let Deck { track, step, .. } = self.decks[d];
        let score = &scores()[track];
        for tone in &score.steps[step] {
            self.add_to(*tone, d);
        }
        if live {
            self.overlays(d, step, score.chords[step / 16]);
        }
        self.decks[d].step += 1;
        self.decks[d].until += self.rate as f64 * 15.0 / score.bpm as f64;
        self.peak_voices = self.peak_voices.max(self.voices.iter().flatten().count());
    }
    fn overlay(&mut self, mut tone: Tone, d: usize) {
        tone.layer = score::OVERLAY;
        self.add_to(tone, d);
    }
    /// 实时点缀声部：超载琶音、危险节奏、升空上行、雪天钟声、雷暴定音鼓与竞速冲刺
    fn overlays(&mut self, d: usize, step: usize, chord: [u8; 4]) {
        let f = self.frame;
        let beat = step % 16;
        if matches!(self.cue, Cue::Combat | Cue::Boss) {
            if f.overdrive && step % 2 == 1 {
                let mut tone =
                    Tone::new(Wave::Pulse, frequency(chord[step % 4] + 24), 0.045, 0.065);
                tone.pan = if step % 4 == 1 { -0.6 } else { 0.6 };
                tone.shape = 0.125;
                self.overlay(tone, d);
            }
            if f.danger > 0.2 && (beat == 2 || beat == 10) {
                let mut tone = Tone::new(Wave::Fm, 140.0, f.danger * 0.12, 0.16);
                tone.end_hz = 65.0;
                tone.decay = 0.12;
                tone.sustain = 0.0;
                self.overlay(tone, d);
            }
        }
        // 升空：鼓点加密，琶音随高度逐级升高，接近入轨时小鼓滚奏
        if f.riser > 0.01 && self.cue == Cue::Flight {
            let level = f.riser;
            let height = f.altitude.clamp(0.0, 1.0);
            let mut hat = Tone::new(Wave::Noise, 8000.0, 0.03 * level, 0.02);
            hat.shape = -0.2;
            hat.sustain = 0.0;
            hat.decay = 0.02;
            hat.pan = 0.3;
            self.overlay(hat, d);
            if step % 4 == 0 {
                let mut kick = Tone::new(Wave::Sine, 150.0, 0.34 * level, 0.12);
                kick.end_hz = 44.0;
                kick.decay = 0.1;
                kick.sustain = 0.05;
                self.overlay(kick, d);
            }
            if step % 2 == 0 {
                let octave = 12 * (1 + (height * 3.0) as u8).min(3);
                let note = chord[(step / 2) % 4] + octave;
                let mut arp = Tone::new(Wave::Saw, frequency(note), 0.035 * level, 0.09);
                arp.pan = if step % 4 == 0 { -0.5 } else { 0.5 };
                arp.sustain = 0.2;
                arp.send = 0.2;
                self.overlay(arp, d);
            }
            if height > 0.8 && step % 2 == 1 {
                let mut roll = Tone::new(Wave::Noise, 1200.0, 0.06 * level * height, 0.05);
                roll.shape = 0.55;
                roll.sustain = 0.0;
                self.overlay(roll, d);
            }
        }
        // 雪天：高音钟声零星落下
        if f.snow > 0.05 && self.cue == Cue::Flight && (beat == 0 || beat == 11) {
            let note = chord[(step / 16 + beat) % 4] + 36;
            let mut bell = Tone::new(Wave::Fm, frequency(note), 0.07 * f.snow, 0.5);
            bell.shape = 0.9;
            bell.decay = 0.4;
            bell.sustain = 0.1;
            bell.release = 0.4;
            bell.send = 0.4;
            bell.pan = if beat == 0 { -0.4 } else { 0.4 };
            self.overlay(bell, d);
        }
        // 雷暴：隔小节一声低沉的定音鼓
        if f.storm > 0.05 && self.cue == Cue::Flight && step % 32 == 0 {
            let mut drum = Tone::new(Wave::Sine, 82.0, 0.3 * f.storm, 0.6);
            drum.end_hz = 52.0;
            drum.attack = 0.004;
            drum.decay = 0.5;
            drum.sustain = 0.1;
            drum.release = 0.3;
            self.overlay(drum, d);
        }
        // 竞速冲刺与金币倒计时：十六分镲片与高八度琶音
        if f.finale > 0.01 && matches!(self.cue, Cue::Race | Cue::Coins) {
            let mut hat = Tone::new(Wave::Noise, 8000.0, 0.03 * f.finale, 0.02);
            hat.shape = -0.2;
            hat.sustain = 0.0;
            hat.decay = 0.02;
            self.overlay(hat, d);
            if step % 2 == 1 {
                let mut arp = Tone::new(
                    Wave::Pulse,
                    frequency(chord[step % 4] + 36),
                    0.03 * f.finale,
                    0.06,
                );
                arp.shape = 0.125;
                arp.pan = if step % 4 == 1 { -0.6 } else { 0.6 };
                self.overlay(arp, d);
            }
        }
    }
    pub fn render(&mut self, destination: &mut [f32]) {
        for stereo in destination.chunks_exact_mut(2) {
            let value = self.sample();
            stereo.copy_from_slice(&value);
        }
    }
    pub fn sample(&mut self) -> [f32; 2] {
        for d in 0..2 {
            if self.decks[d].playing {
                if self.decks[d].until <= 0.0 {
                    self.tick(d);
                }
                self.decks[d].until -= 1.0;
            }
        }
        let dt = 1.0 / self.rate;
        // 时间段与天气缓慢过渡，战斗强度变化更快
        let targets = layer_targets(self.cue, &self.frame);
        let speed = if self.cue == Cue::Flight { 0.5 } else { 1.6 };
        for (layer, target) in self.layers.iter_mut().zip(targets) {
            *layer += (target - *layer) * (dt * speed).min(1.0);
        }
        self.night += (self.frame.night - self.night) * dt * 0.5;
        let mut gains = [0.0; 2];
        for d in 0..2 {
            let deck = &mut self.decks[d];
            deck.gain = (deck.gain + deck.fade * dt).clamp(0.0, 1.0);
            if deck.fade > 0.0 && deck.gain >= 1.0 {
                deck.fade = 0.0;
            }
            if deck.fade < 0.0 && deck.gain <= 0.0 && deck.playing {
                deck.playing = false;
                self.silence(d);
            }
            // 等功率曲线，交叉过程中总响度保持平稳
            let g = self.decks[d].gain;
            gains[d] = g * (2.0 - g);
        }
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
                let mut value = voice.sample(dt, &self.tables);
                if voice.tone.priority == 1 {
                    let gain = gains[voice.deck as usize & 1]
                        * self.layers[voice.tone.layer as usize % LAYERS];
                    value = [value[0] * gain, value[1] * gain];
                }
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
            music[0] * (0.055 + 0.04 * self.night) + room[1] * 0.48,
            music[1] * (0.055 + 0.04 * self.night) + room[0] * 0.48,
        ];
        self.reverb_index = (self.reverb_index + 1) % self.reverb.len();
        self.engine_gain += (self.frame.engine - self.engine_gain) * dt * 6.0;
        self.boost += (self.frame.boost - self.boost) * dt * 8.0;
        self.ascent += (self.frame.ascent - self.ascent) * dt * 3.0;
        self.vacuum += (f32::from(self.frame.vacuum) - self.vacuum) * dt * 1.5;
        let (base, saw, triangle, wind, brightness) =
            ENGINES[self.frame.vehicle as usize % ENGINES.len()];
        let hz = base + self.engine_gain * base * 0.5 + self.boost * 85.0;
        // 真空中没有风噪，只剩机体传导的低沉涡轮声
        let air = 1.0 - self.vacuum * 0.9;
        let turbine = self.engine[0].sample(Wave::Saw, hz * dt, 0.5, &self.tables) * saw
            + self.engine[1].sample(Wave::Triangle, hz * 2.01 * dt, 0.5, &self.tables) * triangle
            + self.engine[2].sample(
                Wave::Noise,
                1000.0 * dt,
                brightness + self.boost * 0.25,
                &self.tables,
            ) * (wind + self.boost * 0.065)
                * air;
        // 轨道推进：火箭低频轰鸣，随高度升高的气流哨音
        let rumble = self.engine[3].sample(Wave::Noise, 0.0, 0.035, &self.tables)
            * self.ascent
            * (0.16 - self.frame.altitude.clamp(0.0, 1.0) * 0.08);
        let whistle = self.engine[4].sample(
            Wave::Sine,
            (220.0 + self.frame.altitude.clamp(0.0, 1.0) * 760.0) * dt,
            0.5,
            &self.tables,
        ) * self.ascent
            * 0.016
            * air;
        // 天气环境声：雨滴沙沙声与时强时弱的风声，真空中消失
        self.rain += (self.frame.rain - self.rain) * dt * 0.8;
        self.wind += (self.frame.wind - self.wind) * dt * 0.8;
        self.haze += (self.frame.haze - self.haze) * dt * 0.5;
        let hiss = self.engine[5].sample(Wave::Noise, 0.0, -0.3, &self.tables) * self.rain * 0.022;
        let gust = 0.65 + 0.35 * self.tables.sine(self.clock as f32 * 0.13);
        let gale =
            self.engine[6].sample(Wave::Noise, 0.0, 0.025, &self.tables) * self.wind * gust * 0.09;
        let engine = turbine * self.engine_gain + rumble + whistle + (hiss + gale) * air;
        let mut result = [0.0; 2];
        for channel in 0..2 {
            let wet = music[channel] + echo[channel] + room[channel];
            self.filtered[channel] += (wet - self.filtered[channel])
                * if self.frame.paused {
                    0.055
                } else {
                    // 真空与雾、雪、沙尘让配乐更暗更闷
                    0.65 - self.vacuum * 0.3 - self.haze * 0.22 - self.night * 0.12
                };
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
