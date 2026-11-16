use glam::{Vec3, vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Vehicle {
    #[default]
    Falcon,
    Dart,
    Atlas,
    Wasp,
}

impl Vehicle {
    pub const ALL: [Self; 4] = [Self::Falcon, Self::Dart, Self::Atlas, Self::Wasp];

    pub fn name(self) -> &'static str {
        match self {
            Self::Falcon => "FALCON",
            Self::Dart => "DART",
            Self::Atlas => "ATLAS",
            Self::Wasp => "WASP",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Falcon => "BALANCED / TWIN ENGINE",
            Self::Dart => "FAST / LIGHT INTERCEPTOR",
            Self::Atlas => "ARMORED / HEAVY BOMBER",
            Self::Wasp => "AGILE / TWIN ROTOR",
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            Self::Falcon => 92.0,
            Self::Dart => 116.0,
            Self::Atlas => 78.0,
            Self::Wasp => 68.0,
        }
    }

    pub fn turn(self) -> f32 {
        match self {
            Self::Falcon => 1.25,
            Self::Dart => 1.5,
            Self::Atlas => 0.95,
            Self::Wasp => 1.8,
        }
    }

    pub fn shield(self) -> f32 {
        match self {
            Self::Falcon => 100.0,
            Self::Dart => 75.0,
            Self::Atlas => 150.0,
            Self::Wasp => 90.0,
        }
    }

    pub fn power(self) -> f32 {
        match self {
            Self::Atlas => 1.25,
            Self::Wasp => 0.9,
            _ => 1.0,
        }
    }

    pub fn shape(self) -> Vec3 {
        match self {
            Self::Falcon => Vec3::ONE,
            Self::Dart => vec3(0.65, 1.18, 0.8),
            Self::Atlas => vec3(1.35, 0.9, 1.2),
            Self::Wasp => vec3(0.82, 0.65, 1.4),
        }
    }

    pub fn tint(self) -> [u8; 3] {
        match self {
            Self::Falcon => [210, 221, 232],
            Self::Dart => [80, 205, 245],
            Self::Atlas => [231, 176, 78],
            Self::Wasp => [131, 222, 116],
        }
    }
}
