use super::oscillator::{Tone, Wave, frequency};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Menu,
    Flight,
    Race,
    Combat,
    Boss,
    Victory,
    Defeat,
}
impl Cue {
    pub const ALL: [Self; 7] = [
        Self::Menu,
        Self::Flight,
        Self::Race,
        Self::Combat,
        Self::Boss,
        Self::Victory,
        Self::Defeat,
    ];
    pub fn name(self) -> &'static str {
        [
            "menu", "flight", "race", "combat", "boss", "victory", "defeat",
        ][self as usize]
    }
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
    loop_start: usize,
    sections: Vec<Section>,
    melody: Vec<String>,
    harmony: Vec<String>,
}

pub struct Score {
    pub title: String,
    pub bpm: u32,
    pub loop_start: usize,
    pub steps: Vec<Vec<Tone>>,
    pub chords: Vec<[u8; 4]>,
}
impl Score {
    pub fn seconds(&self) -> f32 {
        self.steps.len() as f32 * 15.0 / self.bpm as f32
    }
}

pub fn scores() -> &'static [Score; 7] {
    static SCORES: OnceLock<[Score; 7]> = OnceLock::new();
    SCORES.get_or_init(|| {
        [
            include_str!("../../content/music/menu.toml"),
            include_str!("../../content/music/flight.toml"),
            include_str!("../../content/music/race.toml"),
            include_str!("../../content/music/combat.toml"),
            include_str!("../../content/music/boss.toml"),
            include_str!("../../content/music/victory.toml"),
            include_str!("../../content/music/defeat.toml"),
        ]
        .map(|source| compile(toml::from_str(source).expect("音乐乐谱格式错误")))
    })
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

