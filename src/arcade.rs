use crate::flight::{Flight, angle};
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
            Self::Combat => "4 WAVES / RANDOM BOSS",
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Weapon {
    #[default]
    Gun,
    Missile,
    Laser,
    Plasma,
    Orb,
}

impl Weapon {
    pub const ALL: [Self; 5] = [
        Self::Gun,
        Self::Missile,
        Self::Laser,
        Self::Plasma,
        Self::Orb,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Gun => "GUN",
            Self::Missile => "MISSILE",
            Self::Laser => "LASER",
            Self::Plasma => "PLASMA",
            Self::Orb => "ORB",
        }
    }
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Gun => [255, 225, 128],
            Self::Missile => [255, 141, 67],
            Self::Laser => [88, 242, 255],
            Self::Plasma => [244, 94, 255],
            Self::Orb => [130, 255, 151],
        }
    }
    fn interval(self) -> f32 {
        [0.14, 0.65, 0.34, 0.42, 0.85][self as usize]
    }
    fn cost(self) -> f32 {
        [0.0, 9.0, 5.0, 7.0, 12.0][self as usize]
    }
}

#[derive(Clone)]
pub struct Gate {
    pub position: Vec3,
    pub normal: Vec3,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Coin,
    Upgrade(Weapon),
    Energy,
    Shield,
    Bomb,
    Support,
    Time,
}

pub struct Pickup {
    pub position: Vec3,
    pub kind: PickupKind,
    pub life: f32,
    pub collected: bool,
}

pub struct Enemy {
    pub id: u32,
    pub position: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub max_health: f32,
    pub kind: u8,
    pub phase: f32,
    pub cooldown: f32,
    pub hit: f32,
}

impl Enemy {
    pub fn boss_name(&self) -> &'static str {
        match self.kind {
            3 => "DREADNOUGHT",
            4 => "SKY SERPENT",
            _ => "TEMPEST",
        }
    }

    pub fn radius(&self) -> f32 {
        if self.kind >= 3 {
            24.0
        } else {
            8.0 + self.kind as f32 * 2.0
        }
    }
}

pub struct Shot {
    pub position: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub damage: f32,
    pub weapon: Weapon,
    pub hostile: bool,
    pub target: Option<u32>,
    pub radius: f32,
}

pub struct Effect {
    pub position: Vec3,
    pub life: f32,
    pub duration: f32,
    pub radius: f32,
    pub color: [u8; 3],
}

