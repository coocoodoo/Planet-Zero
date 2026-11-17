use glam::{Vec3, vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Vehicle {
    #[default]
    Falcon,
    Dart,
    Atlas,
    Wasp,
    Paper,
}

impl Vehicle {
    pub const ALL: [Self; 5] = [
        Self::Falcon,
        Self::Dart,
        Self::Atlas,
        Self::Wasp,
        Self::Paper,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Falcon => "FALCON",
            Self::Dart => "DART",
            Self::Atlas => "ATLAS",
            Self::Wasp => "WASP",
            Self::Paper => "PAPER",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Falcon => "BALANCED / TWIN ENGINE",
            Self::Dart => "FAST / LIGHT INTERCEPTOR",
            Self::Atlas => "ARMORED / HEAVY BOMBER",
            Self::Wasp => "AGILE / TWIN ROTOR",
            Self::Paper => "LIGHT / FOLDED PAPER GLIDER",
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            Self::Falcon => 92.0,
            Self::Dart => 116.0,
            Self::Atlas => 78.0,
            Self::Wasp => 68.0,
            Self::Paper => 62.0,
        }
    }

    pub fn turn(self) -> f32 {
        match self {
            Self::Falcon => 1.25,
            Self::Dart => 1.5,
            Self::Atlas => 0.95,
            Self::Wasp => 1.8,
            Self::Paper => 1.65,
        }
    }

    pub fn shield(self) -> f32 {
        match self {
            Self::Falcon => 100.0,
            Self::Dart => 75.0,
            Self::Atlas => 150.0,
            Self::Wasp => 90.0,
            Self::Paper => 65.0,
        }
    }

    pub fn power(self) -> f32 {
        match self {
            Self::Atlas => 1.25,
            Self::Wasp => 0.9,
            Self::Paper => 0.8,
            _ => 1.0,
        }
    }

    pub fn shape(self) -> Vec3 {
        match self {
            Self::Falcon => Vec3::ONE,
            Self::Dart => vec3(0.65, 1.18, 0.8),
            Self::Atlas => vec3(1.35, 0.9, 1.2),
            Self::Wasp => vec3(0.82, 0.65, 1.4),
            Self::Paper => vec3(1.0, 1.0, 1.0),
        }
    }

    pub fn tint(self) -> [u8; 3] {
        match self {
            Self::Falcon => [210, 221, 232],
            Self::Dart => [80, 205, 245],
            Self::Atlas => [231, 176, 78],
            Self::Wasp => [131, 222, 116],
            Self::Paper => [244, 241, 220],
        }
    }
}
