use super::*;
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

impl Mission {
    pub fn cycle_weapon(&mut self) {
        self.weapon = Weapon::ALL[(self.weapon as usize + 1) % 5];
        self.fire_timer = 0.0;
        self.sounds.push(crate::audio::Sound::Navigate);
    }
    pub fn activate_overdrive(&mut self) {
        if self.mode == Mode::Combat
            && self.countdown <= 0.0
            && self.overdrive >= 100.0
            && self.overdrive_time <= 0.0
        {
            self.overdrive = 0.0;
            self.overdrive_time = 8.0;
            self.announce("OVERDRIVE");
            self.sounds.push(crate::audio::Sound::Overdrive);
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
        self.sounds.push(crate::audio::Sound::Nova);
        self.flash = 0.65;
        self.shots.retain(|s| !s.hostile);
        for enemy in &mut self.enemies {
            enemy.health -= if enemy.is_boss() { 130.0 } else { 200.0 };
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
    pub(super) fn aim(&self, position: Vec3, forward: Vec3, cone: f32) -> Option<(u32, Vec3)> {
        self.enemies
            .iter()
            .filter(|e| {
                e.health > 0.0 && (e.position - position).normalize_or_zero().dot(forward) > cone
            })
            .min_by(|a, b| {
                let score = |e: &Enemy| {
                    e.position.distance_squared(position)
                        * if self.phase() == Phase::Strike
                            && self.weapon == Weapon::Missile
                            && e.is_ground()
                        {
                            0.08
                        } else {
                            1.0
                        }
                };
                score(a).total_cmp(&score(b))
            })
            .map(|e| (e.id, e.position))
    }
    pub(super) fn fire(&mut self, flight: &Flight) {
        let w = if self.energy >= self.weapon.cost() {
            self.weapon
        } else {
            Weapon::Gun
        };
        let level = self.levels[w as usize];
        self.energy -= w.cost();
        self.fire_timer = w.interval()
            / (1.0 + (level - 1) as f32 * 0.12)
            / if self.overdrive_time > 0.0 { 1.4 } else { 1.0 };
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
            * (1.0 + self.upgrades[0] as f32 * 0.08)
            * if self.overdrive_time > 0.0 { 1.25 } else { 1.0 }
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
            self.overdrive = (self.overdrive + hits as f32 * 0.35).min(100.0);
            if self.shots.len() < 180 {
                self.shots.push(Shot {
                    position: end,
                    previous: origin,
                    velocity: Vec3::ZERO,
                    life: 0.1,
                    damage: 0.0,
                    weapon: w,
                    hostile: false,
                    grazed: false,
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
                    grazed: false,
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
        self.sounds.push(match w {
            Weapon::Gun => crate::audio::Sound::Gun,
            Weapon::Missile => crate::audio::Sound::Missile,
            Weapon::Laser => crate::audio::Sound::Laser,
            Weapon::Plasma => crate::audio::Sound::Plasma,
            Weapon::Orb => crate::audio::Sound::Orb,
        });
    }
}
