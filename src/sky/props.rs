//! Procedural surface props: flora, rocks, crystals and points of interest, placed by hashing
//! 48-unit chunks. Props are never stored: the same chunk always produces the same props, and
//! only the ids of harvested ones are remembered.
use super::biome::Biome;
use super::inventory::Item;
use super::names;
use super::rng::{Rng, hash3};
use crate::render::{ExtraTriangle, mix, rgb};
use crate::world::{SIZE, World, index};
use glam::{Vec2, Vec3, vec2, vec3};

pub const CHUNK: f32 = 48.0;
/// Props further than this are not drawn.
pub const DRAW_DISTANCE: f32 = 120.0;
/// Inside this distance props use their full mesh.
pub const DETAIL_DISTANCE: f32 = 45.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    Flora,
    Rock,
    Crystal,
    /// Point of interest: ancient monolith
    Monolith,
    /// Point of interest: crashed supply pod
    Pod,
    /// Point of interest: rich mineral deposit
    Deposit,
    /// Point of interest: guarded weapon cache
    Cache,
    /// Point of interest: ruined pillars with an inscription
    Ruin,
    /// Point of interest: signal beacon that marks nearby points of interest
    Beacon,
}
impl PropKind {
    pub fn is_poi(self) -> bool {
        matches!(
            self,
            Self::Monolith | Self::Pod | Self::Deposit | Self::Cache | Self::Ruin | Self::Beacon
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Flora => "FLORA",
            Self::Rock => "MINERAL",
            Self::Crystal => "CRYSTAL",
            Self::Monolith => "MONOLITH",
            Self::Pod => "CRASHED POD",
            Self::Deposit => "DEPOSIT",
            Self::Cache => "WEAPON CACHE",
            Self::Ruin => "RUIN",
            Self::Beacon => "BEACON",
        }
    }
    /// What key 7 does at this prop, if anything.
    pub fn interaction(self) -> Option<&'static str> {
        match self {
            Self::Pod | Self::Cache => Some("OPEN"),
            Self::Monolith | Self::Ruin => Some("READ"),
            Self::Beacon => Some("SIGNAL"),
            _ => None,
        }
    }
    /// Credits for the first scan.
    pub fn discovery(self) -> u32 {
        match self {
            Self::Flora => 12,
            Self::Rock => 4,
            Self::Crystal => 8,
            Self::Monolith => 150,
            Self::Pod => 80,
            Self::Deposit => 60,
            Self::Cache => 90,
            Self::Ruin => 100,
            Self::Beacon => 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub id: u64,
    pub kind: PropKind,
    pub position: Vec3,
    pub size: f32,
    pub yaw: f32,
}
impl Prop {
    /// Element and amount harvested when mined out; `None` for props that are not mined.
    pub fn harvest(&self, biome: Biome) -> Option<(Item, u16)> {
        let amount = |base: f32| (base * self.size).round().max(1.0) as u16;
        match self.kind {
            PropKind::Flora => Some((Item::Carbon, amount(6.0))),
            PropKind::Rock => Some((Item::Iron, amount(9.0))),
            PropKind::Crystal => Some((biome.mineral(), amount(7.0))),
            PropKind::Deposit => Some((biome.mineral(), 60)),
            _ => None,
        }
    }
    /// Seconds of beam time to mine out at the base beam rate.
    pub fn toughness(&self) -> f32 {
        match self.kind {
            PropKind::Flora => 0.8 + self.size * 0.3,
            PropKind::Rock => 1.2 + self.size * 0.5,
            PropKind::Crystal => 1.0 + self.size * 0.4,
            PropKind::Deposit => 6.0,
            _ => f32::INFINITY,
        }
    }
    /// Procedural species or mineral name shown by the scanner.
    pub fn name(&self) -> String {
        match self.kind {
            PropKind::Flora | PropKind::Rock | PropKind::Crystal => names::name(self.id, 77),
            k => k.label().to_owned(),
        }
    }
    /// Axis-aligned half extents of the bounding box (for the scanner frame).
    pub fn half_extent(&self) -> Vec3 {
        let s = self.size;
        match self.kind {
            PropKind::Flora => vec3(1.6 * s, 1.6 * s, 2.6 * s),
            PropKind::Rock => vec3(1.1 * s, 1.1 * s, 0.6 * s),
            PropKind::Crystal => vec3(0.7 * s, 0.7 * s, 1.4 * s),
            PropKind::Monolith => vec3(1.4, 1.4, 6.0),
            PropKind::Pod => vec3(2.6, 2.6, 1.4),
            PropKind::Deposit => vec3(2.4, 2.4, 1.6),
            PropKind::Cache => vec3(1.8, 1.8, 1.2),
            PropKind::Ruin => vec3(4.0, 4.0, 4.0),
            PropKind::Beacon => vec3(0.8, 0.8, 5.0),
        }
    }
    pub fn center(&self) -> Vec3 {
        self.position + Vec3::Z * self.half_extent().z
    }
}

/// Props in chunk `(cx, cy)` of a planet; harvested ids are skipped by the caller.
pub fn chunk(world: &World, planet_seed: u32, biome: Biome, cx: i32, cy: i32, out: &mut Vec<Prop>) {
    let seed = hash3(cx, cy, 7, planet_seed as u64);
    let mut rng = Rng::new(seed);
    let (weights, density) = biome.props();
    let count = rng.below(density * 2 + 1);
    let origin = vec2(cx as f32, cy as f32) * CHUNK;
    let place = |rng: &mut Rng, kind: PropKind, size: f32, n: u32, out: &mut Vec<Prop>| {
        let p = origin + vec2(rng.range(1.0, CHUNK - 1.0), rng.range(1.0, CHUNK - 1.0));
        if p.x < 8.0 || p.y < 8.0 || p.x > SIZE as f32 - 8.0 || p.y > SIZE as f32 - 8.0 {
            return;
        }
        let at = index(p.x as i32, p.y as i32, SIZE);
        if world.materials[at] < 4 && !world.kind.vacuum() {
            return; // water
        }
        let ground = |x: f32, y: f32| super::walker::ground(world, x, y);
        let z = ground(p.x, p.y);
        let slope = (ground(p.x + 2.0, p.y) - z)
            .abs()
            .max((ground(p.x, p.y + 2.0) - z).abs());
        if slope > 3.0 {
            return;
        }
        out.push(Prop {
            id: seed ^ (n as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15),
            kind,
            position: vec3(p.x, p.y, z - 0.2),
            size,
            yaw: rng.range(0.0, std::f32::consts::TAU),
        });
    };
    for n in 0..count {
        let kind = [PropKind::Flora, PropKind::Rock, PropKind::Crystal][rng.weighted(&weights)];
        let size = rng.range(0.7, 1.6);
        place(&mut rng, kind, size, n, out);
    }
    // one chunk in ~30 has a point of interest
    if rng.below(30) == 0 {
        let kinds = [
            PropKind::Monolith,
            PropKind::Pod,
            PropKind::Deposit,
            PropKind::Cache,
            PropKind::Ruin,
            PropKind::Beacon,
        ];
        let kind = kinds[rng.weighted(&[2, 3, 3, 2, 2, 1])];
        place(&mut rng, kind, 1.0, 99, out);
    }
}

/// All props within `radius` of `center`, skipping harvested ids. Reuses `out`.
pub fn nearby(
    world: &World,
    planet_seed: u32,
    biome: Biome,
    center: Vec2,
    radius: f32,
    harvested: &std::collections::HashSet<u64>,
    out: &mut Vec<Prop>,
) {
    out.clear();
    let (x0, x1) = (
        ((center.x - radius) / CHUNK).floor() as i32,
        ((center.x + radius) / CHUNK).floor() as i32,
    );
    let (y0, y1) = (
        ((center.y - radius) / CHUNK).floor() as i32,
        ((center.y + radius) / CHUNK).floor() as i32,
    );
    let start = out.len();
    for cy in y0..=y1 {
        for cx in x0..=x1 {
            chunk(world, planet_seed, biome, cx, cy, out);
        }
    }
    let mut i = start;
    while i < out.len() {
        let p = out[i];
        if harvested.contains(&p.id)
            || p.position.truncate().distance_squared(center) > radius * radius
        {
            out.swap_remove(i);
        } else {
            i += 1;
        }
    }
}

fn tri(out: &mut Vec<ExtraTriangle>, a: Vec3, b: Vec3, c: Vec3, color: u32, glow: bool) {
    out.push(ExtraTriangle {
        points: [a, b, c],
        color,
        glow,
    });
}

/// A ring of `n` points around `center` at radius `r` (rotated by `yaw`).
fn ring(center: Vec3, r: f32, n: usize, yaw: f32) -> [Vec3; 6] {
    std::array::from_fn(|i| {
        let a = yaw + (i % n) as f32 / n as f32 * std::f32::consts::TAU;
        center + vec3(a.cos() * r, a.sin() * r, 0.0)
    })
}

/// Closed spindle: `n`-gon ring at `mid`, tips at `top` and `bottom`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spindle(
    out: &mut Vec<ExtraTriangle>,
    bottom: Vec3,
    mid: Vec3,
    top: Vec3,
    r: f32,
    n: usize,
    yaw: f32,
    colors: (u32, u32),
    glow: bool,
) {
    let ring = ring(mid, r, n, yaw);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        tri(out, a, b, top, colors.0, glow);
        tri(out, b, a, bottom, colors.1, glow);
    }
}

