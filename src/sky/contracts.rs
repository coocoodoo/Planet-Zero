//! Station contracts: one offer per station visit. Destroy drones, scan flora on a named planet,
//! or deliver elements, for credits and sometimes a weapon.
use super::galaxy::{StarSystem, SystemId};
use super::inventory::Item;
use super::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    Drones(u16),
    /// Scan this many new flora on planet `planet` of the contract's system
    Flora {
        planet: u8,
        count: u16,
    },
    Deliver {
        item: Item,
        amount: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contract {
    pub system: SystemId,
    pub goal: Goal,
    pub reward: u32,
    /// Also pays a weapon (rolled from the contract seed)
    pub weapon: bool,
    pub seed: u32,
    pub progress: u16,
}

impl Contract {
    /// The offer at `system`'s station on visit number `visit`.
    pub fn offer(system: &StarSystem, visit: u32) -> Self {
        let mut rng = Rng::new(system.seed ^ (visit as u64).wrapping_mul(0x51ed));
        let goal = match rng.below(3) {
            0 => Goal::Drones(2 + rng.below(4) as u16),
            1 => Goal::Flora {
                planet: rng.below(system.atmospheric() as u32) as u8,
                count: 4 + rng.below(6) as u16,
            },
            _ => Goal::Deliver {
                item: Item::ELEMENTS[rng.below(5) as usize],
                amount: 30 + 10 * rng.below(5) as u16,
            },
        };
        let reward = match goal {
            Goal::Drones(n) => 220 * n as u32,
            Goal::Flora { count, .. } => 110 * count as u32,
            Goal::Deliver { item, amount } => item.price() * amount as u32 * 2,
        };
        Self {
            system: system.id,
            goal,
            reward,
            weapon: rng.below(3) == 0,
            seed: rng.next_u32(),
            progress: 0,
        }
    }
    /// Target count for the progress bar
    pub fn target(&self) -> u16 {
        match self.goal {
            Goal::Drones(n) => n,
            Goal::Flora { count, .. } => count,
            Goal::Deliver { amount, .. } => amount,
        }
    }
    pub fn text(&self, system: &StarSystem) -> String {
        match self.goal {
            Goal::Drones(n) => format!("DESTROY {n} DRONES"),
            Goal::Flora { planet, count } => {
                let name = system
                    .planets
                    .get(planet as usize)
                    .map_or("A PLANET", |p| p.name.as_str());
                format!("SCAN {count} FLORA ON {name}")
            }
            Goal::Deliver { item, amount } => format!("DELIVER {amount} {}", item.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::galaxy::Galaxy;
    #[test]
    fn offers_vary_by_visit_and_pay_for_the_work() {
        let galaxy = Galaxy::default();
        let system = galaxy.system(galaxy.start());
        let mut kinds = std::collections::HashSet::new();
        for visit in 0..40 {
            let c = Contract::offer(&system, visit);
            assert_eq!(c, Contract::offer(&system, visit));
            assert!(c.reward >= 200 && c.target() > 0);
            if let Goal::Flora { planet, .. } = c.goal {
                assert!((planet as usize) < system.atmospheric());
            }
            assert!(c.text(&system).len() <= 40, "{}", c.text(&system));
            kinds.insert(std::mem::discriminant(&c.goal));
        }
        assert_eq!(kinds.len(), 3);
    }
}
