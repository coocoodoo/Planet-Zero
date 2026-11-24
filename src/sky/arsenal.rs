//! Hand weapons: seven archetypes, four rarities and sixteen perks. A weapon is just
//! `(archetype, seed, rarity)`; its perks and rolled stats are re-derived from the seed, so the
//! same weapon always comes back the same and saves stay tiny. The table follows the engine's
//! `gameplay::weapons::Spec` pattern: one row per archetype.
use super::rng::Rng;
use crate::audio::Sound;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Archetype {
    MiningBeam,
    PulseLaser,
    Scatter,
    Rail,
    Mortar,
    Arc,
    Cryo,
}

/// How a weapon delivers damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fire {
    /// Continuous beam (damage, heat and charge are per second)
    Beam,
    /// Visible projectiles (one or more per shot)
    Bolt,
    /// Instant piercing line
    Lance,
    /// Projectile that falls under gravity and bursts
    Lob,
    /// Instant arc that jumps between targets
    Chain,
}

pub struct Spec {
    pub name: &'static str,
    /// Short name for tight HUD spots
    pub short: &'static str,
    pub fire: Fire,
    /// Seconds between shots (beams: 0)
    pub interval: f32,
    pub damage: f32,
    /// Heat per shot (beams: per second); 1.0 overheats
    pub heat: f32,
    /// Battery per shot (beams: per second)
    pub charge: f32,
    pub range: f32,
    /// Projectile speed (bolts and lobs)
    pub speed: f32,
    pub pellets: u8,
    /// Spread between pellets, radians
    pub spread: f32,
    pub splash: f32,
    /// Extra targets an arc jumps to
    pub chain: u8,
    /// Mining power against props (1.0 = the old tool)
    pub mining: f32,
    /// Seconds a hit slows a drone
    pub slow: f32,
    pub color: [u8; 3],
    pub sound: Sound,
}

const SPECS: [Spec; 7] = [
    Spec {
        name: "MINING BEAM",
        short: "BEAM",
        fire: Fire::Beam,
        interval: 0.0,
        damage: 9.0,
        heat: 0.32,
        charge: 1.0 / 90.0,
        range: 32.0,
        speed: 0.0,
        pellets: 1,
        spread: 0.0,
        splash: 0.0,
        chain: 0,
        mining: 1.0,
        slow: 0.0,
        color: [255, 140, 80],
        sound: Sound::Mine,
    },
    Spec {
        name: "PULSE LASER",
        short: "PULSE",
        fire: Fire::Bolt,
        interval: 0.16,
        damage: 7.0,
        heat: 0.06,
        charge: 0.002,
        range: 110.0,
        speed: 170.0,
        pellets: 1,
        spread: 0.01,
        splash: 0.0,
        chain: 0,
        mining: 0.35,
        slow: 0.0,
        color: [120, 230, 255],
        sound: Sound::Laser,
    },
    Spec {
        name: "SCATTER BLASTER",
        short: "SCAT",
        fire: Fire::Bolt,
        interval: 0.55,
        damage: 5.0,
        heat: 0.16,
        charge: 0.004,
        range: 45.0,
        speed: 140.0,
        pellets: 5,
        spread: 0.07,
        splash: 0.0,
        chain: 0,
        mining: 0.3,
        slow: 0.0,
        color: [255, 200, 90],
        sound: Sound::Flak,
    },
    Spec {
        name: "RAIL LANCE",
        short: "RAIL",
        fire: Fire::Lance,
        interval: 1.1,
        damage: 34.0,
        heat: 0.3,
        charge: 0.008,
        range: 160.0,
        speed: 0.0,
        pellets: 1,
        spread: 0.0,
        splash: 0.0,
        chain: 0,
        mining: 0.5,
        slow: 0.0,
        color: [230, 140, 255],
        sound: Sound::Rail,
    },
    Spec {
        name: "PLASMA MORTAR",
        short: "MORT",
        fire: Fire::Lob,
        interval: 0.9,
        damage: 26.0,
        heat: 0.22,
        charge: 0.007,
        range: 90.0,
        speed: 55.0,
        pellets: 1,
        spread: 0.0,
        splash: 7.0,
        chain: 0,
        mining: 0.6,
        slow: 0.0,
        color: [150, 255, 120],
        sound: Sound::Plasma,
    },
    Spec {
        name: "ARC CASTER",
        short: "ARC",
        fire: Fire::Chain,
        interval: 0.45,
        damage: 11.0,
        heat: 0.14,
        charge: 0.005,
        range: 40.0,
        speed: 0.0,
        pellets: 1,
        spread: 0.0,
        splash: 0.0,
        chain: 2,
        mining: 0.4,
        slow: 0.0,
        color: [170, 200, 255],
        sound: Sound::Orb,
    },
    Spec {
        name: "CRYO BEAM",
        short: "CRYO",
        fire: Fire::Beam,
        interval: 0.0,
        damage: 14.0,
        heat: 0.26,
        charge: 1.0 / 70.0,
        range: 38.0,
        speed: 0.0,
        pellets: 1,
        spread: 0.0,
        splash: 0.0,
        chain: 0,
        mining: 0.5,
        slow: 1.2,
        color: [150, 235, 255],
        sound: Sound::Swarm,
    },
];

