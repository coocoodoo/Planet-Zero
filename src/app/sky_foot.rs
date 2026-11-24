//! Cardputer Sky on foot: weapons, guard drones, interactions, the weapons view, lore, and the
//! station armory. `app::sky` calls into this from its key and frame handlers.
use super::*;
use crate::audio::Sound;
use crate::flight::heading;
use crate::keys::*;
use crate::sky::arsenal::{Fire, Rarity, Stats, Weapon};
use crate::sky::combat::{
    Effect, EffectKind, MAX_EFFECTS, MAX_PROJECTILES, PLAYER_RADIUS, Projectile, push_capped,
    ray_sphere, terrain_hit,
};
use crate::sky::drones::{Drone, DroneKind, MAX_DRONES};
use crate::sky::inventory::Item;
use crate::sky::missions::Event;
use crate::sky::props::{self, Prop, PropKind};
use crate::sky::walker::{Walker, ground};
use crate::sky::{LOCKER, Overlay, Phase};
use glam::{Vec2, Vec3, vec2, vec3};

const CYAN: [u8; 3] = [111, 237, 218];
const GOLD: [u8; 3] = [255, 208, 105];
const RED: [u8; 3] = [255, 112, 130];
const WHITE: [u8; 3] = [225, 238, 244];
/// How close you must be to open, read or signal
pub const INTERACT_DISTANCE: f32 = 6.5;
/// Maximum tilt fine-aim offsets
pub const AIM_YAW: f32 = 0.21;
pub const AIM_PITCH: f32 = 0.16;
/// Drones further than this give up
const LEASH: f32 = 220.0;

/// What key 7 (or E) does right now on foot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Context {
    Board,
    Open(Prop),
    Read(Prop),
    Signal(Prop),
    /// Aboard a station: someone to talk to
    Talk(usize),
    Swap,
}
impl Context {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Board => "BOARD",
            Self::Open(_) => "OPEN",
            Self::Read(_) => "READ",
            Self::Signal(_) => "SIGNAL",
            Self::Talk(_) => "TALK",
            Self::Swap => "SWAP",
        }
    }
}

/// A drone's hit radius for shots from `origin`: forgiving, and more so far away, because a
/// drone 20 units off is only a few pixels wide on the Cardputer's screen.
pub fn hitbox(drone: &Drone, origin: Vec3) -> f32 {
    drone.radius() + 0.5 + drone.position.distance(origin) * 0.035
}

/// What a ray from the crosshair hits first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Drone(usize),
    Prop(Prop),
    Ground,
    Nothing,
}

impl App {
    // ----- aiming ---------------------------------------------------------------------------

    /// Feed the fused gyroscope angles (radians). On foot, small tilts fine-aim the view
    /// relative to the pose when it was last centred.
    pub fn sky_tilt(&mut self, roll: f32, pitch: f32, ready: bool) {
        if !self.explore() || self.sky.phase != Phase::OnFoot || !self.tilt_aim || !ready {
            self.sky.aim = Vec2::ZERO;
            self.sky.tilt_ref = None;
            return;
        }
        let (r0, p0) = *self.sky.tilt_ref.get_or_insert((roll, pitch));
        self.sky.aim = vec2(
            ((roll - r0) * 0.7).clamp(-AIM_YAW, AIM_YAW),
            ((pitch - p0) * 0.7).clamp(-AIM_PITCH, AIM_PITCH),
        );
    }
    /// Recentre the fine-aim (K, landing).
    pub fn sky_recenter(&mut self) {
        self.sky.tilt_ref = None;
        self.sky.aim = Vec2::ZERO;
    }
    /// The on-foot viewpoint: walker head plus fine-aim offsets.
    pub(super) fn foot_eye(&self) -> crate::flight::Eye {
        let mut eye = self.sky.walker.eye();
        eye.yaw += self.sky.aim.x;
        eye.pitch += self.sky.aim.y;
        eye
    }
    fn aim_ray(&self) -> (Vec3, Vec3) {
        let camera = crate::flight::Camera::new(&self.flight, WIDTH, HEIGHT);
        let (origin, direction) = camera.ray(super::sky::CROSSHAIR);
        (origin, direction.normalize_or_zero())
    }
    /// Ease the view toward the nearest drone (or minable prop for mining weapons) inside the
    /// weapon's assist cone.
    fn assist_aim(&mut self, stats: &Stats, dt: f32) {
        if !self.aim_assist {
            return;
        }
        let (origin, direction) = self.aim_ray();
        let mut best: Option<(f32, Vec3)> = None;
        let consider = |point: Vec3, best: &mut Option<(f32, Vec3)>| {
            let to = point - origin;
            let distance = to.length();
            if distance > stats.range || distance < 1.0 {
                return;
            }
            let angle = direction.dot(to / distance).clamp(-1.0, 1.0).acos();
            if angle < stats.assist && best.is_none_or(|(a, _)| angle < a) {
                *best = Some((angle, point));
            }
        };
        for drone in &self.sky.drones {
            consider(drone.position, &mut best);
        }
        if best.is_none() && stats.mining >= 1.0 {
            for prop in &self.sky.props {
                if prop.harvest(crate::sky::biome::Biome::Lush).is_some() {
                    consider(prop.center(), &mut best);
                }
            }
        }
        let Some((_, point)) = best else { return };
        let to = point - origin;
        let yaw = to.x.atan2(-to.y) - self.sky.aim.x;
        let pitch = (to.z / to.truncate().length().max(0.01)).atan() - self.sky.aim.y;
        let w = &mut self.sky.walker;
        let k = (9.0 * dt).min(1.0);
        w.yaw += crate::flight::angle(yaw - w.yaw) * k;
        w.pitch += (pitch - w.pitch) * k;
    }

