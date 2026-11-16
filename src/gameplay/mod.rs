use crate::flight::{Flight, angle};
use crate::gameplay::progression::{self as campaign, Loadout, Notice, NoticeKind, Phase};
use crate::world::World;
use glam::{Vec3, vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Free,
    Race,
    Coins,
    Combat,
}

impl Mode {
    pub const ALL: [Self; 4] = [Self::Free, Self::Race, Self::Coins, Self::Combat];
    pub fn name(self) -> &'static str {
        match self {
            Self::Free => "FREE FLIGHT",
            Self::Race => "CHECKPOINT RACE",
            Self::Coins => "COIN RUSH",
            Self::Combat => "SKY ASSAULT",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Free => "EXPLORE / WEATHER / AEROBATICS",
            Self::Race => "18 GATES / BEAT THE CLOCK",
            Self::Coins => "COLLECT / CHAIN / SCORE",
            Self::Combat => "3 ACTS / OBJECTIVES / BOSS",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "free" => Some(Self::Free),
            "race" => Some(Self::Race),
            "coins" => Some(Self::Coins),
            "combat" => Some(Self::Combat),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Difficulty {
    #[default]
    Cadet,
    Pilot,
    Ace,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Cadet, Self::Pilot, Self::Ace];
    pub fn name(self) -> &'static str {
        match self {
            Self::Cadet => "CADET",
            Self::Pilot => "PILOT",
            Self::Ace => "ACE",
        }
    }
    pub fn factor(self) -> f32 {
        [0.75, 1.0, 1.4][self as usize]
    }
}

mod combat;
mod director;
mod entities;
mod objectives;
pub mod progression;
#[cfg(test)]
mod tests;
mod weapons;
pub use entities::*;
pub use weapons::Weapon;

pub struct Mission {
    pub level: u32,
    pub phase_index: usize,
    pub phase_elapsed: f32,
    pub objective: u32,
    pub convoy: Option<Convoy>,
    pub upgrades: [u8; 3],
    pub overdrive: f32,
    pub overdrive_time: f32,
    pub grazes: u32,
    pub notices: [Option<Notice>; 3],
    phase_started: bool,
    boss_defeated: bool,
    spawn_timer: f32,
    supply_timer: f32,

    pub mode: Mode,
    pub difficulty: Difficulty,
    pub elapsed: f32,
    pub countdown: f32,
    pub remaining: f32,
    pub score: u32,
    pub combo: u32,
    pub best_combo: u32,
    pub combo_time: f32,
    pub health: f32,
    pub max_health: f32,
    pub energy: f32,
    pub weapon: Weapon,
    pub levels: [u8; 5],
    pub bombs: u8,
    pub support: f32,
    pub invulnerable: f32,
    pub flash: f32,
    pub message: &'static str,
    pub message_time: f32,
    pub gates: Vec<Gate>,
    pub gate_index: usize,
    pub pickups: Vec<Pickup>,
    pub enemies: Vec<Enemy>,
    pub shots: Vec<Shot>,
    pub effects: Vec<Effect>,
    pub wave: u32,
    pub kills: u32,
    pub collected: u32,
    pub perfect: u32,
    pub outcome: Option<bool>,
    pub autofire: bool,
    pub shots_fired: u32,
    pub hits: u32,
    fire_timer: f32,
    support_timer: f32,

    random: u32,
    next_id: u32,
}