impl Archetype {
    pub const ALL: [Self; 7] = [
        Self::MiningBeam,
        Self::PulseLaser,
        Self::Scatter,
        Self::Rail,
        Self::Mortar,
        Self::Arc,
        Self::Cryo,
    ];
    pub fn spec(self) -> &'static Spec {
        &SPECS[self as usize]
    }
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(i: usize) -> Self {
        Self::ALL[i.min(6)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Exotic,
}
impl Rarity {
    pub const ALL: [Self; 4] = [Self::Common, Self::Uncommon, Self::Rare, Self::Exotic];
    pub fn perks(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        ["COMMON", "UNCOMMON", "RARE", "EXOTIC"][self as usize]
    }
    pub fn color(self) -> [u8; 3] {
        [
            [215, 222, 228],
            [120, 230, 120],
            [100, 220, 255],
            [255, 205, 80],
        ][self as usize]
    }
    pub fn from_index(i: usize) -> Self {
        Self::ALL[i.min(3)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Perk {
    Overclocked,
    HeatSink,
    LongBarrel,
    Splitter,
    Ricochet,
    Piercing,
    Volatile,
    Chain,
    Prospector,
    Efficient,
    CryoRounds,
    Siphon,
    CritLens,
    Homing,
    Salvager,
    Stabilized,
}
impl Perk {
    pub const ALL: [Self; 16] = [
        Self::Overclocked,
        Self::HeatSink,
        Self::LongBarrel,
        Self::Splitter,
        Self::Ricochet,
        Self::Piercing,
        Self::Volatile,
        Self::Chain,
        Self::Prospector,
        Self::Efficient,
        Self::CryoRounds,
        Self::Siphon,
        Self::CritLens,
        Self::Homing,
        Self::Salvager,
        Self::Stabilized,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Overclocked => "OVERCLOCKED",
            Self::HeatSink => "HEAT SINK",
            Self::LongBarrel => "LONG BARREL",
            Self::Splitter => "SPLITTER",
            Self::Ricochet => "RICOCHET",
            Self::Piercing => "PIERCING",
            Self::Volatile => "VOLATILE",
            Self::Chain => "CHAIN",
            Self::Prospector => "PROSPECTOR",
            Self::Efficient => "EFFICIENT",
            Self::CryoRounds => "CRYO ROUNDS",
            Self::Siphon => "SIPHON",
            Self::CritLens => "CRIT LENS",
            Self::Homing => "HOMING",
            Self::Salvager => "SALVAGER",
            Self::Stabilized => "STABILIZED",
        }
    }
    /// Word placed in front of the weapon's name
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Overclocked => "OVERCLOCKED",
            Self::HeatSink => "COOLED",
            Self::LongBarrel => "LONG",
            Self::Splitter => "SPLIT",
            Self::Ricochet => "BOUNCING",
            Self::Piercing => "PIERCING",
            Self::Volatile => "VOLATILE",
            Self::Chain => "FORKED",
            Self::Prospector => "PROSPECTOR",
            Self::Efficient => "EFFICIENT",
            Self::CryoRounds => "FROST",
            Self::Siphon => "SIPHON",
            Self::CritLens => "KEEN",
            Self::Homing => "SEEKING",
            Self::Salvager => "SALVAGER",
            Self::Stabilized => "STEADY",
        }
    }
    /// One letter for the HUD
    pub fn letter(self) -> char {
        "OHLSRPVCMEFYKGXT".as_bytes()[self as usize] as char
    }
    pub fn effect(self) -> &'static str {
        match self {
            Self::Overclocked => "+35% RATE +20% HEAT",
            Self::HeatSink => "-35% HEAT",
            Self::LongBarrel => "+40% RANGE",
            Self::Splitter => "+2 PROJECTILES",
            Self::Ricochet => "BOLTS BOUNCE ONCE",
            Self::Piercing => "SHOTS PASS THROUGH",
            Self::Volatile => "KILLS EXPLODE",
            Self::Chain => "+1 ARC TARGET",
            Self::Prospector => "+50% MINING YIELD",
            Self::Efficient => "-40% BATTERY USE",
            Self::CryoRounds => "HITS SLOW DRONES",
            Self::Siphon => "HITS RESTORE SHIELD",
            Self::CritLens => "15% CRIT X2.5",
            Self::Homing => "BOLTS SEEK TARGETS",
            Self::Salvager => "+50% DRONE LOOT",
            Self::Stabilized => "-50% SPREAD +AIM",
        }
    }
    /// Whether the perk does anything on this archetype.
    pub fn fits(self, archetype: Archetype) -> bool {
        let fire = archetype.spec().fire;
        match self {
            Self::Splitter | Self::Ricochet | Self::Homing => {
                matches!(fire, Fire::Bolt | Fire::Lob)
            }
            Self::Piercing => fire == Fire::Bolt,
            Self::Chain => fire == Fire::Chain,
            Self::Stabilized => archetype.spec().spread > 0.0 || fire == Fire::Beam,
            Self::CryoRounds => archetype != Archetype::Cryo,
            _ => true,
        }
    }
}

