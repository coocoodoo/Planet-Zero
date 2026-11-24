//! Save file: one `key value…` line per field, so it survives version changes and can be read by
//! hand. Stored next to Flight Club's records (`~/.local/share/flight-club/sky-save-v1.txt`), or
//! at `$FLIGHT_CLUB_SKY_SAVE`.
use super::arsenal::{Archetype, Rarity, Weapon};
use super::contracts::{Contract, Goal};
use super::galaxy::SystemId;
use super::inventory::Inventory;
use super::inventory::Item;
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
    pub contract: Option<Contract>,
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
        let mut s = String::from("cardputer-sky 2\n");
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
        if let Some(c) = self.contract {
            let (kind, a, b) = match c.goal {
                Goal::Drones(n) => (0, n as u32, 0),
                Goal::Flora { planet, count } => (1, planet as u32, count as u32),
                Goal::Deliver { item, amount } => (2, item.index() as u32, amount as u32),
            };
            s += &format!(
                "contract {} {} {} {kind} {a} {b} {} {} {} {}\n",
                c.system.x,
                c.system.y,
                c.system.z,
                c.reward,
                u8::from(c.weapon),
                c.seed,
                c.progress
            );
        }
        s
    }
    pub fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let version = lines.next()?.trim();
        if version != "cardputer-sky 1" && version != "cardputer-sky 2" {
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
            contract: None,
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
                "contract" => {
                    let goal = match u(3)? {
                        0 => Goal::Drones(u(4)? as u16),
                        1 => Goal::Flora {
                            planet: u(4)? as u8,
                            count: u(5)? as u16,
                        },
                        _ => Goal::Deliver {
                            item: *Item::ALL.get(u(4)? as usize)?,
                            amount: u(5)? as u16,
                        },
                    };
                    save.contract = Some(Contract {
                        system: SystemId::new(u(0)? as i32, u(1)? as i32, u(2)? as i32),
                        goal,
                        reward: u(6)? as u32,
                        weapon: u(7)? != 0,
                        seed: u(8)? as u32,
                        progress: u(9)? as u16,
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
            contract: Some(Contract {
                system: SystemId::new(-3, 12, 1),
                goal: Goal::Deliver {
                    item: Item::Sodium,
                    amount: 40,
                },
                reward: 1440,
                weapon: true,
                seed: 77,
                progress: 3,
            }),
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
