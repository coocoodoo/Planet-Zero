//! Cardputer Sky: procedural space exploration and survival built on the Flight Club engine.
//! Game logic and session state live here; `app::sky` connects them to the engine, `ui::sky`
//! draws them, and the renderer's `Extras` shows props, ships, stations and asteroids.
pub mod arsenal;
pub mod biome;
pub mod combat;
pub mod contracts;
pub mod drones;
pub mod galaxy;
pub mod inventory;
pub mod lore;
pub mod names;
pub mod png;
pub mod props;
pub mod rng;
pub mod save;
pub mod station;
pub mod survival;
pub mod walker;

use arsenal::{Stats, Weapon};
use galaxy::{Galaxy, StarSystem, SystemId};
use glam::{Vec2, Vec3};
use inventory::{Inventory, Item};
use save::{Place, SaveData};
use station::Upgrades;
use std::collections::HashSet;
use survival::Vitals;
use walker::Walker;

/// What the player is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Flying,
    /// Ship settling onto `to` over `LANDING_SECONDS`
    Landing {
        t: f32,
        from: Vec3,
        to: Vec3,
    },
    OnFoot,
    /// Ship lifting off from `from`
    Takeoff {
        t: f32,
        from: Vec3,
    },
    /// Autopilot into the station's bay
    Docking {
        t: f32,
        from: Vec3,
    },
    Docked,
}
pub const LANDING_SECONDS: f32 = 3.0;
pub const TAKEOFF_SECONDS: f32 = 2.4;
pub const DOCKING_SECONDS: f32 = 3.0;

/// Full-screen panels drawn over the paused 3D view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    None,
    Galaxy,
    System,
    Inventory,
    Station,
    /// An inscription read at a monolith or ruin
    Lore,
}