pub struct Mission {
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
    wave_timer: f32,
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
        flight.set_demo(false);
        flight.change_world(world);
        flight.yaw = 0.0;
        flight.camera_yaw = 0.0;
        flight.position.z = 150.0;
        flight.follow = flight.position;
        flight.wing_count = if mode == Mode::Free { 2 } else { 0 };
        let mut mission = Self {
            mode,
            difficulty,
            elapsed: 0.0,
            countdown: if mode == Mode::Free { 0.0 } else { 3.0 },
            remaining: match mode {
                Mode::Race => 110.0 - difficulty as u8 as f32 * 15.0,
                Mode::Coins => 90.0,
                Mode::Combat => 300.0,
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
            bombs: if flight.vehicle == crate::vehicle::Vehicle::Atlas {
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
            wave_timer: 1.0,
            random: world.seed ^ 0x75ab31f5,
            next_id: 0,
        };
        if mode == Mode::Race || mode == Mode::Coins {
            let mut position = flight.position;
            let count = if mode == Mode::Race { 18 } else { 96 };
            for i in 0..count {
                let t = i as f32;
                let yaw = (t * if mode == Mode::Race { 0.44 } else { 0.12 }).sin() * 0.52;
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

    pub fn coin_goal(&self) -> u32 {
        [3000, 6000, 12000][self.difficulty as usize]
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
    }

    pub fn target(&self, flight: &Flight) -> Option<Vec3> {
        match self.mode {
            Mode::Race => self.gates.get(self.gate_index).map(|g| g.position),
            Mode::Coins => self
                .pickups
                .iter()
                .filter(|p| !p.collected)
                .min_by(|a, b| {
                    a.position
                        .distance_squared(flight.position)
                        .total_cmp(&b.position.distance_squared(flight.position))
                })
                .map(|p| p.position),
            Mode::Combat => self
                .enemies
                .iter()
                .filter(|e| e.health > 0.0)
                .min_by(|a, b| {
                    a.position
                        .distance_squared(flight.position)
                        .total_cmp(&b.position.distance_squared(flight.position))
                })
                .map(|e| e.position),
            Mode::Free => None,
        }
    }

    pub fn cycle_weapon(&mut self) {
        self.weapon = Weapon::ALL[(self.weapon as usize + 1) % 5];
        self.fire_timer = 0.0;
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

    pub fn bomb(&mut self) {
        if self.mode != Mode::Combat
            || self.bombs == 0
            || self.countdown > 0.0
            || self.outcome.is_some()
        {
            return;
        }
        self.bombs -= 1;
        self.flash = 0.65;
        self.shots.retain(|s| !s.hostile);
        for enemy in &mut self.enemies {
            enemy.health -= if enemy.kind >= 3 { 130.0 } else { 200.0 };
            enemy.hit = 0.3;
        }
        self.invulnerable = 2.0;
        self.announce("NOVA / BULLETS CLEARED");
    }

    pub fn dodge(&mut self) {
        if self.invulnerable <= 0.0 {
            self.invulnerable = 0.7;
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

    fn collect(&mut self, dt: f32, flight: &Flight, previous: Vec3) {
        let mut received = Vec::new();
        for p in &mut self.pickups {
            if p.collected {
                continue;
            }
            p.life -= dt;
            if p.life <= 0.0 {
                p.collected = true;
                continue;
            }
            let distance = segment_distance(p.position, previous, flight.position);
            if self.mode == Mode::Combat && distance < 100.0 {
                p.position = p.position.lerp(flight.position, (dt * 2.5).min(1.0));
            }
            if distance < if self.mode == Mode::Coins { 23.0 } else { 18.0 } {
                p.collected = true;
                received.push((p.kind, p.position));
            }
        }
        if self.mode == Mode::Combat {
            self.pickups.retain(|p| !p.collected);
        }
        for (kind, position) in received {
            self.collected += 1;
            match kind {
                PickupKind::Coin => {
                    self.combo = (self.combo + 1).min(10);
                    self.best_combo = self.best_combo.max(self.combo);
                    self.combo_time = 5.0 - self.difficulty as usize as f32;
                    self.score += 100 * self.combo;
                    self.announce("COIN CHAIN");
                }
                PickupKind::Time => {
                    self.remaining += 8.0;
                    self.combo_time = 5.0 - self.difficulty as usize as f32;
                    self.score += 250;
                    self.announce("TIME +8 SECONDS");
                }
                PickupKind::Upgrade(w) => {
                    self.levels[w as usize] = (self.levels[w as usize] + 1).min(3);
                    self.weapon = w;
                    self.energy = (self.energy + 25.0).min(100.0);
                    self.announce("WEAPON UPGRADE");
                }
                PickupKind::Energy => {
                    self.energy = 100.0;
                    self.announce("ENERGY REFILLED");
                }
                PickupKind::Shield => {
                    self.health = (self.health + 40.0).min(self.max_health);
                    self.announce("SHIELD REPAIRED");
                }
                PickupKind::Bomb => {
                    self.bombs = (self.bombs + 1).min(5);
                    self.announce("NOVA +1");
                }
                PickupKind::Support => {
                    self.support = 30.0;
                    self.announce("WINGMEN / 30 SECONDS");
                }
            }
            self.effect(position, 12.0, [255, 227, 104]);
        }
    }

    fn spawn_wave(&mut self, flight: &Flight) {
        self.wave += 1;
        let boss = self.wave == 5;
        self.announce(if boss {
            "WARNING / BOSS APPROACH"
        } else {
            "ENEMY WAVE INCOMING"
        });
        let forward = vec3(flight.yaw.sin(), -flight.yaw.cos(), 0.0);
        let right = vec3(flight.yaw.cos(), flight.yaw.sin(), 0.0);
        for i in 0..if boss { 1 } else { 3 + self.wave } {
            let phase = self.random() * std::f32::consts::TAU;
            let kind = if boss { 3 } else { (self.random() * 3.0) as u8 };
            let health = if boss {
                2400.0
            } else {
                [42.0, 64.0, 95.0][kind as usize]
            } * self.difficulty.factor();
            self.next_id += 1;
            self.enemies.push(Enemy {
                id: self.next_id,
                position: flight.position
                    + forward * (210.0 + i as f32 * 24.0)
                    + right * (phase.sin() * 90.0),
                yaw: flight.yaw,
                health,
                max_health: health,
                kind,
                phase,
                cooldown: 1.0 + i as f32 * 0.3,
                hit: 0.0,
            });
        }
        if !boss {
            let kind = if self.wave == 1 {
                PickupKind::Support
            } else {
                PickupKind::Upgrade(Weapon::ALL[(self.wave as usize - 1) % 5])
            };
            self.pickups.push(Pickup {
                position: flight.position + forward * 85.0,
                kind,
                life: 20.0,
                collected: false,
            });
        }
    }

    fn aim(&self, position: Vec3, forward: Vec3, cone: f32) -> Option<(u32, Vec3)> {
        self.enemies
            .iter()
            .filter(|e| {
                e.health > 0.0 && (e.position - position).normalize_or_zero().dot(forward) > cone
            })
            .min_by(|a, b| {
                a.position
                    .distance_squared(position)
                    .total_cmp(&b.position.distance_squared(position))
            })
            .map(|e| (e.id, e.position))
    }

    fn fire(&mut self, flight: &Flight) {
        let w = if self.energy >= self.weapon.cost() {
            self.weapon
        } else {
            Weapon::Gun
        };
        let level = self.levels[w as usize];
        self.energy -= w.cost();
        self.fire_timer = w.interval() / (1.0 + (level - 1) as f32 * 0.12);
        let forward = flight.axes()[1];
        let right = vec3(flight.yaw.cos(), flight.yaw.sin(), 0.0);
        let origin = flight.position + forward * 12.0;
        let aim = self.aim(
            origin,
            forward,
            if w == Weapon::Missile { 0.65 } else { 0.94 },
        );
        let direction = aim.map_or(forward, |(_, p)| (p - origin).normalize_or_zero());
        let damage = [7.0, 34.0, 17.0, 25.0, 48.0][w as usize]
            * flight.vehicle.power()
            * (1.0 + (level - 1) as f32 * 0.3);
        let count = if w == Weapon::Gun {
            level as i32 + 1
        } else if w == Weapon::Missile && level == 3 {
            2
        } else {
            1
        };
        if w == Weapon::Laser {
            let end = origin + direction * 650.0;
            let mut hits = 0;
            for enemy in &mut self.enemies {
                if enemy.health > 0.0
                    && segment_distance(enemy.position, origin, end)
                        < enemy.radius() + 2.0 * level as f32
                {
                    enemy.health -= damage;
                    enemy.hit = 0.12;
                    hits += 1;
                }
            }
            self.hits += hits;
            if self.shots.len() < 180 {
                self.shots.push(Shot {
                    position: end,
                    previous: origin,
                    velocity: Vec3::ZERO,
                    life: 0.1,
                    damage: 0.0,
                    weapon: w,
                    hostile: false,
                    target: None,
                    radius: level as f32,
                });
            }
        } else {
            for i in 0..count {
                if self.shots.len() >= 180 {
                    break;
                }
                let side = i as f32 - (count - 1) as f32 * 0.5;
                let position = origin + right * side * 3.5;
                let velocity = (direction + right * side * 0.018).normalize()
                    * [460.0, 260.0, 0.0, 310.0, 185.0][w as usize];
                self.shots.push(Shot {
                    position,
                    previous: position,
                    velocity,
                    life: 3.8,
                    damage,
                    weapon: w,
                    hostile: false,
                    target: aim.map(|v| v.0),
                    radius: if w == Weapon::Orb {
                        7.0
                    } else if w == Weapon::Plasma {
                        4.0
                    } else {
                        1.5
                    },
                });
            }
        }
        self.shots_fired += 1;
    }

    fn combat(&mut self, dt: f32, flight: &mut Flight, world: &World, previous: Vec3, fire: bool) {
        self.energy = (self.energy + dt * 9.0).min(100.0);
        self.support = (self.support - dt).max(0.0);
        flight.wing_count = if self.support > 0.0 { 2 } else { 0 };
        self.wave_timer -= dt;
        if self.enemies.is_empty() && self.wave_timer <= 0.0 {
            if self.wave == 5 {
                self.score += (self.health * 30.0 + self.remaining * 10.0) as u32;
                self.outcome = Some(true);
                return;
            }
            self.spawn_wave(flight);
        }
        self.fire_timer -= dt;
        if fire && self.fire_timer <= 0.0 {
            self.fire(flight);
        }
        self.support_timer -= dt;
        if self.support > 0.0 && self.support_timer <= 0.0 {
            self.support_timer = 0.5;
            for wing in flight.wingmen.iter().take(2) {
                if let Some((id, target)) = self.aim(wing.position, flight.axes()[1], -0.2)
                    && self.shots.len() < 180
                {
                    self.shots.push(Shot {
                        position: wing.position,
                        previous: wing.position,
                        velocity: (target - wing.position).normalize_or_zero() * 330.0,
                        life: 3.0,
                        damage: 12.0,
                        weapon: Weapon::Missile,
                        hostile: false,
                        target: Some(id),
                        radius: 1.5,
                    });
                }
            }
        }
        let forward = vec3(flight.yaw.sin(), -flight.yaw.cos(), 0.0);
        let right = vec3(flight.yaw.cos(), flight.yaw.sin(), 0.0);
        let difficulty = self.difficulty.factor();
        let mut collision = false;
        for enemy in &mut self.enemies {
            if enemy.health <= 0.0 {
                continue;
            }
            enemy.hit = (enemy.hit - dt).max(0.0);
            enemy.phase += dt * (0.55 + enemy.kind as f32 * 0.12);
            let enraged = enemy.kind >= 3 && enemy.health < enemy.max_health * 0.5;
            let distance = if enemy.kind >= 3 {
                235.0
            } else {
                145.0 + enemy.id as f32 % 4.0 * 30.0
            };
            let lateral = enemy.phase.sin()
                * match enemy.kind {
                    3 => 60.0,
                    4 => 115.0,
                    5 => 75.0,
                    _ => 90.0,
                };
            let mut goal = flight.position + forward * distance + right * lateral;
            goal.z = (flight.position.z + (enemy.phase * 0.7).sin() * 24.0).clamp(100.0, 260.0);
            goal.z = goal.z.max(world.height(goal.x, goal.y) + 45.0);
            enemy.position +=
                (goal - enemy.position) * (dt * if enemy.kind >= 3 { 1.2 } else { 1.8 }).min(1.0);
            enemy.yaw += angle(flight.yaw - enemy.yaw) * (dt * 2.0).min(1.0);
            collision |=
                segment_distance(enemy.position, previous, flight.position) < enemy.radius() + 7.0;
            enemy.cooldown -= dt;
            if enemy.cooldown <= 0.0 {
                enemy.cooldown = if enemy.kind >= 3 {
                    if enraged { 0.7 } else { 1.3 }
                } else {
                    2.5 - enemy.kind as f32 * 0.4
                } / difficulty;
                let aim = (flight.position - enemy.position).normalize_or_zero();
                let count = if enemy.kind >= 3 {
                    if enraged { 11 } else { 7 }
                } else {
                    1 + enemy.kind as i32
                };
                for i in 0..count {
                    if self.shots.len() >= 180 {
                        break;
                    }
                    let offset = i as f32 - (count - 1) as f32 * 0.5;
                    let (horizontal, vertical) = match enemy.kind {
                        4 => (
                            offset * 0.025 + (enemy.phase * 2.0).sin() * 0.16,
                            offset * 0.045,
                        ),
                        5 => {
                            let a = i as f32 * std::f32::consts::TAU / count as f32 + enemy.phase;
                            (a.cos() * 0.24, a.sin() * 0.24)
                        }
                        _ => (
                            offset * 0.075,
                            if enraged {
                                (i as f32 * 1.7 + enemy.phase).sin() * 0.16
                            } else {
                                0.0
                            },
                        ),
                    };
                    let velocity = (aim + right * horizontal + Vec3::Z * vertical).normalize()
                        * (95.0 + 20.0 * difficulty);
                    self.shots.push(Shot {
                        position: enemy.position,
                        previous: enemy.position,
                        velocity,
                        life: 5.0,
                        damage: 10.0 * difficulty,
                        weapon: Weapon::Orb,
                        hostile: true,
                        target: None,
                        radius: if enemy.kind >= 3 { 3.5 } else { 2.5 },
                    });
                }
            }
        }
        if collision {
            self.damage(18.0 * difficulty);
        }
        let mut explosions = Vec::new();
        let mut damage = 0.0_f32;
        for shot in &mut self.shots {
            shot.life -= dt;
            if shot.life <= 0.0 || shot.weapon == Weapon::Laser {
                continue;
            }
            shot.previous = shot.position;
            if !shot.hostile && (shot.weapon == Weapon::Missile || shot.weapon == Weapon::Orb) {
                let target = shot
                    .target
                    .and_then(|id| self.enemies.iter().find(|e| e.id == id && e.health > 0.0));
                if let Some(target) = target {
                    let speed = shot.velocity.length();
                    shot.velocity = shot.velocity.lerp(
                        (target.position - shot.position).normalize_or_zero() * speed,
                        (dt * if shot.weapon == Weapon::Missile {
                            5.0
                        } else {
                            1.8
                        })
                        .min(1.0),
                    );
                }
            }
            shot.position += shot.velocity * dt;
            if shot.hostile {
                let relative_before = shot.previous - previous;
                let relative_after = shot.position - flight.position;
                if segment_distance(Vec3::ZERO, relative_before, relative_after) < shot.radius + 6.0
                {
                    damage = damage.max(shot.damage);
                    shot.life = 0.0;
                }
            } else {
                for enemy in &mut self.enemies {
                    if enemy.health > 0.0
                        && segment_distance(enemy.position, shot.previous, shot.position)
                            < enemy.radius() + shot.radius
                    {
                        enemy.health -= shot.damage;
                        enemy.hit = 0.16;
                        shot.life = 0.0;
                        self.hits += 1;
                        explosions.push((enemy.position, shot.weapon, shot.damage, enemy.id));
                        break;
                    }
                }
            }
            if shot.position.z < world.height(shot.position.x, shot.position.y) {
                shot.life = 0.0;
            }
        }
        if damage > 0.0 {
            self.damage(damage);
        }
        for (position, weapon, damage, id) in explosions {
            let radius = match weapon {
                Weapon::Plasma => 32.0,
                Weapon::Orb => 60.0,
                Weapon::Missile => 18.0,
                _ => 5.0,
            };
            if radius > 10.0 {
                for enemy in &mut self.enemies {
                    if enemy.id != id && enemy.position.distance(position) < radius {
                        enemy.health -= damage * 0.6;
                        enemy.hit = 0.15;
                    }
                }
            }
            self.effect(position, radius, weapon.color());
        }
        self.shots.retain(|s| s.life > 0.0);
        let dead: Vec<_> = self
            .enemies
            .iter()
            .filter(|e| e.health <= 0.0)
            .map(|e| (e.position, e.kind))
            .collect();
        self.enemies.retain(|e| e.health > 0.0);
        for (position, kind) in dead {
            self.kills += 1;
            self.combo = (self.combo + 1).min(10);
            self.combo_time = 5.0;
            self.best_combo = self.best_combo.max(self.combo);
            self.score += (if kind >= 3 {
                10000
            } else {
                300 + kind as u32 * 150
            }) * (1 + self.combo / 3);
            self.effect(
                position,
                if kind >= 3 { 90.0 } else { 23.0 },
                [255, 160, 66],
            );
            let choice = (self.random() * 10.0) as usize;
            let kind = match choice {
                0..=4 => PickupKind::Upgrade(Weapon::ALL[choice]),
                5 => PickupKind::Energy,
                6 => PickupKind::Shield,
                7 => PickupKind::Bomb,
                8 => PickupKind::Support,
                _ => PickupKind::Shield,
            };
            if self.pickups.len() < 32 {
                self.pickups.push(Pickup {
                    position,
                    kind,
                    life: 18.0,
                    collected: false,
                });
            }
            if self.enemies.is_empty() {
                self.wave_timer = 3.0;
                self.announce("SECTOR CLEAR / RESUPPLY");
                self.energy = (self.energy + 20.0).min(100.0);
            }
        }
        if self.health <= 0.0 {
            self.outcome = Some(false);
            self.announce("AIRCRAFT LOST");
        }
    }

    fn damage(&mut self, amount: f32) {
        if self.invulnerable <= 0.0 {
            self.health = (self.health - amount).max(0.0);
            self.invulnerable = 0.9;
            self.announce("SHIELD HIT / ROLL TO EVADE");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::Controls;
    use crate::vehicle::Vehicle;
    use std::sync::OnceLock;

    fn world() -> &'static World {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(|| World::new(7))
    }

    #[test]
    fn gates_require_forward_crossing_and_preserve_order() {
        let world = world();
        let mut flight = Flight::new(world);
        let mut m = Mission::new(Mode::Race, Difficulty::Cadet, &mut flight, world);
        m.countdown = 0.0;
        let gate = m.gates[0].clone();
        let before = gate.position - gate.normal * 80.0;
        flight.position = gate.position + gate.normal * 80.0;
        m.step(1.0 / 30.0, &mut flight, world, before, false);
        assert_eq!((m.gate_index, m.perfect, m.score), (1, 1, 750));
        let gate = m.gates[1].clone();
        flight.position = gate.position - gate.normal * 5.0;
        m.step(
            1.0 / 30.0,
            &mut flight,
            world,
            gate.position + gate.normal * 5.0,
            false,
        );
        assert_eq!(m.gate_index, 1);
        flight.position = gate.position + gate.normal * 5.0 + Vec3::Z * 80.0;
        m.step(
            1.0 / 30.0,
            &mut flight,
            world,
            gate.position - gate.normal * 5.0 + Vec3::Z * 80.0,
            false,
        );
        assert_eq!(m.gate_index, 1);
    }

    #[test]
    fn coin_chain_expires_and_pickups_apply_upgrades() {
        let world = world();
        let mut f = Flight::new(world);
        let mut m = Mission::new(Mode::Coins, Difficulty::Cadet, &mut f, world);
        m.countdown = 0.0;
        for i in 0..2 {
            let p = m.pickups[i].position;
            f.position = p;
            m.step(0.01, &mut f, world, p, false);
        }
        assert_eq!((m.combo, m.score), (2, 300));
        let p = f.position;
        m.step(5.1, &mut f, world, p, false);
        assert_eq!(m.combo, 0);
        m.pickups.clear();
        for kind in [
            PickupKind::Upgrade(Weapon::Laser),
            PickupKind::Upgrade(Weapon::Laser),
            PickupKind::Upgrade(Weapon::Laser),
            PickupKind::Energy,
            PickupKind::Shield,
            PickupKind::Bomb,
            PickupKind::Support,
        ] {
            m.pickups.push(Pickup {
                position: p,
                kind,
                life: 10.0,
                collected: false,
            });
        }
        m.health = 10.0;
        m.energy = 0.0;
        m.collect(0.1, &f, p);
        assert_eq!(m.levels[Weapon::Laser as usize], 3);
        assert_eq!(m.weapon, Weapon::Laser);
        assert_eq!(m.energy, 100.0);
        assert_eq!(m.health, 50.0);
        assert_eq!(m.bombs, 3);
        assert_eq!(m.support, 30.0);
    }

    #[test]
    fn weapons_hit_and_boss_enters_enraged_phase() {
        let world = world();
        for weapon in Weapon::ALL {
            let mut f = Flight::new(world);
            let mut m = Mission::new(Mode::Combat, Difficulty::Cadet, &mut f, world);
            m.countdown = 0.0;
            m.wave = 4;
            m.spawn_wave(&f);
            m.weapon = weapon;
            m.enemies[0].position = f.position + f.axes()[1] * 180.0;
            let before = m.enemies[0].health;
            for _ in 0..180 {
                let p = f.position;
                m.step(1.0 / 30.0, &mut f, world, p, true);
            }
            assert!(
                m.enemies[0].health < before,
                "{} must damage target",
                weapon.name()
            );
            assert!(m.energy >= 0.0 && m.shots.len() <= 180 && m.effects.len() <= 48);
        }
        let mut f = Flight::new(world);
        let mut m = Mission::new(Mode::Combat, Difficulty::Ace, &mut f, world);
        m.countdown = 0.0;
        m.wave = 4;
        m.spawn_wave(&f);
        m.enemies[0].health = m.enemies[0].max_health * 0.4;
        m.enemies[0].cooldown = 0.0;
        let p = f.position;
        m.step(0.01, &mut f, world, p, false);
        assert_eq!(m.shots.iter().filter(|s| s.hostile).count(), 11);
        let before = m.enemies[0].health;
        m.bomb();
        assert!(m.shots.iter().all(|s| !s.hostile));
        assert_eq!(m.enemies[0].health, before - 130.0);
    }

    #[test]
    fn all_vehicles_can_finish_race_and_coin_routes() {
        let world = world();
        for vehicle in Vehicle::ALL {
            for mode in [Mode::Race, Mode::Coins] {
                let mut f = Flight::new(world);
                f.vehicle = vehicle;
                let mut m = Mission::new(mode, Difficulty::Ace, &mut f, world);
                for _ in 0..4500 {
                    let Some(target) = m.target(&f) else {
                        break;
                    };
                    let delta = target - f.position;
                    let controls = Controls {
                        turn: (angle(delta.x.atan2(-delta.y) - f.yaw) * 2.2 / vehicle.turn())
                            .clamp(-1.0, 1.0),
                        climb: (delta.z * 2.0 / (f.speed * 0.55)).clamp(-1.0, 1.0),
                        manual: true,
                        ..Default::default()
                    };
                    let p = f.position;
                    if m.countdown <= 0.0 {
                        f.step(1.0 / 30.0, &controls, world);
                    }
                    m.step(1.0 / 30.0, &mut f, world, p, false);
                    if m.outcome.is_some() {
                        break;
                    }
                }
                println!(
                    "{} {}: {:?}, {:.1}s score={} gates={} coins={}",
                    vehicle.name(),
                    mode.name(),
                    m.outcome,
                    m.elapsed,
                    m.score,
                    m.gate_index,
                    m.collected
                );
                assert_eq!(m.outcome, Some(true), "{} {}", vehicle.name(), mode.name());
            }
        }
    }

    #[test]
    fn full_combat_can_be_won_with_bounded_entities() {
        let world = world();
        let mut f = Flight::new(world);
        let mut m = Mission::new(Mode::Combat, Difficulty::Cadet, &mut f, world);
        for i in 0..9000 {
            let target = m.target(&f).unwrap_or(f.position + f.axes()[1] * 200.0);
            let delta = target - f.position;
            let controls = Controls {
                turn: (angle(delta.x.atan2(-delta.y) - f.yaw) * 2.0 / f.vehicle.turn())
                    .clamp(-1.0, 1.0),
                climb: (delta.z * 1.5 / (f.speed * 0.55)).clamp(-1.0, 1.0),
                manual: true,
                ..Default::default()
            };
            let p = f.position;
            if m.countdown <= 0.0 {
                f.step(1.0 / 30.0, &controls, world);
            }
            if i % 150 == 0 {
                f.roll();
                m.dodge();
            }
            if m.wave == 5 && i % 450 == 0 {
                m.bomb();
            }
            m.step(1.0 / 30.0, &mut f, world, p, true);
            assert!(
                m.enemies.len() <= 16
                    && m.shots.len() <= 180
                    && m.effects.len() <= 48
                    && m.pickups.len() <= 32
            );
            if m.outcome.is_some() {
                break;
            }
        }
        println!(
            "combat {:?}: {:.1}s kills={} score={} shield={}",
            m.outcome, m.elapsed, m.kills, m.score, m.health
        );
        assert_eq!(m.outcome, Some(true));
        assert_eq!(m.wave, 5);
        assert_eq!(m.kills, 23);
    }
}
