//! Missions from station mission agents: sixteen kinds with seeded variations, a board of six
//! offers per station visit, up to three active at once, and guild standing that raises rewards
//! and unlocks harder work. Progress comes from game events; rewards are claimed at any station.
use super::galaxy::{Galaxy, StarSystem, SystemId};
use super::inventory::Item;
use super::names;
use super::rng::Rng;

pub const MAX_ACTIVE: usize = 3;
pub const BOARD: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Destroy pirates in the target system
    Bounty,
    /// Destroy a named pirate ace (appears with a raid after accepting)
    Ace,
    /// Destroy guard drones on a planet
    Purge,
    /// Scan new flora on a planet
    FloraSurvey,
    /// Scan new minerals (rocks and crystals) on a planet
    MineralSurvey,
    /// Read inscriptions at monoliths or ruins anywhere
    Relics,
    /// Mine asteroids with the ship
    Asteroids,
    /// Bring elements to any station
    Delivery,
    /// Bring drone cores to any station
    Salvage,
    /// Carry data to the station of another system
    Courier,
    /// Reach another system
    Explore,
    /// Land on a named planet
    Landfall,
    /// Open weapon caches
    CacheRaid,
    /// Signal from beacons
    Beacons,
    /// Open a crashed pod on a named planet
    Rescue,
    /// Visit several new systems
    LongHaul,
}
impl Kind {
    pub const ALL: [Self; 16] = [
        Self::Bounty,
        Self::Ace,
        Self::Purge,
        Self::FloraSurvey,
        Self::MineralSurvey,
        Self::Relics,
        Self::Asteroids,
        Self::Delivery,
        Self::Salvage,
        Self::Courier,
        Self::Explore,
        Self::Landfall,
        Self::CacheRaid,
        Self::Beacons,
        Self::Rescue,
        Self::LongHaul,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Bounty => "PIRATE BOUNTY",
            Self::Ace => "HUNT THE ACE",
            Self::Purge => "DRONE PURGE",
            Self::FloraSurvey => "FLORA SURVEY",
            Self::MineralSurvey => "MINERAL SURVEY",
            Self::Relics => "RELIC HUNT",
            Self::Asteroids => "ASTEROID MINING",
            Self::Delivery => "SUPPLY RUN",
            Self::Salvage => "DRONE SALVAGE",
            Self::Courier => "COURIER",
            Self::Explore => "SCOUTING",
            Self::Landfall => "LANDFALL",
            Self::CacheRaid => "CACHE RAID",
            Self::Beacons => "BEACON NETWORK",
            Self::Rescue => "RESCUE",
            Self::LongHaul => "LONG HAUL",
        }
    }
    /// Weight on the board: common jobs show up more often
    fn weight(self) -> u32 {
        match self {
            Self::Bounty | Self::FloraSurvey | Self::Delivery | Self::Asteroids => 10,
            Self::Purge | Self::MineralSurvey | Self::Salvage | Self::Landfall => 8,
            Self::Courier | Self::Explore | Self::Relics | Self::Rescue => 6,
            Self::CacheRaid | Self::Beacons => 5,
            Self::Ace | Self::LongHaul => 3,
        }
    }
    /// Whether a planet of the target system is named
    pub fn needs_planet(self) -> bool {
        matches!(
            self,
            Self::Purge | Self::FloraSurvey | Self::MineralSurvey | Self::Landfall | Self::Rescue
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prize {
    None,
    Item(Item, u16),
    Weapon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mission {
    pub kind: Kind,
    /// Where the mission was offered
    pub origin: SystemId,
    /// Where it has to be done (same as origin for local work)
    pub target: SystemId,
    /// Planet index in the target system, if named
    pub planet: u8,
    /// Element to deliver
    pub item: Item,
    pub count: u16,
    pub progress: u16,
    pub reward: u32,
    pub prize: Prize,
    pub seed: u32,
    /// 0–4: harder work at higher guild ranks
    pub tier: u8,
}

/// Something that happened in the game, as missions see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    PirateDown {
        ace: bool,
    },
    DroneDown,
    Scanned {
        flora: u16,
        minerals: u16,
    },
    Read,
    AsteroidMined,
    CacheOpened,
    BeaconSignalled,
    PodOpened,
    Landed,
    /// Arrived in a system (`new`: never visited before)
    Arrived {
        new: bool,
    },
    Docked,
}

/// Guild ranks by standing points.
pub const RANKS: [(&str, u32); 5] = [
    ("ROOKIE", 0),
    ("FREELANCER", 5),
    ("VETERAN", 15),
    ("ACE", 35),
    ("LEGEND", 70),
];
pub fn rank(standing: u32) -> usize {
    RANKS
        .iter()
        .rposition(|&(_, need)| standing >= need)
        .unwrap_or(0)
}

impl Mission {
    /// A mission of `kind` offered at `system`, from the seed.
    pub fn new(kind: Kind, system: &StarSystem, galaxy: &Galaxy, seed: u32, tier: u8) -> Self {
        let mut rng = Rng::new(seed as u64 ^ system.seed);
        let t = tier as u16;
        let far = || -> Option<SystemId> {
            let near = galaxy.neighbors(system.id, 3.0 + tier as f32, 12);
            near.get((seed as usize >> 4) % near.len().max(1)).copied()
        };
        let mut target = system.id;
        let mut item = Item::Carbon;
        let count = match kind {
            Kind::Bounty => 2 + rng.below(2 + t as u32) as u16,
            Kind::Ace => 1,
            Kind::Purge => 3 + rng.below(3) as u16 + t * 2,
            Kind::FloraSurvey => 4 + rng.below(5) as u16 + t * 2,
            Kind::MineralSurvey => 5 + rng.below(6) as u16 + t * 2,
            Kind::Relics => 1 + rng.below(2) as u16 + t / 2,
            Kind::Asteroids => 3 + rng.below(4) as u16 + t * 2,
            Kind::Delivery => {
                item = Item::ELEMENTS[rng.below(5) as usize];
                (30 + 10 * rng.below(5) as u16 + 20 * t).min(240)
            }
            Kind::Salvage => {
                item = Item::DroneCore;
                1 + rng.below(2) as u16 + t
            }
            Kind::Courier | Kind::Explore => {
                target = far().unwrap_or(system.id);
                1
            }
            Kind::Landfall | Kind::Rescue => 1,
            Kind::CacheRaid => 1 + t / 2,
            Kind::Beacons => 2 + t,
            Kind::LongHaul => 3 + t,
        };
        let planet = if kind.needs_planet() {
            let n = if kind == Kind::Landfall {
                system.planets.len()
            } else {
                system.atmospheric()
            };
            rng.below(n.max(1) as u32) as u8
        } else {
            u8::MAX
        };
        let base = match kind {
            Kind::Bounty => 300 * count as u32,
            Kind::Ace => 1800,
            Kind::Purge => 180 * count as u32,
            Kind::FloraSurvey => 90 * count as u32,
            Kind::MineralSurvey => 60 * count as u32,
            Kind::Relics => 400 * count as u32,
            Kind::Asteroids => 120 * count as u32,
            Kind::Delivery => item.price() * count as u32 * 2,
            Kind::Salvage => 500 * count as u32,
            Kind::Courier => 900,
            Kind::Explore => 700,
            Kind::Landfall => 450,
            Kind::CacheRaid => 700 * count as u32,
            Kind::Beacons => 220 * count as u32,
            Kind::Rescue => 600,
            Kind::LongHaul => 600 * count as u32,
        };
        let reward =
            (base as f32 * (1.0 + 0.25 * tier as f32) * rng.range(0.9, 1.15)) as u32 / 10 * 10;
        let prize = match rng.below(10) {
            0..=1 => Prize::Weapon,
            2..=4 => Prize::Item(
                [
                    Item::JumpCell,
                    Item::RerollCell,
                    Item::ShieldCell,
                    Item::LiftCell,
                    Item::HullPatch,
                ][rng.below(5) as usize],
                1,
            ),
            _ => Prize::None,
        };
        let prize = if kind == Kind::Ace {
            Prize::Weapon
        } else {
            prize
        };
        Self {
            kind,
            origin: system.id,
            target,
            planet,
            item,
            count: count.max(1),
            progress: 0,
            reward,
            prize,
            seed,
            tier,
        }
    }
    /// The six offers at a station on visit `visit`, for a pilot of guild rank `rank`.
    pub fn board(system: &StarSystem, galaxy: &Galaxy, visit: u32, rank: usize) -> Vec<Self> {
        let mut rng = Rng::new(system.seed ^ (visit as u64).wrapping_mul(0x5a17));
        let weights: Vec<u32> = Kind::ALL.iter().map(|k| k.weight()).collect();
        let mut out = Vec::with_capacity(BOARD);
        let mut guard = 0;
        while out.len() < BOARD && guard < 64 {
            guard += 1;
            let kind = Kind::ALL[rng.weighted(&weights)];
            if out.iter().filter(|m: &&Mission| m.kind == kind).count() >= 1 {
                continue;
            }
            let tier = (rank as u32 + rng.below(2)).min(4) as u8;
            out.push(Self::new(kind, system, galaxy, rng.next_u32(), tier));
        }
        out
    }
    /// Name of the pirate ace for an ace hunt.
    pub fn ace_name(&self) -> String {
        names::name(self.seed as u64, 55)
    }
    /// Count an event. `here` is the current system, `planet` the planet you are on.
    pub fn observe(&mut self, event: Event, here: SystemId, planet: Option<usize>) {
        if self.complete_by_count() {
            return;
        }
        let in_target = here == self.target;
        let on_planet = in_target && planet == Some(self.planet as usize);
        let add = match (self.kind, event) {
            (Kind::Bounty, Event::PirateDown { .. }) if in_target => 1,
            (Kind::Ace, Event::PirateDown { ace: true }) if in_target => 1,
            (Kind::Purge, Event::DroneDown) if on_planet => 1,
            (Kind::FloraSurvey, Event::Scanned { flora, .. }) if on_planet => flora,
            (Kind::MineralSurvey, Event::Scanned { minerals, .. }) if on_planet => minerals,
            (Kind::Relics, Event::Read) => 1,
            (Kind::Asteroids, Event::AsteroidMined) => 1,
            (Kind::Courier, Event::Docked) if in_target => 1,
            (Kind::Explore, Event::Arrived { .. }) if in_target => 1,
            (Kind::Landfall, Event::Landed) if on_planet => 1,
            (Kind::CacheRaid, Event::CacheOpened) => 1,
            (Kind::Beacons, Event::BeaconSignalled) => 1,
            (Kind::Rescue, Event::PodOpened) if on_planet => 1,
            (Kind::LongHaul, Event::Arrived { new: true }) => 1,
            _ => 0,
        };
        self.progress = (self.progress + add).min(self.count);
    }
    fn complete_by_count(&self) -> bool {
        !matches!(self.kind, Kind::Delivery | Kind::Salvage) && self.progress >= self.count
    }
    /// Done and ready to claim (deliveries need the goods in the inventory).
    pub fn done(&self, inventory: &super::inventory::Inventory) -> bool {
        match self.kind {
            Kind::Delivery | Kind::Salvage => inventory.count(self.item) >= self.count,
            _ => self.progress >= self.count,
        }
    }
    /// Progress for the tracker (deliveries count what you carry).
    pub fn shown(&self, inventory: &super::inventory::Inventory) -> u16 {
        match self.kind {
            Kind::Delivery | Kind::Salvage => inventory.count(self.item).min(self.count),
            _ => self.progress,
        }
    }
    /// One-line description (at most 36 characters).
    pub fn describe(&self, galaxy: &Galaxy) -> String {
        let target = galaxy.system(self.target);
        let planet = target
            .planets
            .get(self.planet as usize)
            .map_or_else(String::new, |p| p.name.clone());
        let mut text = match self.kind {
            Kind::Bounty => format!("DOWN {} PIRATES IN {}", self.count, target.name),
            Kind::Ace => format!("DOWN THE ACE {}", self.ace_name()),
            Kind::Purge => format!("DOWN {} DRONES ON {planet}", self.count),
            Kind::FloraSurvey => format!("SCAN {} FLORA ON {planet}", self.count),
            Kind::MineralSurvey => format!("SCAN {} MINERALS ON {planet}", self.count),
            Kind::Relics => format!("READ {} INSCRIPTIONS", self.count),
            Kind::Asteroids => format!("MINE {} ASTEROIDS", self.count),
            Kind::Delivery => format!("DELIVER {} {}", self.count, self.item.name()),
            Kind::Salvage => format!("DELIVER {} DRONE CORES", self.count),
            Kind::Courier => format!("CARRY DATA TO {}", target.name),
            Kind::Explore => format!("REACH THE {} SYSTEM", target.name),
            Kind::Landfall => format!("LAND ON {planet}"),
            Kind::CacheRaid => format!("OPEN {} WEAPON CACHES", self.count),
            Kind::Beacons => format!("SIGNAL {} BEACONS", self.count),
            Kind::Rescue => format!("FIND A CRASHED POD ON {planet}"),
            Kind::LongHaul => format!("VISIT {} NEW SYSTEMS", self.count),
        };
        text.truncate(36);
        text
    }
}

/// Short tracker label: "BOUNTY 1/3".
pub fn tracker(m: &Mission, inventory: &super::inventory::Inventory) -> String {
    let title = m.kind.title().split(' ').next_back().unwrap_or("");
    format!("{title} {}/{}", m.shown(inventory), m.count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::inventory::Inventory;
    #[test]
    fn boards_are_varied_deterministic_and_valid() {
        let galaxy = Galaxy::default();
        let system = galaxy.system(galaxy.start());
        let mut kinds = std::collections::HashSet::new();
        for visit in 0..60 {
            let board = Mission::board(&system, &galaxy, visit, 0);
            assert_eq!(board, Mission::board(&system, &galaxy, visit, 0));
            assert_eq!(board.len(), BOARD);
            for m in &board {
                kinds.insert(m.kind);
                assert!(m.reward >= 100 && m.count >= 1);
                assert!(m.describe(&galaxy).len() <= 36, "{}", m.describe(&galaxy));
                if m.kind.needs_planet() {
                    assert!((m.planet as usize) < system.planets.len());
                }
                if matches!(m.kind, Kind::Courier | Kind::Explore) {
                    assert_ne!(m.target, system.id, "travel missions lead elsewhere");
                }
            }
        }
        assert_eq!(kinds.len(), Kind::ALL.len(), "every kind appears");
    }
    #[test]
    fn events_advance_only_the_right_missions() {
        let galaxy = Galaxy::default();
        let system = galaxy.system(galaxy.start());
        let mut bounty = Mission::new(Kind::Bounty, &system, &galaxy, 1, 0);
        let other = SystemId::new(99, 99, 0);
        bounty.observe(Event::PirateDown { ace: false }, other, None);
        assert_eq!(bounty.progress, 0, "pirates elsewhere do not count");
        for _ in 0..10 {
            bounty.observe(Event::PirateDown { ace: false }, system.id, None);
        }
        assert_eq!(bounty.progress, bounty.count);
        assert!(bounty.done(&Inventory::default()));
        let mut survey = Mission::new(Kind::FloraSurvey, &system, &galaxy, 2, 0);
        let p = survey.planet as usize;
        survey.observe(
            Event::Scanned {
                flora: 3,
                minerals: 9,
            },
            system.id,
            Some(p + 1),
        );
        assert_eq!(survey.progress, 0, "wrong planet");
        survey.observe(
            Event::Scanned {
                flora: 3,
                minerals: 9,
            },
            system.id,
            Some(p),
        );
        assert_eq!(survey.progress, 3.min(survey.count));
        let delivery = Mission::new(Kind::Delivery, &system, &galaxy, 3, 0);
        let mut inv = Inventory::default();
        assert!(!delivery.done(&inv));
        inv.add(delivery.item, delivery.count);
        assert!(delivery.done(&inv));
        assert_eq!(
            tracker(&delivery, &inv),
            format!("RUN {0}/{0}", delivery.count)
        );
    }
    #[test]
    fn ranks_and_tiers_raise_rewards() {
        assert_eq!(rank(0), 0);
        assert_eq!(rank(15), 2);
        assert_eq!(rank(1000), 4);
        let galaxy = Galaxy::default();
        let system = galaxy.system(galaxy.start());
        let easy = Mission::new(Kind::Bounty, &system, &galaxy, 5, 0);
        let hard = Mission::new(Kind::Bounty, &system, &galaxy, 5, 4);
        assert!(hard.reward > easy.reward && hard.count >= easy.count);
    }
}
