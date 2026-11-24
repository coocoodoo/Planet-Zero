//! Voxel busts of station people for the talk panel: a small grid of coloured cubes per person,
//! shaped by species (crested TALLFOLK, broad STONEKIN, floating LANTERNS) and dressed by role.
//! `ui::sky_bust` ray-traces it into the portrait.
use super::interior::{Npc, Role};
use crate::render::{mix, rgb};

/// Grid size: x left to right (as you face them), y front to back, z up.
pub const SIZE: [usize; 3] = [16, 12, 22];
/// Cell flags (the low 24 bits hold the colour as packed by `render::rgb`)
pub const SOLID: u32 = 1 << 24;
/// Lit from within: eyes, badges, lantern heads
pub const GLOW: u32 = 1 << 25;

pub struct Bust {
    pub cells: Vec<u32>,
}

impl Bust {
    fn empty() -> Self {
        Self {
            cells: vec![0; SIZE[0] * SIZE[1] * SIZE[2]],
        }
    }
    fn index(x: i32, y: i32, z: i32) -> Option<usize> {
        let inside = (0..SIZE[0] as i32).contains(&x)
            && (0..SIZE[1] as i32).contains(&y)
            && (0..SIZE[2] as i32).contains(&z);
        inside.then(|| (z as usize * SIZE[1] + y as usize) * SIZE[0] + x as usize)
    }
    pub fn get(&self, x: i32, y: i32, z: i32) -> u32 {
        Self::index(x, y, z).map_or(0, |i| self.cells[i])
    }
    fn set(&mut self, x: i32, y: i32, z: i32, color: u32, glow: bool) {
        if let Some(i) = Self::index(x, y, z) {
            self.cells[i] = (color & 0xff_ffff) | SOLID | if glow { GLOW } else { 0 };
        }
    }
    fn clear(&mut self, x: i32, y: i32, z: i32) {
        if let Some(i) = Self::index(x, y, z) {
            self.cells[i] = 0;
        }
    }
    /// Fill a box (`hi` exclusive), each cube's colour varied a little so the voxels read.
    fn fill(&mut self, lo: [i32; 3], hi: [i32; 3], color: u32, seed: u64, glow: bool) {
        for z in lo[2]..hi[2] {
            for y in lo[1]..hi[1] {
                for x in lo[0]..hi[0] {
                    self.set(x, y, z, grain(color, seed, x, y, z), glow);
                }
            }
        }
    }
    pub fn solid(&self) -> usize {
        self.cells.iter().filter(|c| *c & SOLID != 0).count()
    }
}

/// A small per-voxel tint so a flat surface still shows its cubes.
fn grain(color: u32, seed: u64, x: i32, y: i32, z: i32) -> u32 {
    let h = (seed ^ (x as u64).wrapping_mul(0x9e37_79b9) ^ (y as u64).wrapping_mul(0x85eb_ca6b))
        .wrapping_mul(0xc2b2_ae35)
        ^ (z as u64).wrapping_mul(0x27d4_eb2f);
    let t = ((h >> 29) % 100) as f32 / 100.0;
    if t < 0.5 {
        mix(color, rgb(0, 0, 0), (0.5 - t) * 0.16)
    } else {
        mix(color, rgb(255, 255, 255), (t - 0.5) * 0.12)
    }
}

/// Build someone's bust.
pub fn bust(npc: &Npc) -> Bust {
    let mut b = Bust::empty();
    let s = npc.seed;
    let skin = npc.skin;
    let cloth = npc.cloth;
    let dark_cloth = mix(cloth, rgb(0, 0, 0), 0.35);
    // shoulders and chest, a collar on top
    b.fill([1, 2, 0], [15, 10, 3], dark_cloth, s, false);
    b.fill([2, 2, 3], [14, 10, 6], cloth, s, false);
    b.fill(
        [4, 3, 6],
        [12, 9, 7],
        mix(cloth, rgb(255, 255, 255), 0.15),
        s,
        false,
    );
    // round the shoulders off
    for (x, z) in [(1, 2), (14, 2), (2, 5), (13, 5)] {
        for y in 2..10 {
            b.clear(x, y, z);
        }
    }
    match npc.species {
        0 => tallfolk(&mut b, skin, cloth, s),
        1 => stonekin(&mut b, skin, s),
        _ => lantern(&mut b, skin, cloth, s),
    }
    dress(&mut b, npc);
    b
}

/// Tall narrow head, long face and a swept-back crest.
fn tallfolk(b: &mut Bust, skin: u32, cloth: u32, s: u64) {
    b.fill([6, 4, 6], [10, 8, 8], skin, s, false);
    b.fill([5, 3, 8], [11, 9, 18], skin, s, false);
    b.fill([6, 3, 18], [10, 9, 19], skin, s, false);
    // narrow chin
    for x in [5, 10] {
        for z in 8..10 {
            b.clear(x, 3, z);
        }
    }
    // crest from the crown sweeping back and up
    let crest = mix(cloth, rgb(255, 255, 255), 0.25);
    for (i, y) in (4..11).enumerate() {
        let top = 19 + (i as i32 / 2).min(2);
        b.fill([7, y, 18], [9, y + 1, top + 1], crest, s, false);
    }
    // pale eyes, a thin mouth
    let eye = rgb(255, 240, 200);
    for x in [6, 9] {
        b.set(x, 3, 14, eye, true);
    }
    b.fill(
        [7, 3, 10],
        [9, 4, 11],
        mix(skin, rgb(0, 0, 0), 0.45),
        s,
        false,
    );
}

