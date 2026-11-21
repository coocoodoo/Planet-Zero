use super::oscillator::{Tone, Wave, frequency};
use crate::world::MapKind;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Menu,
    Flight,
    Race,
    Coins,
    Combat,
    Boss,
    Victory,
    Defeat,
}
impl Cue {
    pub const ALL: [Self; 8] = [
        Self::Menu,
        Self::Flight,
        Self::Race,
        Self::Coins,
        Self::Combat,
        Self::Boss,
        Self::Victory,
        Self::Defeat,
    ];
    pub fn name(self) -> &'static str {
        [
            "menu", "flight", "race", "coins", "combat", "boss", "victory", "defeat",
        ][self as usize]
    }
    fn parse(name: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|cue| cue.name() == name)
            .unwrap_or_else(|| panic!("未知配乐场景 {name}"))
    }
    /// 结算曲播完后循环尾段，其余场景播完整首后随机换曲
    pub fn rotates(self) -> bool {
        !matches!(self, Self::Victory | Self::Defeat)
    }
    /// 结算曲快速切入
    pub fn urgent(self) -> bool {
        matches!(self, Self::Victory | Self::Defeat)
    }
}

/// 编配层：混音器按时间段、天气与战斗强度分别调节音量
pub const LEAD: u8 = 0;
pub const COUNTER: u8 = 1;
pub const PAD: u8 = 2;
pub const BASS: u8 = 3;
pub const ARP: u8 = 4;
pub const DRUMS: u8 = 5;
pub const HATS: u8 = 6;
/// 混音器实时生成的点缀声部，不受编配层调节
pub const OVERLAY: u8 = 7;
pub const LAYERS: usize = 8;

const MAP_NAMES: [&str; 12] = [
    "islands",
    "alpine",
    "canyon",
    "desert",
    "wetlands",
    "volcanic",
    "moon",
    "space",
    "fjords",
    "icebergs",
    "city",
    "highlands",
];
fn parse_map(name: &str) -> MapKind {
    MAP_NAMES
        .iter()
        .position(|n| *n == name)
        .map(|i| MapKind::ALL[i])
        .unwrap_or_else(|| panic!("未知地图 {name}"))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    phrase: usize,
    energy: f32,
    lead: bool,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    title: String,
    bpm: u32,
    style: String,
    cues: Vec<String>,
    #[serde(default)]
    maps: Vec<String>,
    loop_start: usize,
    lead: Option<String>,
    lead_duty: Option<f32>,
    bass: Option<String>,
    bass_line: Option<String>,
    groove: Option<String>,
    arp: Option<String>,
    arp_wave: Option<String>,
    #[serde(default)]
    swing: f32,
    #[serde(default = "one")]
    pad: f32,
    counter_wave: Option<String>,
    sections: Vec<Section>,
    melody: Vec<String>,
    harmony: Vec<String>,
    #[serde(default)]
    counter: Vec<String>,
}
fn one() -> f32 {
    1.0
}

pub struct Score {
    pub title: String,
    pub bpm: u32,
    pub loop_start: usize,
    /// 切换进入时的起始步：跳过无旋律或低能量的前奏，直接进入主题
    pub entry: usize,
    pub steps: Vec<Vec<Tone>>,
    pub chords: Vec<[u8; 4]>,
    pub cues: Vec<Cue>,
    pub maps: Vec<MapKind>,
}
impl Score {
    pub fn seconds(&self) -> f32 {
        self.steps.len() as f32 * 15.0 / self.bpm as f32
    }
}