/// Box from `min` to `max` (12 triangles; bottom omitted).
pub(crate) fn cuboid(
    out: &mut Vec<ExtraTriangle>,
    center: Vec3,
    half: Vec3,
    yaw: f32,
    color: u32,
    glow: bool,
) {
    let (s, c) = yaw.sin_cos();
    let corner = |x: f32, y: f32, z: f32| {
        let (lx, ly) = (x * half.x, y * half.y);
        center + vec3(lx * c - ly * s, lx * s + ly * c, z * half.z)
    };
    let p = [
        corner(-1.0, -1.0, -1.0),
        corner(1.0, -1.0, -1.0),
        corner(1.0, 1.0, -1.0),
        corner(-1.0, 1.0, -1.0),
        corner(-1.0, -1.0, 1.0),
        corner(1.0, -1.0, 1.0),
        corner(1.0, 1.0, 1.0),
        corner(-1.0, 1.0, 1.0),
    ];
    let faces = [
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    for f in faces {
        tri(out, p[f[0]], p[f[1]], p[f[2]], color, glow);
        tri(out, p[f[0]], p[f[2]], p[f[3]], color, glow);
    }
}

/// Append the triangles of a prop. `detail` false draws a cheaper far version.
pub fn mesh(prop: &Prop, palette: [u32; 3], time: f32, detail: bool, out: &mut Vec<ExtraTriangle>) {
    let p = prop.position;
    let s = prop.size;
    let n = if detail { 5 } else { 3 };
    match prop.kind {
        PropKind::Flora => {
            let [leaf, _, _] = palette;
            let trunk = mix(leaf, rgb(70, 50, 36), 0.7);
            let sway = super::rng::fast_sin(time * 1.3 + prop.yaw * 5.0) * 0.12 * s;
            let top = p + vec3(sway, 0.0, 2.6 * s);
            // trunk as a thin spindle (near only), canopy as a wider one
            if detail {
                spindle(
                    out,
                    p,
                    p + Vec3::Z * 0.8 * s,
                    top,
                    0.22 * s,
                    3,
                    prop.yaw,
                    (trunk, trunk),
                    false,
                );
            }
            let canopy_mid = p + vec3(sway * 0.8, 0.0, 2.9 * s);
            spindle(
                out,
                p + Vec3::Z * 1.6 * s,
                canopy_mid,
                p + vec3(sway * 1.2, 0.0, 4.6 * s),
                1.5 * s,
                n,
                prop.yaw,
                (
                    mix(leaf, rgb(255, 255, 255), 0.12),
                    mix(leaf, rgb(0, 0, 0), 0.25),
                ),
                false,
            );
        }
        PropKind::Rock => {
            let [_, rock, _] = palette;
            spindle(
                out,
                p - Vec3::Z * 0.3,
                p + Vec3::Z * 0.45 * s,
                p + Vec3::Z * 1.25 * s,
                1.15 * s,
                n,
                prop.yaw,
                (rock, mix(rock, rgb(0, 0, 0), 0.3)),
                false,
            );
        }
        PropKind::Crystal => {
            let [_, _, crystal] = palette;
            spindle(
                out,
                p,
                p + Vec3::Z * 0.9 * s,
                p + Vec3::Z * 2.8 * s,
                0.55 * s,
                if detail { 4 } else { 3 },
                prop.yaw,
                (crystal, mix(crystal, rgb(0, 0, 0), 0.35)),
                true,
            );
            if detail {
                let side = p + vec3(prop.yaw.cos(), prop.yaw.sin(), 0.0) * 0.8 * s;
                spindle(
                    out,
                    side,
                    side + Vec3::Z * 0.5 * s,
                    side + vec3(prop.yaw.cos() * 0.6, prop.yaw.sin() * 0.6, 1.6 * s),
                    0.32 * s,
                    3,
                    prop.yaw + 1.0,
                    (crystal, mix(crystal, rgb(0, 0, 0), 0.35)),
                    true,
                );
            }
        }
        PropKind::Monolith => {
            cuboid(
                out,
                p + Vec3::Z * 6.0,
                vec3(1.2, 0.5, 6.0),
                prop.yaw,
                rgb(36, 34, 44),
                false,
            );
            let pulse = 0.5 + 0.5 * super::rng::fast_sin(time * 2.0);
            let glow = mix(rgb(80, 40, 160), rgb(160, 255, 250), pulse);
            cuboid(
                out,
                p + Vec3::Z * 8.5,
                vec3(1.25, 0.55, 0.35),
                prop.yaw,
                glow,
                true,
            );
        }
        PropKind::Pod => {
            cuboid(
                out,
                p + Vec3::Z * 1.1,
                vec3(2.4, 1.3, 1.1),
                prop.yaw,
                rgb(196, 200, 210),
                false,
            );
            cuboid(
                out,
                p + Vec3::Z * 2.5,
                vec3(0.3, 0.3, 0.4),
                prop.yaw,
                rgb(255, 120, 60),
                true,
            );
            let fin = p + vec3(prop.yaw.cos(), prop.yaw.sin(), 0.0) * 2.6 + Vec3::Z * 1.0;
            tri(
                out,
                fin,
                fin + Vec3::Z * 2.4,
                fin + vec3(-prop.yaw.sin(), prop.yaw.cos(), 0.0) * 1.2,
                rgb(150, 156, 170),
                false,
            );
        }
        PropKind::Cache => {
            let lid = rgb(70, 78, 96);
            cuboid(
                out,
                p + Vec3::Z * 1.0,
                vec3(1.7, 1.1, 1.0),
                prop.yaw,
                rgb(48, 54, 66),
                false,
            );
            cuboid(
                out,
                p + Vec3::Z * 2.1,
                vec3(1.8, 1.2, 0.15),
                prop.yaw,
                lid,
                false,
            );
            let pulse = 0.5 + 0.5 * super::rng::fast_sin(time * 3.0);
            let glow = mix(rgb(120, 40, 20), rgb(255, 180, 60), pulse);
            cuboid(
                out,
                p + Vec3::Z * 1.0,
                vec3(1.75, 0.2, 0.15),
                prop.yaw,
                glow,
                true,
            );
        }
        PropKind::Ruin => {
            let [_, rock, _] = palette;
            let stone = mix(rock, rgb(200, 190, 170), 0.35);
            for k in 0..if detail { 4 } else { 2 } {
                let a = prop.yaw + k as f32 * 1.57;
                let height = [5.0, 2.4, 3.6, 1.4][k];
                let base = p + vec3(a.cos(), a.sin(), 0.0) * 3.0;
                cuboid(
                    out,
                    base + Vec3::Z * height * 0.5,
                    vec3(0.6, 0.6, height * 0.5),
                    a,
                    stone,
                    false,
                );
            }
            cuboid(
                out,
                p + Vec3::Z * 0.3,
                vec3(1.0, 0.7, 0.3),
                prop.yaw,
                mix(stone, rgb(0, 0, 0), 0.3),
                false,
            );
        }
        PropKind::Beacon => {
            spindle(
                out,
                p,
                p + Vec3::Z * 1.0,
                p + Vec3::Z * 8.0,
                0.5,
                3,
                prop.yaw,
                (rgb(150, 156, 170), rgb(90, 96, 110)),
                false,
            );
            let blink = if (time * 1.5).fract() < 0.5 {
                rgb(255, 80, 70)
            } else {
                rgb(120, 30, 30)
            };
            cuboid(
                out,
                p + Vec3::Z * 8.4,
                Vec3::splat(0.45),
                prop.yaw,
                blink,
                true,
            );
        }
        PropKind::Deposit => {
            let [_, rock, crystal] = palette;
            spindle(
                out,
                p - Vec3::Z * 0.2,
                p + Vec3::Z * 0.4,
                p + Vec3::Z * 1.2,
                2.4,
                5,
                prop.yaw,
                (rock, rock),
                false,
            );
            for k in 0..if detail { 4 } else { 2 } {
                let a = prop.yaw + k as f32 * 1.7;
                let base = p + vec3(a.cos() * 1.2, a.sin() * 1.2, 0.6);
                spindle(
                    out,
                    base,
                    base + Vec3::Z * 0.6,
                    base + vec3(a.cos() * 0.5, a.sin() * 0.5, 2.6),
                    0.45,
                    3,
                    a,
                    (crystal, mix(crystal, rgb(0, 0, 0), 0.3)),
                    true,
                );
            }
        }
    }
}

/// Low-poly asteroid for space (a lumpy double cone).
pub fn asteroid(center: Vec3, radius: f32, seed: u64, out: &mut Vec<ExtraTriangle>) {
    let mut rng = Rng::new(seed);
    let rock = mix(rgb(110, 100, 92), rgb(150, 134, 116), rng.unit());
    let n = 6;
    let yaw = rng.range(0.0, 6.0);
    let tilt = vec3(rng.range(-0.3, 0.3), rng.range(-0.3, 0.3), 0.0);
    let ring: [Vec3; 6] = std::array::from_fn(|i| {
        let a = yaw + i as f32 / n as f32 * std::f32::consts::TAU;
        let r = radius * rng.range(0.75, 1.15);
        center
            + vec3(
                a.cos() * r,
                a.sin() * r,
                (tilt.x * a.cos() + tilt.y * a.sin()) * radius,
            )
    });
    let top = center + vec3(rng.range(-0.3, 0.3), rng.range(-0.3, 0.3), 0.9) * radius;
    let bottom = center - vec3(rng.range(-0.3, 0.3), rng.range(-0.3, 0.3), 0.85) * radius;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        tri(out, a, b, top, rock, false);
        tri(out, b, a, bottom, mix(rock, rgb(0, 0, 0), 0.3), false);
    }
}

