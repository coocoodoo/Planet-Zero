//! Pirate raiders in space: they dive at the ship, fire when lined up, break off, and come round
//! again. Random raids are rare (every 30 minutes to 3 hours of play); bounty and ace missions
//! call their own targets.
use super::combat::Projectile;
use super::names;
use super::rng::Rng;
use glam::{Vec3, vec3};

pub const MAX_PIRATES: usize = 4;
/// Seconds of play between random raids
pub const RAID_MIN: f32 = 30.0 * 60.0;
pub const RAID_MAX: f32 = 3.0 * 60.0 * 60.0;
/// Pirates hit the ship within this radius
pub const SHIP_RADIUS: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Attack,
    BreakOff,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pirate {
    pub id: u32,
    pub position: Vec3,
    pub forward: Vec3,
    pub speed: f32,
    pub health: f32,
    pub max_health: f32,
    pub ace: bool,
    pub name: String,
    pub cooldown: f32,
    mode: Mode,
    mode_time: f32,
    pub flash: f32,
    side: f32,
}

/// Seconds until the next random raid.
pub fn next_raid(rng: &mut Rng) -> f32 {
    rng.range(RAID_MIN, RAID_MAX)
}

impl Pirate {
    pub fn new(id: u32, position: Vec3, ace: bool, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let health = if ace { 220.0 } else { 70.0 };
        Self {
            id,
            position,
            forward: -position.normalize_or_zero(),
            speed: if ace { 175.0 } else { 150.0 },
            health,
            max_health: health,
            ace,
            name: if ace {
                names::name(seed, 55)
            } else {
                "RAIDER".to_owned()
            },
            cooldown: rng.range(1.0, 2.5),
            mode: Mode::Attack,
            mode_time: 0.0,
            flash: 0.0,
            side: if rng.below(2) == 0 { 1.0 } else { -1.0 },
        }
    }
    /// A raid of `n` raiders (plus an ace if asked) appearing ahead of the ship.
    pub fn wave(
        first_id: u32,
        ship: Vec3,
        forward: Vec3,
        n: usize,
        ace: bool,
        rng: &mut Rng,
    ) -> Vec<Self> {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        (0..n + usize::from(ace))
            .map(|i| {
                let spread = (i as f32 - n as f32 * 0.5) * 220.0;
                let at = ship
                    + forward * rng.range(1600.0, 2100.0)
                    + right * spread
                    + Vec3::Z * rng.range(-200.0, 200.0);
                let mut p = Self::new(
                    first_id + i as u32,
                    at,
                    ace && i == n,
                    rng.next_u32() as u64,
                );
                p.forward = (ship - at).normalize_or_zero();
                p
            })
            .collect()
    }
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
    /// Fly toward an intercept with the ship, fire when lined up, break off when close.
    pub fn step(
        &mut self,
        ship: Vec3,
        ship_velocity: Vec3,
        dt: f32,
        rng: &mut Rng,
    ) -> Option<Projectile> {
        self.flash = (self.flash - dt).max(0.0);
        self.cooldown -= dt;
        self.mode_time += dt;
        let to = ship - self.position;
        let distance = to.length().max(0.01);
        let lead = ship + ship_velocity * (distance / 700.0);
        let want = match self.mode {
            Mode::Attack => {
                if distance < 230.0 {
                    self.mode = Mode::BreakOff;
                    self.mode_time = 0.0;
                    self.side = -self.side;
                }
                (lead - self.position).normalize_or_zero()
            }
            Mode::BreakOff => {
                if self.mode_time > 3.0 {
                    self.mode = Mode::Attack;
                    self.mode_time = 0.0;
                }
                let away = -to / distance;
                let across = away.cross(Vec3::Z).normalize_or(Vec3::X) * self.side;
                (away + across * 0.8 + Vec3::Z * 0.2).normalize_or_zero()
            }
        };
        // limited turn rate gives dogfights their arcs
        let turn = if self.ace { 1.5 } else { 1.1 } * dt;
        let angle = self.forward.dot(want).clamp(-1.0, 1.0).acos();
        if angle > 0.0001 {
            let rotation = glam::Quat::from_rotation_arc(self.forward, want);
            self.forward = (glam::Quat::IDENTITY.slerp(rotation, (turn / angle).min(1.0))
                * self.forward)
                .normalize_or_zero();
        }
        self.position += self.forward * self.speed * dt;
        let aim = (lead - self.position).normalize_or_zero();
        if self.mode == Mode::Attack
            && distance < 900.0
            && self.forward.dot(aim) > 0.975
            && self.cooldown <= 0.0
        {
            self.cooldown = if self.ace { 0.5 } else { 0.8 } * rng.range(0.8, 1.2);
            let jitter = vec3(
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
            ) * 0.012;
            let direction = (aim + jitter).normalize_or_zero();
            return Some(Projectile {
                position: self.position + direction * 12.0,
                velocity: direction * 620.0,
                life: 2.0,
                damage: if self.ace { 9.0 } else { 6.0 },
                hostile: true,
                color: [255, 90, 70],
                gravity: 0.0,
                splash: 0.0,
                bounces: 0,
                pierce: false,
                homing: 0.0,
                slow: 0.0,
                siphon: 0.0,
                volatile: false,
                mining: 0.0,
                last_hit: self.id,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raids_are_rare() {
        let mut rng = Rng::new(4);
        for _ in 0..500 {
            let t = next_raid(&mut rng);
            assert!((RAID_MIN..RAID_MAX).contains(&t));
        }
    }
    #[test]
    fn pirates_close_in_fire_and_break_off() {
        let mut rng = Rng::new(9);
        let ship = Vec3::new(100.0, -40.0, 30.0);
        let mut wave = Pirate::wave(1, ship, Vec3::Y, 2, true, &mut rng);
        assert_eq!(wave.len(), 3);
        assert_eq!(wave.iter().filter(|p| p.ace).count(), 1);
        let pirate = &mut wave[0];
        let start = pirate.position.distance(ship);
        let mut shots = 0;
        let mut closest = f32::INFINITY;
        for _ in 0..30 * 40 {
            if let Some(shot) = pirate.step(ship, Vec3::ZERO, 1.0 / 30.0, &mut rng) {
                shots += 1;
                assert!(shot.hostile);
                assert!(
                    shot.velocity
                        .normalize()
                        .dot((ship - shot.position).normalize())
                        > 0.95
                );
            }
            closest = closest.min(pirate.position.distance(ship));
        }
        assert!(start > 1500.0);
        assert!(closest < 300.0, "it closes in ({closest})");
        assert!(closest > 50.0, "and breaks off before ramming");
        assert!(shots >= 3, "{shots}");
    }
}
