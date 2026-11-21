use super::*;
use crate::audio::Sound;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Weapon {
    #[default]
    Gun,
    Missile,
    Laser,
    Plasma,
    Orb,
    Flak,
    Rail,
    Swarm,
}

/// 单种弹药的全部参数；新增弹药只需在 `SPECS` 中加一行
pub struct Spec {
    pub name: &'static str,
    /// HUD 与拾取物上的字母
    pub letter: char,
    pub color: [u8; 3],
    pub sound: Sound,
    /// 开火间隔（秒）与能量消耗
    interval: f32,
    cost: f32,
    damage: f32,
    /// 弹速；0 表示瞬时光束
    pub speed: f32,
    life: f32,
    /// 弹体碰撞半径
    radius: f32,
    /// 命中后的溅射半径；不大于 10 时只有火花
    pub splash: f32,
    /// 追踪转向速率；0 表示直线飞行
    pub homing: f32,
    /// 自动瞄准的视锥（与机头方向夹角余弦）
    cone: f32,
    /// 每级弹丸数与相邻弹丸的横向散布
    pellets: [u8; 3],
    spread: f32,
    /// 穿透：命中后继续飞行
    pub pierce: bool,
    /// 渲染为曳光弹
    pub tracer: bool,
}

const SPECS: [Spec; Weapon::COUNT] = [
    Spec {
        name: "GUN",
        letter: 'G',
        color: [255, 225, 128],
        sound: Sound::Gun,
        interval: 0.14,
        cost: 0.0,
        damage: 7.0,
        speed: 460.0,
        life: 3.8,
        radius: 1.5,
        splash: 5.0,
        homing: 0.0,
        cone: 0.94,
        pellets: [2, 3, 4],
        spread: 0.018,
        pierce: false,
        tracer: true,
    },
    Spec {
        name: "MISSILE",
        letter: 'M',
        color: [255, 141, 67],
        sound: Sound::Missile,
        interval: 0.65,
        cost: 9.0,
        damage: 34.0,
        speed: 260.0,
        life: 3.8,
        radius: 1.5,
        splash: 18.0,
        homing: 5.0,
        cone: 0.65,
        pellets: [1, 1, 2],
        spread: 0.018,
        pierce: false,
        tracer: true,
    },
    Spec {
        name: "LASER",
        letter: 'L',
        color: [88, 242, 255],
        sound: Sound::Laser,
        interval: 0.34,
        cost: 5.0,
        damage: 17.0,
        speed: 0.0,
        life: 0.1,
        radius: 0.0,
        splash: 5.0,
        homing: 0.0,
        cone: 0.94,
        pellets: [1, 1, 1],
        spread: 0.0,
        pierce: true,
        tracer: false,
    },
    Spec {
        name: "PLASMA",
        letter: 'P',
        color: [244, 94, 255],
        sound: Sound::Plasma,
        interval: 0.42,
        cost: 7.0,
        damage: 25.0,
        speed: 310.0,
        life: 3.8,
        radius: 4.0,
        splash: 32.0,
        homing: 0.0,
        cone: 0.94,
        pellets: [1, 1, 1],
        spread: 0.018,
        pierce: false,
        tracer: false,
    },
    Spec {
        name: "ORB",
        letter: 'O',
        color: [130, 255, 151],
        sound: Sound::Orb,
        interval: 0.85,
        cost: 12.0,
        damage: 48.0,
        speed: 185.0,
        life: 3.8,
        radius: 7.0,
        splash: 60.0,
        homing: 1.8,
        cone: 0.94,
        pellets: [1, 1, 1],
        spread: 0.018,
        pierce: false,
        tracer: false,
    },
    // 近距霰弹：大散布、射程短，每颗弹丸带小范围溅射
    Spec {
        name: "FLAK",
        letter: 'F',
        color: [255, 196, 64],
        sound: Sound::Flak,
        interval: 0.5,
        cost: 4.0,
        damage: 8.0,
        speed: 380.0,
        life: 0.75,
        radius: 2.0,
        splash: 14.0,
        homing: 0.0,
        cone: 0.85,
        pellets: [5, 7, 9],
        spread: 0.06,
        pierce: false,
        tracer: true,
    },
    // 电磁轨道炮：极高弹速、高伤害，贯穿沿线全部目标
    Spec {
        name: "RAIL",
        letter: 'R',
        color: [186, 168, 255],
        sound: Sound::Rail,
        interval: 1.1,
        cost: 14.0,
        damage: 80.0,
        speed: 950.0,
        life: 1.0,
        radius: 2.0,
        splash: 5.0,
        homing: 0.0,
        cone: 0.97,
        pellets: [1, 1, 1],
        spread: 0.0,
        pierce: true,
        tracer: true,
    },
    // 蜂群微型导弹：每枚锁定不同目标
    Spec {
        name: "SWARM",
        letter: 'H',
        color: [255, 112, 160],
        sound: Sound::Swarm,
        interval: 0.9,
        cost: 10.0,
        damage: 13.0,
        speed: 230.0,
        life: 3.2,
        radius: 1.5,
        splash: 12.0,
        homing: 7.0,
        cone: 0.3,
        pellets: [3, 4, 6],
        spread: 0.12,
        pierce: false,
        tracer: false,
    },
];

