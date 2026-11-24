//! Space station economy: buy and sell prices, survey data, and ship and suit upgrades.
use super::inventory::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upgrade {
    /// Faster pulse drive, larger pulse tank
    Drive,
    /// Longer warp range
    Warp,
    /// Faster mining beam
    Beam,
    /// Slower hazard drain
    Shield,
    /// Stronger suit shield
    Suit,
    /// Longer jetpack burn
    Jetpack,
}
impl Upgrade {
    pub const ALL: [Self; 6] = [
        Self::Drive,
        Self::Warp,
        Self::Beam,
        Self::Shield,
        Self::Suit,
        Self::Jetpack,
    ];
    pub const MAX: u8 = 3;
    pub fn name(self) -> &'static str {
        match self {
            Self::Drive => "PULSE DRIVE",
            Self::Warp => "WARP RANGE",
            Self::Beam => "MINING BEAM",
            Self::Shield => "HAZARD SHIELD",
            Self::Suit => "SUIT SHIELD",
            Self::Jetpack => "JETPACK",
        }
    }
    pub fn effect(self) -> &'static str {
        match self {
            Self::Drive => "+25% BOOST",
            Self::Warp => "+1.5 CELLS",
            Self::Beam => "+40% MINING",
            Self::Shield => "-18% DRAIN",
            Self::Suit => "+25% SHIELD",
            Self::Jetpack => "+30% BURN",
        }
    }
    /// Price of the next level (`level` is the current one).
    pub fn cost(self, level: u8) -> u32 {
        let base = [900, 1200, 700, 800, 850, 600][self as usize];
        base * (level as u32 + 1) * (level as u32 + 1)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Upgrades {
    pub levels: [u8; 6],
}
impl Upgrades {
    pub fn level(&self, upgrade: Upgrade) -> u8 {
        self.levels[upgrade as usize]
    }
    pub fn warp_range(&self) -> f32 {
        3.2 + 1.5 * self.level(Upgrade::Warp) as f32
    }
    pub fn beam_rate(&self) -> f32 {
        1.0 + 0.4 * self.level(Upgrade::Beam) as f32
    }
    pub fn drive(&self) -> f32 {
        1.0 + 0.25 * self.level(Upgrade::Drive) as f32
    }
    pub fn suit(&self) -> f32 {
        1.0 + 0.25 * self.level(Upgrade::Suit) as f32
    }
    pub fn jetpack(&self) -> f32 {
        1.0 + 0.3 * self.level(Upgrade::Jetpack) as f32
    }
}

/// Buy and sell price of an item at a station with the given market factor.
pub fn prices(item: Item, market: f32) -> (u32, u32) {
    let base = item.price() as f32 * market;
    (
        (base * 1.25).ceil() as u32,
        (base * 0.8).floor().max(1.0) as u32,
    )
}

/// Credits paid for each unit of survey data.
pub const DATA_PRICE: u32 = 15;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buying_costs_more_than_selling_and_upgrades_escalate() {
        for item in Item::ALL {
            for market in [0.8, 1.0, 1.2] {
                let (buy, sell) = prices(item, market);
                assert!(buy > sell && sell >= 1);
            }
        }
        for u in Upgrade::ALL {
            assert!(u.cost(2) > u.cost(1) && u.cost(1) > u.cost(0));
        }
        let mut up = Upgrades::default();
        up.levels[Upgrade::Warp as usize] = 2;
        assert!(up.warp_range() > 6.0);
    }
}
