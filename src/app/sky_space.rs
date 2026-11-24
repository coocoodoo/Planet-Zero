//! Cardputer Sky in space and aboard stations: pirate raids and dogfights, ship cannons,
//! asteroid kinds, the walkable station hangar with its people, conversations, and missions.
use super::*;
use crate::audio::Sound;
use crate::keys::*;
use crate::sky::arsenal::{Rarity, Weapon};
use crate::sky::combat::{
    Effect, EffectKind, MAX_EFFECTS, MAX_PROJECTILES, Projectile, push_capped, ray_sphere,
};
use crate::sky::interior::{self, Role};
use crate::sky::inventory::Item;
use crate::sky::missions::{self, Event, Kind, MAX_ACTIVE, Mission, Prize};
use crate::sky::pirates::{self, MAX_PIRATES, Pirate, SHIP_RADIUS};
use crate::sky::walker::{WalkInput, Walker};
use crate::sky::{Location, Overlay, Phase};
use glam::{Vec3, vec2, vec3};

const CYAN: [u8; 3] = [111, 237, 218];
const GOLD: [u8; 3] = [255, 208, 105];
const RED: [u8; 3] = [255, 112, 130];
const WHITE: [u8; 3] = [225, 238, 244];

/// Asteroid kinds: common rock, rich (glowing veins), rare (golden).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rock {
    Common,
    Rich,
    Rare,
}
pub fn rock_kind(system_seed: u64, i: u32) -> Rock {
    match crate::sky::rng::hash(&[system_seed, i as u64, 0x40c]) % 100 {
        0..=4 => Rock::Rare,
        5..=19 => Rock::Rich,
        _ => Rock::Common,
    }
}
/// Seconds before mined asteroids grow back
const REGROW: f32 = 300.0;
const CANNON_SPEED: f32 = 900.0;

impl App {
    // ----- missions -------------------------------------------------------------------------

