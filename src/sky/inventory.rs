//! Elements, crafted items, the 16-slot inventory and crafting recipes.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    /// Recharges the survey tool; crafting
    Carbon,
    /// Hull repair; crafting
    Iron,
    /// Recharges hazard protection
    Sodium,
    /// Recharges life support
    Oxygen,
    /// Pulse drive and lift fuel; jump cells
    Tritium,
    /// One hyperspace jump to a nearby star
    JumpCell,
    /// Fully repairs the ship's shield
    HullPatch,
    /// Refills the lift thrusters for one take-off
    LiftCell,
    /// Salvaged from destroyed drones; crafting and trade
    DroneCore,
    /// Rerolls the perks of the equipped weapon
    RerollCell,
    /// Refills the suit shield
    ShieldCell,
}
impl Item {
    pub const ALL: [Self; 11] = [
        Self::Carbon,
        Self::Iron,
        Self::Sodium,
        Self::Oxygen,
        Self::Tritium,
        Self::JumpCell,
        Self::HullPatch,
        Self::LiftCell,
        Self::DroneCore,
        Self::RerollCell,
        Self::ShieldCell,
    ];
    pub const ELEMENTS: [Self; 5] = [
        Self::Carbon,
        Self::Iron,
        Self::Sodium,
        Self::Oxygen,
        Self::Tritium,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Carbon => "C",
            Self::Iron => "Fe",
            Self::Sodium => "Na",
            Self::Oxygen => "O2",
            Self::Tritium => "H3",
            Self::JumpCell => "JC",
            Self::HullPatch => "HP",
            Self::LiftCell => "LC",
            Self::DroneCore => "DC",
            Self::RerollCell => "RC",
            Self::ShieldCell => "SC",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Carbon => "CARBON",
            Self::Iron => "IRON",
            Self::Sodium => "SODIUM",
            Self::Oxygen => "OXYGEN",
            Self::Tritium => "TRITIUM",
            Self::JumpCell => "JUMP CELL",
            Self::HullPatch => "HULL PATCH",
            Self::LiftCell => "LIFT CELL",
            Self::DroneCore => "DRONE CORE",
            Self::RerollCell => "REROLL CELL",
            Self::ShieldCell => "SHIELD CELL",
        }
    }
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Carbon => [120, 220, 110],
            Self::Iron => [200, 200, 210],
            Self::Sodium => [255, 200, 70],
            Self::Oxygen => [255, 110, 100],
            Self::Tritium => [90, 230, 255],
            Self::JumpCell => [200, 140, 255],
            Self::HullPatch => [255, 160, 70],
            Self::LiftCell => [120, 170, 255],
            Self::DroneCore => [255, 90, 90],
            Self::RerollCell => [255, 205, 80],
            Self::ShieldCell => [140, 200, 255],
        }
    }
    pub fn stack(self) -> u16 {
        match self {
            _ if self.is_element() => 250,
            Self::DroneCore => 20,
            _ => 5,
        }
    }
    pub fn is_element(self) -> bool {
        self.index() < 5
    }
    /// Base station price in credits
    pub fn price(self) -> u32 {
        [8, 10, 18, 16, 24, 1400, 420, 300, 250, 900, 260][self.index()]
    }
    /// What USE does with this item, for the inventory hint line.
    pub fn use_hint(self) -> &'static str {
        match self {
            Self::Carbon => "CHARGE TOOL",
            Self::Iron => "REPAIR SHIELD",
            Self::Sodium => "CHARGE HAZARD",
            Self::Oxygen => "CHARGE LIFE",
            Self::Tritium => "FUEL DRIVE",
            Self::JumpCell => "LOAD FOR WARP",
            Self::HullPatch => "FULL REPAIR",
            Self::LiftCell => "FILL THRUSTERS",
            Self::DroneCore => "CRAFT OR SELL",
            Self::RerollCell => "REROLL WEAPON PERKS",
            Self::ShieldCell => "FILL SUIT SHIELD",
        }
    }
}