const SOURCES: [&str; 27] = [
    include_str!("../../content/music/menu.toml"),
    include_str!("../../content/music/hangar-lights.toml"),
    include_str!("../../content/music/glacier-drift.toml"),
    include_str!("../../content/music/torii-festival.toml"),
    include_str!("../../content/music/fuji-morning.toml"),
    include_str!("../../content/music/red-mesa.toml"),
    include_str!("../../content/music/lotus-pavilion.toml"),
    include_str!("../../content/music/rue-de-lumiere.toml"),
    include_str!("../../content/music/lighthouse-keeper.toml"),
    include_str!("../../content/music/windmill-reel.toml"),
    include_str!("../../content/music/mirage-dunes.toml"),
    include_str!("../../content/music/magma-core.toml"),
    include_str!("../../content/music/sea-of-tranquility.toml"),
    include_str!("../../content/music/orbital-silence.toml"),
    include_str!("../../content/music/starlane.toml"),
    include_str!("../../content/music/race.toml"),
    include_str!("../../content/music/checkpoint-fever.toml"),
    include_str!("../../content/music/coin-cascade.toml"),
    include_str!("../../content/music/combat.toml"),
    include_str!("../../content/music/steel-rain.toml"),
    include_str!("../../content/music/crimson-horizon.toml"),
    include_str!("../../content/music/boss.toml"),
    include_str!("../../content/music/leviathan.toml"),
    include_str!("../../content/music/victory.toml"),
    include_str!("../../content/music/clear-skies.toml"),
    include_str!("../../content/music/defeat.toml"),
    include_str!("../../content/music/long-way-home.toml"),
];

pub fn scores() -> &'static [Score] {
    static SCORES: OnceLock<Vec<Score>> = OnceLock::new();
    SCORES.get_or_init(|| {
        SOURCES
            .iter()
            .map(|source| compile(toml::from_str(source).expect("音乐乐谱格式错误")))
            .collect()
    })
}

/// 场景曲库：地图专属曲目优先，没有专属时使用通用曲目
pub fn playlist(cue: Cue, map: MapKind) -> Vec<usize> {
    let tracks = scores();
    let of_cue = |i: &usize| tracks[*i].cues.contains(&cue);
    let specific: Vec<usize> = (0..tracks.len())
        .filter(of_cue)
        .filter(|i| tracks[*i].maps.contains(&map))
        .collect();
    if !specific.is_empty() {
        return specific;
    }
    let generic: Vec<usize> = (0..tracks.len())
        .filter(of_cue)
        .filter(|i| tracks[*i].maps.is_empty())
        .collect();
    if !generic.is_empty() {
        return generic;
    }
    (0..tracks.len()).filter(of_cue).collect()
}

fn midi(name: &str) -> u8 {
    let bytes = name.as_bytes();
    let mut semitone = match bytes[0] {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => panic!("未知音名 {name}"),
    };
    if bytes.get(1) == Some(&b'#') {
        semitone += 1;
    }
    let octave = (bytes[bytes.len() - 1] - b'0') as i32;
    ((octave + 1) * 12 + semitone) as u8
}

fn wave(name: &str) -> Wave {
    match name {
        "pulse" => Wave::Pulse,
        "saw" => Wave::Saw,
        "triangle" => Wave::Triangle,
        "sine" => Wave::Sine,
        "fm" => Wave::Fm,
        _ => panic!("未知音色 {name}"),
    }
}