/// Space station: an octahedral hub with a docking ring and lights.
pub fn station(center: Vec3, time: f32, out: &mut Vec<ExtraTriangle>) {
    let hull = rgb(176, 184, 198);
    let dark = rgb(90, 98, 116);
    spindle(
        out,
        center - Vec3::Z * 70.0,
        center,
        center + Vec3::Z * 70.0,
        52.0,
        4,
        0.4,
        (hull, dark),
        false,
    );
    let spin = time * 0.15;
    for k in 0..8 {
        let a0 = spin + k as f32 / 8.0 * std::f32::consts::TAU;
        let a1 = spin + (k as f32 + 0.8) / 8.0 * std::f32::consts::TAU;
        let r = 120.0;
        let p0 = center + vec3(a0.cos() * r, a0.sin() * r, 0.0);
        let p1 = center + vec3(a1.cos() * r, a1.sin() * r, 0.0);
        tri(
            out,
            p0 - Vec3::Z * 10.0,
            p1 - Vec3::Z * 10.0,
            p1 + Vec3::Z * 10.0,
            dark,
            false,
        );
        tri(
            out,
            p0 - Vec3::Z * 10.0,
            p1 + Vec3::Z * 10.0,
            p0 + Vec3::Z * 10.0,
            hull,
            false,
        );
        // spokes
        tri(
            out,
            center,
            p0 - Vec3::Z * 3.0,
            p0 + Vec3::Z * 3.0,
            dark,
            false,
        );
        let light = if (k + (time * 2.0) as usize).is_multiple_of(4) {
            rgb(120, 255, 200)
        } else {
            rgb(255, 180, 80)
        };
        cuboid(
            out,
            p0 + Vec3::Z * 12.0,
            vec3(4.0, 4.0, 3.0),
            a0,
            light,
            true,
        );
    }
    // docking bay: glowing slot facing +X
    cuboid(
        out,
        center + vec3(52.0, 0.0, 0.0),
        vec3(6.0, 18.0, 10.0),
        0.0,
        rgb(80, 230, 255),
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapKind;
    #[test]
    fn props_are_deterministic_dry_and_on_the_ground() {
        let world = World::generate(MapKind::Islands, 4242);
        let mut a = Vec::new();
        let mut b = Vec::new();
        let center = vec2(world.spawn.0, world.spawn.1);
        let none = Default::default();
        nearby(&world, 4242, Biome::Lush, center, 400.0, &none, &mut a);
        nearby(&world, 4242, Biome::Lush, center, 400.0, &none, &mut b);
        assert_eq!(a, b);
        assert!(a.len() > 40, "{}", a.len());
        for p in &a {
            assert!(world.materials[index(p.position.x as i32, p.position.y as i32, SIZE)] >= 4);
            let ground = crate::sky::walker::ground(&world, p.position.x, p.position.y);
            assert!((p.position.z - ground).abs() < 0.5);
        }
        let mut harvested = std::collections::HashSet::new();
        harvested.insert(a[0].id);
        nearby(&world, 4242, Biome::Lush, center, 400.0, &harvested, &mut b);
        assert_eq!(b.len(), a.len() - 1);
        let ids: std::collections::HashSet<_> = a.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), a.len());
    }
    #[test]
    fn meshes_stay_small_and_far_versions_are_cheaper() {
        let palette = Biome::Lush.palette(1);
        for kind in [
            PropKind::Flora,
            PropKind::Rock,
            PropKind::Crystal,
            PropKind::Monolith,
            PropKind::Pod,
            PropKind::Deposit,
        ] {
            let prop = Prop {
                id: 1,
                kind,
                position: Vec3::ZERO,
                size: 1.2,
                yaw: 0.3,
            };
            let (mut near, mut far) = (Vec::new(), Vec::new());
            mesh(&prop, palette, 0.0, true, &mut near);
            mesh(&prop, palette, 0.0, false, &mut far);
            assert!(
                near.len() <= 40 && far.len() <= near.len() && !far.is_empty(),
                "{kind:?}"
            );
        }
    }
}
