use super::*;
pub(super) struct Random(pub(super) u32);
impl Random {
    pub(super) fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let mut t = (self.0 ^ (self.0 >> 15)).wrapping_mul(1 | self.0);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        ((t ^ (t >> 14)) as f64 / 4294967296.0) as f32
    }
}

pub fn hash(n: u32) -> u8 {
    let mut h = (n ^ (n >> 13)).wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    (h >> 24) as u8
}

pub fn index(x: i32, y: i32, size: usize) -> usize {
    (y as usize & (size - 1)) * size + (x as usize & (size - 1))
}

pub(super) fn octave(field: &mut [f32], cell: usize, amplitude: f32, random: &mut Random) {
    let count = NOISE_SIZE / cell;
    let lattice: Vec<f32> = (0..count * count).map(|_| random.next()).collect();
    for y in 0..NOISE_SIZE {
        let y0 = y / cell;
        let y1 = (y0 + 1) % count;
        let f = (y % cell) as f32 / cell as f32;
        let ty = f * f * (3.0 - 2.0 * f);
        for x in 0..NOISE_SIZE {
            let x0 = x / cell;
            let x1 = (x0 + 1) % count;
            let f = (x % cell) as f32 / cell as f32;
            let tx = f * f * (3.0 - 2.0 * f);
            let a = lattice[y0 * count + x0];
            let b = lattice[y0 * count + x1];
            let c = lattice[y1 * count + x0];
            let d = lattice[y1 * count + x1];
            field[y * NOISE_SIZE + x] +=
                (a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty) * amplitude;
        }
    }
}

pub(super) fn normalize(field: &mut [f32]) {
    let low = field.iter().copied().fold(f32::INFINITY, f32::min);
    let high = field.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    for v in field {
        *v = (*v - low) / (high - low);
    }
}