    // ----- context key ------------------------------------------------------------------------

    pub fn sky_context(&self) -> Context {
        if self.sky.phase == Phase::Station {
            // the pad comes first: the traveller's walk passes close by it
            if self.near_pad() {
                return Context::Board;
            }
            return self.station_listener().map_or(Context::Swap, Context::Talk);
        }
        let here = self.sky.walker.position;
        if self
            .sky
            .parked
            .is_some_and(|p| p.position.distance(here) < super::sky::BOARD_DISTANCE)
        {
            return Context::Board;
        }
        let near = self
            .sky
            .props
            .iter()
            .filter(|p| p.kind.interaction().is_some() && !self.sky.harvested.contains(&p.id))
            .map(|p| {
                (
                    p.position.truncate().distance(here.truncate()) - p.half_extent().x,
                    p,
                )
            })
            .filter(|(d, _)| *d < INTERACT_DISTANCE)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match near.map(|(_, p)| *p) {
            Some(p) if matches!(p.kind, PropKind::Pod | PropKind::Cache) => Context::Open(p),
            Some(p) if p.kind == PropKind::Beacon => Context::Signal(p),
            Some(p) => Context::Read(p),
            None => Context::Swap,
        }
    }
    pub(super) fn sky_interact(&mut self) {
        match self.sky_context() {
            Context::Board if self.sky.phase == Phase::Station => self.sky_undock(),
            Context::Board => self.sky_board_ship(),
            Context::Talk(i) => self.talk(i),
            Context::Open(prop) => self.open_prop(prop),
            Context::Read(prop) => self.read_prop(prop),
            Context::Signal(prop) => self.signal(prop),
            Context::Swap => self.swap_weapon(1),
        }
    }
    pub(super) fn swap_weapon(&mut self, direction: i32) {
        if self.sky.loadout.cycle(direction) {
            if let Some(w) = self.sky.loadout.weapon() {
                self.sky.notice(w.name(), w.rarity.color());
            }
            self.sounds
                .push(Sound::WeaponSwitch(self.sky.loadout.active as u8));
        }
        self.sky.beam = None;
    }
    fn open_prop(&mut self, prop: Prop) {
        self.sky.harvested.insert(prop.id);
        self.sky.props.retain(|p| p.id != prop.id);
        let mut rng = crate::sky::rng::Rng::new(prop.id ^ 0x0be7);
        if prop.kind == PropKind::Cache {
            let weapon = Weapon::roll(prop.id, Rarity::Rare, self.sky.alert.level);
            self.sky.credits += 50;
            self.give_weapon(weapon);
            self.sky.alert.raise(0.6);
            for (k, kind) in [DroneKind::Scout, DroneKind::Scout, DroneKind::Warden]
                .into_iter()
                .enumerate()
            {
                let a = k as f32 * 2.1 + rng.range(0.0, 1.0);
                let at = prop.position + vec3(a.cos(), a.sin(), 0.0) * 30.0 + Vec3::Z * 12.0;
                self.spawn_drone(kind, at);
            }
            self.sky.notice("GUARDIANS AWAKE", RED);
            self.mission_event(Event::CacheOpened);
            self.sounds.push(Sound::Boss);
            return;
        }
        // crashed pod: supplies and sometimes a weapon
        let item = [
            Item::JumpCell,
            Item::LiftCell,
            Item::HullPatch,
            Item::ShieldCell,
        ][rng.below(4) as usize];
        self.sky.credits += 100;
        if self.sky.inventory.add(item, 1) == 1 {
            self.sky
                .notice(format!("POD: {} +100 CR", item.name()), GOLD);
        }
        if rng.below(10) < 3 {
            self.give_weapon(Weapon::roll(prop.id ^ 0x77, Rarity::Uncommon, 0.0));
        }
        self.sky.alert.raise(0.15);
        self.sounds.push(Sound::Upgrade);
        self.mission_event(Event::PodOpened);
    }
    fn read_prop(&mut self, prop: Prop) {
        let lines = crate::sky::lore::fragment(prop.id);
        self.sky.lore = Some((prop.kind.label().to_owned(), lines));
        self.sky.overlay = Overlay::Lore;
        self.sounds.push(Sound::Scan);
        self.mission_event(Event::Read);
        if self.sky.scanned.insert(prop.id ^ 0x10e) {
            self.sky.credits += 50;
            self.sky.data += 3;
            self.sky.discoveries += 1;
            self.sky.notice("INSCRIPTION RECORDED +50 CR", GOLD);
            if prop.kind == PropKind::Ruin && (prop.id >> 9).is_multiple_of(3) {
                self.sky.inventory.add(Item::RerollCell, 1);
                self.sky
                    .notice("FOUND A REROLL CELL", Item::RerollCell.color());
            }
        }
    }
    fn signal(&mut self, prop: Prop) {
        let Some(planet) = self.sky.planet().cloned() else {
            return;
        };
        let mut found = Vec::new();
        props::nearby(
            &self.world,
            planet.seed,
            planet.biome,
            prop.position.truncate(),
            600.0,
            &self.sky.harvested,
            &mut found,
        );
        found.retain(|p| p.kind.is_poi() && p.id != prop.id);
        let n = found.len();
        self.sky.marked = found;
        self.mission_event(Event::BeaconSignalled);
        if self.sky.scanned.insert(prop.id ^ 0xbea) {
            self.sky.credits += 30;
        }
        self.sky.notice(format!("BEACON: {n} SIGNALS MARKED"), CYAN);
        self.sounds.push(Sound::Scan);
    }
    pub(super) fn give_weapon(&mut self, weapon: Weapon) {
        if self.sky.loadout.add(weapon) {
            self.sky
                .notice(format!("NEW {}", weapon.name()), weapon.rarity.color());
            self.sounds.push(Sound::Upgrade);
        } else {
            let (iron, carbon) = weapon.scrap();
            self.sky.inventory.add(Item::Iron, iron);
            self.sky.inventory.add(Item::Carbon, carbon);
            self.sky.notice("LOCKER FULL / SCRAPPED", RED);
        }
    }