/// 民族与原声乐器的合成参数：拨弦类快速衰减，吹奏与拉弦类带颤音和气声
#[derive(Clone, Copy)]
struct Instrument {
    wave: Wave,
    shape: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    vibrato: f32,
    gain: f32,
    send: f32,
    /// 第二个略微失谐的簧片，手风琴的颤音感
    detune: f32,
    /// 起音时的气声噪音
    breath: f32,
}
impl Instrument {
    fn apply(self, t: &mut Tone) {
        t.wave = self.wave;
        t.shape = self.shape;
        t.attack = self.attack;
        t.decay = self.decay;
        t.sustain = self.sustain;
        t.release = self.release;
        t.vibrato = self.vibrato;
        t.gain = self.gain;
        t.send = self.send;
    }
    /// 推入主音及失谐簧片、气声等附属声部
    fn push(self, t: Tone, slot: &mut Vec<Tone>) {
        slot.push(t);
        if self.detune > 0.0 {
            let mut twin = t;
            twin.hz *= 1.0 + self.detune;
            twin.end_hz = twin.hz;
            twin.gain *= 0.6;
            twin.pan = -t.pan;
            slot.push(twin);
        }
        if self.breath > 0.0 {
            let mut air = Tone::new(Wave::Noise, 1000.0, self.breath, t.hold.min(0.12));
            air.shape = 0.35;
            air.attack = 0.01;
            air.decay = 0.08;
            air.sustain = 0.0;
            air.pan = t.pan;
            air.layer = t.layer;
            slot.push(air);
        }
    }
}
fn instrument(name: &str) -> Option<Instrument> {
    let pluck = |wave, shape, decay, gain| Instrument {
        wave,
        shape,
        attack: 0.002,
        decay,
        sustain: 0.05,
        release: 0.12,
        vibrato: 0.0,
        gain,
        send: 0.25,
        detune: 0.0,
        breath: 0.0,
    };
    let blown = |wave, vibrato, gain, breath| Instrument {
        wave,
        shape: 0.5,
        attack: 0.04,
        decay: 0.1,
        sustain: 0.8,
        release: 0.09,
        vibrato,
        gain,
        send: 0.3,
        detune: 0.0,
        breath,
    };
    Some(match name {
        "koto" => pluck(Wave::Pulse, 0.18, 0.28, 0.2),
        "harp" => pluck(Wave::Triangle, 0.5, 0.5, 0.26),
        "oud" => pluck(Wave::Saw, 0.5, 0.22, 0.12),
        "guitar" => Instrument {
            sustain: 0.12,
            vibrato: 0.003,
            send: 0.18,
            ..pluck(Wave::Pulse, 0.35, 0.35, 0.2)
        },
        "flute" => blown(Wave::Sine, 0.009, 0.26, 0.05),
        "whistle" => blown(Wave::Triangle, 0.006, 0.22, 0.04),
        "fiddle" => Instrument {
            attack: 0.06,
            send: 0.25,
            ..blown(Wave::Saw, 0.011, 0.14, 0.0)
        },
        "erhu" => Instrument {
            attack: 0.05,
            send: 0.28,
            ..blown(Wave::Saw, 0.016, 0.13, 0.0)
        },
        "accordion" => Instrument {
            shape: 0.3,
            attack: 0.02,
            decay: 0.05,
            sustain: 0.85,
            release: 0.06,
            send: 0.15,
            detune: 0.007,
            ..blown(Wave::Pulse, 0.0, 0.1, 0.0)
        },
        "bell" => Instrument {
            shape: 0.9,
            decay: 0.6,
            sustain: 0.1,
            release: 0.5,
            send: 0.35,
            ..pluck(Wave::Fm, 0.9, 0.6, 0.14)
        },
        _ => return None,
    })
}

