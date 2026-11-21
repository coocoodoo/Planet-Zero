pub use crate::content::levels::operation;
use crate::gameplay::Weapon;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Patrol,
    Escort,
    Strike,
    Survival,
    Boss,
}
impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Patrol => "AIR PATROL",
            Self::Escort => "CONVOY ESCORT",
            Self::Strike => "BATTERY STRIKE",
            Self::Survival => "HOLD THE LINE",
            Self::Boss => "BOSS INTERCEPT",
        }
    }
    pub fn icon(self) -> char {
        match self {
            Self::Patrol => 'P',
            Self::Escort => 'C',
            Self::Strike => 'T',
            Self::Survival => 'S',
            Self::Boss => 'B',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upgrade {
    Power,
    Armor,
    Reactor,
}
impl Upgrade {
    pub const ALL: [Self; 3] = [Self::Power, Self::Armor, Self::Reactor];
    pub fn name(self) -> &'static str {
        match self {
            Self::Power => "POWER +8%",
            Self::Armor => "ARMOR +20",
            Self::Reactor => "CHARGE +20%",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loadout {
    pub levels: [u8; Weapon::COUNT],
    pub weapon: Weapon,
    pub bombs: u8,
    pub upgrades: [u8; 3],
    pub shield: f32,
}
impl Default for Loadout {
    fn default() -> Self {
        Self {
            levels: [1; Weapon::COUNT],
            weapon: Weapon::Gun,
            bombs: 2,
            upgrades: [0; 3],
            shield: 1.0,
        }
    }
}
impl Loadout {
    pub fn upgrade(&mut self, upgrade: Upgrade) {
        let slot = &mut self.upgrades[upgrade as usize];
        *slot = (*slot + 1).min(10);
        self.shield = (self.shield + 0.45).min(1.0);
        self.bombs = (self.bombs + 1).min(5);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Upgrade,
    Shield,
    Energy,
    Bomb,
    Wing,
    Wave,
    Boss,
    Hit,
    Clear,
    Objective,
    Overdrive,
}
impl NoticeKind {
    pub fn icon(self) -> char {
        match self {
            Self::Upgrade => '+',
            Self::Shield => 'S',
            Self::Energy => 'A',
            Self::Bomb => 'B',
            Self::Wing => 'W',
            Self::Wave => 'V',
            Self::Boss => 'X',
            Self::Hit => '!',
            Self::Clear => 'K',
            Self::Objective => 'F',
            Self::Overdrive => 'Y',
        }
    }
}
#[derive(Clone, Copy)]
pub struct Notice {
    pub kind: NoticeKind,
    pub life: f32,
}
