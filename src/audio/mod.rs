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
pub use score::{Cue, scores};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Navigate,
    Confirm,
    Countdown(u8),
    Roll,
    Gun,
    Missile,
    Laser,
    Plasma,
    Orb,
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
        }
    }
}