/// 鼓组每小节十六步的位掩码
struct Groove {
    kick: u16,
    snare: u16,
    ghost: u16,
    hat: u16,
    dense: u16,
    open: u16,
    low_tom: u16,
    high_tom: u16,
    /// 木鱼、梆子等短促的木质敲击
    block: u16,
    /// 沙锤与刷子
    shaker: u16,
}
const fn mask(steps: &[u8]) -> u16 {
    let mut m = 0;
    let mut i = 0;
    while i < steps.len() {
        m |= 1 << steps[i];
        i += 1;
    }
    m
}
const EIGHTHS: u16 = mask(&[0, 2, 4, 6, 8, 10, 12, 14]);
const OFFBEATS: u16 = mask(&[1, 3, 5, 7, 9, 11, 13, 15]);
fn groove(name: &str) -> Option<Groove> {
    let none = Groove {
        kick: 0,
        snare: 0,
        ghost: 0,
        hat: 0,
        dense: 0,
        open: 0,
        low_tom: 0,
        high_tom: 0,
        block: 0,
        shaker: 0,
    };
    Some(match name {
        "drive" => Groove {
            kick: mask(&[0, 6, 8, 10]),
            snare: mask(&[4, 12]),
            hat: EIGHTHS,
            dense: OFFBEATS,
            ..none
        },
        "steady" => Groove {
            kick: mask(&[0, 8]),
            snare: mask(&[12]),
            hat: EIGHTHS,
            ..none
        },
        "four" => Groove {
            kick: mask(&[0, 4, 8, 12]),
            snare: mask(&[4, 12]),
            hat: mask(&[0, 4, 8, 12]),
            dense: OFFBEATS,
            open: mask(&[2, 6, 10, 14]),
            ..none
        },
        "half" => Groove {
            kick: mask(&[0, 10]),
            snare: mask(&[8]),
            hat: mask(&[0, 4, 8, 12]),
            dense: mask(&[2, 6, 10, 14]),
            ..none
        },
        "break" => Groove {
            kick: mask(&[0, 7, 10]),
            snare: mask(&[4, 12]),
            ghost: mask(&[9, 15]),
            hat: EIGHTHS,
            dense: OFFBEATS,
            ..none
        },
        "march" => Groove {
            kick: mask(&[0, 3, 8, 11]),
            snare: mask(&[4, 12]),
            ghost: mask(&[14, 15]),
            hat: EIGHTHS,
            low_tom: mask(&[10]),
            ..none
        },
        "tribal" => Groove {
            kick: mask(&[0, 8]),
            snare: mask(&[12]),
            hat: OFFBEATS,
            low_tom: mask(&[0, 3, 6]),
            high_tom: mask(&[10, 11, 14]),
            ..none
        },
        "shuffle" => Groove {
            kick: mask(&[0, 10]),
            snare: mask(&[4, 12]),
            hat: mask(&[0, 3, 4, 7, 8, 11, 12, 15]),
            ..none
        },
        // 太鼓：低音大鼓与高音缔太鼓交替，木梆点缀
        "taiko" => Groove {
            low_tom: mask(&[0, 8, 11]),
            high_tom: mask(&[4, 6, 12, 14]),
            block: mask(&[2, 10]),
            ..none
        },
        // 禅意：一声太鼓与零星木鱼
        "zen" => Groove {
            low_tom: mask(&[0]),
            block: mask(&[8, 14]),
            ..none
        },
        // 埃及 Maqsum：咚、哒、哒、咚、哒，沙锤补空拍
        "maqsum" => Groove {
            low_tom: mask(&[0, 8]),
            high_tom: mask(&[2, 6, 12]),
            shaker: mask(&[4, 10, 14]),
            ..none
        },
        // 西部火车节奏：刷子连续十六分，小鼓重拍
        "train" => Groove {
            kick: mask(&[0, 8]),
            snare: mask(&[4, 12]),
            shaker: 0xffff,
            ..none
        },
        // 凯尔特里尔舞：宝思兰鼓
        "reel" => Groove {
            low_tom: mask(&[0, 8]),
            high_tom: mask(&[3, 6, 11, 14]),
            shaker: OFFBEATS,
            ..none
        },
        // 巴黎缪塞特：低音与刷子的嘭恰
        "musette" => Groove {
            kick: mask(&[0, 8]),
            snare: mask(&[4, 12]),
            shaker: mask(&[2, 6, 10, 14]),
            ..none
        },
        // 中式庙会：堂鼓与梆子
        "temple" => Groove {
            low_tom: mask(&[0, 8, 10]),
            block: mask(&[4, 12, 14]),
            ..none
        },
        // 北欧：缓慢的框鼓心跳
        "nordic" => Groove {
            low_tom: mask(&[0, 3]),
            shaker: mask(&[8]),
            ..none
        },
        "none" => return None,
        _ => panic!("未知鼓组 {name}"),
    })
}
fn on(bits: u16, step: usize) -> bool {
    bits & (1 << step) != 0
}

