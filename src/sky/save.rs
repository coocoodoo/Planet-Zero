//! Save file: one `key value…` line per field, so it survives version changes and can be read by
//! hand. Stored next to Flight Club's records (`~/.local/share/flight-club/sky-save-v1.txt`), or
//! at `$FLIGHT_CLUB_SKY_SAVE`.
use super::arsenal::{Archetype, Rarity, Weapon};
use super::galaxy::SystemId;
use super::inventory::Inventory;
use super::inventory::Item;
use super::missions::{Kind, Mission, Prize};
use super::station::Upgrades;
use super::survival::Vitals;
use glam::Vec3;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// In the system's space map at this position
    Space(Vec3, f32),
    /// Flying over planet `index` at this position and heading
    Flying(usize, Vec3, f32),
    /// Standing on planet `index`, ship parked at `ship` (with heading)
    Landed { planet: usize, ship: Vec3, yaw: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveData {
    pub galaxy: u64,
    pub system: SystemId,
    pub course: usize,
    pub place: Place,
    pub credits: u32,
    pub data: u32,
    pub inventory: Inventory,
    pub upgrades: Upgrades,
    pub vitals: Vitals,
    pub visited: Vec<SystemId>,
    pub discoveries: u32,
    /// Three weapon slots (version 2)
    pub equipped: [Option<Weapon>; 3],
    pub locker: Vec<Weapon>,
    pub active: usize,
    /// Drones destroyed in total
    pub destroyed: u32,
    /// Active missions (version 3)
    pub missions: Vec<Mission>,
    pub standing: u32,
    pub missions_done: u32,
    /// Seconds of play until the next random pirate raid (0 or less: roll a new one)
    pub raid: f32,
    /// Station dockings (rotates armory stock and contracts)
    pub visits: u32,
}

fn weapon_text(w: Option<Weapon>) -> String {
    w.map_or("-".to_owned(), |w| {
        format!("{}.{}.{}", w.archetype.index(), w.seed, w.rarity as u8)
    })
}
fn parse_weapon(word: &str) -> Option<Weapon> {
    let mut parts = word.split('.');
    let archetype = Archetype::from_index(parts.next()?.parse().ok()?);
    let seed = parts.next()?.parse().ok()?;
    let rarity = Rarity::from_index(parts.next()?.parse().ok()?);
    Some(Weapon {
        archetype,
        seed,
        rarity,
    })
}

pub fn path() -> Option<PathBuf> {
    if cfg!(test) {
        return std::env::var_os("FLIGHT_CLUB_SKY_SAVE_TEST").map(PathBuf::from);
    }
    std::env::var_os("FLIGHT_CLUB_SKY_SAVE")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
                .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
                .map(|p| p.join("flight-club/sky-save-v1.txt"))
        })
}

fn v3(v: Vec3) -> String {
    format!("{:.2} {:.2} {:.2}", v.x, v.y, v.z)
}