impl Weapon {
    pub const COUNT: usize = 8;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Gun,
        Self::Missile,
        Self::Laser,
        Self::Plasma,
        Self::Orb,
        Self::Flak,
        Self::Rail,
        Self::Swarm,
    ];
    pub fn spec(self) -> &'static Spec {
        &SPECS[self as usize]
    }
    pub fn name(self) -> &'static str {
        self.spec().name
    }
    pub fn letter(self) -> char {
        self.spec().letter
    }
    pub fn color(self) -> [u8; 3] {
        self.spec().color
    }
}

impl Mission {
    pub fn cycle_weapon(&mut self) {
        self.weapon = Weapon::ALL[(self.weapon as usize + 1) % Weapon::COUNT];
        self.fire_timer = 0.0;
        self.sounds.push(Sound::WeaponSwitch(self.weapon as u8));
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
            self.sounds.push(Sound::Overdrive);
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
        self.sounds.push(Sound::Nova);
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
    /// 视锥内的存活敌机，按距离（打击阶段导弹优先地面目标）由近到远
    fn targets(&self, position: Vec3, forward: Vec3, cone: f32) -> Vec<(f32, u32, Vec3)> {
        let ground_first = self.phase() == Phase::Strike && self.weapon == Weapon::Missile;
        let mut targets: Vec<_> = self
            .enemies
            .iter()
            .filter(|e| {
                e.health > 0.0 && (e.position - position).normalize_or_zero().dot(forward) > cone
            })
            .map(|e| {
                let weight = if ground_first && e.is_ground() {
                    0.08
                } else {
                    1.0
                };
                (
                    e.position.distance_squared(position) * weight,
                    e.id,
                    e.position,
                )
            })
            .collect();
        targets.sort_by(|a, b| a.0.total_cmp(&b.0));
        targets
    }
    pub(super) fn aim(&self, position: Vec3, forward: Vec3, cone: f32) -> Option<(u32, Vec3)> {
        self.targets(position, forward, cone)
            .first()
            .map(|&(_, id, p)| (id, p))
    }
    pub(super) fn fire(&mut self, flight: &Flight) {
        let w = if self.energy >= self.weapon.spec().cost {
            self.weapon
        } else {
            Weapon::Gun
        };
        let spec = w.spec();
        let level = self.levels[w as usize];
        let overdrive = self.overdrive_time > 0.0;
        self.energy -= spec.cost;
        self.fire_timer =
            spec.interval / (1.0 + (level - 1) as f32 * 0.12) / if overdrive { 1.4 } else { 1.0 };
        let forward = flight.axes()[1];
        let right = vec3(flight.yaw.cos(), flight.yaw.sin(), 0.0);
        let origin = flight.position + forward * 12.0;
        let targets = self.targets(origin, forward, spec.cone);
        let direction = targets
            .first()
            .map_or(forward, |t| (t.2 - origin).normalize_or_zero());
        let damage = spec.damage
            * (1.0 + self.upgrades[0] as f32 * 0.08)
            * if overdrive { 1.25 } else { 1.0 }
            * flight.vehicle.power()
            * (1.0 + (level - 1) as f32 * 0.3);
        if spec.speed == 0.0 {
            self.beam(w, origin, direction * 650.0, damage, level);
        } else {
            let count = spec.pellets[level.clamp(1, 3) as usize - 1] as usize;
            for i in 0..count {
                if self.shots.len() >= 180 {
                    break;
                }
                let side = i as f32 - (count - 1) as f32 * 0.5;
                // 霰弹在竖直方向上交错，形成弹幕而非一条横线
                let lift = if w == Weapon::Flak {
                    (i as f32 * 2.4).sin() * spec.spread * 0.6
                } else {
                    0.0
                };
                let position = origin + right * side * 3.5;
                let velocity = (direction + right * side * spec.spread + Vec3::Z * lift)
                    .normalize()
                    * spec.speed;
                let target = if w == Weapon::Swarm && !targets.is_empty() {
                    Some(targets[i % targets.len()].1)
                } else if spec.pierce {
                    None
                } else {
                    targets.first().map(|t| t.1)
                };
                self.shots.push(Shot {
                    position,
                    previous: position,
                    velocity,
                    life: spec.life,
                    damage,
                    weapon: w,
                    hostile: false,
                    grazed: false,
                    target,
                    radius: spec.radius,
                });
            }
        }
        self.shots_fired += 1;
        self.muzzle = 0.06;
        self.sounds.push(spec.sound);
    }
    /// 瞬时光束：沿线命中全部敌机，只留一帧光迹
    fn beam(&mut self, w: Weapon, origin: Vec3, ray: Vec3, damage: f32, level: u8) {
        let end = origin + ray;
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
                life: w.spec().life,
                damage: 0.0,
                weapon: w,
                hostile: false,
                grazed: false,
                target: None,
                radius: level as f32,
            });
        }
    }
}
