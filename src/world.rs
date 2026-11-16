pub const SIZE: usize = 2048;
pub const CLOUD_SIZE: usize = 1024;
pub const CLOUD_ALT: usize = 40;
pub const MAX_HEIGHT: usize = 48;

pub struct World {
    pub heights: Vec<u8>,
    pub materials: Vec<u8>,
    pub clouds: Vec<u8>,
    pub spawn: (f32, f32),
}

struct Random(u32);

impl Random {
    fn next(&mut self) -> f32 {
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

fn octave(field: &mut [f32], cell: usize, amplitude: f32, random: &mut Random) {
    let count = SIZE / cell;
    let lattice: Vec<f32> = (0..count * count).map(|_| random.next()).collect();
    for y in 0..SIZE {
        let y0 = y / cell;
        let y1 = (y0 + 1) % count;
        let f = (y % cell) as f32 / cell as f32;
        let ty = f * f * (3.0 - 2.0 * f);
        for x in 0..SIZE {
            let x0 = x / cell;
            let x1 = (x0 + 1) % count;
            let f = (x % cell) as f32 / cell as f32;
            let tx = f * f * (3.0 - 2.0 * f);
            let a = lattice[y0 * count + x0];
            let b = lattice[y0 * count + x1];
            let c = lattice[y1 * count + x0];
            let d = lattice[y1 * count + x1];
            field[y * SIZE + x] +=
                (a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty) * amplitude;
        }
    }
}

impl World {
    pub fn new(seed: u32) -> Self {
        let mut random = Random(seed.wrapping_mul(7919).wrapping_add(13));
        for _ in 0..65536 {
            random.next();
        }
        let mut elevation = vec![0.0_f32; SIZE * SIZE];
        for (cell, amp) in [
            (512, 0.55),
            (256, 1.0),
            (128, 0.5),
            (64, 0.25),
            (32, 0.11),
            (16, 0.05),
        ] {
            octave(&mut elevation, cell, amp, &mut random);
        }
        let low = elevation.iter().copied().fold(f32::INFINITY, f32::min);
        let high = elevation.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut bins = [0_usize; 1024];
        for e in &mut elevation {
            *e = (*e - low) / (high - low);
            bins[(*e * 1023.0) as usize] += 1;
        }
        let target = (SIZE * SIZE) as f32 * 0.67;
        let mut accumulated = 0;
        let mut sea = 0.5;
        for (b, count) in bins.iter().enumerate() {
            accumulated += count;
            if accumulated as f32 >= target {
                sea = (b + 1) as f32 / 1023.0;
                break;
            }
        }
        let span = (1.0 - sea - 0.012).max(0.05);
        let mut world = Self {
            heights: vec![0; SIZE * SIZE],
            materials: vec![0; SIZE * SIZE],
            clouds: vec![0; CLOUD_SIZE * CLOUD_SIZE],
            spawn: (1024.0, 1024.0),
        };
        for y in 0..SIZE as i32 {
            for x in 0..SIZE as i32 {
                let i = index(x, y, SIZE);
                let d = elevation[i] - sea;
                let noise = hash(i as u32);
                if d < 0.0 {
                    let depth = -d + (noise as f32 - 128.0) * 0.00004;
                    world.materials[i] = if depth > 0.07 {
                        0
                    } else if depth > 0.03 {
                        1
                    } else if depth > 0.008 {
                        2
                    } else {
                        3
                    };
                } else if d < 0.012 {
                    world.materials[i] = if noise < 34 { 5 } else { 4 };
                } else {
                    let h = (d - 0.012) / span;
                    let level = if h < 0.5 {
                        1 + (h * 16.0) as u8
                    } else if h < 0.78 {
                        9 + ((h - 0.5) / 0.07) as u8
                    } else {
                        13 + ((h - 0.78) / 0.09) as u8
                    }
                    .min(15);
                    world.heights[i] = if level <= 8 {
                        level * 2
                    } else {
                        16 + (level - 8) * 3
                    };
                    let gradient =
                        elevation[index(x - 2, y - 2, SIZE)] - elevation[index(x + 2, y + 2, SIZE)];
                    world.materials[i] = if level >= 13 {
                        if gradient > 0.007 { 16 } else { 15 }
                    } else if level >= 9 {
                        if gradient < -0.007 {
                            14
                        } else if gradient > 0.007 || noise < 26 {
                            13
                        } else {
                            12
                        }
                    } else if gradient < -0.007 {
                        8
                    } else if gradient > 0.007 || noise < 22 {
                        7
                    } else if noise > 240 {
                        8
                    } else {
                        6
                    };
                }
            }
        }
        for gy in (0..SIZE as i32).step_by(5) {
            for gx in (0..SIZE as i32).step_by(5) {
                let noise = hash((gy * 977 + gx * 131 + 7) as u32);
                let cx = gx + (noise & 3) as i32;
                let cy = gy + ((noise >> 2) & 3) as i32;
                let i = index(cx, cy, SIZE);
                let h0 = world.heights[i];
                if !(6..=8).contains(&world.materials[i]) || h0 > 14 {
                    continue;
                }
                let density =
                    (elevation[index(SIZE as i32 - 1 - cx + 911, cy + 613, SIZE)] - sea) * 30.0;
                if density <= 0.0 || density * 255.0 < hash(i as u32 + 99) as f32 {
                    continue;
                }
                let radius = 2 + ((noise >> 4) & 1) as i32;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let distance = ((dx * dx + dy * dy) as f32).sqrt();
                        if distance > radius as f32 + 0.3 {
                            continue;
                        }
                        let at = index(cx + dx, cy + dy, SIZE);
                        if world.materials[at] < 4 {
                            continue;
                        }
                        let height = h0
                            + 1
                            + ((1.0 - distance / (radius as f32 + 0.5)) * (radius + 1) as f32)
                                .round() as u8;
                        if height > world.heights[at] {
                            world.heights[at] = height;
                            world.materials[at] = if distance < 1.1 {
                                9
                            } else if distance < radius as f32 - 0.5 {
                                10
                            } else {
                                11
                            };
                        }
                    }
                }
            }
        }
        let mut best = -1;
        for _ in 0..400 {
            let x = (random.next() * SIZE as f32) as i32;
            let y = (random.next() * SIZE as f32) as i32;
            if world.materials[index(x, y, SIZE)] > 2 {
                continue;
            }
            let nearby = (0..8)
                .filter(|a| {
                    let angle = *a as f32 * std::f32::consts::FRAC_PI_4;
                    world.materials[index(
                        x + (angle.cos() * 120.0) as i32,
                        y + (angle.sin() * 120.0) as i32,
                        SIZE,
                    )] >= 4
                })
                .count() as i32;
            if nearby > best && nearby <= 5 {
                best = nearby;
                world.spawn = (x as f32, y as f32);
            }
        }
        for cluster in 0..200 {
            let rank = (cluster & 15) as u8;
            let cx = random.next() * CLOUD_SIZE as f32;
            let cy = random.next() * CLOUD_SIZE as f32;
            let scale = 0.6 + random.next() * random.next() * 1.25;
            let angle = 0.35 + (random.next() - 0.5) * 1.3;
            let length = (12.0 + random.next() * 24.0) * scale;
            let puffs = 4 + (random.next() * 7.0) as i32;
            for puff in 0..puffs {
                let t = puff as f32 / (puffs - 1) as f32 * 2.0 - 1.0;
                let px = cx + angle.cos() * t * length + (random.next() - 0.5) * 9.0 * scale;
                let py = cy + angle.sin() * t * length + (random.next() - 0.5) * 9.0 * scale;
                let r = (6.0 + random.next() * 8.0) * scale * (1.0 - 0.45 * t.abs());
                let thickness = (r * 0.6).round().clamp(2.0, 8.0);
                let extent = r.ceil() as i32;
                for dy in -extent..=extent {
                    for dx in -extent..=extent {
                        let distance = (dx * dx + dy * dy) as f32 / (r * r);
                        if distance > 1.0 {
                            continue;
                        }
                        let height = ((1.0 - distance).sqrt() * thickness).round().max(1.0) as u8;
                        let at = index(px.round() as i32 + dx, py.round() as i32 + dy, CLOUD_SIZE);
                        if height > world.clouds[at] & 15 {
                            world.clouds[at] = (rank << 4) | height;
                        }
                    }
                }
            }
        }
        world
    }

    pub fn height(&self, x: f32, y: f32) -> f32 {
        self.heights[index(x.floor() as i32, y.floor() as i32, SIZE)] as f32
    }

    pub fn cloud(&self, x: f32, y: f32, time: f32) -> usize {
        let cell = self.clouds[index(
            (x - time * 7.0).floor() as i32,
            (y - time * 3.0).floor() as i32,
            CLOUD_SIZE,
        )];
        if cell >> 4 < 7 {
            (cell & 15) as usize
        } else {
            0
        }
    }
}