    /// Tell every active mission what happened; announce the ones that just finished.
    pub(super) fn mission_event(&mut self, event: Event) {
        let here = self.sky.system.id;
        let planet = match self.sky.location {
            Location::Planet(i) => Some(i),
            _ => None,
        };
        let inventory = self.sky.inventory;
        let mut finished = Vec::new();
        for m in &mut self.sky.missions {
            let before = m.done(&inventory);
            m.observe(event, here, planet);
            if !before && m.done(&inventory) {
                finished.push(m.kind.title());
            }
        }
        for title in finished {
            self.sky
                .notice(format!("{title} DONE / CLAIM AT A STATION"), GOLD);
            self.sounds.push(Sound::Upgrade);
        }
    }
    pub(super) fn refresh_board(&mut self) {
        let rank = missions::rank(self.sky.standing);
        self.sky.board = Mission::board(&self.sky.system, &self.sky.galaxy, self.sky.visits, rank);
        // drop offers you already hold
        let held: Vec<u32> = self.sky.missions.iter().map(|m| m.seed).collect();
        self.sky.board.retain(|m| !held.contains(&m.seed));
    }
    pub(super) fn missions_key(&mut self, code: u16) -> bool {
        let list_len = if self.sky.mission_tab == 0 {
            self.sky.board.len()
        } else {
            self.sky.missions.len()
        }
        .max(1);
        match code {
            UP => self.sky.mission_select = (self.sky.mission_select + list_len - 1) % list_len,
            DOWN => self.sky.mission_select = (self.sky.mission_select + 1) % list_len,
            KEY4 | ENTER => self.mission_primary(),
            KEY5 => self.abandon_mission(),
            KEY6 | LEFT | RIGHT => {
                self.sky.mission_tab ^= 1;
                self.sky.mission_select = 0;
            }
            KEY8 | TAB => self.sky.overlay = Overlay::None,
            _ => return !matches!(code, PAUSE),
        }
        if matches!(code, UP | DOWN | KEY6 | LEFT | RIGHT) {
            self.sounds.push(Sound::Navigate);
        }
        true
    }
    /// Accept an offer, or claim a finished mission.
    fn mission_primary(&mut self) {
        let i = self.sky.mission_select;
        if self.sky.mission_tab == 0 {
            let Some(&m) = self.sky.board.get(i) else {
                return;
            };
            if self.sky.missions.len() >= MAX_ACTIVE {
                self.sky.notice("THREE JOBS AT A TIME", RED);
                self.sounds.push(Sound::Hazard);
                return;
            }
            self.sky.board.remove(i);
            self.sky.mission_select = self
                .sky
                .mission_select
                .min(self.sky.board.len().saturating_sub(1));
            if matches!(m.kind, Kind::Bounty | Kind::Ace) {
                // the targets turn up soon after you leave the station
                self.sky.bounty = Some((m.target, m.kind == Kind::Ace, 20.0));
            }
            self.sky.missions.push(m);
            self.sky
                .notice(format!("ACCEPTED {}", m.kind.title()), CYAN);
            self.sounds.push(Sound::Confirm);
            return;
        }
        let Some(&m) = self.sky.missions.get(i) else {
            return;
        };
        if !m.done(&self.sky.inventory) {
            self.sky
                .notice(missions::tracker(&m, &self.sky.inventory), WHITE);
            return;
        }
        if matches!(m.kind, Kind::Delivery | Kind::Salvage) {
            self.sky.inventory.take(m.item, m.count);
        }
        self.sky.missions.remove(i);
        self.sky.mission_select = self
            .sky
            .mission_select
            .min(self.sky.missions.len().saturating_sub(1));
        self.sky.credits += m.reward;
        self.sky.missions_done += 1;
        let before = missions::rank(self.sky.standing);
        self.sky.standing += 1 + m.tier as u32;
        self.sky
            .notice(format!("{} PAID +{} CR", m.kind.title(), m.reward), GOLD);
        match m.prize {
            Prize::Item(item, n) => {
                self.sky.inventory.add(item, n);
                self.sky
                    .notice(format!("BONUS {}", item.name()), item.color());
            }
            Prize::Weapon => {
                let rarity = if m.tier >= 3 {
                    Rarity::Rare
                } else {
                    Rarity::Uncommon
                };
                self.give_weapon(Weapon::roll(
                    m.seed as u64 ^ 0xb0,
                    rarity,
                    0.3 + m.tier as f32 * 0.15,
                ));
            }
            Prize::None => {}
        }
        let after = missions::rank(self.sky.standing);
        if after > before {
            self.sky
                .notice(format!("GUILD RANK: {}", missions::RANKS[after].0), GOLD);
        }
        self.sounds.push(Sound::Upgrade);
        self.sky_save();
    }
    fn abandon_mission(&mut self) {
        if self.sky.mission_tab != 1 || self.sky.missions.is_empty() {
            return;
        }
        let m = self
            .sky
            .missions
            .remove(self.sky.mission_select.min(self.sky.missions.len() - 1));
        self.sky.mission_select = 0;
        self.sky
            .notice(format!("ABANDONED {}", m.kind.title()), WHITE);
    }

    // ----- pirates and ship weapons ---------------------------------------------------------

