//! Deterministic seeding: everything in the universe is derived from integer seeds, never stored.

/// SplitMix64 step: a fast, well-mixed 64-bit hash of `x`.
pub const fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Hash several integers into one 64-bit value.
pub fn hash(parts: &[u64]) -> u64 {
    parts
        .iter()
        .fold(0x51_7cc1_b727_220a_u64, |acc, &p| splitmix64(acc ^ p))
}

/// Hash a signed grid coordinate with a seed.
pub fn hash3(x: i32, y: i32, z: i32, seed: u64) -> u64 {
    hash(&[seed, x as u32 as u64, y as u32 as u64, z as u32 as u64])
}

/// Small deterministic generator (xorshift32 seeded through SplitMix64).
#[derive(Clone, Copy, Debug)]
pub struct Rng(u32);
impl Rng {
    pub fn new(seed: u64) -> Self {
        let s = splitmix64(seed) as u32;
        Self(if s == 0 { 0x6d2b_79f5 } else { s })
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform float in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1 << 24) as f32
    }
    /// Uniform float in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    /// Uniform integer in [0, n).
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u32() as u64 * n as u64) >> 32) as u32
    }
    /// Pick an index with the given weights.
    pub fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u32 = weights.iter().sum();
        let mut pick = self.below(total.max(1));
        for (i, &w) in weights.iter().enumerate() {
            if pick < w {
                return i;
            }
            pick -= w;
        }
        weights.len() - 1
    }
}

const LUT_SIZE: usize = 1024;
static SINE: std::sync::OnceLock<[f32; LUT_SIZE]> = std::sync::OnceLock::new();

/// Table sine for per-prop animation and map rotation (no libm call in the inner loops).
pub fn fast_sin(angle: f32) -> f32 {
    let table = SINE.get_or_init(|| {
        std::array::from_fn(|i| (i as f32 / LUT_SIZE as f32 * std::f32::consts::TAU).sin())
    });
    let t = angle / std::f32::consts::TAU * LUT_SIZE as f32;
    table[(t.floor() as i64).rem_euclid(LUT_SIZE as i64) as usize]
}
pub fn fast_cos(angle: f32) -> f32 {
    fast_sin(angle + std::f32::consts::FRAC_PI_2)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeds_are_deterministic_and_spread() {
        assert_eq!(hash3(1, 2, 3, 9), hash3(1, 2, 3, 9));
        assert_ne!(hash3(1, 2, 3, 9), hash3(1, 2, 4, 9));
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let mut buckets = [0; 8];
        for _ in 0..8000 {
            let v = a.unit();
            assert_eq!(v, b.unit());
            assert!((0.0..1.0).contains(&v));
            buckets[(v * 8.0) as usize] += 1;
        }
        assert!(buckets.iter().all(|&n| (850..1150).contains(&n)));
        assert_eq!(Rng::new(5).weighted(&[0, 0, 3]), 2);
    }
    #[test]
    fn table_sine_tracks_the_real_one() {
        for i in 0..100 {
            let a = i as f32 * 0.37 - 15.0;
            assert!((fast_sin(a) - a.sin()).abs() < 0.01);
            assert!((fast_cos(a) - a.cos()).abs() < 0.01);
        }
    }
}