/// 低音每小节的发声位置与相对根音的音程
fn bass_line(name: &str) -> &'static [(usize, u8)] {
    match name {
        "drive" => &[(0, 0), (3, 0), (6, 12), (8, 0), (10, 7), (14, 12)],
        "cruise" => &[(0, 0), (6, 12), (8, 0), (10, 7), (14, 12)],
        "eighths" => &[
            (0, 0),
            (2, 0),
            (4, 12),
            (6, 0),
            (8, 0),
            (10, 0),
            (12, 12),
            (14, 7),
        ],
        "gallop" => &[
            (0, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (6, 0),
            (7, 0),
            (8, 0),
            (10, 0),
            (11, 0),
            (12, 7),
            (14, 5),
            (15, 7),
        ],
        "walk" => &[(0, 0), (4, 7), (8, 12), (12, 7)],
        "oompah" => &[(0, 0), (4, 7), (8, 0), (12, 7)],
        "long" => &[(0, 0), (8, 7)],
        "pedal" => &[(0, 0)],
        _ => panic!("未知低音型 {name}"),
    }
}

/// 琶音的和弦音序与间隔步数
fn arpeggio(name: &str) -> Option<(&'static [usize], usize)> {
    Some(match name {
        "classic" => (&[0, 2, 1, 3, 2, 1, 3, 1], 2),
        "up" => (&[0, 1, 2, 3], 2),
        "down" => (&[3, 2, 1, 0], 2),
        "bounce" => (&[0, 1, 2, 3, 2, 1], 2),
        "sixteenths" => (&[0, 1, 2, 3, 2, 1, 2, 3], 1),
        "slow" => (&[0, 2, 1, 3], 4),
        "none" => return None,
        _ => panic!("未知琶音 {name}"),
    })
}

