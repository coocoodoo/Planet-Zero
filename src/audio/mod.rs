mod backend;
mod effects;
pub mod export;
mod mixer;
mod oscillator;
mod score;
#[cfg(test)]
mod tests;

pub use backend::Output;
pub use mixer::Mixer;
pub use score::{Cue, LAYERS, playlist, scores};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Navigate,
    Confirm,
    Countdown(u8),
    Roll,
    Warp,
    Gun,
    Missile,
    Laser,
    Plasma,
    Orb,
    Flak,
    Rail,
    Swarm,
    Impact,
    Explosion(bool),
    Damage,
    Coin(u8),
    Pickup,
    Upgrade,
    Support,
    Gate(bool),
    Nova,
    Overdrive,
    Phase,
    Boss,
    BoostStart,
    OrbitReady,
    OrbitIgnite,
    Breakthrough,
    OrbitArrival,
    Reentry,
    Landing,
    EnemyFire(bool),
    Graze,
    Flyby,
    WeaponSwitch(u8),
    Toggle(bool),
    View,
    Wave,
    Destroyed,
    Thunder,
}
impl Sound {
    /// 每种音效独立的限频槽位与最短间隔（秒）
    pub(crate) fn throttle(self) -> (usize, f64) {
        match self {
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
            Sound::Warp => (22, 0.6),
            Sound::BoostStart => (23, 0.8),
            Sound::OrbitReady => (24, 2.0),
            Sound::OrbitIgnite => (25, 1.5),
            Sound::Breakthrough => (26, 2.0),
            Sound::OrbitArrival => (27, 2.0),
            Sound::Reentry => (28, 2.0),
            Sound::Landing => (29, 2.0),
            Sound::EnemyFire(false) => (30, 0.12),
            Sound::EnemyFire(true) => (31, 0.3),
            Sound::Graze => (32, 0.08),
            Sound::Flyby => (33, 0.5),
            Sound::WeaponSwitch(_) => (34, 0.06),
            Sound::Toggle(_) => (35, 0.08),
            Sound::View => (36, 0.06),
            Sound::Wave => (37, 2.0),
            Sound::Destroyed => (38, 2.0),
            Sound::Thunder => (39, 1.2),
            Sound::Flak => (40, 0.12),
            Sound::Rail => (41, 0.3),
            Sound::Swarm => (42, 0.16),
        }
    }
}
pub(crate) const THROTTLE_SLOTS: usize = 43;

/// 实时输出的选曲种子，每次启动随机
#[allow(dead_code)]
fn entropy() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32)
}

#[derive(Clone, Copy, Default)]
pub struct Events {
    items: [Option<Sound>; 32],
}
impl Events {
    pub fn push(&mut self, sound: Sound) {
        if let Some(slot) = self.items.iter_mut().find(|s| s.is_none()) {
            *slot = Some(sound);
        }
    }
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
    pub fn iter(&self) -> impl Iterator<Item = Sound> + '_ {
        self.items.iter().flatten().copied()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub profile: u8,
    pub volume: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            profile: 0,
            volume: 7,
        }
    }
}
impl Settings {
    pub fn name(self) -> &'static str {
        ["ALL", "MUSIC", "EFFECTS", "MUTE"][self.profile as usize]
    }
    pub fn music(self) -> bool {
        self.profile < 2
    }
    pub fn effects(self) -> bool {
        self.profile == 0 || self.profile == 2
    }
}

#[derive(Clone, Copy)]
pub struct Frame {
    pub cue: Cue,
    pub paused: bool,
    pub engine: f32,
    pub boost: f32,
    pub danger: f32,
    pub overdrive: bool,
    pub settings: Settings,
    pub events: Events,
    /// 当前地图，决定地图专属配乐
    pub map: crate::world::MapKind,
    /// 机型序号，决定引擎音色
    pub vehicle: u8,
    /// 大气层内轨道推进强度
    pub ascent: f32,
    /// 升空进度，0 为地面，1 为入轨
    pub altitude: f32,
    /// 处于真空环境
    pub vacuum: bool,
    /// 当前天气，决定天气配乐
    pub weather: crate::content::weather::Weather,
    /// 雨声、风声与雾气对配乐的遮蔽强度，0 到 1
    pub rain: f32,
    pub wind: f32,
    pub haze: f32,
    pub snow: f32,
    pub storm: f32,
    /// 黄昏与夜晚的程度，探索配乐夜晚更安静
    pub twilight: f32,
    pub night: f32,
    /// 城市入夜的合成器浪潮强度，0 到 1
    pub synthwave: f32,
    /// 竞速与战斗的激烈程度，0 到 1，决定加入哪些编配层
    pub intensity: f32,
    /// 竞速冲刺段与金币倒计时
    pub finale: f32,
    /// 轨道上升期间压低配乐，音效保持独立播放
    pub music_ducked: bool,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            cue: Cue::Menu,
            paused: false,
            engine: 0.0,
            boost: 0.0,
            danger: 0.0,
            overdrive: false,
            settings: Settings::default(),
            events: Events::default(),
            map: crate::world::MapKind::Islands,
            vehicle: 0,
            ascent: 0.0,
            altitude: 0.0,
            vacuum: false,
            weather: crate::content::weather::Weather::Clear,
            rain: 0.0,
            wind: 0.0,
            haze: 0.0,
            snow: 0.0,
            storm: 0.0,
            twilight: 0.0,
            night: 0.0,
            synthwave: 0.0,
            intensity: 1.0,
            finale: 0.0,
            music_ducked: false,
        }
    }
}
