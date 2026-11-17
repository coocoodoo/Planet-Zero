use super::Weapon;
use glam::Vec3;
#[derive(Clone)]
pub struct Gate {
    pub position: Vec3,
    pub normal: Vec3,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Coin,
    Upgrade(Weapon),
    Energy,
    Shield,
    Bomb,
    Support,
    Time,
}

#[derive(Clone)]
pub struct Pickup {
    pub position: Vec3,
    pub kind: PickupKind,
    pub life: f32,
    pub collected: bool,
}

#[derive(Clone)]
pub struct Enemy {
    pub id: u32,
    pub position: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub max_health: f32,
    pub kind: u8,
    pub phase: f32,
    pub cooldown: f32,
    pub hit: f32,
    pub aim_point: Vec3,
}

impl Enemy {
    pub fn is_boss(&self) -> bool {
        (3..=5).contains(&self.kind)
    }
    pub fn is_ground(&self) -> bool {
        self.kind == 6
    }

    pub fn boss_name(&self) -> &'static str {
        match self.kind {
            3 => "DREADNOUGHT",
            4 => "SKY SERPENT",
            _ => "TEMPEST",
        }
    }

    pub fn radius(&self) -> f32 {
        match self.kind {
            3..=5 => 24.0,
            6 => 14.0,
            7 => 8.0,
            8 => 12.0,
            _ => 8.0 + self.kind as f32 * 2.0,
        }
    }
}

#[derive(Clone)]
pub struct Shot {
    pub position: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub damage: f32,
    pub weapon: Weapon,
    pub hostile: bool,
    pub grazed: bool,
    pub target: Option<u32>,
    pub radius: f32,
}

#[derive(Clone)]
pub struct Effect {
    pub position: Vec3,
    pub life: f32,
    pub duration: f32,
    pub radius: f32,
    pub color: [u8; 3],
}

#[derive(Clone)]
pub struct Convoy {
    pub position: Vec3,
    pub previous: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub cooldown: f32,
}
