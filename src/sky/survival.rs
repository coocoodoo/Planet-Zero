//! Survival systems: suit (life support, hazard protection, jetpack), survey tool (charge and
//! heat) and ship (shield, pulse drive, lift thrusters). All values run from 0 to 1.
use super::biome::Hazard;
use super::inventory::Item;
use super::station::Upgrades;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vitals {
    pub life: f32,
    pub hazard: f32,
    pub jetpack: f32,
    pub tool: f32,
    pub heat: f32,
    /// Seconds left before an overheated tool works again
    pub overheat: f32,
    pub shield: f32,
    pub pulse: f32,
    pub lift: f32,
    /// Seconds since the last low-protection warning beep
    pub warned: f32,
    /// Suit shield against drone fire
    pub suit: f32,
    /// Seconds before the suit shield starts recharging
    pub suit_delay: f32,
}
impl Default for Vitals {
    fn default() -> Self {
        Self {
            life: 1.0,
            hazard: 1.0,
            jetpack: 1.0,
            tool: 1.0,
            heat: 0.0,
            overheat: 0.0,
            shield: 1.0,
            pulse: 1.0,
            lift: 1.0,
            warned: 0.0,
            suit: 1.0,
            suit_delay: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Protection or life support below 25 %: beep
    Warning,
    /// Life support ran out: the suit collapses
    Collapse,
    /// Tool overheated
    Overheat,
}

/// Seconds a full charge lasts.
pub const LIFE_SECONDS: f32 = 420.0;
pub const HAZARD_SECONDS: f32 = 150.0;
pub const JETPACK_SECONDS: f32 = 2.6;
pub const TOOL_SECONDS: f32 = 90.0;
pub const PULSE_SECONDS: f32 = 14.0;

impl Vitals {
    /// One on-foot step. `exposure` is the biome hazard strength (0 when safe or sheltered).
    pub fn step_suit(
        &mut self,
        dt: f32,
        hazard: (Hazard, f32),
        upgrades: &Upgrades,
        jetting: bool,
    ) -> Option<Event> {
        let mut event = None;
        let exposure = if hazard.0 == Hazard::Safe {
            0.0
        } else {
            hazard.1
        };
        let shielding = 1.0 - 0.18 * upgrades.level(super::station::Upgrade::Shield) as f32;
        self.hazard = (self.hazard - exposure * shielding * dt / HAZARD_SECONDS).max(0.0);
        let drain = if exposure > 0.0 && self.hazard <= 0.0 {
            6.0
        } else {
            1.0
        };
        self.life = (self.life - drain * dt / LIFE_SECONDS).max(0.0);
        self.suit_delay = (self.suit_delay - dt).max(0.0);
        if self.suit_delay <= 0.0 {
            self.suit = (self.suit + dt * 0.25).min(1.0);
        }
        if jetting {
            let tank = JETPACK_SECONDS * upgrades.jetpack();
            self.jetpack = (self.jetpack - dt / tank).max(0.0);
        } else {
            self.jetpack = (self.jetpack + dt * 0.45).min(1.0);
        }
        self.warned += dt;
        let low = self.life < 0.25 || (exposure > 0.0 && self.hazard < 0.25);
        if low && self.warned > 2.5 {
            self.warned = 0.0;
            event = Some(Event::Warning);
        }
        if self.life <= 0.0 {
            event = Some(Event::Collapse);
        }
        event
    }
    /// Take a hit. The suit shield absorbs it; what gets through drains life support.
    /// `capacity` scales the shield (suit upgrade). Returns true when the shield breaks.
    pub fn damage(&mut self, amount: f32, capacity: f32) -> bool {
        self.suit_delay = 3.0;
        let hit = amount / (100.0 * capacity.max(0.1));
        let had = self.suit > 0.0;
        if self.suit >= hit {
            self.suit -= hit;
            return false;
        }
        let overflow = hit - self.suit;
        self.suit = 0.0;
        self.life = (self.life - overflow * 0.4).max(0.0);
        had
    }
    /// Inside the ship the suit recharges and the drive recovers.
    pub fn step_ship(&mut self, dt: f32, boosting: bool) {
        self.life = (self.life + dt / 60.0).min(1.0);
        self.hazard = (self.hazard + dt / 40.0).min(1.0);
        self.jetpack = 1.0;
        self.cool(dt);
        if boosting {
            self.pulse = (self.pulse - dt / PULSE_SECONDS).max(0.0);
        } else {
            self.pulse = (self.pulse + dt / 30.0).min(1.0);
        }
    }
    pub fn cool(&mut self, dt: f32) {
        self.overheat = (self.overheat - dt).max(0.0);
        self.heat = (self.heat - dt * 0.35).max(0.0);
    }
    /// Whether the tool can fire this frame.
    pub fn tool_ready(&self) -> bool {
        self.overheat <= 0.0 && self.tool > 0.0
    }
    /// Fire the tool for `dt`; returns Overheat when it locks out.
    pub fn fire_tool(&mut self, dt: f32) -> Option<Event> {
        self.tool = (self.tool - dt / TOOL_SECONDS).max(0.0);
        self.heat += dt * 0.32;
        if self.heat >= 1.0 {
            self.heat = 1.0;
            self.overheat = 2.5;
            return Some(Event::Overheat);
        }
        None
    }
    /// Use one item to recharge the matching system. Returns units consumed.
    pub fn recharge(&mut self, item: Item, available: u16) -> u16 {
        let fill = |value: &mut f32, per_unit: f32| -> u16 {
            let missing = (1.0 - *value).max(0.0);
            let units = ((missing / per_unit).ceil() as u16).min(available);
            *value = (*value + units as f32 * per_unit).min(1.0);
            units
        };
        match item {
            Item::Oxygen => fill(&mut self.life, 0.02),
            Item::Sodium => fill(&mut self.hazard, 0.025),
            Item::Carbon => fill(&mut self.tool, 0.04),
            Item::Iron => fill(&mut self.shield, 0.02),
            // tritium tops up the lift thrusters first, then the pulse drive
            Item::Tritium if self.lift < 1.0 => fill(&mut self.lift, 0.05),
            Item::Tritium => fill(&mut self.pulse, 0.05),
            Item::HullPatch => fill(&mut self.shield, 1.0),
            Item::LiftCell => fill(&mut self.lift, 1.0),
            Item::ShieldCell => fill(&mut self.suit, 1.0),
            Item::JumpCell | Item::DroneCore | Item::RerollCell => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hazard_drains_protection_then_life_quickly() {
        let mut v = Vitals::default();
        let up = Upgrades::default();
        let mut collapsed = false;
        let mut seconds = 0.0;
        while !collapsed && seconds < 1000.0 {
            collapsed = v.step_suit(0.1, (Hazard::Heat, 1.0), &up, false) == Some(Event::Collapse);
            seconds += 0.1;
        }
        assert!(collapsed);
        assert!((150.0..260.0).contains(&seconds), "{seconds}");
        let mut safe = Vitals::default();
        for _ in 0..3000 {
            safe.step_suit(0.1, (Hazard::Safe, 0.0), &up, false);
        }
        assert_eq!(safe.hazard, 1.0);
        assert!(safe.life > 0.2);
    }
    #[test]
    fn recharge_uses_only_what_is_missing() {
        let mut v = Vitals {
            life: 0.5,
            ..Default::default()
        };
        assert_eq!(v.recharge(Item::Oxygen, 100), 25);
        assert_eq!(v.life, 1.0);
        assert_eq!(v.recharge(Item::Oxygen, 100), 0);
        v.tool = 0.0;
        assert_eq!(v.recharge(Item::Carbon, 10), 10);
        assert!((v.tool - 0.4).abs() < 0.001);
    }
    #[test]
    fn suit_shield_absorbs_hits_then_life_takes_the_rest() {
        let mut v = Vitals::default();
        assert!(!v.damage(50.0, 1.0));
        assert!((v.suit - 0.5).abs() < 0.001);
        assert_eq!(v.life, 1.0);
        assert!(v.damage(80.0, 1.0), "the shield breaks");
        assert_eq!(v.suit, 0.0);
        assert!(v.life < 1.0);
        let up = Upgrades::default();
        v.step_suit(1.0, (Hazard::Safe, 0.0), &up, false);
        assert_eq!(v.suit, 0.0, "no recharge right after a hit");
        for _ in 0..80 {
            v.step_suit(0.1, (Hazard::Safe, 0.0), &up, false);
        }
        assert!(v.suit > 0.9);
    }
    #[test]
    fn tool_overheats_and_cools() {
        let mut v = Vitals::default();
        let mut event = None;
        for _ in 0..40 {
            event = event.or(v.fire_tool(0.1));
        }
        assert_eq!(event, Some(Event::Overheat));
        assert!(!v.tool_ready());
        v.cool(3.0);
        assert!(v.tool_ready());
    }
}