pub fn segment_distance(point: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((point - a).dot(ab) / ab.length_squared().max(0.0001)).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

impl Mission {
    pub fn new(mode: Mode, difficulty: Difficulty, flight: &mut Flight, world: &World) -> Self {
        Self::for_level(mode, difficulty, flight, world, 1)
    }
    pub fn for_level(
        mode: Mode,
        difficulty: Difficulty,
        flight: &mut Flight,
        world: &World,
        level: u32,
    ) -> Self {
        flight.set_demo(false);
        flight.change_world(world);
        flight.yaw = 0.0;
        flight.camera_yaw = 0.0;
        flight.position.z = 150.0;
        flight.follow = flight.position;
        flight.wing_count = if mode == Mode::Free { 2 } else { 0 };
        let mut mission = Self {
            level: level.max(1),
            phase_index: 0,
            phase_elapsed: 0.0,
            objective: 0,
            convoy: None,
            upgrades: [0; 3],
            overdrive: 0.0,
            overdrive_time: 0.0,
            grazes: 0,
            notices: [None; 3],
            phase_started: false,
            boss_defeated: false,
            spawn_timer: 0.0,
            supply_timer: 24.0,
            mode,
            difficulty,
            elapsed: 0.0,
            countdown: if mode == Mode::Free { 0.0 } else { 3.0 },
            remaining: match mode {
                Mode::Race => 110.0 - difficulty as u8 as f32 * 15.0,
                Mode::Coins => 90.0,
                Mode::Combat => 480.0,
                Mode::Free => 0.0,
            },
            score: 0,
            combo: 0,
            best_combo: 0,
            combo_time: 0.0,
            health: flight.vehicle.shield(),
            max_health: flight.vehicle.shield(),
            energy: 100.0,
            weapon: Weapon::Gun,
            levels: [1; 5],
            bombs: if flight.vehicle == crate::content::vehicles::Vehicle::Atlas {
                3
            } else {
                2
            },
            support: 0.0,
            invulnerable: 0.0,
            flash: 0.0,
            message: "READY",
            message_time: 3.0,
            gates: vec![],
            gate_index: 0,
            pickups: Vec::with_capacity(100),
            enemies: Vec::with_capacity(16),
            shots: Vec::with_capacity(180),
            effects: Vec::with_capacity(48),
            wave: 0,
            kills: 0,
            collected: 0,
            perfect: 0,
            outcome: None,
            autofire: true,
            shots_fired: 0,
            hits: 0,
            fire_timer: 0.0,
            support_timer: 0.0,

            random: world.seed ^ 0x75ab31f5,
            next_id: 0,
        };
        if mode == Mode::Race || mode == Mode::Coins {
            let mut position = flight.position;
            let count = if mode == Mode::Race {
                18 + (level.saturating_sub(1) % 6) * 2
            } else {
                96 + (level.saturating_sub(1) % 6) * 8
            };
            mission.remaining += level.saturating_sub(1).min(5) as f32 * 8.0;
            for i in 0..count {
                let t = i as f32;
                let curve = 1.0 + (level.saturating_sub(1) % 6) as f32 * 0.05;
                let yaw = ((t * if mode == Mode::Race { 0.44 } else { 0.12 })
                    + (level.saturating_sub(1) % 6) as f32 * 0.4)
                    .sin()
                    * 0.52
                    * curve;
                let normal = vec3(yaw.sin(), -yaw.cos(), 0.0);
                position += normal * if mode == Mode::Race { 220.0 } else { 65.0 };
                position.z = (150.0 + (t * 0.35).sin() * 25.0)
                    .max(world.height(position.x, position.y) + 65.0);
                if mode == Mode::Race {
                    mission.gates.push(Gate {
                        position,
                        normal,
                        radius: [32.0, 27.0, 23.0][difficulty as usize],
                    });
                } else {
                    mission.pickups.push(Pickup {
                        position,
                        kind: if i > 0 && i % 16 == 0 {
                            PickupKind::Time
                        } else {
                            PickupKind::Coin
                        },
                        life: 200.0,
                        collected: false,
                    });
                }
            }
        }
        mission
    }
    pub fn loadout(&self) -> Loadout {
        Loadout {
            levels: self.levels,
            weapon: self.weapon,
            bombs: self.bombs,
            upgrades: self.upgrades,
            shield: (self.health / self.max_health).clamp(0.0, 1.0),
        }
    }
    pub fn equip(&mut self, gear: &Loadout) {
        self.levels = gear.levels;
        self.weapon = gear.weapon;
        self.bombs = gear.bombs;
        self.upgrades = gear.upgrades;
        self.max_health += gear.upgrades[1] as f32 * 20.0;
        self.health = self.max_health * gear.shield;
    }
    pub fn shuffle_encounters(&mut self, seed: u32) {
        self.random = seed.max(1);
    }
    fn random(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        self.random as f32 / u32::MAX as f32
    }
    pub fn announce(&mut self, message: &'static str) {
        self.message = message;
        self.message_time = 2.0;
        let kind = match message {
            "WEAPON UPGRADE" => NoticeKind::Upgrade,
            "SHIELD REPAIRED" => NoticeKind::Shield,
            "ENERGY REFILLED" => NoticeKind::Energy,
            "NOVA +1" | "NOVA / BULLETS CLEARED" => NoticeKind::Bomb,
            "WINGMEN / 30 SECONDS" => NoticeKind::Wing,
            "WARNING / BOSS APPROACH" => NoticeKind::Boss,
            "SHIELD HIT / ROLL TO EVADE" | "CONVOY HIT" => NoticeKind::Hit,
            "SECTOR CLEAR / RESUPPLY" => NoticeKind::Clear,
            "OVERDRIVE" => NoticeKind::Overdrive,
            "OBJECTIVE COMPLETE" => NoticeKind::Objective,
            _ => NoticeKind::Wave,
        };
        if let Some(slot) = self
            .notices
            .iter_mut()
            .find(|n| n.is_some_and(|n| n.kind == kind))
        {
            *slot = Some(Notice { kind, life: 2.0 });
        } else {
            self.notices.rotate_right(1);
            self.notices[0] = Some(Notice { kind, life: 2.0 });
        }
    }
    fn effect(&mut self, position: Vec3, radius: f32, color: [u8; 3]) {
        if self.effects.len() < 48 {
            self.effects.push(Effect {
                position,
                radius,
                color,
                life: 0.6,
                duration: 0.6,
            });
        }
    }
    pub fn step(
        &mut self,
        dt: f32,
        flight: &mut Flight,
        world: &World,
        previous: Vec3,
        fire: bool,
    ) {
        if self.outcome.is_some() {
            return;
        }
        self.message_time = (self.message_time - dt).max(0.0);
        self.flash = (self.flash - dt).max(0.0);
        self.invulnerable = (self.invulnerable - dt).max(0.0);
        self.effects.iter_mut().for_each(|e| e.life -= dt);
        self.effects.retain(|e| e.life > 0.0);
        if self.countdown > 0.0 {
            self.countdown = (self.countdown - dt).max(0.0);
            return;
        }
        for slot in &mut self.notices {
            if let Some(n) = slot {
                n.life -= dt;
                if n.life <= 0.0 {
                    *slot = None;
                }
            }
        }
        self.overdrive_time = (self.overdrive_time - dt).max(0.0);
        self.elapsed += dt;
        if self.mode == Mode::Free {
            return;
        }
        self.remaining = (self.remaining - dt).max(0.0);
        self.combo_time = (self.combo_time - dt).max(0.0);
        if self.combo_time == 0.0 {
            self.combo = 0;
        }
        if self.mode == Mode::Race {
            if let Some(g) = self.gates.get(self.gate_index) {
                let a = (previous - g.position).dot(g.normal);
                let b = (flight.position - g.position).dot(g.normal);
                if a <= 0.0 && b >= 0.0 {
                    let intersection =
                        previous.lerp(flight.position, (-a / (b - a).max(0.0001)).clamp(0.0, 1.0));
                    let distance = intersection.distance(g.position);
                    if distance <= g.radius {
                        self.gate_index += 1;
                        self.score += 500;
                        let perfect = distance < g.radius * 0.4;
                        if perfect {
                            self.score += 250;
                            self.perfect += 1;
                        }
                        self.effect(g.position, 32.0, [88, 255, 190]);
                        self.announce(if perfect {
                            "PERFECT GATE +750"
                        } else {
                            "CHECKPOINT +500"
                        });
                    } else {
                        self.announce("MISSED / TURN BACK");
                    }
                }
            }
            if self.gate_index == self.gates.len() {
                self.score += (self.remaining * 100.0) as u32;
                self.outcome = Some(true);
            }
        }
        self.collect(dt, flight, previous);
        if self.mode == Mode::Coins && self.pickups.iter().all(|p| p.collected) {
            self.outcome = Some(true);
        }
        if self.mode == Mode::Combat {
            self.combat(dt, flight, world, previous, fire || self.autofire);
        }
        if self.remaining <= 0.0 && self.outcome.is_none() {
            self.outcome = Some(self.mode == Mode::Coins && self.score >= self.coin_goal());
            self.announce("TIME COMPLETE");
        }
    }
}
