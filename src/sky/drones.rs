//! Guard drones: the only enemies. They hover over the terrain, keep their preferred range,
//! strafe, and fire only with a clear line of sight. An alert level (raised by mining, looting
//! and shooting, falling when quiet) decides when patrols come.
use super::combat::{Projectile, clear_line};
use super::rng::Rng;
use super::walker::ground;
use crate::render::{ExtraTriangle, mix, rgb};
use crate::world::World;
use glam::{Vec3, vec3};

pub const MAX_DRONES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DroneKind {
    Scout,
    Warden,
    Hive,
}

pub struct DroneSpec {
    pub name: &'static str,
    pub health: f32,
    pub speed: f32,
    /// Distance kept from the player
    pub range: f32,
    pub hover: f32,
    pub interval: f32,
    pub damage: f32,
    pub bolt_speed: f32,
    pub radius: f32,
    /// Glow before a shot, seconds
    pub telegraph: f32,
    pub credits: u32,
    /// Chance of a drone core
    pub core: f32,
    /// Chance of a weapon
    pub weapon: f32,
}

const SPECS: [DroneSpec; 3] = [
    DroneSpec {
        name: "SCOUT",
        health: 22.0,
        speed: 16.0,
        range: 22.0,
        hover: 6.0,
        interval: 1.6,
        damage: 6.0,
        bolt_speed: 46.0,
        radius: 1.2,
        telegraph: 0.2,
        credits: 20,
        core: 0.2,
        weapon: 0.05,
    },
    DroneSpec {
        name: "WARDEN",
        health: 70.0,
        speed: 9.0,
        range: 30.0,
        hover: 9.0,
        interval: 2.4,
        damage: 14.0,
        bolt_speed: 34.0,
        radius: 2.0,
        telegraph: 0.45,
        credits: 60,
        core: 0.5,
        weapon: 0.12,
    },
    DroneSpec {
        name: "HIVE",
        health: 140.0,
        speed: 5.0,
        range: 42.0,
        hover: 13.0,
        interval: 3.2,
        damage: 10.0,
        bolt_speed: 30.0,
        radius: 3.0,
        telegraph: 0.45,
        credits: 140,
        core: 0.85,
        weapon: 0.3,
    },
];

