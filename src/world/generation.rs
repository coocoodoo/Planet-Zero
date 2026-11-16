use super::*;

impl World {
    pub fn generate(kind: MapKind, seed: u32) -> Self {
        let profile = crate::content::maps::definition(kind);
        let mut random = Random(seed.wrapping_mul(7919).wrapping_add(13));
        let mut elevation = vec![0.0_f32; NOISE_SIZE * NOISE_SIZE];
        let mut moisture = vec![0.0_f32; NOISE_SIZE * NOISE_SIZE];
        for &(cell, amp) in &profile.elevation_octaves {
            octave(&mut elevation, cell, amp, &mut random);
        }
        for (cell, amp) in [(128, 1.0), (32, 0.3), (8, 0.08)] {
            octave(&mut moisture, cell, amp, &mut random);
        }
        normalize(&mut elevation);
        normalize(&mut moisture);
        let mut bins = [0_usize; 1024];
        for &e in &elevation {
            bins[(e * 1023.0) as usize] += 1;
        }
        let coverage = profile.water_coverage;
        let mut count = 0;
        let mut sea = 0.5;
        for (i, &n) in bins.iter().enumerate() {
            count += n;
            if count as f32 >= elevation.len() as f32 * coverage {
                sea = i as f32 / 1023.0;
                break;
            }
        }
        let mut lava = vec![false; elevation.len()];
        for y in 0..NOISE_SIZE {
            for x in 0..NOISE_SIZE {
                let at = y * NOISE_SIZE + x;
                let e = elevation[at];
                let m = moisture[at];
                elevation[at] = match kind {
                    MapKind::Islands => (e - sea) / (1.0 - sea) * 59.0,
                    MapKind::Alpine => {
                        let ridge =
                            (e * e * 79.0 + 5.0) * ((m - 0.49).abs() / 0.075).clamp(0.0, 1.0);
                        if ridge < 1.1 { -1.0 } else { ridge }
                    }
                    MapKind::Canyon => {
                        let canyon = ((m - 0.5).abs() - 0.025).max(0.0) / 0.075;
                        if canyon < 0.08 {
                            -1.0
                        } else {
                            (14.0 + (e * 6.0).floor() * 8.0) * canyon.min(1.0)
                        }
                    }
                    MapKind::Desert => {
                        if m > 0.73 && e < 0.4 {
                            -1.0
                        } else {
                            8.0 + e * 19.0
                                + (x as f32 * 0.13 + y as f32 * 0.047 + m * 6.0).sin() * 5.0
                                + (e - 0.64).max(0.0) * 78.0
                        }
                    }
                    MapKind::Wetlands => {
                        if (m - 0.5).abs() < 0.028 {
                            -1.0
                        } else {
                            (e - sea) * 30.0
                        }
                    }
                    MapKind::Volcanic => {
                        let d = ((x as f32 - 532.0).powi(2) + (y as f32 - 508.0).powi(2)).sqrt();
                        let cone = (1.0 - d / 155.0).max(0.0) * 79.0;
                        lava[at] = d < 25.0 || cone > 22.0 && (m - 0.5).abs() < 0.017;
                        if d < 30.0 {
                            27.0 + d * 0.35
                        } else {
                            ((e - 0.38) * 55.0).max(cone)
                        }
                    }
                } * profile.height_scale;
            }
        }
        let mut world = Self {
            heights: vec![0; SIZE * SIZE],
            materials: vec![0; SIZE * SIZE],
            clouds: vec![0; CLOUD_SIZE * CLOUD_SIZE],
            spawn: (2048.0, 2048.0),
            kind,
            seed,
        };
        for y in 0..SIZE {
            let gy = y / CELL;
            let gy1 = (gy + 1) % NOISE_SIZE;
            let ty = (y % CELL) as f32 / CELL as f32;
            for x in 0..SIZE {
                let gx = x / CELL;
                let gx1 = (gx + 1) % NOISE_SIZE;
                let tx = (x % CELL) as f32 / CELL as f32;
                let a = elevation[gy * NOISE_SIZE + gx];
                let b = elevation[gy * NOISE_SIZE + gx1];
                let c = elevation[gy1 * NOISE_SIZE + gx];
                let d = elevation[gy1 * NOISE_SIZE + gx1];
                let h = a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty;
                let at = y * SIZE + x;
                let n = hash(at as u32 ^ seed);
                let level = h.max(0.0).round().min(79.0) as u8;
                world.heights[at] = level;
                world.materials[at] = if h < 0.2 {
                    if kind == MapKind::Wetlands {
                        if n < 15 { 3 } else { 2 }
                    } else if h < -12.0 {
                        0
                    } else if h < -5.0 {
                        1
                    } else if h < -0.6 {
                        2
                    } else {
                        3
                    }
                } else {
                    match kind {
                        MapKind::Islands => {
                            if h < 1.6 {
                                if n < 40 { 5 } else { 4 }
                            } else if h > 45.0 {
                                15 + u8::from(n < 80)
                            } else if h > 28.0 {
                                12 + n % 3
                            } else {
                                6 + u8::from(n < 40) + u8::from(n > 240) * 2
                            }
                        }
                        MapKind::Alpine => {
                            if h > 44.0 {
                                if n < 40 { 26 } else { 15 + u8::from(n < 90) }
                            } else if h > 25.0 {
                                12 + n % 3
                            } else {
                                6 + u8::from(n < 60)
                            }
                        }
                        MapKind::Canyon => {
                            if h < 4.0 {
                                17
                            } else if (level / 5).is_multiple_of(2) {
                                19
                            } else {
                                20
                            }
                        }
                        MapKind::Desert => {
                            if (b - a) + (d - c) > 0.0 {
                                17
                            } else {
                                18
                            }
                        }
                        MapKind::Wetlands => {
                            if h < 1.8 {
                                25
                            } else if n < 70 {
                                6
                            } else {
                                24
                            }
                        }
                        MapKind::Volcanic => {
                            if lava[gy * NOISE_SIZE + gx] {
                                23
                            } else if level > 50 || n < 50 {
                                22
                            } else {
                                21
                            }
                        }
                    }
                };
            }
        }
        if matches!(kind, MapKind::Islands | MapKind::Alpine | MapKind::Wetlands) {
            for gy in (0..SIZE as i32).step_by(6) {
                for gx in (0..SIZE as i32).step_by(6) {
                    let n = hash((gy * 977 + gx * 131) as u32 ^ seed);
                    let cx = gx + (n & 3) as i32;
                    let cy = gy + ((n >> 2) & 3) as i32;
                    let at = index(cx, cy, SIZE);
                    let material = world.materials[at];
                    if !(6..=8).contains(&material) && material != 24 {
                        continue;
                    }
                    if n > profile.tree_threshold {
                        continue;
                    }
                    let base = world.heights[at];
                    let radius = 1 + (n & 1) as i32;
                    for dy in -radius..=radius {
                        for dx in -radius..=radius {
                            let distance = dx.abs() + dy.abs();
                            if distance > radius + 1 {
                                continue;
                            }
                            let cell = index(cx + dx, cy + dy, SIZE);
                            if world.materials[cell] < 6 {
                                continue;
                            }
                            let height = base + 2 + (radius + 1 - distance).max(0) as u8;
                            if height > world.heights[cell] {
                                world.heights[cell] = height;
                                world.materials[cell] = if distance == 0 {
                                    9
                                } else if dx + dy < 0 {
                                    10
                                } else {
                                    11
                                };
                            }
                        }
                    }
                }
            }
        }
        let mut best = -1.0;
        for _ in 0..600 {
            let x = (random.next() * SIZE as f32) as i32;
            let y = (random.next() * SIZE as f32) as i32;
            let base = world.height(x as f32, y as f32);
            let mut score = 0.0;
            for a in 0..8 {
                let angle = a as f32 * std::f32::consts::FRAC_PI_4;
                let at = index(
                    x + (angle.sin() * 160.0) as i32,
                    y - (angle.cos() * 160.0) as i32,
                    SIZE,
                );
                score += (world.heights[at] as f32 - base).abs();
                if world.materials[at] < 4 {
                    score += 4.0;
                }
            }
            if score > best && base < 45.0 {
                best = score;
                world.spawn = (x as f32, y as f32);
            }
        }
        if kind == MapKind::Volcanic {
            world.spawn = (532.0 * CELL as f32 - 180.0, 508.0 * CELL as f32 + 630.0);
        }
        for cluster in 0..profile.cloud_clusters {
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
}