/// Equipped weapons, the locker, and per-slot heat.
#[derive(Clone, Debug, PartialEq)]
pub struct Loadout {
    pub equipped: [Option<Weapon>; 3],
    /// Cached stats of each slot (recomputed only when a slot changes)
    pub stats: [Option<Stats>; 3],
    pub heat: [f32; 3],
    /// Seconds each slot stays locked after overheating
    pub overheat: [f32; 3],
    pub locker: Vec<Weapon>,
    pub active: usize,
    /// Seconds until the active weapon may fire again
    pub cooldown: f32,
}
pub const LOCKER: usize = 6;
impl Loadout {
    pub fn new(equipped: [Option<Weapon>; 3], locker: Vec<Weapon>, active: usize) -> Self {
        let mut loadout = Self {
            equipped,
            stats: [None; 3],
            heat: [0.0; 3],
            overheat: [0.0; 3],
            locker,
            active: active.min(2),
            cooldown: 0.0,
        };
        for (slot, weapon) in equipped.into_iter().enumerate() {
            loadout.set(slot, weapon);
        }
        if loadout.equipped[loadout.active].is_none() {
            loadout.cycle(1);
        }
        loadout
    }
    pub fn set(&mut self, slot: usize, weapon: Option<Weapon>) {
        self.equipped[slot] = weapon;
        self.stats[slot] = weapon.map(Weapon::stats);
        self.heat[slot] = 0.0;
        self.overheat[slot] = 0.0;
    }
    pub fn weapon(&self) -> Option<Weapon> {
        self.equipped[self.active]
    }
    pub fn stats(&self) -> Option<Stats> {
        self.stats[self.active]
    }
    /// Next equipped slot in `direction`; returns whether the active slot changed.
    pub fn cycle(&mut self, direction: i32) -> bool {
        for step in 1..=3 {
            let slot = (self.active as i32 + direction * step).rem_euclid(3) as usize;
            if self.equipped[slot].is_some() {
                let changed = slot != self.active;
                self.active = slot;
                self.cooldown = self.cooldown.max(0.25);
                return changed;
            }
        }
        false
    }
    /// Store a new weapon: an empty slot first, then the locker. False when everything is full.
    pub fn add(&mut self, weapon: Weapon) -> bool {
        if let Some(slot) = self.equipped.iter().position(Option::is_none) {
            self.set(slot, Some(weapon));
            return true;
        }
        if self.locker.len() < LOCKER {
            self.locker.push(weapon);
            return true;
        }
        false
    }
    pub fn count(&self) -> usize {
        self.equipped.iter().flatten().count() + self.locker.len()
    }
    /// Every weapon in display order: slots 0–2, then the locker.
    pub fn all(&self) -> Vec<Option<Weapon>> {
        let mut list = self.equipped.to_vec();
        list.extend(self.locker.iter().map(|&w| Some(w)));
        list
    }
    pub fn cool(&mut self, dt: f32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        for slot in 0..3 {
            self.overheat[slot] = (self.overheat[slot] - dt).max(0.0);
            self.heat[slot] = (self.heat[slot] - dt * 0.35).max(0.0);
        }
    }
    pub fn ready(&self) -> bool {
        self.weapon().is_some() && self.overheat[self.active] <= 0.0 && self.cooldown <= 0.0
    }
}
impl Default for Loadout {
    fn default() -> Self {
        let pulse = Weapon {
            archetype: arsenal::Archetype::PulseLaser,
            seed: 3,
            rarity: arsenal::Rarity::Common,
        };
        Self::new([Some(Weapon::starter()), Some(pulse), None], Vec::new(), 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Space,
    Planet(usize),
    /// A world that is not part of the current system (should not happen)
    Elsewhere,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parked {
    pub planet: usize,
    pub position: Vec3,
    pub yaw: f32,
}

/// A target under the mining beam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Prop(props::Prop),
    Asteroid(u32),
}
impl Target {
    pub fn id(&self) -> u64 {
        match self {
            Self::Prop(p) => p.id,
            Self::Asteroid(i) => 0xa57e_0000_0000 | *i as u64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub text: String,
    pub color: [u8; 3],
    pub time: f32,
}

pub struct Sky {
    pub active: bool,
    pub galaxy: Galaxy,
    pub system: StarSystem,
    /// Planet bound to the engine's Earth slot (the one you can fly down to)
    pub course: usize,
    pub location: Location,
    pub phase: Phase,
    pub overlay: Overlay,
    pub walker: Walker,
    pub parked: Option<Parked>,
    pub vitals: Vitals,
    pub inventory: Inventory,
    pub credits: u32,
    /// Unsold survey data
    pub data: u32,
    pub upgrades: Upgrades,
    pub visited: Vec<SystemId>,
    pub discoveries: u32,
    /// Ids scanned this session (props, planets)
    pub scanned: HashSet<u64>,
    /// Props mined out this session
    pub harvested: HashSet<u64>,
    /// Asteroids destroyed in the current system
    pub destroyed: Vec<u32>,
    /// Props near the player, refreshed as they move
    pub props: Vec<props::Prop>,
    pub props_center: Vec2,
    pub props_planet: Option<(SystemId, usize)>,
    pub target: Option<Target>,
    pub mining: bool,
    pub mine_progress: f32,
    /// Beam end point for drawing
    pub beam: Option<Vec3>,
    /// Scanner wave: seconds since the last pulse
    pub scan_age: f32,
    /// Props revealed by the last scan
    pub revealed: Vec<props::Prop>,
    pub jetting: bool,
    pub notices: Vec<Notice>,
    pub neighbors: Vec<SystemId>,
    pub map_select: usize,
    pub map_tilt: f32,
    pub system_select: usize,
    pub inventory_select: usize,
    pub crafting: bool,
    pub craft_select: usize,
    pub station_tab: usize,
    pub station_select: usize,
    /// Star a warp tunnel has been opened to
    pub warp_target: Option<SystemId>,
    /// Flying to the station on autopilot
    pub autodock: bool,
    pub save_timer: f32,
    pub time: f32,
    /// First-person bob phase
    pub stride: f32,
    pub save_error: bool,
    /// Where to put the player when the session starts (from the save file)
    pub spawn: Option<Place>,
    /// The parked ship as drawn (a copy of the player's aircraft)
    pub ship: Option<crate::flight::Flight>,
    pub loadout: Loadout,
    pub drones: Vec<drones::Drone>,
    pub projectiles: Vec<combat::Projectile>,
    pub effects: Vec<combat::Effect>,
    pub alert: drones::Alert,
    pub combat_rng: rng::Rng,
    pub next_drone: u32,
    /// Drones destroyed in total
    pub drones_destroyed: u32,
    /// Accumulated mining damage per prop
    pub prop_damage: std::collections::HashMap<u64, f32>,
    /// Seconds left on the hit marker
    pub hit_marker: f32,
    /// Recent hits on the player: where they came from and how long ago
    pub hurt: Vec<(Vec3, f32)>,
    pub contract: Option<contracts::Contract>,
    pub visits: u32,
    /// Weapons for sale at the docked station
    pub armory: [Option<Weapon>; 3],
    /// Title and lines of the inscription being read
    pub lore: Option<(String, [String; 3])>,
    /// Points of interest marked by a beacon on this planet
    pub marked: Vec<props::Prop>,
    /// Inventory shows the weapons view
    pub arms: bool,
    pub arms_select: usize,
    /// Tilt fine-aim offset (yaw, pitch) in radians
    pub aim: Vec2,
    /// Device roll and pitch when fine-aim was centred
    pub tilt_ref: Option<(f32, f32)>,
    /// Time of the last back press (double tap turns around)
    pub back_tap: f32,
    pub footstep: f32,
}

impl Default for Sky {
    fn default() -> Self {
        let galaxy = Galaxy::default();
        let id = galaxy.start();
        Self {
            active: false,
            galaxy,
            system: galaxy.system(id),
            course: 0,
            location: Location::Elsewhere,
            phase: Phase::Flying,
            overlay: Overlay::None,
            walker: Walker {
                position: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                velocity: Vec3::ZERO,
                grounded: true,
                turn_hold: 0.0,
                quick_turn: 0.0,
                look_idle: 0.0,
            },
            parked: None,
            vitals: Vitals::default(),
            inventory: Self::starting_inventory(),
            credits: 250,
            data: 0,
            upgrades: Upgrades::default(),
            visited: vec![id],
            discoveries: 0,
            scanned: HashSet::new(),
            harvested: HashSet::new(),
            destroyed: Vec::new(),
            props: Vec::with_capacity(256),
            props_center: Vec2::splat(-1e9),
            props_planet: None,
            target: None,
            mining: false,
            mine_progress: 0.0,
            beam: None,
            scan_age: f32::INFINITY,
            revealed: Vec::new(),
            jetting: false,
            notices: Vec::new(),
            neighbors: Vec::new(),
            map_select: 0,
            map_tilt: 0.5,
            system_select: 0,
            inventory_select: 0,
            crafting: false,
            craft_select: 0,
            station_tab: 0,
            station_select: 0,
            warp_target: None,
            autodock: false,
            save_timer: 0.0,
            time: 0.0,
            stride: 0.0,
            save_error: false,
            spawn: None,
            ship: None,
            loadout: Loadout::default(),
            drones: Vec::with_capacity(drones::MAX_DRONES),
            projectiles: Vec::with_capacity(combat::MAX_PROJECTILES),
            effects: Vec::with_capacity(combat::MAX_EFFECTS),
            alert: drones::Alert::default(),
            combat_rng: rng::Rng::new(id.x as u64 ^ 0xd2),
            next_drone: 1,
            drones_destroyed: 0,
            prop_damage: std::collections::HashMap::new(),
            hit_marker: 0.0,
            hurt: Vec::new(),
            contract: None,
            visits: 0,
            armory: [None; 3],
            lore: None,
            marked: Vec::new(),
            arms: false,
            arms_select: 0,
            aim: Vec2::ZERO,
            tilt_ref: None,
            back_tap: -10.0,
            footstep: 0.0,
        }
    }
}

impl Sky {
    fn starting_inventory() -> Inventory {
        let mut inventory = Inventory::default();
        inventory.add(Item::Carbon, 40);
        inventory.add(Item::Iron, 30);
        inventory.add(Item::Oxygen, 20);
        inventory.add(Item::Sodium, 20);
        inventory.add(Item::Tritium, 30);
        inventory.add(Item::LiftCell, 1);
        inventory
    }
    pub fn notice(&mut self, text: impl Into<String>, color: [u8; 3]) {
        if self.notices.len() >= 3 {
            self.notices.remove(0);
        }
        self.notices.push(Notice {
            text: text.into(),
            color,
            time: 3.5,
        });
    }
    pub fn planet(&self) -> Option<&galaxy::Planet> {
        match self.location {
            Location::Planet(i) => self.system.planets.get(i),
            _ => None,
        }
    }
    /// Move to another star system (after a warp or when loading a save).
    pub fn enter_system(&mut self, id: SystemId) -> bool {
        self.system = self.galaxy.system(id);
        self.course = 0;
        self.destroyed.clear();
        self.warp_target = None;
        self.neighbors.clear();
        let new = !self.visited.contains(&id);
        if new {
            self.visited.push(id);
        }
        new
    }
    pub fn refresh_neighbors(&mut self) {
        self.neighbors = self.galaxy.neighbors(self.system.id, 6.5, 24);
        self.map_select = self.map_select.min(self.neighbors.len().saturating_sub(1));
    }
    pub fn in_range(&self, id: SystemId) -> bool {
        self.system.id.cells(id) <= self.upgrades.warp_range()
    }
    pub fn to_save(&self, place: Place) -> SaveData {
        SaveData {
            galaxy: self.galaxy.seed,
            system: self.system.id,
            course: self.course,
            place,
            credits: self.credits,
            data: self.data,
            inventory: self.inventory,
            upgrades: self.upgrades,
            vitals: self.vitals,
            visited: self.visited.clone(),
            discoveries: self.discoveries,
            equipped: self.loadout.equipped,
            locker: self.loadout.locker.clone(),
            active: self.loadout.active,
            destroyed: self.drones_destroyed,
            contract: self.contract,
            visits: self.visits,
        }
    }
    pub fn apply_save(&mut self, save: &SaveData) {
        self.galaxy = Galaxy { seed: save.galaxy };
        self.enter_system(save.system);
        self.course = save.course.min(self.system.atmospheric().saturating_sub(1));
        self.credits = save.credits;
        self.data = save.data;
        self.inventory = save.inventory;
        self.upgrades = save.upgrades;
        self.vitals = save.vitals;
        self.visited = save.visited.clone();
        if !self.visited.contains(&save.system) {
            self.visited.push(save.system);
        }
        self.discoveries = save.discoveries;
        self.loadout = Loadout::new(save.equipped, save.locker.clone(), save.active);
        self.drones_destroyed = save.destroyed;
        self.contract = save.contract;
        self.visits = save.visits;
    }
    /// Leaving a planet: drones, shots and beacon marks stay behind.
    pub fn clear_combat(&mut self) {
        self.drones.clear();
        self.projectiles.clear();
        self.effects.clear();
        self.marked.clear();
        self.alert = drones::Alert::default();
        self.prop_damage.clear();
        self.hurt.clear();
    }
    /// Time-based bookkeeping that does not depend on the engine.
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        self.scan_age += dt;
        self.hit_marker = (self.hit_marker - dt).max(0.0);
        for hurt in &mut self.hurt {
            hurt.1 += dt;
        }
        self.hurt.retain(|h| h.1 < 0.8);
        for effect in &mut self.effects {
            effect.time += dt;
        }
        self.effects.retain(|e| e.time < e.duration);
        for notice in &mut self.notices {
            notice.time -= dt;
        }
        self.notices.retain(|n| n.time > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loadout_cycles_slots_and_overflows_into_the_locker() {
        let mut l = Loadout::default();
        assert_eq!(l.weapon(), Some(Weapon::starter()));
        assert!(l.cycle(1));
        assert_eq!(l.active, 1);
        assert!(l.cycle(1), "skips the empty third slot");
        assert_eq!(l.active, 0);
        for seed in 0..7 {
            assert!(l.add(Weapon::roll(seed, arsenal::Rarity::Common, 0.0)));
        }
        assert_eq!(l.count(), 3 + LOCKER);
        assert!(!l.add(Weapon::starter()));
        assert!(l.stats.iter().all(Option::is_some));
    }
    #[test]
    fn new_session_starts_on_a_lush_world_and_round_trips_a_save() {
        let mut sky = Sky::default();
        assert_eq!(sky.system.planets[0].biome, biome::Biome::Lush);
        sky.credits = 777;
        sky.inventory.add(Item::JumpCell, 2);
        let save = sky.to_save(Place::Space(Vec3::new(1.0, 2.0, 3.0), 0.5));
        let mut other = Sky::default();
        other.apply_save(&SaveData::from_text(&save.to_text()).unwrap());
        assert_eq!(other.credits, 777);
        assert_eq!(other.inventory, sky.inventory);
        assert_eq!(other.system, sky.system);
        assert_eq!(other.loadout, sky.loadout);
        for _ in 0..5 {
            other.notice("X", [0; 3]);
        }
        assert_eq!(other.notices.len(), 3);
        other.tick(4.0);
        assert!(other.notices.is_empty());
    }
}