impl DroneKind {
    pub fn spec(self) -> &'static DroneSpec {
        &SPECS[self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drone {
    pub id: u32,
    pub kind: DroneKind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub health: f32,
    pub cooldown: f32,
    /// Seconds left in the pre-shot glow (0 when not about to fire)
    pub telegraph: f32,
    pub slow: f32,
    /// +1 or -1: which way it circles
    pub strafe: f32,
    /// Hit flash, seconds
    pub flash: f32,
    /// Hive: seconds to the next scout launch
    pub launch: f32,
    pub sight: bool,
    pub time: f32,
}

/// What a drone did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Action {
    pub shot: Option<Projectile>,
    /// A hive wants to launch a scout here
    pub launch: Option<Vec3>,
}

impl Drone {
    pub fn new(id: u32, kind: DroneKind, position: Vec3, rng: &mut Rng) -> Self {
        Self {
            id,
            kind,
            position,
            velocity: Vec3::ZERO,
            health: kind.spec().health,
            cooldown: rng.range(0.8, 2.0),
            telegraph: 0.0,
            slow: 0.0,
            strafe: if rng.below(2) == 0 { 1.0 } else { -1.0 },
            flash: 0.0,
            launch: 6.0,
            sight: false,
            time: rng.range(0.0, 10.0),
        }
    }
    pub fn radius(&self) -> f32 {
        self.kind.spec().radius
    }
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
    /// Move toward the hover point near `target` (the player's chest) and maybe fire.
    pub fn step(&mut self, world: &World, target: Vec3, dt: f32, rng: &mut Rng) -> Action {
        let spec = self.kind.spec();
        let mut action = Action::default();
        self.time += dt;
        self.flash = (self.flash - dt).max(0.0);
        self.slow = (self.slow - dt).max(0.0);
        let pace = if self.slow > 0.0 { 0.5 } else { 1.0 };
        // circle the player at the preferred range
        let flat = (self.position - target).truncate();
        let distance = flat.length().max(0.01);
        let around = flat / distance;
        let angle = around.y.atan2(around.x) + self.strafe * 0.35 * pace * dt * 3.0;
        let goal = target.truncate() + glam::vec2(angle.cos(), angle.sin()) * spec.range;
        let floor =
            ground(world, goal.x, goal.y).max(ground(world, self.position.x, self.position.y));
        let goal = goal.extend((floor + spec.hover).max(target.z + spec.hover * 0.5))
            + Vec3::Z * (self.time * 1.7).sin() * 0.8;
        let wish = (goal - self.position).clamp_length_max(spec.speed * pace);
        self.velocity += (wish - self.velocity) * (2.5 * dt).min(1.0);
        self.position += self.velocity * dt;
        let below = ground(world, self.position.x, self.position.y) + 2.5;
        if self.position.z < below {
            self.position.z = below;
            self.velocity.z = self.velocity.z.max(0.0);
        }
        // fire with line of sight
        let to = target - self.position;
        let reach = to.length();
        self.sight = reach < spec.range * 2.2 && clear_line(world, self.position, target);
        if self.telegraph > 0.0 {
            self.telegraph -= dt;
            if self.telegraph <= 0.0 {
                self.telegraph = 0.0;
                if self.sight {
                    let aim = (to / reach.max(0.01)
                        + vec3(
                            rng.range(-0.04, 0.04),
                            rng.range(-0.04, 0.04),
                            rng.range(-0.02, 0.03),
                        ))
                    .normalize_or_zero();
                    action.shot = Some(Projectile {
                        position: self.position + aim * (self.radius() + 0.5),
                        velocity: aim * spec.bolt_speed,
                        life: spec.range * 3.0 / spec.bolt_speed,
                        damage: spec.damage,
                        hostile: true,
                        color: [255, 80, 70],
                        gravity: 0.0,
                        splash: if self.kind == DroneKind::Warden {
                            2.5
                        } else {
                            0.0
                        },
                        bounces: 0,
                        pierce: false,
                        homing: 0.0,
                        slow: 0.0,
                        siphon: 0.0,
                        volatile: false,
                        mining: 0.0,
                        last_hit: 0,
                    });
                }
            }
        } else if self.sight {
            self.cooldown -= dt * pace;
            if self.cooldown <= 0.0 {
                self.cooldown = spec.interval * rng.range(0.8, 1.2);
                self.telegraph = spec.telegraph;
            }
        }
        if self.kind == DroneKind::Hive {
            self.launch -= dt;
            if self.launch <= 0.0 {
                self.launch = 9.0;
                action.launch = Some(self.position - Vec3::Z * 2.0);
            }
        }
        action
    }
    pub fn mesh(&self, tint: u32, out: &mut Vec<ExtraTriangle>) {
        let hull = if self.flash > 0.0 {
            rgb(255, 255, 255)
        } else {
            mix(rgb(70, 76, 92), tint, 0.35)
        };
        let dark = mix(hull, rgb(0, 0, 0), 0.4);
        let glow = mix(
            rgb(255, 60, 50),
            rgb(255, 240, 200),
            (self.telegraph * 4.0).min(1.0),
        );
        let p = self.position;
        let spin = self.time * 1.5;
        let r = self.radius();
        match self.kind {
            DroneKind::Scout => {
                super::props::spindle(
                    out,
                    p - Vec3::Z * r * 0.7,
                    p,
                    p + Vec3::Z * r * 0.7,
                    r,
                    4,
                    spin,
                    (hull, dark),
                    false,
                );
                super::props::cuboid(
                    out,
                    p + vec3(spin.cos(), spin.sin(), 0.0) * r * 0.9,
                    Vec3::splat(0.3),
                    spin,
                    glow,
                    true,
                );
            }
            DroneKind::Warden => {
                super::props::cuboid(
                    out,
                    p,
                    vec3(r * 0.8, r * 0.8, r * 0.5),
                    spin * 0.3,
                    hull,
                    false,
                );
                for side in [-1.0, 1.0] {
                    let pod =
                        p + vec3((spin * 0.3).cos(), (spin * 0.3).sin(), 0.0) * side * r * 1.1;
                    super::props::spindle(
                        out,
                        pod - Vec3::Z * 0.6,
                        pod,
                        pod + Vec3::Z * 0.6,
                        0.6,
                        3,
                        spin,
                        (dark, dark),
                        false,
                    );
                }
                super::props::cuboid(
                    out,
                    p + Vec3::Z * r * 0.55,
                    vec3(0.5, 0.5, 0.25),
                    spin,
                    glow,
                    true,
                );
            }
            DroneKind::Hive => {
                super::props::spindle(
                    out,
                    p - Vec3::Z * r,
                    p,
                    p + Vec3::Z * r * 0.8,
                    r,
                    6,
                    spin * 0.2,
                    (hull, dark),
                    false,
                );
                for k in 0..3 {
                    let a = spin + k as f32 * std::f32::consts::TAU / 3.0;
                    super::props::cuboid(
                        out,
                        p + vec3(a.cos(), a.sin(), 0.0) * r * 1.2,
                        Vec3::splat(0.45),
                        a,
                        glow,
                        true,
                    );
                }
            }
        }
    }
}

/// Guard alert: 0 calm … 1 every drone in range is hunting you.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Alert {
    pub level: f32,
    /// Seconds since anything raised the alert
    pub quiet: f32,
    /// Seconds until the next patrol may arrive
    pub patrol: f32,
}
impl Alert {
    pub fn raise(&mut self, amount: f32) {
        self.level = (self.level + amount).min(1.0);
        self.quiet = 0.0;
    }
    pub fn decay(&mut self, dt: f32) {
        self.quiet += dt;
        self.patrol = (self.patrol - dt).max(0.0);
        if self.quiet > 12.0 {
            self.level = (self.level - dt * 0.02).max(0.0);
        }
    }
    /// A patrol to send now, if the alert calls for one.
    pub fn patrol_kind(&mut self, active: usize, rng: &mut Rng) -> Option<DroneKind> {
        if self.patrol > 0.0 || active >= MAX_DRONES || self.level < 0.3 {
            return None;
        }
        let wanted = 1 + (self.level * 3.0) as usize;
        if active >= wanted {
            return None;
        }
        self.patrol = 7.0;
        Some(if self.level > 0.85 && rng.below(3) == 0 {
            DroneKind::Hive
        } else if self.level > 0.55 && rng.below(2) == 0 {
            DroneKind::Warden
        } else {
            DroneKind::Scout
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapKind;
    #[test]
    fn drones_keep_their_range_and_shoot_only_with_line_of_sight() {
        let world = World::generate(MapKind::Desert, 5);
        let (x, y) = world.spawn;
        let target = vec3(x, y, ground(&world, x, y) + 1.4);
        let mut rng = Rng::new(3);
        let mut drone = Drone::new(
            1,
            DroneKind::Scout,
            target + vec3(60.0, 0.0, 20.0),
            &mut rng,
        );
        let mut shots = 0;
        for _ in 0..600 {
            let action = drone.step(&world, target, 1.0 / 30.0, &mut rng);
            if let Some(shot) = action.shot {
                shots += 1;
                assert!(shot.hostile);
                assert!(
                    shot.velocity
                        .normalize()
                        .dot((target - shot.position).normalize())
                        > 0.9
                );
            }
            assert!(drone.position.z >= ground(&world, drone.position.x, drone.position.y) + 2.4);
        }
        let flat = (drone.position - target).truncate().length();
        assert!((12.0..35.0).contains(&flat), "{flat}");
        assert!(shots >= 3, "{shots}");
        // a target underground can never be seen
        let hidden = target - Vec3::Z * 30.0;
        let mut quiet = Drone::new(
            2,
            DroneKind::Scout,
            target + vec3(20.0, 0.0, 10.0),
            &mut rng,
        );
        for _ in 0..300 {
            assert!(
                quiet
                    .step(&world, hidden, 1.0 / 30.0, &mut rng)
                    .shot
                    .is_none()
            );
        }
    }
    #[test]
    fn alert_calls_patrols_and_cools_down() {
        let mut alert = Alert::default();
        let mut rng = Rng::new(1);
        assert!(alert.patrol_kind(0, &mut rng).is_none());
        alert.raise(0.4);
        assert_eq!(alert.patrol_kind(0, &mut rng), Some(DroneKind::Scout));
        assert!(
            alert.patrol_kind(0, &mut rng).is_none(),
            "one patrol at a time"
        );
        alert.raise(1.0);
        alert.patrol = 0.0;
        assert!(alert.patrol_kind(MAX_DRONES, &mut rng).is_none());
        for _ in 0..30 * 70 {
            alert.decay(1.0 / 30.0);
        }
        assert!(alert.level < 0.1);
    }
}