    /// Random raids, bounty waves, dogfights and ship cannons. Called every frame in space.
    pub(super) fn pirate_step(&mut self, dt: f32) {
        let ship = self.flight.position;
        let forward = self.flight.axes()[1];
        let flying = self.sky.phase == Phase::Flying && self.orbit.transition.is_none();
        let near_station = ship.distance(self.sky.system.station) < 1500.0;
        if flying && self.sky.pirates.is_empty() && !near_station {
            if self.sky.raid <= 0.0 {
                self.sky.raid = pirates::next_raid(&mut self.sky.combat_rng);
                let n = 2 + self.sky.combat_rng.below(2) as usize;
                self.spawn_pirates(n, false);
                self.sky.notice("PIRATE RAID INBOUND", RED);
            } else if let Some((target, ace, wait)) = self.sky.bounty
                && target == self.sky.system.id
            {
                let wait = wait - dt;
                if wait <= 0.0 {
                    let left = self
                        .sky
                        .missions
                        .iter()
                        .filter(|m| m.kind == Kind::Bounty && m.target == target)
                        .map(|m| (m.count - m.progress) as usize)
                        .max()
                        .unwrap_or(0)
                        .min(3);
                    self.spawn_pirates(left.max(usize::from(!ace)), ace);
                    self.sky.bounty = None;
                    self.sky.notice(
                        if ace {
                            "THE ACE IS HERE"
                        } else {
                            "BOUNTY TARGETS SIGHTED"
                        },
                        RED,
                    );
                } else {
                    self.sky.bounty = Some((target, ace, wait));
                }
            }
        }
        // pirates fly and shoot
        let velocity = forward * self.flight.speed;
        for i in 0..self.sky.pirates.len() {
            if let Some(shot) =
                self.sky.pirates[i].step(ship, velocity, dt, &mut self.sky.combat_rng)
            {
                push_capped(&mut self.sky.projectiles, shot, MAX_PROJECTILES);
                self.sounds.push(Sound::EnemyFire(self.sky.pirates[i].ace));
            }
        }
        self.sky
            .pirates
            .retain(|p| p.position.distance(ship) < 6000.0);
        // bolts
        let mut i = 0;
        while i < self.sky.projectiles.len() {
            let mut p = self.sky.projectiles[i];
            p.life -= dt;
            let from = p.position;
            p.position += p.velocity * dt;
            let travel = p.position - from;
            let length = travel.length().max(0.0001);
            let direction = travel / length;
            let mut gone = p.life <= 0.0;
            if p.hostile {
                if ray_sphere(from, direction, length, ship, SHIP_RADIUS).is_some() {
                    self.ship_hit(p.damage, from - travel * 30.0);
                    gone = true;
                }
            } else if let Some(h) = self
                .sky
                .pirates
                .iter()
                .position(|t| ray_sphere(from, direction, length, t.position, 11.0).is_some())
            {
                self.damage_pirate(h, p.damage);
                gone = true;
            }
            if i >= self.sky.projectiles.len() {
                // the hit destroyed the ship, which clears the fight
                break;
            }
            if gone {
                push_capped(
                    &mut self.sky.effects,
                    Effect::new(p.position, EffectKind::Spark, p.color, 3.0, 0.25),
                    MAX_EFFECTS,
                );
                self.sky.projectiles.remove(i);
            } else {
                self.sky.projectiles[i] = p;
                i += 1;
            }
        }
    }
    pub(super) fn spawn_pirates(&mut self, n: usize, ace: bool) {
        let room = MAX_PIRATES.saturating_sub(self.sky.pirates.len());
        let n = n.min(room.saturating_sub(usize::from(ace)));
        let id = self.sky.next_drone;
        let wave = Pirate::wave(
            id,
            self.flight.position,
            self.flight.axes()[1],
            n,
            ace,
            &mut self.sky.combat_rng,
        );
        self.sky.next_drone += wave.len() as u32;
        self.sky.pirates.extend(wave);
        self.sounds.push(Sound::Boss);
    }
    /// Ship cannons: fast bolts along the nose, nudged toward a pirate near the crosshair.
    pub(super) fn fire_cannon(&mut self, dt: f32) {
        let v = &mut self.sky.vitals;
        if v.overheat > 0.0 {
            v.cool(dt);
            return;
        }
        if self.sky.cannon > 0.0 {
            return;
        }
        self.sky.cannon = 0.12;
        let [right, forward, up] = self.flight.axes();
        let muzzle = self.flight.position + forward * 12.0;
        let mut aim = forward;
        if let Some(target) = self
            .sky
            .pirates
            .iter()
            .filter(|p| p.position.distance(muzzle) < 1800.0)
            .map(|p| {
                let lead =
                    p.position + p.forward * p.speed * (p.position.distance(muzzle) / CANNON_SPEED);
                (lead, (lead - muzzle).normalize_or_zero())
            })
            // a wide assist cone: lining up a fast raider by tilting a small device is hard
            .filter(|(_, d)| d.dot(forward) > 0.94)
            .max_by(|a, b| a.1.dot(forward).total_cmp(&b.1.dot(forward)))
        {
            aim = (target.0 - muzzle).normalize_or_zero();
        }
        let side = if (self.sky.time * 8.0) as i32 % 2 == 0 {
            3.0
        } else {
            -3.0
        };
        let shot = Projectile {
            position: muzzle + right * side - up * 1.0,
            velocity: aim * CANNON_SPEED + forward * self.flight.speed,
            life: 2.2,
            damage: 10.0,
            hostile: false,
            color: [120, 230, 255],
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
        push_capped(&mut self.sky.projectiles, shot, MAX_PROJECTILES);
        self.sounds.push(Sound::Gun);
        let v = &mut self.sky.vitals;
        v.heat += 0.035;
        if v.heat >= 1.0 {
            v.heat = 1.0;
            v.overheat = 2.0;
            self.sky.notice("CANNONS OVERHEATED", RED);
        }
    }
    fn damage_pirate(&mut self, i: usize, amount: f32) {
        let pirate = &mut self.sky.pirates[i];
        pirate.health -= amount;
        pirate.flash = 0.1;
        self.sky.hit_marker = 0.15;
        if pirate.health > 0.0 {
            return;
        }
        let pirate = self.sky.pirates.swap_remove(i);
        push_capped(
            &mut self.sky.effects,
            Effect::new(
                pirate.position,
                EffectKind::Burst,
                [255, 160, 70],
                30.0,
                0.8,
            ),
            MAX_EFFECTS,
        );
        self.sounds.push(Sound::Explosion(true));
        let credits = if pirate.ace {
            900
        } else {
            160 + self.sky.combat_rng.below(120)
        };
        self.sky.credits += credits;
        let tritium = 10 + self.sky.combat_rng.below(25) as u16;
        self.sky.inventory.add(Item::Tritium, tritium);
        let name = if pirate.ace {
            pirate.name.clone()
        } else {
            "RAIDER".to_owned()
        };
        self.sky.notice(format!("{name} DOWN +{credits} CR"), GOLD);
        if pirate.ace || self.sky.combat_rng.below(6) == 0 {
            let rarity = if pirate.ace {
                Rarity::Rare
            } else {
                Rarity::Uncommon
            };
            let seed = self.sky.combat_rng.next_u32() as u64;
            self.give_weapon(Weapon::roll(seed, rarity, 0.4));
        }
        self.sky.pirates_down += 1;
        self.mission_event(Event::PirateDown { ace: pirate.ace });
        if self.sky.pirates.is_empty() {
            self.sky.notice("SKIES CLEAR", CYAN);
        }
    }
    fn ship_hit(&mut self, damage: f32, from: Vec3) {
        self.sky.vitals.shield = (self.sky.vitals.shield - damage / 120.0).max(0.0);
        self.sky.hurt.push((from, 0.0));
        self.sounds.push(Sound::Damage);
        if self.sky.vitals.shield <= 0.0 {
            self.ship_destroyed();
        }
    }
    /// Shield gone: the ship limps back to the station, a third of the cargo lost.
    fn ship_destroyed(&mut self) {
        for item in Item::ELEMENTS {
            let lost = self.sky.inventory.count(item) / 3;
            self.sky.inventory.take(item, lost);
        }
        self.sky.pirates.clear();
        self.sky.projectiles.clear();
        self.sky.vitals.shield = 0.4;
        let safe = self.sky.system.station + vec3(420.0, 0.0, 0.0);
        self.flight.translate(safe - self.flight.position);
        self.flight.speed = 40.0;
        self.flight.warp_flash = 1.0;
        self.sky.autodock = false;
        self.sky.notice("SHIP CRIPPLED / CARGO LOST", RED);
        self.sky.notice("TOWED TO THE STATION", WHITE);
        self.sounds.push(Sound::Destroyed);
    }
    /// What key 5 does in space: mine when an asteroid is under the crosshair, otherwise fire.
    pub fn space_aim_rock(&self) -> bool {
        let camera = crate::flight::Camera::new(&self.flight, WIDTH, HEIGHT);
        (0..self.sky.system.asteroid_count).any(|i| {
            if self.sky.destroyed.contains(&i) {
                return false;
            }
            let (center, radius) = self.sky.system.asteroid(i);
            if center.distance(self.flight.position) > 900.0 {
                return false;
            }
            camera.project(center).is_some_and(|(p, depth, scale)| {
                depth > 0.0 && p.distance(vec2(160.0, 73.0)) < radius * scale + 14.0
            })
        })
    }
    /// Yield of a mined asteroid by kind.
    pub(super) fn rock_yield(&mut self, i: u32, radius: f32) {
        match rock_kind(self.sky.system.seed, i) {
            Rock::Common => {
                self.collect_item(Item::Tritium, (radius * 0.9) as u16);
                self.collect_item(Item::Iron, (radius * 0.3) as u16);
            }
            Rock::Rich => {
                self.collect_item(Item::Tritium, (radius * 1.4) as u16);
                self.collect_item(Item::Sodium, (radius * 0.5) as u16);
                self.sky.notice("RICH VEIN", [90, 230, 255]);
            }
            Rock::Rare => {
                self.collect_item(Item::Iron, (radius * 0.8) as u16);
                let credits = 150 + radius as u32 * 4;
                self.sky.credits += credits;
                self.sky.notice(format!("PRECIOUS ORE +{credits} CR"), GOLD);
            }
        }
        let (center, _) = self.sky.system.asteroid(i);
        push_capped(
            &mut self.sky.effects,
            Effect::new(
                center,
                EffectKind::Burst,
                [220, 200, 170],
                radius * 1.2,
                0.7,
            ),
            MAX_EFFECTS,
        );
        self.sky.asteroids_mined += 1;
        self.mission_event(Event::AsteroidMined);
    }
    pub(super) fn regrow_asteroids(&mut self, dt: f32) {
        self.sky.regrow += dt;
        if self.sky.regrow > REGROW {
            self.sky.regrow = 0.0;
            self.sky.destroyed.clear();
        }
    }

    // ----- the station hangar ---------------------------------------------------------------

    /// After docking: on foot in the hangar beside the ship.
    pub(super) fn enter_station(&mut self) {
        let (at, yaw) = interior::arrival();
        self.sky.phase = Phase::Station;
        self.sky.overlay = Overlay::None;
        self.sky.walker = Walker::new_flat(at, yaw);
        self.sky.pirates.clear();
        self.sky.projectiles.clear();
        self.sky.traveller_gift = false;
        self.stock_station();
        self.refresh_board();
        // the ship sits on the pad, nose to the door
        let mut ship = self.flight.clone();
        ship.wingmen.clear();
        ship.trails.clear();
        ship.eye = None;
        ship.space_attitude = None;
        ship.position = interior::ORIGIN + interior::PAD.extend(3.0);
        ship.yaw = std::f32::consts::PI;
        ship.pitch = 0.0;
        ship.bank = 0.0;
        ship.spin = 0.0;
        ship.speed = 0.0;
        ship.boost = 0.0;
        self.sky.ship = Some(ship);
        self.flight.eye = Some(self.foot_eye());
        self.flight.speed = 0.0;
        let name = self.sky.system.name.clone();
        self.sky
            .notice(format!("WELCOME ABOARD {name} STATION"), CYAN);
        self.sky.notice("WALK UP TO SOMEONE AND PRESS 7", WHITE);
        self.mission_event(Event::Docked);
    }
    pub(super) fn step_station(&mut self, dt: f32) {
        let held = |key| f32::from(self.keys.contains(&key));
        let input = WalkInput {
            forward: held(UP) - held(DOWN),
            strafe: held(LOOK_RIGHT) - held(LOOK_LEFT),
            turn: held(RIGHT) - held(LEFT),
            look: held(LOOK_UP) - held(LOOK_DOWN),
            jet: false,
            run: self.keys.contains(&BOOST) || self.keys.contains(&BOOST_RIGHT),
            level: true,
        };
        let bounds = (
            interior::ORIGIN.truncate() + interior::WALK_MIN,
            interior::ORIGIN.truncate() + interior::WALK_MAX,
        );
        let before = self.sky.walker.position;
        self.sky
            .walker
            .step_on(&|_, _| 0.0, bounds, input, dt, false);
        self.sky.walker.position = interior::clamp(self.sky.walker.position);
        let moved = (self.sky.walker.position - before).truncate().length();
        self.sky.footstep += moved;
        if self.sky.footstep > 2.6 {
            self.sky.footstep = 0.0;
            self.sounds.push(Sound::Footstep);
        }
        self.flight.eye = Some(self.foot_eye());
        self.flight.time += dt;
        self.sky.vitals.step_ship(dt, false);
    }
    pub fn station_people(&self) -> Vec<interior::Npc> {
        interior::people(&self.sky.system, self.sky.visits, self.sky.time)
    }
    /// Someone close enough to talk to.
    pub fn station_listener(&self) -> Option<usize> {
        let here = self.sky.walker.position;
        self.station_people()
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    i,
                    interior::talk_spot(n).truncate().distance(here.truncate()),
                )
            })
            .filter(|(_, d)| *d < 4.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    pub fn near_pad(&self) -> bool {
        let local = (self.sky.walker.position - interior::ORIGIN).truncate();
        local.distance(interior::PAD) < 12.0
    }
    pub(super) fn talk(&mut self, i: usize) {
        self.sky.talking = i;
        self.sky.chat = 0;
        self.sky.overlay = Overlay::Talk;
        self.sounds.push(Sound::Confirm);
    }
    pub(super) fn talk_key(&mut self, code: u16) -> bool {
        let people = self.station_people();
        let Some(npc) = people.get(self.sky.talking).cloned() else {
            self.sky.overlay = Overlay::None;
            return true;
        };
        match code {
            KEY4 | ENTER => match npc.role {
                Role::Agent => {
                    self.sky.overlay = Overlay::Missions;
                    self.sky.mission_tab = 0;
                    self.sky.mission_select = 0;
                }
                Role::Trader | Role::Outfitter | Role::Technician => {
                    self.sky.overlay = Overlay::Station;
                    self.sky.station_tab = match npc.role {
                        Role::Trader => 0,
                        Role::Outfitter => 1,
                        _ => 2,
                    };
                    self.sky.station_select = 0;
                }
                Role::Traveller => self.traveller_gift(),
            },
            KEY5 => {
                self.sky.chat += 1;
                self.sounds.push(Sound::Navigate);
            }
            KEY8 | TAB | KEY_E | KEY7 => self.sky.overlay = Overlay::None,
            _ => return !matches!(code, PAUSE),
        }
        true
    }
    fn traveller_gift(&mut self) {
        if self.sky.traveller_gift {
            self.sky.notice("THAT IS ALL I KNOW", WHITE);
            return;
        }
        self.sky.traveller_gift = true;
        let mut rng =
            crate::sky::rng::Rng::new(self.sky.system.seed ^ self.sky.visits as u64 ^ 0x91f7);
        match rng.below(4) {
            0 => {
                let credits = 80 + rng.below(140);
                self.sky.credits += credits;
                self.sky
                    .notice(format!("A GIFT FOR THE ROAD +{credits} CR"), GOLD);
            }
            1 => {
                let item =
                    [Item::ShieldCell, Item::LiftCell, Item::RerollCell][rng.below(3) as usize];
                self.sky.inventory.add(item, 1);
                self.sky
                    .notice(format!("TAKE THIS {}", item.name()), item.color());
            }
            2 => {
                self.sky.raid = self.sky.raid.min(90.0);
                self.sky.notice("RAIDERS ARE HUNTING NEAR HERE", RED);
            }
            _ => {
                let id = self
                    .sky
                    .galaxy
                    .neighbors(self.sky.system.id, 6.0, 8)
                    .last()
                    .copied();
                if let Some(id) = id {
                    let name = self.sky.galaxy.system(id).name;
                    self.sky.notice(format!("THEY SAY {name} IS RICH"), CYAN);
                }
            }
        }
        self.sounds.push(Sound::Pickup);
    }
    /// The hangar, its people and the parked ship for the renderer.
    pub(super) fn station_scene(&mut self) {
        let people = self.station_people();
        let eye = self.sky.walker.position;
        let extras = &mut self.renderer.extras;
        interior::hangar(&self.sky.system, self.sky.time, &mut extras.triangles);
        for npc in &people {
            interior::person(npc, self.sky.time, eye, &mut extras.triangles);
        }
        if let Some(ship) = &self.sky.ship {
            extras.ships.push(ship.clone());
        }
    }
    /// Pirate ships, bolts, bursts and asteroid veins for the renderer (space).
    pub(super) fn space_scene(&mut self) {
        // rich rocks glow blue, rare ones gold: three crystal shards on their faces
        let here = self.flight.position;
        let system = &self.sky.system;
        let veins: Vec<(Vec3, f32, u32)> = (0..system.asteroid_count)
            .filter(|i| !self.sky.destroyed.contains(i))
            .filter_map(|i| {
                let (center, radius) = system.asteroid(i);
                let color = match rock_kind(system.seed, i) {
                    Rock::Common => return None,
                    Rock::Rich => crate::render::rgb(90, 230, 255),
                    Rock::Rare => crate::render::rgb(255, 210, 90),
                };
                (center.distance(here) < 3200.0).then_some((center, radius, color))
            })
            .collect();
        let extras = &mut self.renderer.extras;
        for (center, radius, color) in veins {
            for k in 0..3 {
                let a = k as f32 * 2.1 + radius;
                let dir = vec3(a.cos(), a.sin(), 0.35 * (k as f32 - 1.0)).normalize();
                let base = center + dir * radius * 0.8;
                crate::sky::props::spindle(
                    &mut extras.triangles,
                    base,
                    base + dir * radius * 0.15,
                    base + dir * radius * 0.45,
                    radius * 0.12,
                    3,
                    a,
                    (color, color),
                    true,
                );
            }
        }
        if self.sky.pirate_model.is_none() {
            let mut model = self.flight.clone();
            model.wingmen.clear();
            model.trails.clear();
            model.eye = None;
            model.space = true;
            self.sky.pirate_model = Some(model);
        }
        let model = self.sky.pirate_model.as_ref().unwrap();
        for p in &self.sky.pirates {
            let mut ship = model.clone();
            ship.vehicle = if p.ace {
                crate::content::vehicles::Vehicle::Atlas
            } else {
                crate::content::vehicles::Vehicle::Dart
            };
            ship.position = p.position;
            ship.follow = p.position;
            ship.speed = p.speed;
            ship.space_attitude = Some(glam::Quat::from_rotation_arc(-Vec3::Y, p.forward));
            extras.ships.push(ship);
        }
        let shard =
            |out: &mut Vec<crate::render::ExtraTriangle>, at: Vec3, size: f32, color: u32| {
                crate::sky::props::spindle(
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
                1.8,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::Sky;
    use crate::world::World;

    fn tap(app: &mut App, code: u16) {
        app.key(code, 1);
        app.key(code, 0);
    }
    /// An Explore session flying in the starting system's space.
    fn in_space() -> App {
        let key = Sky::default().system.planets[0].key();
        let mut app = App::new(World::generate(key.kind, key.seed));
        app.mode = Mode::Explore;
        app.start();
        let space = app.sky.system.space_key();
        let world = std::sync::Arc::new(World::generate(space.kind, space.seed));
        app.flight.change_world(&world);
        app.world = world;
        app.sky_sync_location();
        assert_eq!(app.sky.location, Location::Space);
        let far = app.sky.system.station + vec3(3000.0, 2000.0, 400.0);
        app.flight.translate(far - app.flight.position);
        app
    }

    #[test]
    fn docking_walks_you_into_the_hangar_to_talk_take_work_and_leave() {
        let mut app = in_space();
        app.sky.phase = Phase::Docking {
            t: 0.99,
            from: app.flight.position,
        };
        app.advance(3);
        assert_eq!(app.sky.phase, Phase::Station);
        assert!(app.flight.eye.is_some());
        assert_eq!(app.sky.board.len(), missions::BOARD);
        app.draw();
        assert!(
            app.renderer.extras.triangles.len() > 300,
            "the hangar is drawn"
        );
        assert_eq!(app.renderer.extras.ships.len(), 1, "your ship on the pad");
        // walk up to the mission agent and talk
        let agent = app
            .station_people()
            .into_iter()
            .position(|n| n.role == Role::Agent)
            .unwrap();
        let spot = interior::talk_spot(&app.station_people()[agent]);
        app.sky.walker.position = spot;
        app.advance(1);
        assert_eq!(app.sky_context(), crate::app::Context::Talk(agent));
        tap(&mut app, KEY7);
        assert_eq!(app.sky.overlay, Overlay::Talk);
        tap(&mut app, KEY5);
        assert_eq!(app.sky.chat, 1);
        tap(&mut app, KEY4);
        assert_eq!(app.sky.overlay, Overlay::Missions);
        for _ in 0..3 {
            tap(&mut app, KEY4);
        }
        assert_eq!(app.sky.missions.len(), MAX_ACTIVE);
        tap(&mut app, KEY4);
        assert_eq!(app.sky.missions.len(), MAX_ACTIVE, "three at most");
        tap(&mut app, KEY8);
        // back to the ship: board and undock
        app.sky.walker.position = interior::ORIGIN + vec3(9.0, 0.0, 0.0);
        app.advance(1);
        assert_eq!(app.sky_context(), crate::app::Context::Board);
        tap(&mut app, KEY7);
        assert_eq!(app.sky.phase, Phase::Flying);
        assert!(app.flight.eye.is_none());
        assert!(app.flight.position.distance(app.sky.system.station) < 600.0);
    }
    #[test]
    fn raids_come_when_due_and_cannons_down_raiders_for_bounties() {
        let mut app = in_space();
        let galaxy = app.sky.galaxy;
        let mut bounty = Mission::new(Kind::Bounty, &app.sky.system, &galaxy, 3, 0);
        bounty.count = 1;
        app.sky.missions.push(bounty);
        app.sky.raid = 10.0;
        app.advance(30);
        assert!(app.sky.pirates.is_empty(), "not due yet");
        app.sky.raid = 0.0;
        app.advance(2);
        assert!(!app.sky.pirates.is_empty(), "raid arrives");
        assert!(
            app.sky.raid >= pirates::RAID_MIN - 1.0,
            "next raid is far off"
        );
        // park a raider in front of the guns and fire
        let ahead = app.flight.position + app.flight.axes()[1] * 300.0;
        app.sky.pirates.truncate(1);
        app.sky.pirates[0].position = ahead;
        app.sky.pirates[0].health = 15.0;
        app.key(KEY5, 1);
        for _ in 0..60 {
            let ahead = app.flight.position + app.flight.axes()[1] * 300.0;
            if let Some(p) = app.sky.pirates.first_mut() {
                p.position = ahead;
                p.cooldown = 9.0;
            }
            app.advance(1);
        }
        app.key(KEY5, 0);
        assert!(app.sky.pirates.is_empty(), "raider down");
        assert!(
            app.sky.missions[0].done(&app.sky.inventory),
            "bounty counted"
        );
    }
    #[test]
    fn a_bolt_that_cripples_the_ship_ends_the_fight_cleanly() {
        let mut app = in_space();
        app.spawn_pirates(2, false);
        app.sky.vitals.shield = 0.01;
        let ship = app.flight.position;
        // two bolts about to hit, the first of which cripples the ship
        let id = app.sky.pirates[0].id;
        for _ in 0..2 {
            let from = ship + Vec3::X * 20.0;
            app.sky.projectiles.push(crate::sky::combat::Projectile {
                position: from,
                velocity: -Vec3::X * 620.0,
                life: 2.0,
                damage: 6.0,
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
                last_hit: id,
            });
        }
        app.advance(2);
        assert!(app.sky.pirates.is_empty());
        assert!(app.flight.position.distance(app.sky.system.station) < 600.0);
    }
    #[test]
    fn a_crippled_ship_is_towed_home_and_loses_cargo() {
        let mut app = in_space();
        app.sky.inventory.add(Item::Tritium, 90);
        let before = app.sky.inventory.count(Item::Tritium);
        app.sky.vitals.shield = 0.01;
        app.ship_hit(50.0, app.flight.position + Vec3::X * 100.0);
        assert!(app.sky.inventory.count(Item::Tritium) < before);
        assert!(app.flight.position.distance(app.sky.system.station) < 600.0);
        assert!(app.sky.vitals.shield > 0.3);
    }
    #[test]
    fn asteroids_come_in_three_kinds_and_grow_back() {
        let system = Sky::default().system;
        let kinds: Vec<Rock> = (0..400).map(|i| rock_kind(system.seed, i)).collect();
        let rare = kinds.iter().filter(|k| **k == Rock::Rare).count();
        let rich = kinds.iter().filter(|k| **k == Rock::Rich).count();
        assert!((5..40).contains(&rare), "{rare}");
        assert!((30..100).contains(&rich), "{rich}");
        let mut app = in_space();
        app.sky.destroyed = vec![1, 2, 3];
        app.regrow_asteroids(REGROW + 1.0);
        assert!(app.sky.destroyed.is_empty());
    }
}