pub const SLOTS: usize = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    pub counts: [u16; 11],
}
impl Inventory {
    pub fn count(&self, item: Item) -> u16 {
        self.counts[item.index()]
    }
    fn slots_for(item: Item, count: u16) -> usize {
        (count as usize).div_ceil(item.stack() as usize)
    }
    pub fn slots_used(&self) -> usize {
        Item::ALL
            .iter()
            .map(|&i| Self::slots_for(i, self.count(i)))
            .sum()
    }
    /// Add up to `amount`; returns how many fit.
    pub fn add(&mut self, item: Item, amount: u16) -> u16 {
        let mut added = 0;
        while added < amount {
            let next = self.count(item) + 1;
            let extra_slots = Self::slots_for(item, next) - Self::slots_for(item, self.count(item));
            if self.slots_used() + extra_slots > SLOTS {
                break;
            }
            self.counts[item.index()] = next;
            added += 1;
        }
        added
    }
    pub fn take(&mut self, item: Item, amount: u16) -> bool {
        if self.count(item) < amount {
            return false;
        }
        self.counts[item.index()] -= amount;
        true
    }
    /// Slot contents in display order: each slot is (item, amount).
    pub fn slots(&self) -> [Option<(Item, u16)>; SLOTS] {
        let mut out = [None; SLOTS];
        let mut at = 0;
        for item in Item::ALL {
            let mut left = self.count(item);
            while left > 0 && at < SLOTS {
                let n = left.min(item.stack());
                out[at] = Some((item, n));
                left -= n;
                at += 1;
            }
        }
        out
    }
}

pub struct Recipe {
    pub output: Item,
    pub inputs: &'static [(Item, u16)],
}
pub const RECIPES: [Recipe; 5] = [
    Recipe {
        output: Item::JumpCell,
        inputs: &[(Item::Tritium, 40), (Item::Iron, 20)],
    },
    Recipe {
        output: Item::HullPatch,
        inputs: &[(Item::Iron, 30), (Item::Carbon, 10)],
    },
    Recipe {
        output: Item::LiftCell,
        inputs: &[(Item::Tritium, 15), (Item::Iron, 5)],
    },
    Recipe {
        output: Item::RerollCell,
        inputs: &[
            (Item::DroneCore, 1),
            (Item::Tritium, 30),
            (Item::Sodium, 20),
        ],
    },
    Recipe {
        output: Item::ShieldCell,
        inputs: &[(Item::Sodium, 15), (Item::Iron, 10)],
    },
];
impl Recipe {
    pub fn possible(&self, inventory: &Inventory) -> bool {
        self.inputs.iter().all(|&(i, n)| inventory.count(i) >= n)
    }
    /// Craft one; false if inputs are missing or there is no room.
    pub fn craft(&self, inventory: &mut Inventory) -> bool {
        if !self.possible(inventory) {
            return false;
        }
        let mut trial = *inventory;
        for &(i, n) in self.inputs {
            trial.take(i, n);
        }
        if trial.add(self.output, 1) != 1 {
            return false;
        }
        *inventory = trial;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_respects_sixteen_slots_and_stacks() {
        let mut inv = Inventory::default();
        assert_eq!(inv.add(Item::Carbon, 600), 600);
        assert_eq!(inv.slots_used(), 3);
        for item in [Item::Iron, Item::Sodium, Item::Oxygen, Item::Tritium] {
            assert_eq!(inv.add(item, 750), 750);
        }
        assert_eq!(inv.slots_used(), 15);
        assert_eq!(inv.add(Item::JumpCell, 9), 5);
        assert_eq!(inv.slots_used(), 16);
        assert_eq!(inv.add(Item::Carbon, 200), 150);
        assert_eq!(inv.slots().iter().flatten().count(), 16);
    }
    #[test]
    fn crafting_consumes_inputs_once() {
        let mut inv = Inventory::default();
        inv.add(Item::Tritium, 50);
        inv.add(Item::Iron, 25);
        assert!(RECIPES[0].craft(&mut inv));
        assert_eq!(inv.count(Item::JumpCell), 1);
        assert_eq!((inv.count(Item::Tritium), inv.count(Item::Iron)), (10, 5));
        assert!(!RECIPES[0].craft(&mut inv));
    }
}