impl SaveData {
    pub fn to_text(&self) -> String {
        let mut s = String::from("cardputer-sky 3\n");
        s += &format!("galaxy {}\n", self.galaxy);
        s += &format!(
            "system {} {} {}\n",
            self.system.x, self.system.y, self.system.z
        );
        s += &format!("course {}\n", self.course);
        match self.place {
            Place::Space(p, yaw) => s += &format!("space {} {yaw:.3}\n", v3(p)),
            Place::Flying(i, p, yaw) => s += &format!("flying {i} {} {yaw:.3}\n", v3(p)),
            Place::Landed { planet, ship, yaw } => {
                s += &format!("landed {planet} {} {yaw:.3}\n", v3(ship))
            }
        }
        s += &format!("credits {}\ndata {}\n", self.credits, self.data);
        s += "inventory";
        for c in self.inventory.counts {
            s += &format!(" {c}");
        }
        s += "\nupgrades";
        for l in self.upgrades.levels {
            s += &format!(" {l}");
        }
        let v = &self.vitals;
        s += &format!(
            "\nvitals {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} {:.3}\n",
            v.life, v.hazard, v.tool, v.shield, v.pulse, v.lift, v.suit
        );
        s += "visited";
        for id in &self.visited {
            s += &format!(" {},{},{}", id.x, id.y, id.z);
        }
        s += &format!("\ndiscoveries {}\n", self.discoveries);
        s += "weapons";
        for w in self.equipped {
            s += &format!(" {}", weapon_text(w));
        }
        s += "\nlocker";
        for w in &self.locker {
            s += &format!(" {}", weapon_text(Some(*w)));
        }
        s += &format!(
            "\nactive {}\ndestroyed {}\nvisits {}\n",
            self.active, self.destroyed, self.visits
        );
        s += &format!(
            "standing {}\nmissions_done {}\nraid {:.0}\n",
            self.standing, self.missions_done, self.raid
        );
        for m in &self.missions {
            let (prize, prize_item, prize_n) = match m.prize {
                Prize::None => (0, 0, 0),
                Prize::Item(item, n) => (1, item.index(), n),
                Prize::Weapon => (2, 0, 0),
            };
            s += &format!(
                "mission {} {} {} {} {} {} {} {} {} {} {} {} {prize} {prize_item} {prize_n} {} {}\n",
                Kind::ALL.iter().position(|k| *k == m.kind).unwrap_or(0),
                m.origin.x,
                m.origin.y,
                m.origin.z,
                m.target.x,
                m.target.y,
                m.target.z,
                m.planet,
                m.item.index(),
                m.count,
                m.progress,
                m.reward,
                m.seed,
                m.tier
            );
        }
        s
    }
    pub fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let version = lines.next()?.trim();
        if !matches!(
            version,
            "cardputer-sky 1" | "cardputer-sky 2" | "cardputer-sky 3"
        ) {
            return None;
        }
        let mut save = Self {
            galaxy: super::galaxy::DEFAULT_SEED,
            system: SystemId::default(),
            course: 0,
            place: Place::Space(Vec3::ZERO, 0.0),
            credits: 0,
            data: 0,
            inventory: Inventory::default(),
            upgrades: Upgrades::default(),
            vitals: Vitals::default(),
            visited: vec![],
            discoveries: 0,
            // version 1 saves predate weapons: they get the starter beam
            equipped: [Some(Weapon::starter()), None, None],
            locker: vec![],
            active: 0,
            destroyed: 0,
            missions: vec![],
            standing: 0,
            missions_done: 0,
            raid: 0.0,
            visits: 0,
        };
        for line in lines {
            let mut words = line.split_whitespace();
            let Some(key) = words.next() else { continue };
            let rest: Vec<&str> = words.collect();
            let f = |i: usize| rest.get(i).and_then(|w| w.parse::<f32>().ok());
            let u = |i: usize| rest.get(i).and_then(|w| w.parse::<i64>().ok());
            match key {
                "galaxy" => save.galaxy = rest.first()?.parse().ok()?,
                "system" => save.system = SystemId::new(u(0)? as i32, u(1)? as i32, u(2)? as i32),
                "course" => save.course = u(0)? as usize,
                "space" => save.place = Place::Space(Vec3::new(f(0)?, f(1)?, f(2)?), f(3)?),
                "flying" => {
                    save.place =
                        Place::Flying(u(0)? as usize, Vec3::new(f(1)?, f(2)?, f(3)?), f(4)?)
                }
                "landed" => {
                    save.place = Place::Landed {
                        planet: u(0)? as usize,
                        ship: Vec3::new(f(1)?, f(2)?, f(3)?),
                        yaw: f(4)?,
                    }
                }
                "credits" => save.credits = u(0)? as u32,
                "data" => save.data = u(0)? as u32,
                "inventory" => {
                    for (i, c) in save.inventory.counts.iter_mut().enumerate() {
                        *c = u(i).unwrap_or(0).clamp(0, 9999) as u16;
                    }
                }
                "upgrades" => {
                    for (i, l) in save.upgrades.levels.iter_mut().enumerate() {
                        *l = u(i).unwrap_or(0).clamp(0, 3) as u8;
                    }
                }
                "vitals" => {
                    let v = &mut save.vitals;
                    for (i, value) in [
                        &mut v.life,
                        &mut v.hazard,
                        &mut v.tool,
                        &mut v.shield,
                        &mut v.pulse,
                        &mut v.lift,
                        &mut v.suit,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        *value = f(i).unwrap_or(1.0).clamp(0.05, 1.0);
                    }
                }
                "visited" => {
                    save.visited = rest
                        .iter()
                        .filter_map(|w| {
                            let mut p = w.split(',').map(|n| n.parse::<i32>().ok());
                            Some(SystemId::new(p.next()??, p.next()??, p.next()??))
                        })
                        .collect()
                }
                "discoveries" => save.discoveries = u(0)? as u32,
                "weapons" => {
                    for (slot, word) in save.equipped.iter_mut().zip(rest.iter()) {
                        *slot = parse_weapon(word);
                    }
                }
                "locker" => {
                    save.locker = rest
                        .iter()
                        .filter_map(|w| parse_weapon(w))
                        .take(6)
                        .collect()
                }
                "active" => save.active = (u(0)? as usize).min(2),
                "destroyed" => save.destroyed = u(0)? as u32,
                "visits" => save.visits = u(0)? as u32,
                "standing" => save.standing = u(0)? as u32,
                "missions_done" => save.missions_done = u(0)? as u32,
                "raid" => save.raid = f(0)?,
                "mission" if save.missions.len() < super::missions::MAX_ACTIVE => {
                    let item = |i: usize| {
                        *Item::ALL
                            .get(u(i).unwrap_or(0) as usize)
                            .unwrap_or(&Item::Carbon)
                    };
                    save.missions.push(Mission {
                        kind: *Kind::ALL.get(u(0)? as usize)?,
                        origin: SystemId::new(u(1)? as i32, u(2)? as i32, u(3)? as i32),
                        target: SystemId::new(u(4)? as i32, u(5)? as i32, u(6)? as i32),
                        planet: u(7)? as u8,
                        item: item(8),
                        count: u(9)? as u16,
                        progress: u(10)? as u16,
                        reward: u(11)? as u32,
                        prize: match u(12)? {
                            1 => Prize::Item(item(13), u(14)? as u16),
                            2 => Prize::Weapon,
                            _ => Prize::None,
                        },
                        seed: u(15)? as u32,
                        tier: u(16)? as u8,
                    });
                }
                _ => {}
            }
        }
        Some(save)
    }
    pub fn load() -> Option<Self> {
        Self::from_text(&std::fs::read_to_string(path()?).ok()?)
    }
    pub fn store(&self) -> std::io::Result<()> {
        let Some(path) = path() else { return Ok(()) };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // write then rename so a power cut never leaves half a save
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, self.to_text())?;
        std::fs::rename(temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_round_trips_through_text() {
        let inventory = Inventory {
            counts: [3, 40, 0, 250, 7, 1, 0, 2, 4, 1, 0],
        };
        let save = SaveData {
            galaxy: 99,
            system: SystemId::new(-3, 12, 1),
            course: 2,
            place: Place::Landed {
                planet: 1,
                ship: Vec3::new(1024.5, 990.25, 41.0),
                yaw: 1.25,
            },
            credits: 1234,
            data: 17,
            inventory,
            upgrades: Upgrades {
                levels: [1, 0, 3, 2, 1, 0],
            },
            vitals: Vitals {
                life: 0.5,
                ..Default::default()
            },
            visited: vec![SystemId::new(1, 2, 3), SystemId::new(-4, 0, -1)],
            discoveries: 8,
            equipped: [
                Some(Weapon::starter()),
                None,
                Some(Weapon::roll(5, Rarity::Rare, 0.0)),
            ],
            locker: vec![Weapon::roll(9, Rarity::Common, 0.0)],
            active: 2,
            destroyed: 7,
            missions: {
                let galaxy = super::super::galaxy::Galaxy::default();
                let system = galaxy.system(galaxy.start());
                let mut m = Mission::new(Kind::Delivery, &system, &galaxy, 7, 2);
                m.progress = 3;
                m.prize = Prize::Item(Item::JumpCell, 1);
                vec![m, Mission::new(Kind::Courier, &system, &galaxy, 9, 0)]
            },
            standing: 12,
            missions_done: 4,
            raid: 4321.0,
            visits: 4,
        };
        let back = SaveData::from_text(&save.to_text()).unwrap();
        assert_eq!(back, save);
        assert!(SaveData::from_text("garbage").is_none());
    }
    #[test]
    fn version_one_saves_still_load_with_the_starter_beam() {
        let v1 = "cardputer-sky 1\ngalaxy 5\nsystem 1 2 3\ncourse 0\nspace 1.00 2.00 3.00 0.000\ncredits 99\ninventory 1 2 3 4 5 0 0 1\nupgrades 1 0 0 0\nvitals 1.000 1.000 1.000 1.000 1.000 1.000\n";
        let save = SaveData::from_text(v1).unwrap();
        assert_eq!(save.credits, 99);
        assert_eq!(save.inventory.counts[..8], [1, 2, 3, 4, 5, 0, 0, 1]);
        assert_eq!(save.equipped, [Some(Weapon::starter()), None, None]);
        assert_eq!(save.upgrades.levels, [1, 0, 0, 0, 0, 0]);
        assert_eq!(save.vitals.suit, 1.0);
    }
}
