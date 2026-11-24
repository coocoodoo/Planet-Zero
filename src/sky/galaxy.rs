//! The galaxy: a grid of star systems indexed by `(gx, gy, gz)`. Nothing is stored; a system is
//! regenerated from its coordinates whenever it is needed.
use super::biome::Biome;
use super::names;
use super::rng::{Rng, hash3};
use crate::render::{Grade, rgb};
use glam::{Vec3, vec3};

/// Galaxy half-size in cells (x and y), and half-thickness (z).
pub const HALF: i32 = 48;
pub const HALF_Z: i32 = 6;
/// Light-years per grid cell, for display.
pub const LY_PER_CELL: f32 = 3.6;
pub const DEFAULT_SEED: u64 = 0xC0DE_5C1E_0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct SystemId {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}
impl SystemId {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: x as i16,
            y: y as i16,
            z: z as i16,
        }
    }
    pub fn cells(self, other: Self) -> f32 {
        let d = vec3(
            (self.x - other.x) as f32,
            (self.y - other.y) as f32,
            (self.z - other.z) as f32,
        );
        d.length()
    }
    pub fn light_years(self, other: Self) -> f32 {
        self.cells(other) * LY_PER_CELL
    }
    pub fn vec(self) -> Vec3 {
        vec3(self.x as f32, self.y as f32, self.z as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarClass {
    Yellow,
    Red,
    Green,
    Blue,
}
impl StarClass {
    pub fn name(self) -> &'static str {
        match self {
            Self::Yellow => "YELLOW",
            Self::Red => "RED",
            Self::Green => "GREEN",
            Self::Blue => "BLUE",
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Self::Yellow => rgb(255, 220, 120),
            Self::Red => rgb(255, 110, 80),
            Self::Green => rgb(130, 255, 140),
            Self::Blue => rgb(130, 180, 255),
        }
    }
    pub fn rgb(self) -> [u8; 3] {
        let [r, g, b, _] = self.color().to_le_bytes();
        [r, g, b]
    }
    /// Ambient light tint of the star, applied in space and on its planets.
    pub fn grade(self) -> Grade {
        let (matrix, offset) = match self {
            Self::Yellow => (Grade::IDENTITY.matrix, [0.0; 3]),
            Self::Red => (
                [[1.06, 0.02, 0.0], [0.0, 0.9, 0.0], [0.0, 0.0, 0.8]],
                [6.0, 0.0, 0.0],
            ),
            Self::Green => (
                [[0.9, 0.04, 0.0], [0.02, 1.06, 0.02], [0.0, 0.04, 0.9]],
                [0.0, 6.0, 0.0],
            ),
            Self::Blue => (
                [[0.9, 0.0, 0.04], [0.0, 0.96, 0.04], [0.0, 0.02, 1.1]],
                [0.0, 0.0, 8.0],
            ),
        };
        Grade { matrix, offset }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Planet {
    pub name: String,
    /// Terrain seed for the engine's world generator (part of the surface map key)
    pub seed: u32,
    pub biome: Biome,
    pub radius_km: u32,
    /// Position on the system map: orbital angle (radians) and distance (AU)
    pub angle: f32,
    pub distance: f32,
}
impl Planet {
    pub fn kind(&self) -> crate::world::MapKind {
        self.biome.map_kind(self.seed)
    }
    pub fn key(&self) -> crate::world::network::MapKey {
        crate::world::network::MapKey {
            kind: self.kind(),
            seed: self.seed,
        }
    }
    pub fn is_moon(&self) -> bool {
        self.biome == Biome::Barren
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StarSystem {
    pub id: SystemId,
    pub seed: u64,
    pub name: String,
    pub class: StarClass,
    /// 2–5 bodies: atmospheric planets first, the airless moon last
    pub planets: Vec<Planet>,
    /// Seed of the system's space map (engine `Space` world)
    pub space_seed: u32,
    /// Station position in the space map
    pub station: Vec3,
    pub asteroid_center: Vec3,
    pub asteroid_count: u32,
    /// Price variation at this system's station, ±20 %
    pub market: f32,
}
impl StarSystem {
    pub fn generate(galaxy: u64, id: SystemId) -> Self {
        let seed = hash3(id.x as i32, id.y as i32, id.z as i32, galaxy);
        let mut rng = Rng::new(seed);
        let class = [
            StarClass::Yellow,
            StarClass::Red,
            StarClass::Green,
            StarClass::Blue,
        ][rng.weighted(&[50, 30, 10, 10])];
        let atmospheric = 1 + rng.below(4) as usize;
        let mut planets = Vec::with_capacity(atmospheric + 1);
        for i in 0..=atmospheric {
            let moon = i == atmospheric;
            let biome = if moon {
                Biome::Barren
            } else if i == 0 && rng.below(3) > 0 {
                // most systems have a habitable first world
                [Biome::Lush, Biome::Lush, Biome::Frozen, Biome::Desert][rng.below(4) as usize]
            } else {
                Biome::ATMOSPHERIC[rng.weighted(&[4, 4, 4, 3, 3, 2])]
            };
            let planet_seed = (seed >> 20) as u32 ^ (i as u32).wrapping_mul(0x9e37_79b9) ^ 0x51;
            planets.push(Planet {
                name: names::name(seed, 10 + i as u64),
                seed: planet_seed & 0x7fff_ffff,
                biome,
                radius_km: if moon {
                    rng.range(900.0, 2400.0) as u32
                } else {
                    rng.range(2800.0, 9000.0) as u32
                },
                angle: rng.range(0.0, std::f32::consts::TAU),
                distance: if moon {
                    0.0
                } else {
                    0.5 + i as f32 * rng.range(0.55, 0.9)
                },
            });
        }
        // the moon orbits the first planet on the system map
        if let Some(moon) = planets.last_mut() {
            moon.distance = 0.5;
        }
        let station_dir = vec3(
            rng.range(-1.0, 1.0),
            rng.range(-1.0, 1.0),
            rng.range(-0.3, 0.4),
        )
        .try_normalize()
        .unwrap_or(Vec3::X);
        let earth = crate::world::orbit::Body::Earth;
        Self {
            id,
            seed,
            name: names::name(seed, 1),
            class,
            planets,
            space_seed: ((seed >> 8) as u32 & 0x7fff_ffff) | 1,
            station: earth.center() + station_dir * (earth.radius() + 3400.0),
            asteroid_center: earth.center()
                + vec3(-station_dir.y, station_dir.x, -station_dir.z * 0.5)
                    .try_normalize()
                    .unwrap_or(Vec3::Y)
                    * (earth.radius() + 3900.0),
            asteroid_count: 26 + rng.below(18),
            market: rng.range(0.8, 1.2),
        }
    }
    pub fn moon(&self) -> &Planet {
        self.planets.last().unwrap()
    }
    pub fn atmospheric(&self) -> usize {
        self.planets.len() - 1
    }
    pub fn space_key(&self) -> crate::world::network::MapKey {
        crate::world::network::MapKey {
            kind: crate::world::MapKind::Space,
            seed: self.space_seed,
        }
    }
    /// Index of the planet (or moon) whose surface map is `key`.
    pub fn planet_of(&self, key: crate::world::network::MapKey) -> Option<usize> {
        self.planets.iter().position(|p| p.key() == key)
    }
    /// Asteroid `i`: position and radius in the space map.
    pub fn asteroid(&self, i: u32) -> (Vec3, f32) {
        let mut rng = Rng::new(self.seed ^ 0xa57e ^ (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let angle =
            i as f32 / self.asteroid_count as f32 * std::f32::consts::TAU + rng.range(-0.1, 0.1);
        let ring = vec3(angle.cos(), angle.sin(), 0.0) * rng.range(260.0, 520.0)
            + vec3(0.0, 0.0, rng.range(-90.0, 90.0));
        (self.asteroid_center + ring, rng.range(14.0, 46.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Galaxy {
    pub seed: u64,
}
impl Default for Galaxy {
    fn default() -> Self {
        Self { seed: DEFAULT_SEED }
    }
}
impl Galaxy {
    /// Star density: thicker in the core and along two spiral arms.
    pub fn exists(&self, id: SystemId) -> bool {
        let (x, y, z) = (id.x as i32, id.y as i32, id.z as i32);
        if x.abs() > HALF || y.abs() > HALF || z.abs() > HALF_Z {
            return false;
        }
        let (fx, fy) = (x as f32, y as f32);
        let r = (fx * fx + fy * fy).sqrt() / HALF as f32;
        let arm = ((fy.atan2(fx) * 2.0 - r * 9.0).cos() * 0.5 + 0.5).powi(3);
        let density =
            (0.05 + 0.22 * arm + 0.25 * (1.0 - r).max(0.0)) * (1.0 - z.abs() as f32 / 9.0);
        (hash3(x, y, z, self.seed ^ 0x57a2) % 10_000) as f32 / 10_000.0 < density
    }
    pub fn system(&self, id: SystemId) -> StarSystem {
        StarSystem::generate(self.seed, id)
    }
    /// Existing systems within `radius` cells of `center`, nearest first, at most `limit`.
    pub fn neighbors(&self, center: SystemId, radius: f32, limit: usize) -> Vec<SystemId> {
        let r = radius.ceil() as i32;
        let mut found = Vec::new();
        for z in -r.min(HALF_Z * 2)..=r.min(HALF_Z * 2) {
            for y in -r..=r {
                for x in -r..=r {
                    let id = SystemId::new(
                        center.x as i32 + x,
                        center.y as i32 + y,
                        center.z as i32 + z,
                    );
                    if id != center && self.exists(id) && id.cells(center) <= radius {
                        found.push(id);
                    }
                }
            }
        }
        found.sort_by(|a, b| a.cells(center).total_cmp(&b.cells(center)));
        found.truncate(limit);
        found
    }
    /// The starting system: the existing star nearest the outer arm at (30, 0, 0) whose first
    /// planet is safe or mildly hazardous.
    pub fn start(&self) -> SystemId {
        let anchor = SystemId::new(30, 0, 0);
        for radius in 1..20 {
            for id in self.neighbors(anchor, radius as f32, 64) {
                let system = self.system(id);
                if system.planets[0].biome == Biome::Lush {
                    return id;
                }
            }
        }
        anchor
    }
    /// Total number of systems (slow; for tests and docs).
    pub fn count(&self) -> usize {
        let mut n = 0;
        for z in -HALF_Z..=HALF_Z {
            for y in -HALF..=HALF {
                for x in -HALF..=HALF {
                    n += usize::from(self.exists(SystemId::new(x, y, z)));
                }
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn galaxy_has_thousands_of_deterministic_systems() {
        let galaxy = Galaxy::default();
        let count = galaxy.count();
        assert!((5_000..40_000).contains(&count), "{count}");
        let start = galaxy.start();
        assert!(galaxy.exists(start));
        let a = galaxy.system(start);
        assert_eq!(a, galaxy.system(start));
        assert_eq!(a.planets[0].biome, Biome::Lush);
        assert!(!galaxy.neighbors(start, 4.0, 16).is_empty());
    }
    #[test]
    fn systems_have_two_to_five_bodies_with_one_moon_last() {
        let galaxy = Galaxy::default();
        let mut seen = [false; 6];
        let mut biomes = std::collections::HashSet::new();
        for id in galaxy.neighbors(galaxy.start(), 8.0, 400) {
            let system = galaxy.system(id);
            let n = system.planets.len();
            assert!((2..=5).contains(&n));
            seen[n] = true;
            assert!(system.moon().is_moon());
            assert_eq!(system.planets.iter().filter(|p| p.is_moon()).count(), 1);
            let keys: std::collections::HashSet<_> =
                system.planets.iter().map(|p| p.key()).collect();
            assert_eq!(keys.len(), n);
            for p in &system.planets {
                biomes.insert(p.biome);
                assert_eq!(
                    system.planet_of(p.key()).map(|i| &system.planets[i]),
                    Some(p)
                );
            }
            let earth = crate::world::orbit::Body::Earth;
            let moon = crate::world::orbit::Body::Moon;
            for point in [system.station, system.asteroid_center] {
                assert!(earth.altitude(point) > earth.orbit_altitude() + 400.0);
                assert!(moon.altitude(point) > moon.orbit_altitude() + 400.0);
            }
        }
        assert!(seen[2] && seen[3] && seen[4] && seen[5]);
        assert_eq!(biomes.len(), 7);
    }
}
