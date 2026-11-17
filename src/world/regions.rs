use super::*;

impl World {
    pub(super) fn region(kind: MapKind, seed: u32) -> Self {
        let mut random = Random(seed);
        let mut elevation = vec![0.0; NOISE_SIZE * NOISE_SIZE];
        let mut detail = vec![0.0; NOISE_SIZE * NOISE_SIZE];
        for &(cell, amp) in &crate::content::maps::definition(kind).elevation_octaves {
            octave(&mut elevation, cell, amp, &mut random);
        }
        octave(&mut detail, 64, 1.0, &mut random);
        octave(&mut detail, 16, 0.3, &mut random);
        normalize(&mut elevation);
        normalize(&mut detail);
        for y in 0..NOISE_SIZE {
            for x in 0..NOISE_SIZE {
                let at = y * NOISE_SIZE + x;
                let e = elevation[at];
                let m = detail[at];
                let phase = y as f32 * std::f32::consts::TAU / NOISE_SIZE as f32;
                elevation[at] = match kind {
                    MapKind::Fjords => {
                        let river = 512.0 + phase.sin() * 130.0 + (phase * 3.0).sin() * 35.0;
                        let distance = (x as f32 - river).abs();
                        let channel = (distance - 60.0 - 30.0 * (phase * 2.0).cos()) / 80.0;
                        if channel < 0.0 {
                            -18.0
                        } else {
                            (channel.min(1.0).powf(0.6) * (65.0 + e * 120.0)).min(190.0)
                        }
                    }
                    MapKind::Icebergs => {
                        if e < 0.52 {
                            -14.0
                        } else {
                            9.0 + ((e - 0.52) * 25.0).floor() * 7.0 + (m - 0.5) * 3.0
                        }
                    }
                    MapKind::City => 3.0 + (e * 3.0).floor(),
                    MapKind::Highlands => {
                        14.0 + e.powi(2) * 156.0 * ((m - 0.5).abs() / 0.12).clamp(0.1, 1.0)
                    }
                    _ => unreachable!(),
                };
            }
        }
        let mut world = Self {
            heights: vec![0; SIZE * SIZE],
            materials: vec![0; SIZE * SIZE],
            clouds: vec![0; CLOUD_SIZE * CLOUD_SIZE],
            obstacles: vec![],
            ceiling: 0,
            spawn: if kind == MapKind::City {
                (2304.0, 3264.0)
            } else {
                if kind == MapKind::Fjords {
                    (1665.0, 3100.0)
                } else {
                    (2048.0, 3100.0)
                }
            },
            kind,
            seed,
            scenery: vec![],
            portals: vec![],
        };
        for y in 0..SIZE {
            for x in 0..SIZE {
                let (gx, gy) = (x as i32 / CELL as i32, y as i32 / CELL as i32);
                let tx = (x % CELL) as f32 / CELL as f32;
                let ty = (y % CELL) as f32 / CELL as f32;
                let a = elevation[index(gx, gy, NOISE_SIZE)];
                let b = elevation[index(gx + 1, gy, NOISE_SIZE)];
                let c = elevation[index(gx, gy + 1, NOISE_SIZE)];
                let d = elevation[index(gx + 1, gy + 1, NOISE_SIZE)];
                let h = a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty;
                let at = y * SIZE + x;
                let n = hash(at as u32 ^ seed);
                world.heights[at] = h.clamp(0.0, 220.0) as u8;
                world.materials[at] = if h < 0.0 {
                    if kind == MapKind::Icebergs { 0 } else { 1 }
                } else {
                    match kind {
                        MapKind::Fjords => {
                            if h > 160.0 {
                                15
                            } else if (b - a + c - a).abs() > 1.5 || h > 140.0 {
                                12 + n % 3
                            } else {
                                6
                            }
                        }
                        MapKind::Icebergs => {
                            if b - a + c - a > 0.2 {
                                26
                            } else if n < 28 {
                                16
                            } else {
                                15
                            }
                        }
                        MapKind::Highlands => {
                            if h > 135.0 {
                                15
                            } else if h > 90.0 {
                                12 + n % 3
                            } else {
                                6 + u8::from(n < 70)
                            }
                        }
                        MapKind::City => {
                            let xx = x % 192;
                            let yy = y % 192;
                            let road = xx < 12 || yy < 12;
                            if road {
                                if (xx == 5 && y % 24 < 12) || (yy == 5 && x % 24 < 12) {
                                    31
                                } else {
                                    30
                                }
                            } else if xx < 20 || yy < 20 {
                                32
                            } else if (x / 192 + y / 192) % 5 != 0 {
                                33
                            } else {
                                6
                            }
                        }
                        _ => unreachable!(),
                    }
                };
            }
        }
        world.add_landmarks();
        world.update_ceiling();
        world
    }
}