    // ----- firing -----------------------------------------------------------------------------

    /// First thing along a ray: a drone, a minable or interactive prop, or the ground.
    pub fn trace(&self, origin: Vec3, direction: Vec3, range: f32) -> (Hit, f32) {
        let ground_t = terrain_hit(&self.world, origin, direction, range);
        let mut best = (
            if ground_t.is_some() {
                Hit::Ground
            } else {
                Hit::Nothing
            },
            ground_t.unwrap_or(range),
        );
        for (i, drone) in self.sky.drones.iter().enumerate() {
            let reach = hitbox(drone, origin);
            if let Some(t) = ray_sphere(origin, direction, best.1, drone.position, reach)
                && t < best.1
            {
                best = (Hit::Drone(i), t);
            }
        }
        for prop in &self.sky.props {
            let radius = prop.half_extent().max_element() * 0.8;
            if let Some(t) = ray_sphere(origin, direction, best.1, prop.center(), radius)
                && t < best.1
            {
                best = (Hit::Prop(*prop), t);
            }
        }
        // the prop the crosshair shows as aimed counts as hit, so what you see is what you mine
        if !matches!(best.0, Hit::Drone(_) | Hit::Prop(_))
            && let Some(prop) = self.aimed_prop()
        {
            let t = prop.center().distance(origin);
            if t <= range && t < best.1 + prop.half_extent().max_element() + 2.0 {
                best = (Hit::Prop(prop), t.min(best.1));
            }
        }
        best
    }
    pub(super) fn fire_weapon(&mut self, dt: f32) {
        let Some(stats) = self.sky.loadout.stats() else {
            return;
        };
        let slot = self.sky.loadout.active;
        if self.sky.loadout.overheat[slot] > 0.0 || self.sky.vitals.tool <= 0.0 {
            self.sky.mining = false;
            self.sky.beam = None;
            if (self.sky.time * 2.0).fract() < dt * 2.0 {
                let text = if self.sky.vitals.tool <= 0.0 {
                    "BATTERY EMPTY / USE CARBON"
                } else {
                    "WEAPON COOLING"
                };
                self.sky.notice(text, GOLD);
            }
            return;
        }
        self.assist_aim(&stats, dt);
        self.flight.eye = Some(self.foot_eye());
        let (origin, direction) = self.aim_ray();
        let camera = crate::flight::Camera::new(&self.flight, WIDTH, HEIGHT);
        let muzzle = origin + camera.right * 0.35 - camera.up * 0.3 + direction * 0.8;
        let beam_rate = self.sky.upgrades.beam_rate();
        if stats.fire == Fire::Beam {
            self.sky.mining = true;
            let (hit, t) = self.trace(origin, direction, stats.range);
            self.sky.beam = Some(origin + direction * t);
            match hit {
                Hit::Drone(i) => self.damage_drone(i, stats.damage * dt, &stats),
                Hit::Prop(prop) => {
                    self.mine(prop, stats.mining * beam_rate * dt, stats.yield_bonus)
                }
                _ => {}
            }
            if (self.sky.time * 8.0).fract() < dt * 8.0 {
                self.sounds.push(stats.sound);
            }
            // a working beam is noticed: the alert holds and creeps up
            self.sky.alert.raise(0.004 * dt);
            self.heat(stats.heat * dt, stats.charge * dt);
            return;
        }
        self.sky.mining = false;
        self.sky.beam = None;
        if self.sky.loadout.cooldown > 0.0 {
            return;
        }
        self.sky.loadout.cooldown = stats.interval;
        self.heat(stats.heat, stats.charge);
        self.sounds.push(stats.sound);
        self.sky.alert.raise(0.01);
        let crit = |rng: &mut crate::sky::rng::Rng| {
            if rng.unit() < stats.crit { 2.5 } else { 1.0 }
        };
        match stats.fire {
            Fire::Bolt | Fire::Lob => {
                // mortar rounds are ballistic: they land where the crosshair points
                let (_, aim_t) = self.trace(origin, direction, stats.range);
                for k in 0..stats.pellets {
                    let rng = &mut self.sky.combat_rng;
                    let spread = if stats.pellets > 1 {
                        (k as f32 - (stats.pellets - 1) as f32 * 0.5) * stats.spread
                            + rng.range(-0.3, 0.3) * stats.spread
                    } else {
                        rng.range(-1.0, 1.0) * stats.spread
                    };
                    let d = (direction
                        + camera.right * spread
                        + camera.up * rng.range(-1.0, 1.0) * stats.spread * 0.5)
                        .normalize_or_zero();
                    let lob = stats.fire == Fire::Lob;
                    const GRAVITY: f32 = 30.0;
                    let (velocity, life) = if lob {
                        let target = origin + d * aim_t;
                        let flight = (target.distance(muzzle) / stats.speed).clamp(0.25, 1.8);
                        (
                            (target - muzzle) / flight + Vec3::Z * 0.5 * GRAVITY * flight,
                            flight + 1.0,
                        )
                    } else {
                        (d * stats.speed, stats.range / stats.speed.max(1.0))
                    };
                    let shot = Projectile {
                        position: muzzle,
                        velocity,
                        life,
                        damage: stats.damage * crit(rng),
                        hostile: false,
                        color: stats.color,
                        gravity: if lob { GRAVITY } else { 0.0 },
                        splash: stats.splash,
                        bounces: stats.bounces,
                        pierce: stats.pierce,
                        homing: stats.homing,
                        slow: stats.slow,
                        siphon: stats.siphon,
                        volatile: stats.volatile,
                        mining: stats.mining * beam_rate * 0.6,
                        last_hit: 0,
                    };
                    push_capped(&mut self.sky.projectiles, shot, MAX_PROJECTILES);
                }
            }
            Fire::Lance => {
                let end =
                    terrain_hit(&self.world, origin, direction, stats.range).unwrap_or(stats.range);
                let damage = stats.damage * crit(&mut self.sky.combat_rng);
                let hits: Vec<usize> = (0..self.sky.drones.len())
                    .filter(|&i| {
                        let d = &self.sky.drones[i];
                        ray_sphere(origin, direction, end, d.position, hitbox(d, origin)).is_some()
                    })
                    .collect();
                for &i in hits.iter().rev() {
                    self.damage_drone(i, damage, &stats);
                }
                if let (Hit::Prop(prop), _) = self.trace(origin, direction, end) {
                    self.mine(prop, stats.mining * beam_rate * 1.5, stats.yield_bonus);
                }
                let effect = Effect::new(
                    muzzle,
                    EffectKind::Line(origin + direction * end),
                    stats.color,
                    1.0,
                    0.18,
                );
                push_capped(&mut self.sky.effects, effect, MAX_EFFECTS);
            }
            Fire::Chain => {
                let damage = stats.damage * crit(&mut self.sky.combat_rng);
                let mut from = muzzle;
                let mut hit: Vec<u32> = Vec::new();
                let first = self
                    .sky
                    .drones
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| {
                        let to = d.position - origin;
                        to.length() < stats.range && direction.dot(to.normalize_or_zero()) > 0.94
                    })
                    .min_by(|a, b| {
                        a.1.position
                            .distance(origin)
                            .total_cmp(&b.1.position.distance(origin))
                    })
                    .map(|(i, _)| i);
                let mut next = first;
                while let Some(i) = next {
                    let drone = self.sky.drones[i];
                    hit.push(drone.id);
                    let effect = Effect::new(
                        from,
                        EffectKind::Line(drone.position),
                        stats.color,
                        1.0,
                        0.2,
                    );
                    push_capped(&mut self.sky.effects, effect, MAX_EFFECTS);
                    from = drone.position;
                    self.damage_drone(i, damage, &stats);
                    if hit.len() > stats.chain as usize {
                        break;
                    }
                    next = self
                        .sky
                        .drones
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| !hit.contains(&d.id) && d.position.distance(from) < 14.0)
                        .min_by(|a, b| {
                            a.1.position
                                .distance(from)
                                .total_cmp(&b.1.position.distance(from))
                        })
                        .map(|(j, _)| j);
                }
                if first.is_none() {
                    let (hit, t) = self.trace(origin, direction, stats.range);
                    if let Hit::Prop(prop) = hit {
                        self.mine(prop, stats.mining * beam_rate, stats.yield_bonus);
                    }
                    let effect = Effect::new(
                        muzzle,
                        EffectKind::Line(origin + direction * t),
                        stats.color,
                        1.0,
                        0.15,
                    );
                    push_capped(&mut self.sky.effects, effect, MAX_EFFECTS);
                }
            }
            Fire::Beam => {}
        }
    }
    fn heat(&mut self, heat: f32, charge: f32) {
        let slot = self.sky.loadout.active;
        self.sky.vitals.tool = (self.sky.vitals.tool - charge).max(0.0);
        let h = &mut self.sky.loadout.heat[slot];
        *h += heat;
        if *h >= 1.0 {
            *h = 1.0;
            self.sky.loadout.overheat[slot] = 2.5;
            self.sky.notice("WEAPON OVERHEATED", RED);
            self.sounds.push(Sound::Hazard);
        }
    }
    /// Mining damage on a prop; harvests it when its toughness is reached.
    pub(super) fn mine(&mut self, prop: Prop, amount: f32, bonus: f32) {
        let Some((item, yield_amount)) = prop.harvest(
            self.sky
                .planet()
                .map_or(crate::sky::biome::Biome::Lush, |p| p.biome),
        ) else {
            return;
        };
        let damage = self.sky.prop_damage.entry(prop.id).or_insert(0.0);
        *damage += amount;
        if *damage < prop.toughness() {
            return;
        }
        self.sky.prop_damage.remove(&prop.id);
        self.sky.harvested.insert(prop.id);
        self.sky.props.retain(|p| p.id != prop.id);
        self.sky.alert.raise(if prop.kind == PropKind::Deposit {
            0.3
        } else {
            0.12
        });
        let amount = (yield_amount as f32 * (1.0 + bonus)).round() as u16;
        self.collect_item(item, amount);
        let effect = Effect::new(prop.center(), EffectKind::Burst, [255, 230, 160], 2.0, 0.35);
        push_capped(&mut self.sky.effects, effect, MAX_EFFECTS);
    }
    pub(super) fn collect_item(&mut self, item: Item, amount: u16) {
        let added = self.sky.inventory.add(item, amount);
        if added > 0 {
            self.sky
                .notice(format!("+{added} {}", item.name()), item.color());
            self.sounds.push(Sound::Pickup);
        }
        if added < amount {
            self.sky.notice("INVENTORY FULL", RED);
        }
    }

    // ----- drones -----------------------------------------------------------------------------

    pub(super) fn spawn_drone(&mut self, kind: DroneKind, at: Vec3) {
        if self.sky.drones.len() >= MAX_DRONES {
            return;
        }
        let id = self.sky.next_drone;
        self.sky.next_drone += 1;
        let floor = ground(&self.world, at.x, at.y) + kind.spec().hover;
        let drone = Drone::new(
            id,
            kind,
            at.with_z(at.z.max(floor)),
            &mut self.sky.combat_rng,
        );
        self.sky.drones.push(drone);
    }
    fn damage_drone(&mut self, i: usize, amount: f32, stats: &Stats) {
        let Some(drone) = self.sky.drones.get_mut(i) else {
            return;
        };
        drone.health -= amount;
        drone.flash = 0.12;
        if stats.slow > 0.0 {
            drone.slow = drone.slow.max(stats.slow);
        }
        self.sky.vitals.suit = (self.sky.vitals.suit + stats.siphon).min(1.0);
        self.sky.hit_marker = 0.15;
        self.sky.alert.raise(0.02);
        if self.sky.drones[i].health <= 0.0 {
            self.destroy_drone(i, stats.volatile, stats.loot);
        }
    }
    fn destroy_drone(&mut self, i: usize, volatile: bool, loot: f32) {
        let drone = self.sky.drones.swap_remove(i);
        let spec = drone.kind.spec();
        push_capped(
            &mut self.sky.effects,
            Effect::new(
                drone.position,
                EffectKind::Burst,
                [255, 170, 80],
                spec.radius * 2.5,
                0.5,
            ),
            MAX_EFFECTS,
        );
        self.sounds
            .push(Sound::Explosion(drone.kind != DroneKind::Scout));
        self.sky.drones_destroyed += 1;
        self.mission_event(Event::DroneDown);
        let rng = &mut self.sky.combat_rng;
        let credits = (spec.credits as f32 * loot) as u32;
        let iron = (rng.range(4.0, 10.0) * loot) as u16;
        let carbon = (rng.range(2.0, 7.0) * loot) as u16;
        let core = rng.unit() < spec.core * loot;
        let weapon = rng.unit() < (spec.weapon + self.sky.alert.level * 0.08) * loot;
        let seed = rng.next_u32() as u64;
        self.sky.credits += credits;
        self.sky
            .notice(format!("{} DOWN +{credits} CR", spec.name), GOLD);
        self.sky.inventory.add(Item::Iron, iron);
        self.sky.inventory.add(Item::Carbon, carbon);
        if core && self.sky.inventory.add(Item::DroneCore, 1) == 1 {
            self.sky.notice("+1 DRONE CORE", Item::DroneCore.color());
        }
        if weapon {
            let w = Weapon::roll(seed, Rarity::Common, self.sky.alert.level);
            self.give_weapon(w);
        }
        if volatile {
            // the wreck bursts: no further chain reactions
            let blast: Vec<usize> = (0..self.sky.drones.len())
                .filter(|&j| self.sky.drones[j].position.distance(drone.position) < 7.0)
                .collect();
            for &j in blast.iter().rev() {
                self.sky.drones[j].health -= 25.0;
                self.sky.drones[j].flash = 0.12;
                if self.sky.drones[j].health <= 0.0 {
                    self.destroy_drone(j, false, loot);
                }
            }
        }
    }
    /// Drones, projectiles, alert and patrols for one on-foot frame.
    pub(super) fn combat_step(&mut self, dt: f32) {
        let world = self.world.clone();
        let chest = self.sky.walker.position + Vec3::Z * 1.4;
        let here = self.sky.walker.position;
        self.sky.loadout.cool(dt);
        self.sky.alert.decay(dt);
        // patrols arrive from beyond the horizon
        if let Some(kind) = self
            .sky
            .alert
            .patrol_kind(self.sky.drones.len(), &mut self.sky.combat_rng)
        {
            let a = self.sky.combat_rng.range(0.0, std::f32::consts::TAU);
            let at = here + vec3(a.cos(), a.sin(), 0.0) * 80.0 + Vec3::Z * 20.0;
            self.spawn_drone(kind, at);
            if self.sky.drones.len() == 1 {
                self.sky.notice("GUARD DRONES INBOUND", RED);
                self.sounds.push(Sound::Wave);
            }
        }
        // drones
        let mut launches = Vec::new();
        for i in 0..self.sky.drones.len() {
            let action = self.sky.drones[i].step(&world, chest, dt, &mut self.sky.combat_rng);
            if let Some(shot) = action.shot {
                push_capped(&mut self.sky.projectiles, shot, MAX_PROJECTILES);
                self.sounds.push(Sound::EnemyFire(
                    self.sky.drones[i].kind != DroneKind::Scout,
                ));
            }
            if let Some(at) = action.launch {
                launches.push(at);
            }
        }
        for at in launches {
            self.spawn_drone(DroneKind::Scout, at);
        }
        self.sky
            .drones
            .retain(|d| d.position.distance(here) < LEASH);
        // projectiles
        let mut i = 0;
        while i < self.sky.projectiles.len() {
            let mut p = self.sky.projectiles[i];
            p.life -= dt;
            p.velocity.z -= p.gravity * dt;
            if p.homing > 0.0
                && let Some(target) = self
                    .sky
                    .drones
                    .iter()
                    .filter(|d| d.position.distance(p.position) < 40.0)
                    .min_by(|a, b| {
                        a.position
                            .distance(p.position)
                            .total_cmp(&b.position.distance(p.position))
                    })
            {
                let speed = p.velocity.length();
                let want = (target.position - p.position).normalize_or_zero() * speed;
                p.velocity = (p.velocity + (want - p.velocity) * (p.homing * dt).min(1.0))
                    .normalize_or_zero()
                    * speed;
            }
            let from = p.position;
            p.position += p.velocity * dt;
            let travel = p.position - from;
            let length = travel.length().max(0.0001);
            let direction = travel / length;
            let mut gone = p.life <= 0.0;
            if p.hostile {
                if ray_sphere(from, direction, length, chest, PLAYER_RADIUS).is_some() {
                    let capacity = self.sky.upgrades.suit();
                    if self.sky.vitals.damage(p.damage, capacity) {
                        self.sky.notice("SUIT SHIELD DOWN", RED);
                        self.sounds.push(Sound::Destroyed);
                    } else {
                        self.sounds.push(Sound::Damage);
                    }
                    self.sky.hurt.push((from - travel * 20.0, 0.0));
                    gone = true;
                }
            } else {
                let hit = self.sky.drones.iter().position(|d| {
                    d.id != p.last_hit
                        && ray_sphere(from, direction, length, d.position, d.radius() + 0.8)
                            .is_some()
                });
                if let Some(h) = hit {
                    let stats = self.projectile_stats(&p);
                    p.last_hit = self.sky.drones[h].id;
                    self.damage_drone(h, p.damage, &stats);
                    if !p.pierce {
                        gone = true;
                    }
                    if p.splash > 0.0 {
                        self.splash(p.position, p.splash, p.damage * 0.6, &p);
                    }
                } else if p.mining > 0.0
                    && let Some(prop) = self
                        .sky
                        .props
                        .iter()
                        .find(|prop| {
                            prop.center().distance(p.position) < prop.half_extent().max_element()
                        })
                        .copied()
                {
                    self.mine(prop, p.mining * 0.5, 0.0);
                    gone = true;
                }
            }
            if !gone && p.position.z <= ground(&world, p.position.x, p.position.y) {
                if p.bounces > 0 {
                    p.bounces -= 1;
                    p.velocity.z = p.velocity.z.abs() * 0.6;
                    p.position.z = ground(&world, p.position.x, p.position.y) + 0.2;
                } else {
                    gone = true;
                    if p.splash > 0.0 && !p.hostile {
                        self.splash(p.position, p.splash, p.damage * 0.6, &p);
                    } else if p.splash > 0.0 && p.position.distance(chest) < p.splash {
                        self.sky
                            .vitals
                            .damage(p.damage * 0.5, self.sky.upgrades.suit());
                        self.sky.hurt.push((p.position, 0.0));
                    }
                }
            }
            if gone {
                let kind = if p.splash > 0.0 {
                    EffectKind::Burst
                } else {
                    EffectKind::Spark
                };
                let effect = Effect::new(p.position, kind, p.color, p.splash.max(0.8), 0.3);
                push_capped(&mut self.sky.effects, effect, MAX_EFFECTS);
                self.sky.projectiles.remove(i);
            } else {
                self.sky.projectiles[i] = p;
                i += 1;
            }
        }
        if self.sky.vitals.life <= 0.0 {
            self.sky.drones.clear();
            self.sky.projectiles.clear();
        }
    }
    fn projectile_stats(&self, p: &Projectile) -> Stats {
        let mut stats = Weapon::starter().stats();
        stats.slow = p.slow;
        stats.siphon = p.siphon;
        stats.volatile = p.volatile;
        stats.loot = self.sky.loadout.stats().map_or(1.0, |s| s.loot);
        stats
    }
    fn splash(&mut self, at: Vec3, radius: f32, damage: f32, p: &Projectile) {
        let stats = self.projectile_stats(p);
        let targets: Vec<usize> = (0..self.sky.drones.len())
            .filter(|&j| {
                self.sky.drones[j].position.distance(at) < radius
                    && self.sky.drones[j].id != p.last_hit
            })
            .collect();
        for &j in targets.iter().rev() {
            let falloff = 1.0 - self.sky.drones[j].position.distance(at) / radius;
            self.damage_drone(j, damage * falloff.max(0.3), &stats);
        }
    }

    // ----- panels -----------------------------------------------------------------------------

    /// Keys in the weapons view of the inventory.
    pub(super) fn arms_key(&mut self, code: u16) -> bool {
        let all = self.sky.loadout.all();
        let n = all.len().max(1);
        match code {
            UP | LEFT => self.sky.arms_select = (self.sky.arms_select + n - 1) % n,
            DOWN | RIGHT => self.sky.arms_select = (self.sky.arms_select + 1) % n,
            KEY4 | ENTER => self.equip_selected(),
            KEY5 => self.scrap_selected(),
            KEY6 => self.sky.arms = false,
            KEY7 => self.open_overlay(Overlay::Galaxy),
            KEY8 | TAB => self.sky.overlay = Overlay::None,
            _ => return !matches!(code, PAUSE),
        }
        if matches!(code, UP | DOWN | LEFT | RIGHT) {
            self.sounds.push(Sound::Navigate);
        }
        true
    }
    fn equip_selected(&mut self) {
        let select = self.sky.arms_select;
        let l = &mut self.sky.loadout;
        if select < 3 {
            if l.equipped[select].is_some() {
                l.active = select;
            }
        } else if let Some(&weapon) = l.locker.get(select - 3) {
            // swap the locker weapon with the active slot
            let active = l.active;
            match l.equipped[active] {
                Some(old) => l.locker[select - 3] = old,
                None => {
                    l.locker.remove(select - 3);
                }
            }
            l.set(active, Some(weapon));
            self.sky
                .notice(format!("EQUIPPED {}", weapon.name()), weapon.rarity.color());
        }
        self.sounds
            .push(Sound::WeaponSwitch(self.sky.loadout.active as u8));
    }
    fn scrap_selected(&mut self) {
        let select = self.sky.arms_select;
        if self.sky.loadout.count() <= 1 {
            self.sky.notice("KEEP AT LEAST ONE WEAPON", RED);
            return;
        }
        let weapon = if select < 3 {
            let w = self.sky.loadout.equipped[select];
            if w.is_some() {
                self.sky.loadout.set(select, None);
                if select == self.sky.loadout.active {
                    self.sky.loadout.cycle(1);
                }
            }
            w
        } else if select - 3 < self.sky.loadout.locker.len() {
            Some(self.sky.loadout.locker.remove(select - 3))
        } else {
            None
        };
        if let Some(w) = weapon {
            let (iron, carbon) = w.scrap();
            self.sky.inventory.add(Item::Iron, iron);
            self.sky.inventory.add(Item::Carbon, carbon);
            self.sky
                .notice(format!("SCRAPPED +{iron} FE +{carbon} C"), GOLD);
            self.sounds.push(Sound::Craft);
            let n = self.sky.loadout.all().len();
            self.sky.arms_select = self.sky.arms_select.min(n.saturating_sub(1));
        }
    }
    /// Reroll the active weapon's perks with a reroll cell.
    pub(super) fn reroll_weapon(&mut self) -> bool {
        let Some(weapon) = self.sky.loadout.weapon() else {
            return false;
        };
        if weapon.rarity == Rarity::Common {
            self.sky.notice("COMMON WEAPONS HAVE NO PERKS", WHITE);
            return false;
        }
        let seed = (self.sky.time * 1000.0) as u64 ^ self.sky.combat_rng.next_u32() as u64;
        let new = weapon.rerolled(seed);
        let active = self.sky.loadout.active;
        self.sky.loadout.set(active, Some(new));
        self.sky
            .notice(format!("REROLLED: {}", new.name()), new.rarity.color());
        self.sounds.push(Sound::Upgrade);
        true
    }

    // ----- station ----------------------------------------------------------------------------

    /// On docking: new armory stock.
    pub(super) fn stock_station(&mut self) {
        self.sky.visits += 1;
        let seed = self.sky.system.seed ^ (self.sky.visits as u64).wrapping_mul(0x9e37_79b9);
        self.sky.armory = std::array::from_fn(|k| {
            let floor = if k == 2 {
                Rarity::Rare
            } else {
                Rarity::Uncommon
            };
            Some(Weapon::roll(seed ^ k as u64, floor, 0.0))
        });
    }
    pub(super) fn buy_armory(&mut self, k: usize) {
        let Some(weapon) = self.sky.armory[k] else {
            self.sky.notice("SOLD OUT", WHITE);
            return;
        };
        let price = (weapon.value() as f32 * self.sky.system.market) as u32;
        if self.sky.credits < price {
            self.sky.notice("NOT ENOUGH CREDITS", RED);
            self.sounds.push(Sound::Hazard);
            return;
        }
        if self.sky.loadout.count() >= 3 + LOCKER {
            self.sky.notice("LOCKER FULL", RED);
            return;
        }
        self.sky.credits -= price;
        self.sky.armory[k] = None;
        self.give_weapon(weapon);
    }
    // ----- scene ------------------------------------------------------------------------------

    /// Drones, projectiles and bursts for the renderer.
    pub(super) fn combat_scene(&mut self) {
        let tint = self.sky.system.class.color();
        let extras = &mut self.renderer.extras;
        for drone in &self.sky.drones {
            drone.mesh(tint, &mut extras.triangles);
        }
        let shard =
            |out: &mut Vec<crate::render::ExtraTriangle>, at: Vec3, size: f32, color: u32| {
                props::spindle(
                    out,
                    at - Vec3::Z * size,
                    at,
                    at + Vec3::Z * size,
                    size,
                    3,
                    0.0,
                    (color, color),
                    true,
                );
            };
        for p in &self.sky.projectiles {
            let [r, g, b] = p.color;
            shard(
                &mut extras.triangles,
                p.position,
                if p.splash > 0.0 { 0.5 } else { 0.25 },
                crate::render::rgb(r, g, b),
            );
        }
        for e in &self.sky.effects {
            if matches!(e.kind, EffectKind::Burst) {
                let [r, g, b] = e.color;
                let size = e.radius * (0.3 + 0.7 * e.age());
                let color = crate::render::mix(
                    crate::render::rgb(r, g, b),
                    crate::render::rgb(255, 255, 230),
                    1.0 - e.age(),
                );
                shard(&mut extras.triangles, e.position, size, color);
            }
        }
    }
    /// Where things are, for the HUD: direction of the ship etc. relative to the walker.
    pub fn sky_heading_to(&self, point: Vec3) -> f32 {
        let d = point - self.sky.walker.position;
        crate::flight::angle(d.x.atan2(-d.y) - self.sky.walker.yaw)
    }
    /// Walker beside the parked ship (after a collapse).
    pub(super) fn wake_at_ship(&mut self) {
        if let Some(parked) = self.sky.parked {
            let world = self.world.clone();
            let (_, right) = heading(parked.yaw);
            let spot = parked.position.truncate() + right.truncate() * 7.0;
            self.sky.walker = Walker::new(&world, spot.x, spot.y, parked.yaw);
        }
        self.sky.drones.clear();
        self.sky.projectiles.clear();
        self.sky.alert = Default::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::Sky;
    use crate::sky::arsenal::Archetype;
    use crate::world::World;

    fn on_foot() -> App {
        let key = Sky::default().system.planets[0].key();
        let mut app = App::new(World::generate(key.kind, key.seed));
        app.mode = Mode::Explore;
        app.start();
        let world = app.world.clone();
        let (x, y) = (2000.0, 2000.0);
        app.sky.walker = Walker::new(&world, x, y, 0.0);
        app.sky.phase = Phase::OnFoot;
        app.flight.eye = Some(app.foot_eye());
        app.sky.props.clear();
        app
    }
    /// Put a drone straight ahead of the crosshair at `distance`.
    fn drone_ahead(app: &mut App, kind: DroneKind, distance: f32) -> u32 {
        let (origin, direction) = app.aim_ray();
        let at = origin + direction * distance;
        let id = app.sky.next_drone;
        app.sky.next_drone += 1;
        let mut drone = Drone::new(id, kind, at, &mut app.sky.combat_rng);
        drone.cooldown = 99.0;
        app.sky.drones.push(drone);
        id
    }

    #[test]
    fn every_archetype_can_destroy_a_drone() {
        for archetype in Archetype::ALL {
            let mut app = on_foot();
            app.aim_assist = false;
            app.sky.walker.pitch = 0.0;
            app.flight.eye = Some(app.foot_eye());
            let weapon = Weapon {
                archetype,
                seed: 11,
                rarity: Rarity::Common,
            };
            app.sky.loadout.set(0, Some(weapon));
            app.sky.loadout.active = 0;
            let distance = if archetype == Archetype::Mortar {
                18.0
            } else {
                12.0
            };
            drone_ahead(&mut app, DroneKind::Scout, distance);
            let mut frames = 0;
            while !app.sky.drones.is_empty() && frames < 30 * 20 {
                app.sky.vitals.tool = 1.0;
                app.sky.loadout.heat = [0.0; 3];
                // no patrols: this test is about one drone
                app.sky.alert = Default::default();
                app.sky.alert.patrol = f32::INFINITY;
                // keep the drone in front of the barrel
                if let Some(d) = app.sky.drones.first_mut() {
                    let (origin, direction) = {
                        let camera = crate::flight::Camera::new(&app.flight, WIDTH, HEIGHT);
                        let (o, d) = camera.ray(super::super::sky::CROSSHAIR);
                        (o, d.normalize())
                    };
                    let target = origin + direction * distance;
                    d.position = target;
                    d.cooldown = 99.0;
                }
                app.fire_weapon(1.0 / 30.0);
                app.combat_step(1.0 / 30.0);
                frames += 1;
            }
            assert!(
                app.sky.drones.is_empty(),
                "{archetype:?} never destroyed the drone"
            );
            assert_eq!(app.sky.drones_destroyed, 1);
        }
    }
    #[test]
    fn drone_fire_hits_the_suit_shield_first() {
        let mut app = on_foot();
        let chest = app.sky.walker.position + Vec3::Z * 1.4;
        let shot = Projectile {
            position: chest + Vec3::X * 5.0,
            velocity: -Vec3::X * 40.0,
            life: 2.0,
            damage: 20.0,
            hostile: true,
            color: [255, 0, 0],
            gravity: 0.0,
            splash: 0.0,
            bounces: 0,
            pierce: false,
            homing: 0.0,
            slow: 0.0,
            siphon: 0.0,
            volatile: false,
            mining: 0.0,
            last_hit: 0,
        };
        app.sky.projectiles.push(shot);
        for _ in 0..10 {
            app.combat_step(1.0 / 30.0);
        }
        assert!(app.sky.projectiles.is_empty());
        assert!(app.sky.vitals.suit < 1.0);
        assert_eq!(app.sky.vitals.life, 1.0);
        assert!(!app.sky.hurt.is_empty());
    }
    #[test]
    fn mining_alerts_the_drones_and_patrols_arrive() {
        let mut app = on_foot();
        app.sky.alert.raise(0.5);
        for _ in 0..30 {
            app.combat_step(1.0 / 30.0);
        }
        assert!(!app.sky.drones.is_empty());
        let d = app.sky.drones[0].position.distance(app.sky.walker.position);
        assert!(d > 50.0, "patrols come from a distance ({d})");
    }
    #[test]
    fn context_key_opens_caches_reads_ruins_and_swaps_otherwise() {
        let mut app = on_foot();
        assert_eq!(app.sky_context(), Context::Swap);
        let before = app.sky.loadout.active;
        app.sky_interact();
        assert_ne!(app.sky.loadout.active, before);
        let here = app.sky.walker.position;
        let cache = Prop {
            id: 4242,
            kind: PropKind::Cache,
            position: here + vec3(3.0, 0.0, 0.0),
            size: 1.0,
            yaw: 0.0,
        };
        app.sky.props.push(cache);
        assert_eq!(app.sky_context(), Context::Open(cache));
        let weapons = app.sky.loadout.count();
        app.sky_interact();
        assert_eq!(app.sky.loadout.count(), weapons + 1);
        let new = app.sky.loadout.all().into_iter().flatten().last().unwrap();
        assert!(new.rarity >= Rarity::Rare);
        assert!(!app.sky.drones.is_empty(), "guardians wake");
        assert!(app.sky.harvested.contains(&4242));
        let ruin = Prop {
            id: 99,
            kind: PropKind::Ruin,
            position: here + vec3(-4.0, 0.0, 0.0),
            size: 1.0,
            yaw: 0.0,
        };
        app.sky.props.push(ruin);
        assert_eq!(app.sky_context(), Context::Read(ruin));
        app.sky_interact();
        assert_eq!(app.sky.overlay, Overlay::Lore);
        assert!(app.sky.lore.is_some());
    }
    #[test]
    fn tilt_fine_aims_within_limits_and_recentres() {
        let mut app = on_foot();
        app.sky_tilt(0.3, -0.1, true);
        assert_eq!(app.sky.aim, Vec2::ZERO, "the first reading sets the centre");
        app.sky_tilt(0.4, 0.0, true);
        assert!((app.sky.aim.x - 0.07).abs() < 0.001 && (app.sky.aim.y - 0.07).abs() < 0.001);
        app.sky_tilt(2.0, 2.0, true);
        assert_eq!(app.sky.aim, vec2(AIM_YAW, AIM_PITCH));
        let eye = app.foot_eye();
        assert!((eye.yaw - app.sky.walker.yaw - AIM_YAW).abs() < 0.001);
        app.sky_recenter();
        assert_eq!(app.sky.aim, Vec2::ZERO);
        app.tilt_aim = false;
        app.sky_tilt(0.9, 0.9, true);
        assert_eq!(app.sky.aim, Vec2::ZERO);
    }
}