fn compile(d: Definition) -> Score {
    let seconds = 15.0 / d.bpm as f32;
    let mut steps = vec![Vec::new(); d.sections.len() * 8 * 16];
    let mut chords = Vec::new();
    let fierce = d.style == "combat" || d.style == "boss";
    let quick = fierce || d.style == "race";
    let soft = d.style == "flight" || d.style == "defeat";
    for (section_index, section) in d.sections.iter().enumerate() {
        for bar in 0..8 {
            let index = section.phrase * 8 + bar;
            let base = (section_index * 8 + bar) * 16;
            let chord: Vec<u8> = d.harmony[index].split_whitespace().map(midi).collect();
            chords.push(chord.clone().try_into().expect("和弦需要四个音符"));
            let energy = section.energy;
            let lead_active = section.lead || (section_index == 0 && bar >= 4);
            let mut cursor = 0;
            for token in d.melody[index].split_whitespace() {
                let (pitch, duration) = token.split_once(':').expect("音符需要时值");
                let length = duration.parse::<usize>().expect("音符时值需要整数");
                if pitch != "-" && lead_active {
                    let wave = if d.style == "boss" {
                        Wave::Fm
                    } else if soft {
                        Wave::Triangle
                    } else {
                        Wave::Pulse
                    };
                    let mut t = Tone::new(
                        wave,
                        frequency(midi(pitch)),
                        if soft { 0.24 } else { 0.14 },
                        seconds * length as f32 * 0.86,
                    );
                    t.pan = -0.14;
                    t.shape = if d.style == "boss" { 1.3 } else { 0.25 };
                    t.attack = if soft { 0.012 } else { 0.004 };
                    t.decay = 0.07;
                    t.sustain = 0.7;
                    t.release = 0.055;
                    t.vibrato = 0.004;
                    t.send = 0.22;
                    steps[base + cursor].push(t);
                    if energy > 0.9 && length >= 3 {
                        let mut harmony = t;
                        harmony.hz *= 0.5;
                        harmony.end_hz = harmony.hz;
                        harmony.gain *= 0.32;
                        harmony.pan = 0.35;
                        steps[base + cursor].push(harmony);
                    }
                }
                cursor += length;
            }
            assert_eq!(cursor, 16, "每小节需要十六个时值");
            for step in 0..16 {
                let slot = &mut steps[base + step];
                let fill = bar == 7 && step >= 12;
                let root = chord[0].saturating_sub(if chord[0] >= 48 { 12 } else { 0 });
                if [0, 3, 6, 8, 10, 14].contains(&step) && (quick || step % 2 == 0) {
                    let bass_note = root
                        + match step {
                            6 | 14 => 12,
                            10 => 7,
                            _ => 0,
                        };
                    let mut bass = Tone::new(
                        if fierce { Wave::Fm } else { Wave::Triangle },
                        frequency(bass_note),
                        0.22,
                        seconds * if step == 0 || step == 8 { 2.6 } else { 1.5 },
                    );
                    bass.shape = 0.65;
                    bass.attack = 0.003;
                    bass.sustain = 0.55;
                    bass.release = 0.035;
                    slot.push(bass);
                }
                if step == 0 {
                    for (i, note) in chord.iter().skip(1).enumerate() {
                        let mut pad =
                            Tone::new(Wave::Triangle, frequency(*note + 12), 0.038, seconds * 13.0);
                        pad.attack = 0.10;
                        pad.decay = 0.20;
                        pad.release = seconds * 3.0;
                        pad.sustain = 0.7;
                        pad.pan = [-0.6, 0.55, 0.0][i % 3];
                        slot.push(pad);
                    }
                }
                if step % 2 == 0 && d.style != "defeat" {
                    let order = [0, 2, 1, 3, 2, 1, 3, 1];
                    let note = chord[order[step / 2] % chord.len()] + 24;
                    let mut arp = Tone::new(
                        if fierce { Wave::Saw } else { Wave::Pulse },
                        frequency(note),
                        0.042 * (0.65 + energy * 0.35),
                        seconds * 0.68,
                    );
                    arp.pan = if (step / 2 + bar) % 2 == 0 {
                        -0.55
                    } else {
                        0.55
                    };
                    arp.shape = 0.125;
                    arp.sustain = 0.15;
                    arp.release = 0.045;
                    arp.send = 0.18;
                    slot.push(arp);
                }
                if energy < 0.45 || d.style == "defeat" {
                    continue;
                }
                let kick = if quick {
                    [0, 6, 8, 10].contains(&step)
                } else {
                    step == 0 || step == 8
                };
                if kick {
                    let mut t = Tone::new(Wave::Sine, 150.0, 0.40, 0.12);
                    t.end_hz = 44.0;
                    t.attack = 0.001;
                    t.decay = 0.10;
                    t.sustain = 0.06;
                    t.release = 0.04;
                    slot.push(t);
                    let mut click = Tone::new(Wave::Noise, 1000.0, 0.045, 0.014);
                    click.attack = 0.001;
                    click.shape = 0.65;
                    click.sustain = 0.0;
                    click.decay = 0.014;
                    slot.push(click);
                }
                let snare = if quick {
                    step == 4 || step == 12
                } else {
                    step == 12
                };
                if snare || (fill && energy > 0.7 && step % 2 == 1) {
                    let mut t =
                        Tone::new(Wave::Noise, 1200.0, if snare { 0.18 } else { 0.10 }, 0.09);
                    t.shape = 0.55;
                    t.attack = 0.001;
                    t.decay = 0.07;
                    t.sustain = 0.12;
                    t.release = 0.055;
                    t.pan = -0.08;
                    slot.push(t);
                    let mut body = Tone::new(Wave::Triangle, 185.0, 0.10, 0.045);
                    body.end_hz = 130.0;
                    body.decay = 0.045;
                    body.sustain = 0.0;
                    slot.push(body);
                }
                if step % 2 == 0 || (quick && energy >= 0.8) {
                    let open = step == 14 && bar % 2 == 1;
                    let mut t = Tone::new(
                        Wave::Noise,
                        8000.0,
                        if step % 2 == 0 { 0.043 } else { 0.023 },
                        if open { 0.16 } else { 0.022 },
                    );
                    t.shape = -0.18;
                    t.attack = 0.001;
                    t.decay = 0.022;
                    t.sustain = if open { 0.16 } else { 0.0 };
                    t.release = if open { 0.07 } else { 0.012 };
                    t.pan = 0.35;
                    t.delay = if step % 2 == 1 { 0.004 } else { 0.0 };
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
                    slot.push(crash);
                }
            }
        }
    }
    Score {
        title: d.title,
        bpm: d.bpm,
        loop_start: d.loop_start * 16,
        steps,
        chords,
    }
}