fn compile(d: Definition) -> Score {
    let seconds = 15.0 / d.bpm as f32;
    let mut steps = vec![Vec::new(); d.sections.len() * 8 * 16];
    let mut chords = Vec::new();
    let fierce = d.style == "combat" || d.style == "boss";
    let quick = fierce || d.style == "race";
    let soft = matches!(d.style.as_str(), "flight" | "defeat" | "ambient");
    let lead_instrument = d.lead.as_deref().and_then(instrument);
    let counter_instrument = d.counter_wave.as_deref().and_then(instrument);
    let arp_instrument = d.arp_wave.as_deref().and_then(instrument);
    let raw = |name: Option<&str>| name.filter(|n| instrument(n).is_none()).map(wave);
    let lead_wave = raw(d.lead.as_deref()).unwrap_or(if d.style == "boss" {
        Wave::Fm
    } else if soft {
        Wave::Triangle
    } else {
        Wave::Pulse
    });
    let lead_gain = match lead_wave {
        Wave::Triangle | Wave::Sine => 0.24,
        Wave::Fm => 0.16,
        _ => 0.14,
    };
    let bass_wave =
        d.bass
            .as_deref()
            .map(wave)
            .unwrap_or(if fierce { Wave::Fm } else { Wave::Triangle });
    let bass_notes =
        bass_line(
            d.bass_line
                .as_deref()
                .unwrap_or(if quick { "drive" } else { "cruise" }),
        );
    let drums = groove(d.groove.as_deref().unwrap_or(if d.style == "defeat" {
        "none"
    } else if quick {
        "drive"
    } else {
        "steady"
    }));
    let arp = arpeggio(d.arp.as_deref().unwrap_or(if d.style == "defeat" {
        "none"
    } else {
        "classic"
    }));
    let arp_wave =
        raw(d.arp_wave.as_deref()).unwrap_or(if fierce { Wave::Saw } else { Wave::Pulse });
    let counter_wave = raw(d.counter_wave.as_deref()).unwrap_or(Wave::Triangle);
    let cues: Vec<Cue> = d.cues.iter().map(|c| Cue::parse(c)).collect();
    let maps: Vec<MapKind> = d.maps.iter().map(|m| parse_map(m)).collect();
    let phrases = d.melody.len() / 8;
    assert!(
        d.melody.len().is_multiple_of(8) && d.harmony.len() == d.melody.len(),
        "{}: 旋律与和声需要按八小节成句",
        d.title
    );
    assert!(
        d.counter.is_empty() || d.counter.len() == d.melody.len(),
        "{}: 副旋律需要与主旋律等长",
        d.title
    );
    let line = |bar: &str, active: bool, slot: &mut Vec<(usize, u8, usize)>| {
        let mut cursor = 0;
        for token in bar.split_whitespace() {
            let (pitch, duration) = token
                .split_once(':')
                .unwrap_or_else(|| panic!("{}: 音符需要时值 {token}", d.title));
            let length = duration.parse::<usize>().expect("音符时值需要整数");
            if pitch != "-" && active {
                slot.push((cursor, midi(pitch), length));
            }
            cursor += length;
        }
        assert_eq!(cursor, 16, "{}: 每小节需要十六个时值：{bar}", d.title);
    };
    for (section_index, section) in d.sections.iter().enumerate() {
        assert!(
            section.phrase < phrases,
            "{}: 段落引用的乐句不存在",
            d.title
        );
        for bar in 0..8 {
            let index = section.phrase * 8 + bar;
            let base = (section_index * 8 + bar) * 16;
            let chord: Vec<u8> = d.harmony[index].split_whitespace().map(midi).collect();
            chords.push(chord.clone().try_into().expect("和弦需要四个音符"));
            let energy = section.energy;
            let lead_active = section.lead || (section_index == 0 && bar >= 4);
            let mut notes = Vec::new();
            line(&d.melody[index], lead_active, &mut notes);
            for (at, note, length) in notes {
                let mut t = Tone::new(
                    lead_wave,
                    frequency(note),
                    lead_gain,
                    seconds * length as f32 * 0.86,
                );
                t.pan = -0.14;
                t.shape = d.lead_duty.unwrap_or(match lead_wave {
                    Wave::Fm => 1.3,
                    _ => 0.25,
                });
                t.attack = if soft { 0.012 } else { 0.004 };
                t.decay = 0.07;
                t.sustain = 0.7;
                t.release = if soft { 0.09 } else { 0.055 };
                t.vibrato = if length >= 4 { 0.006 } else { 0.004 };
                t.send = if soft { 0.3 } else { 0.22 };
                t.layer = LEAD;
                if let Some(i) = lead_instrument {
                    i.apply(&mut t);
                    i.push(t, &mut steps[base + at]);
                } else {
                    steps[base + at].push(t);
                }
                if energy > 0.9 && length >= 3 {
                    let mut harmony = t;
                    harmony.hz *= 0.5;
                    harmony.end_hz = harmony.hz;
                    harmony.gain *= 0.32;
                    harmony.pan = 0.35;
                    harmony.layer = LEAD;
                    steps[base + at].push(harmony);
                }
            }
            if !d.counter.is_empty() {
                let mut notes = Vec::new();
                line(&d.counter[index], energy >= 0.5, &mut notes);
                for (at, note, length) in notes {
                    let mut t = Tone::new(
                        counter_wave,
                        frequency(note),
                        0.10,
                        seconds * length as f32 * 0.9,
                    );
                    t.pan = 0.38;
                    t.shape = 0.4;
                    t.attack = 0.02;
                    t.decay = 0.12;
                    t.sustain = 0.6;
                    t.release = 0.08;
                    t.send = 0.25;
                    t.layer = COUNTER;
                    if let Some(i) = counter_instrument {
                        i.apply(&mut t);
                        t.gain *= 0.55;
                        t.pan = 0.38;
                        i.push(t, &mut steps[base + at]);
                    } else {
                        steps[base + at].push(t);
                    }
                }
            }
            for step in 0..16 {
                let slot = &mut steps[base + step];
                let fill = bar == 7 && step >= 12;
                let root = chord[0].saturating_sub(if chord[0] >= 48 { 12 } else { 0 });
                if let Some((_, interval)) = bass_notes.iter().find(|(at, _)| *at == step) {
                    let long = bass_notes.len() <= 2;
                    let mut bass = Tone::new(
                        bass_wave,
                        frequency(root + interval),
                        0.22,
                        seconds
                            * if long {
                                7.0
                            } else if step == 0 || step == 8 {
                                2.6
                            } else {
                                1.5
                            },
                    );
                    bass.shape = 0.65;
                    bass.attack = if long { 0.02 } else { 0.003 };
                    bass.sustain = if long { 0.7 } else { 0.55 };
                    bass.release = if long { 0.12 } else { 0.035 };
                    bass.layer = BASS;
                    slot.push(bass);
                }
                if step == 0 && d.pad > 0.0 {
                    for (i, note) in chord.iter().skip(1).enumerate() {
                        let mut pad = Tone::new(
                            Wave::Triangle,
                            frequency(*note + 12),
                            0.038 * d.pad,
                            seconds * 13.0,
                        );
                        pad.attack = if soft { 0.25 } else { 0.10 };
                        pad.decay = 0.20;
                        pad.release = seconds * 3.0;
                        pad.sustain = 0.7;
                        pad.pan = [-0.6, 0.55, 0.0][i % 3];
                        pad.send = if soft { 0.12 } else { 0.0 };
                        pad.layer = PAD;
                        slot.push(pad);
                    }
                }
                if let Some((order, every)) = arp
                    && step % every == 0
                {
                    let note = chord[order[(step / every) % order.len()] % chord.len()] + 24;
                    let slow = every >= 4;
                    let mut tone = Tone::new(
                        arp_wave,
                        frequency(note),
                        0.042 * (0.65 + energy * 0.35) * if slow { 1.3 } else { 1.0 },
                        seconds * if slow { 2.4 } else { 0.68 * every as f32 },
                    );
                    tone.pan = if (step / every + bar) % 2 == 0 {
                        -0.55
                    } else {
                        0.55
                    };
                    tone.shape = 0.125;
                    tone.sustain = if slow { 0.4 } else { 0.15 };
                    tone.release = if slow { 0.2 } else { 0.045 };
                    tone.send = if slow { 0.3 } else { 0.18 };
                    tone.layer = ARP;
                    if let Some(i) = arp_instrument {
                        // 拨弦琶音沿用乐器包络，音量按乐器响度折算
                        let gain = tone.gain * i.gain / 0.16;
                        i.apply(&mut tone);
                        tone.gain = gain;
                    }
                    slot.push(tone);
                }
                let Some(g) = &drums else {
                    continue;
                };
                if energy < 0.45 {
                    continue;
                }
                if on(g.kick, step) {
                    let mut t = Tone::new(Wave::Sine, 150.0, 0.40, 0.12);
                    t.end_hz = 44.0;
                    t.attack = 0.001;
                    t.decay = 0.10;
                    t.sustain = 0.06;
                    t.release = 0.04;
                    t.layer = DRUMS;
                    slot.push(t);
                    let mut click = Tone::new(Wave::Noise, 1000.0, 0.045, 0.014);
                    click.attack = 0.001;
                    click.shape = 0.65;
                    click.sustain = 0.0;
                    click.decay = 0.014;
                    click.layer = DRUMS;
                    slot.push(click);
                }
                let snare = on(g.snare, step);
                let ghost = on(g.ghost, step) && energy >= 0.6;
                if snare || ghost || (fill && energy > 0.7 && step % 2 == 1) {
                    let gain = if snare { 0.18 } else { 0.08 };
                    let mut t = Tone::new(Wave::Noise, 1200.0, gain, 0.09);
                    t.shape = 0.55;
                    t.attack = 0.001;
                    t.decay = 0.07;
                    t.sustain = 0.12;
                    t.release = 0.055;
                    t.pan = -0.08;
                    t.layer = DRUMS;
                    slot.push(t);
                    let mut body = Tone::new(Wave::Triangle, 185.0, gain * 0.55, 0.045);
                    body.end_hz = 130.0;
                    body.decay = 0.045;
                    body.sustain = 0.0;
                    body.layer = DRUMS;
                    slot.push(body);
                }
                for (bits, from, to, pan) in [
                    (g.low_tom, 120.0, 70.0, -0.3),
                    (g.high_tom, 210.0, 140.0, 0.3),
                ] {
                    if on(bits, step) {
                        let mut tom = Tone::new(Wave::Sine, from, 0.26, 0.14);
                        tom.end_hz = to;
                        tom.attack = 0.001;
                        tom.decay = 0.12;
                        tom.sustain = 0.05;
                        tom.release = 0.06;
                        tom.pan = pan;
                        tom.layer = DRUMS;
                        slot.push(tom);
                    }
                }
                if on(g.block, step) {
                    let mut block = Tone::new(Wave::Triangle, 1400.0, 0.13, 0.025);
                    block.end_hz = 1250.0;
                    block.attack = 0.001;
                    block.decay = 0.02;
                    block.sustain = 0.0;
                    block.release = 0.02;
                    block.pan = 0.2;
                    block.layer = DRUMS;
                    slot.push(block);
                }
                if on(g.shaker, step) {
                    let mut shake = Tone::new(Wave::Noise, 5000.0, 0.022, 0.03);
                    shake.shape = 0.4;
                    shake.attack = 0.004;
                    shake.decay = 0.025;
                    shake.sustain = 0.0;
                    shake.release = 0.02;
                    shake.pan = -0.3;
                    shake.layer = HATS;
                    slot.push(shake);
                }
                let dense = on(g.dense, step) && energy >= 0.8;
                if on(g.hat, step) || dense {
                    let open = on(g.open, step) || (step == 14 && bar % 2 == 1);
                    let mut t = Tone::new(
                        Wave::Noise,
                        8000.0,
                        if dense { 0.023 } else { 0.043 },
                        if open { 0.16 } else { 0.022 },
                    );
                    t.shape = -0.18;
                    t.attack = 0.001;
                    t.decay = 0.022;
                    t.sustain = if open { 0.16 } else { 0.0 };
                    t.release = if open { 0.07 } else { 0.012 };
                    t.pan = 0.35;
                    t.delay = if step % 2 == 1 { 0.004 } else { 0.0 };
                    t.layer = HATS;
                    slot.push(t);
                }
                if bar == 0 && step == 0 && section_index > 0 {
                    let mut crash = Tone::new(Wave::Noise, 5000.0, 0.11, 0.4);
                    crash.attack = 0.001;
                    crash.decay = 0.4;
                    crash.sustain = 0.03;
                    crash.release = 0.25;
                    crash.pan = 0.45;
                    crash.shape = -0.12;
                    crash.layer = DRUMS;
                    slot.push(crash);
                }
            }
        }
    }
    // 摇摆律动：反拍整体后移
    if d.swing > 0.0 {
        for (i, slot) in steps.iter_mut().enumerate() {
            if i % 2 == 1 {
                for tone in slot {
                    tone.delay += d.swing * seconds;
                }
            }
        }
    }
    let entry = d
        .sections
        .iter()
        .position(|s| s.lead && s.energy >= 0.55)
        .unwrap_or(0)
        * 128;
    Score {
        title: d.title,
        bpm: d.bpm,
        loop_start: d.loop_start * 16,
        entry,
        steps,
        chords,
        cues,
        maps,
    }
}
