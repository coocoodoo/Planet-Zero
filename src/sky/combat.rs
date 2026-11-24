//! Projectiles, impact effects and hit tests for on-foot combat. Fixed caps keep the per-frame
//! cost bounded on the Cardputer: at most 24 projectiles and 16 effects.
use super::walker::ground;
use crate::world::World;
use glam::Vec3;

pub const MAX_PROJECTILES: usize = 24;
pub const MAX_EFFECTS: usize = 16;
/// Radius of the player for incoming fire
pub const PLAYER_RADIUS: f32 = 1.3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projectile {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub damage: f32,
    /// Fired by a drone (hits the player) or by the player (hits drones and props)
    pub hostile: bool,
    pub color: [u8; 3],
    pub gravity: f32,
    pub splash: f32,
    pub bounces: u8,
    pub pierce: bool,
    pub homing: f32,
    pub slow: f32,
    pub siphon: f32,
    pub volatile: bool,
    pub mining: f32,
    /// Last drone hit (piercing shots skip it)
    pub last_hit: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind {
    /// Small flash at an impact
    Spark,
    /// Expanding burst (splash, destroyed drone)
    Burst,
    /// Line to a second point (lance, arc, beam)
    Line(Vec3),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub position: Vec3,
    pub kind: EffectKind,
    pub color: [u8; 3],
    pub radius: f32,
    pub time: f32,
    pub duration: f32,
}
impl Effect {
    pub fn new(
        position: Vec3,
        kind: EffectKind,
        color: [u8; 3],
        radius: f32,
        duration: f32,
    ) -> Self {
        Self {
            position,
            kind,
            color,
            radius,
            time: 0.0,
            duration,
        }
    }
    /// 0 at birth, 1 at the end
    pub fn age(&self) -> f32 {
        (self.time / self.duration.max(0.001)).clamp(0.0, 1.0)
    }
}

/// Push into a capped list, dropping the oldest entry when full.
pub fn push_capped<T>(list: &mut Vec<T>, item: T, cap: usize) {
    if list.len() >= cap {
        list.remove(0);
    }
    list.push(item);
}

/// Distance along a unit ray to a sphere, if hit within `length`.
pub fn ray_sphere(
    origin: Vec3,
    direction: Vec3,
    length: f32,
    center: Vec3,
    radius: f32,
) -> Option<f32> {
    let to = center - origin;
    let along = to.dot(direction);
    let closest = to.length_squared() - along * along;
    if closest > radius * radius {
        return None;
    }
    let t = along - (radius * radius - closest).sqrt();
    let t = if t < 0.0 {
        along + (radius * radius - closest).sqrt()
    } else {
        t
    };
    (0.0..=length).contains(&t).then_some(t)
}

/// Distance along a unit ray to the ground, sampling every 2 units (≤ 80 samples).
pub fn terrain_hit(world: &World, origin: Vec3, direction: Vec3, length: f32) -> Option<f32> {
    let step = (length / 80.0).max(2.0);
    let mut previous = 0.0;
    let mut t = step.min(length);
    loop {
        let p = origin + direction * t;
        if p.z <= ground(world, p.x, p.y) {
            // refine between the last clear sample and this one
            let (mut lo, mut hi) = (previous, t);
            for _ in 0..5 {
                let mid = (lo + hi) * 0.5;
                let q = origin + direction * mid;
                if q.z <= ground(world, q.x, q.y) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            return Some(hi);
        }
        if t >= length {
            return None;
        }
        previous = t;
        t = (t + step).min(length);
    }
}

/// Line of sight between two points (terrain only).
pub fn clear_line(world: &World, from: Vec3, to: Vec3) -> bool {
    let delta = to - from;
    let length = delta.length();
    length < 0.01 || terrain_hit(world, from, delta / length, length - 0.5).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapKind;
    use glam::vec3;
    #[test]
    fn rays_hit_spheres_and_ground() {
        let hit = ray_sphere(Vec3::ZERO, Vec3::X, 100.0, vec3(10.0, 0.5, 0.0), 1.0).unwrap();
        assert!((hit - 9.134).abs() < 0.01);
        assert!(ray_sphere(Vec3::ZERO, Vec3::X, 5.0, vec3(10.0, 0.0, 0.0), 1.0).is_none());
        assert!(ray_sphere(Vec3::ZERO, Vec3::X, 100.0, vec3(10.0, 3.0, 0.0), 1.0).is_none());
        assert!(ray_sphere(Vec3::ZERO, Vec3::X, 100.0, vec3(-10.0, 0.0, 0.0), 1.0).is_none());
        let world = World::generate(MapKind::Highlands, 77);
        let (x, y) = world.spawn;
        let g = ground(&world, x, y);
        let down = terrain_hit(&world, vec3(x, y, g + 20.0), -Vec3::Z, 40.0).unwrap();
        assert!((down - 20.0).abs() < 0.3, "{down}");
        assert!(terrain_hit(&world, vec3(x, y, g + 20.0), Vec3::Z, 40.0).is_none());
        assert!(clear_line(
            &world,
            vec3(x, y, g + 50.0),
            vec3(x + 5.0, y, g + 50.0)
        ));
        assert!(!clear_line(
            &world,
            vec3(x, y, g + 10.0),
            vec3(x, y, g - 10.0)
        ));
        let mut list = Vec::new();
        for i in 0..30 {
            push_capped(&mut list, i, MAX_PROJECTILES);
        }
        assert_eq!(list.len(), MAX_PROJECTILES);
        assert_eq!(list[0], 6);
    }
}