/// A weapon's final numbers after rolls and perks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub fire: Fire,
    pub interval: f32,
    pub damage: f32,
    pub heat: f32,
    pub charge: f32,
    pub range: f32,
    pub speed: f32,
    pub pellets: u8,
    pub spread: f32,
    pub splash: f32,
    pub chain: u8,
    pub mining: f32,
    /// Extra mining yield
    pub yield_bonus: f32,
    pub slow: f32,
    pub crit: f32,
    pub siphon: f32,
    pub homing: f32,
    pub bounces: u8,
    pub pierce: bool,
    pub volatile: bool,
    pub loot: f32,
    /// Aim assist cone, radians
    pub assist: f32,
    pub color: [u8; 3],
    pub sound: Sound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Weapon {
    pub archetype: Archetype,
    pub seed: u32,
    pub rarity: Rarity,
}

impl Weapon {
    /// The mining beam every pilot starts with.
    pub fn starter() -> Self {
        Self {
            archetype: Archetype::MiningBeam,
            seed: 1,
            rarity: Rarity::Common,
        }
    }
    /// Roll a new weapon. `luck` (0–1) shifts rarity up; `min` is a floor.
    pub fn roll(seed: u64, min: Rarity, luck: f32) -> Self {
        let mut rng = Rng::new(seed ^ 0x0a25_e4a1);
        let archetype = Archetype::ALL[1 + rng.below(6) as usize];
        let boost = (luck.clamp(0.0, 1.0) * 30.0) as u32;
        let rolled = Rarity::ALL[rng.weighted(&[55, 28 + boost / 2, 13 + boost / 2, 4 + boost])];
        Self {
            archetype,
            seed: rng.next_u32(),
            rarity: rolled.max(min),
        }
    }
    /// Same archetype and rarity, fresh perks and stat rolls.
    pub fn rerolled(self, seed: u64) -> Self {
        Self {
            seed: (Rng::new(seed ^ self.seed as u64).next_u32()) | 2,
            ..self
        }
    }
    /// Perk choices depend on rarity; stat rolls do not (so a perk's effect is measurable).
    fn perk_rng(self) -> Rng {
        Rng::new(((self.archetype as u64) << 40) ^ ((self.rarity as u64) << 36) ^ self.seed as u64)
    }
    fn stat_rng(self) -> Rng {
        Rng::new(((self.archetype as u64) << 40) ^ 0x57a7 ^ self.seed as u64)
    }
    pub fn perks(self) -> [Option<Perk>; 3] {
        let mut rng = self.perk_rng();
        let mut out = [None; 3];
        let mut n = 0;
        let mut guard = 0;
        while n < self.rarity.perks() && guard < 64 {
            guard += 1;
            let perk = Perk::ALL[rng.below(Perk::ALL.len() as u32) as usize];
            if perk.fits(self.archetype) && !out.contains(&Some(perk)) {
                out[n] = Some(perk);
                n += 1;
            }
        }
        out
    }
    pub fn has(self, perk: Perk) -> bool {
        self.perks().contains(&Some(perk))
    }
    /// MK1–MK3: small extra damage roll
    pub fn mark(self) -> u8 {
        1 + (self.stat_rng().next_u32() >> 7) as u8 % 3
    }
    pub fn name(self) -> String {
        let spec = self.archetype.spec();
        match self.perks()[0] {
            Some(perk) => format!("{} {} MK{}", perk.prefix(), spec.name, self.mark()),
            None => format!("{} MK{}", spec.name, self.mark()),
        }
    }
    pub fn stats(self) -> Stats {
        let spec = self.archetype.spec();
        let mut rng = self.stat_rng();
        rng.next_u32();
        let roll = |rng: &mut Rng| rng.range(0.85, 1.15);
        let quality = 1.0 + 0.08 * self.rarity as u8 as f32 + 0.05 * (self.mark() - 1) as f32;
        let mut s = Stats {
            fire: spec.fire,
            interval: spec.interval / roll(&mut rng).sqrt(),
            damage: spec.damage * roll(&mut rng) * quality,
            heat: spec.heat * roll(&mut rng),
            charge: spec.charge,
            range: spec.range * roll(&mut rng),
            speed: spec.speed,
            pellets: spec.pellets,
            spread: spec.spread,
            splash: spec.splash,
            chain: spec.chain,
            mining: spec.mining,
            yield_bonus: 0.0,
            slow: spec.slow,
            crit: 0.0,
            siphon: 0.0,
            homing: 0.0,
            bounces: 0,
            pierce: false,
            volatile: false,
            loot: 1.0,
            assist: 7.0_f32.to_radians(),
            color: spec.color,
            sound: spec.sound,
        };
        for perk in self.perks().into_iter().flatten() {
            match perk {
                Perk::Overclocked => {
                    s.interval /= 1.35;
                    s.heat *= 1.2;
                    if s.fire == Fire::Beam {
                        s.damage *= 1.35;
                    }
                }
                Perk::HeatSink => s.heat *= 0.65,
                Perk::LongBarrel => {
                    s.range *= 1.4;
                    s.speed *= 1.25;
                }
                Perk::Splitter => {
                    s.pellets += 2;
                    s.spread = s.spread.max(0.04);
                }
                Perk::Ricochet => s.bounces = 1,
                Perk::Piercing => s.pierce = true,
                Perk::Volatile => s.volatile = true,
                Perk::Chain => s.chain += 1,
                Perk::Prospector => s.yield_bonus += 0.5,
                Perk::Efficient => s.charge *= 0.6,
                Perk::CryoRounds => s.slow = s.slow.max(2.0),
                Perk::Siphon => s.siphon = 0.02,
                Perk::CritLens => s.crit = 0.15,
                Perk::Homing => s.homing = 3.0,
                Perk::Salvager => s.loot = 1.5,
                Perk::Stabilized => {
                    s.spread *= 0.5;
                    s.assist = 10.0_f32.to_radians();
                }
            }
        }
        s
    }
    /// Station price in credits
    pub fn value(self) -> u32 {
        let base = [150, 600, 650, 900, 850, 800, 750][self.archetype as usize];
        base * [1, 2, 4, 8][self.rarity as usize]
    }
    /// Iron and carbon recovered by scrapping
    pub fn scrap(self) -> (u16, u16) {
        let n = 1 + self.rarity as u16;
        (15 * n, 8 * n)
    }
    /// Damage per second against a single target, for the comparison card
    pub fn dps(self) -> f32 {
        let s = self.stats();
        match s.fire {
            Fire::Beam => s.damage,
            _ => s.damage * s.pellets as f32 / s.interval.max(0.05),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weapons_are_deterministic_and_named_from_their_perks() {
        for seed in 0..200 {
            let w = Weapon::roll(seed, Rarity::Common, 0.0);
            assert_eq!(w, Weapon::roll(seed, Rarity::Common, 0.0));
            assert_eq!(w.stats(), w.stats());
            assert_ne!(
                w.archetype,
                Archetype::MiningBeam,
                "the beam is the starter only"
            );
            let perks: Vec<_> = w.perks().into_iter().flatten().collect();
            assert_eq!(perks.len(), w.rarity.perks());
            for p in &perks {
                assert!(p.fits(w.archetype));
            }
            let set: std::collections::HashSet<_> = perks.iter().collect();
            assert_eq!(set.len(), perks.len(), "perks are distinct");
            if let Some(first) = perks.first() {
                assert!(w.name().starts_with(first.prefix()));
            }
            assert!(w.name().len() <= 34, "{}", w.name());
        }
        assert_eq!(Weapon::starter().perks(), [None; 3]);
    }
    #[test]
    fn rarity_floor_and_luck_shift_the_distribution() {
        let count = |min, luck| {
            (0..2000)
                .filter(|&s| Weapon::roll(s, min, luck).rarity >= Rarity::Rare)
                .count()
        };
        let plain = count(Rarity::Common, 0.0);
        let lucky = count(Rarity::Common, 1.0);
        assert!((200..500).contains(&plain), "{plain}");
        assert!(lucky > plain + 200);
        assert_eq!(count(Rarity::Rare, 0.0), 2000);
    }
    #[test]
    fn each_perk_changes_what_it_says() {
        // the same seed at COMMON has identical stat rolls and no perks
        let pair = |archetype, perk: Perk| {
            let w = (0..5000)
                .map(|seed| Weapon {
                    archetype,
                    seed,
                    rarity: Rarity::Uncommon,
                })
                .find(|w| w.perks()[0] == Some(perk))
                .unwrap_or_else(|| panic!("{perk:?} on {archetype:?}"));
            let base = Weapon {
                rarity: Rarity::Common,
                ..w
            };
            (w.stats(), base.stats())
        };
        for archetype in Archetype::ALL {
            for perk in Perk::ALL.into_iter().filter(|p| p.fits(archetype)) {
                let (s, b) = pair(archetype, perk);
                let changed = match perk {
                    Perk::Overclocked => {
                        s.heat > b.heat && (s.interval < b.interval || s.fire == Fire::Beam)
                    }
                    Perk::HeatSink => s.heat < b.heat,
                    Perk::LongBarrel => s.range > b.range,
                    Perk::Splitter => s.pellets == b.pellets + 2,
                    Perk::Ricochet => s.bounces == 1,
                    Perk::Piercing => s.pierce,
                    Perk::Volatile => s.volatile,
                    Perk::Chain => s.chain == b.chain + 1,
                    Perk::Prospector => s.yield_bonus > 0.0,
                    Perk::Efficient => s.charge < b.charge,
                    Perk::CryoRounds => s.slow >= 2.0,
                    Perk::Siphon => s.siphon > 0.0,
                    Perk::CritLens => s.crit > 0.0,
                    Perk::Homing => s.homing > 0.0,
                    Perk::Salvager => s.loot > 1.0,
                    Perk::Stabilized => s.assist > b.assist,
                };
                assert!(changed, "{perk:?} on {archetype:?}");
            }
        }
    }
    #[test]
    fn rerolling_keeps_the_kind_and_changes_the_roll() {
        let w = Weapon::roll(42, Rarity::Rare, 0.0);
        let r = w.rerolled(9);
        assert_eq!((r.archetype, r.rarity), (w.archetype, w.rarity));
        assert_ne!(r.seed, w.seed);
        assert!(Weapon::roll(1, Rarity::Exotic, 0.0).value() > Weapon::starter().value());
    }
}