/// Broad flat head under a heavy brow, skin like weathered stone.
fn stonekin(b: &mut Bust, skin: u32, s: u64) {
    b.fill([5, 4, 6], [11, 8, 7], skin, s, false);
    b.fill([3, 2, 7], [13, 10, 14], skin, s, false);
    b.fill([4, 3, 14], [12, 9, 15], skin, s, false);
    // cracks and pale stones
    for z in 7..15 {
        for x in 3..13 {
            let h = (s ^ (x as u64 * 31 + z as u64 * 17)).wrapping_mul(0x2545_f491_4f6c_dd1d);
            if (h >> 59) == 0 {
                b.set(x, 2, z, mix(skin, rgb(0, 0, 0), 0.35), false);
            } else if (h >> 59) == 1 {
                b.set(x, 2, z, mix(skin, rgb(255, 255, 255), 0.2), false);
            }
        }
    }
    // the brow juts out over amber eyes
    b.fill(
        [3, 1, 12],
        [13, 2, 14],
        mix(skin, rgb(40, 30, 20), 0.6),
        s,
        false,
    );
    for x in [5, 6, 9, 10] {
        b.set(x, 2, 11, rgb(255, 200, 120), true);
    }
    b.fill(
        [6, 2, 8],
        [10, 3, 9],
        mix(skin, rgb(0, 0, 0), 0.5),
        s,
        false,
    );
}

/// A glowing orb floating over a high collar, two dark eyes in its light.
fn lantern(b: &mut Bust, skin: u32, cloth: u32, s: u64) {
    b.fill(
        [4, 3, 7],
        [12, 9, 9],
        mix(cloth, rgb(0, 0, 0), 0.2),
        s,
        false,
    );
    let glow = mix(skin, rgb(255, 255, 220), 0.5);
    let center = [7.5_f32, 5.5, 15.0];
    for z in 10..21 {
        for y in 0..12 {
            for x in 0..16 {
                let d = [
                    x as f32 - center[0],
                    (y as f32 - center[1]) * 1.1,
                    z as f32 - center[2],
                ];
                let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                if r < 5.0 {
                    // brighter toward the front and the top
                    let t = ((-d[1] + d[2]) / 10.0 + 0.5).clamp(0.0, 1.0);
                    let c = mix(mix(skin, glow, 0.5), rgb(255, 255, 235), t * 0.6);
                    b.set(x, y, z, grain(c, s, x, y, z), true);
                }
            }
        }
    }
    for x in [6, 9] {
        let front = (0..12).find(|&y| b.get(x, y, 15) & SOLID != 0).unwrap_or(1);
        b.set(x, front, 15, rgb(30, 24, 40), false);
        b.set(x, front, 16, rgb(30, 24, 40), false);
    }
}

/// What each role wears.
fn dress(b: &mut Bust, npc: &Npc) {
    let s = npc.seed;
    match npc.role {
        Role::Agent => {
            // the guild badge and a sash
            b.fill([4, 1, 3], [6, 2, 5], rgb(255, 200, 60), s, true);
            for i in 0..6 {
                b.set(7 + i, 1, 5 - i / 2, rgb(200, 60, 70), false);
            }
        }
        Role::Trader => {
            // a satchel strap across the chest
            for i in 0..10 {
                b.set(3 + i, 1, 5 - (i * 5) / 10, rgb(110, 75, 45), false);
            }
            b.fill([11, 1, 0], [14, 2, 2], rgb(130, 90, 55), s, false);
        }
        Role::Outfitter => {
            // plated shoulders
            let plate = rgb(150, 160, 175);
            b.fill([0, 2, 4], [4, 10, 6], plate, s, false);
            b.fill([12, 2, 4], [16, 10, 6], plate, s, false);
        }
        Role::Technician => {
            // goggles pushed up on the head
            let top = (0..SIZE[2] as i32)
                .rev()
                .find(|&z| b.get(8, 5, z) & SOLID != 0)
                .unwrap_or(14);
            let z = (top - 2).max(9);
            for x in 0..16 {
                if (0..12).any(|y| b.get(x, y, z) & SOLID != 0) {
                    let front = (0..12).find(|&y| b.get(x, y, z) & SOLID != 0).unwrap_or(0);
                    b.set(x, front, z, rgb(60, 60, 70), false);
                }
            }
            for x in [6, 9] {
                let front = (0..12).find(|&y| b.get(x, y, z) & SOLID != 0).unwrap_or(0);
                b.set(x, front, z, rgb(90, 230, 255), true);
            }
        }
        Role::Traveller => {
            // a scarf wound high round the neck
            let scarf = mix(npc.cloth, rgb(220, 160, 90), 0.5);
            b.fill([3, 1, 5], [13, 11, 8], scarf, s, false);
            for y in [1, 10] {
                b.clear(3, y, 7);
                b.clear(12, y, 7);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::galaxy::Galaxy;
    use crate::sky::interior::people;

    #[test]
    fn every_species_and_role_has_a_face_lit_and_dressed() {
        let galaxy = Galaxy::default();
        let mut species = [false; 3];
        let mut id = galaxy.start();
        for _ in 0..40 {
            let system = galaxy.system(id);
            for npc in people(&system, 1, 0.0) {
                species[npc.species as usize % 3] = true;
                let b = bust(&npc);
                assert!((300..2500).contains(&b.solid()), "{}", b.solid());
                // a face at head height, seen from the front
                let face = (0..12).any(|y| b.get(7, y, 14) & SOLID != 0);
                assert!(face);
                // something glows: eyes, a badge, goggles or a lantern head
                assert!(b.cells.iter().any(|c| c & GLOW != 0));
                if npc.role == Role::Agent {
                    assert!(b.get(4, 1, 3) & GLOW != 0, "the guild badge");
                }
            }
            id = galaxy.neighbors(id, 8.0, 4).last().copied().unwrap_or(id);
        }
        assert!(species.iter().filter(|s| **s).count() >= 2);
    }
}
