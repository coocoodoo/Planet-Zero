use crate::flight::Flight;
use crate::render::{mix, rgb};
use crate::world::{SIZE, World, index};

const GREEN: u32 = 0xff99f466;
const AMBER: u32 = 0xff60b0ff;
const INK: u32 = 0xff160f0e;
const MODEL: &[u8] = include_bytes!("../../assets/vehicles/falcon.bin");
const GLYPHS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

struct Layer {
    pixels: Vec<u8>,
    colors: Vec<[u32; 256]>,
}

pub struct Cockpit {
    layers: [Layer; 4],
    pub pixels: Vec<u32>,
    native: Vec<u32>,
    hud: Vec<u32>,
}

impl Cockpit {
    pub fn new() -> Self {
        let sources: [&[u8]; 4] = [
            include_bytes!("../../assets/cockpit/canopy.bin"),
            include_bytes!("../../assets/cockpit/panel.bin"),
            include_bytes!("../../assets/cockpit/deck.bin"),
            include_bytes!("../../assets/cockpit/rear.bin"),
        ];
        Self {
            layers: sources.map(|bytes| {
                let count = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
                let palette: Vec<u32> = bytes[2..2 + count * 4]
                    .chunks_exact(4)
                    .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                    .collect();
                let mut colors = [[0; 256]; 6];
                for (i, &color) in palette.iter().enumerate() {
                    if color == 0 {
                        continue;
                    }
                    colors[0][i] = color;
                    colors[1][i] = mix(color, rgb(255, 231, 178), 0.16);
                    colors[2][i] = color;
                    colors[3][i] = mix(color, rgb(255, 162, 87), 0.16);
                    colors[4][i] = mix(color, rgb(12, 24, 49), 0.45);
                    colors[5][i] = colors[4][i];
                }
                let colors = crate::content::weather::Weather::ALL
                    .into_iter()
                    .flat_map(|weather| {
                        colors.map(|palette| {
                            palette.map(|color| if color == 0 { 0 } else { weather.shade(color) })
                        })
                    })
                    .collect();
                Layer {
                    pixels: bytes[2 + count * 4..].to_vec(),
                    colors,
                }
            }),
            pixels: Vec::new(),
            native: vec![0; 320 * 170],
            hud: vec![0; 320 * 170],
        }
    }

    pub fn prepare(&mut self, flight: &Flight, world: &World, w: usize, h: usize, theme: usize) {
        self.native.fill(0);
        let head = flight.rate * 2.2 + (flight.time * 9.0).sin() * flight.boost * 0.4;
        let lift = flight.pitch.sin() * 5.0;
        let away = 1.0 - ((flight.weights[2] - 0.55) / 0.45).clamp(0.0, 1.0);
        let shifts = [
            (
                -(head * 0.6).round() as i32,
                -(lift * 0.6).round() as i32 - (away * 105.0) as i32,
            ),
            (
                -head.round() as i32,
                -lift.round() as i32 + (away * 90.0) as i32,
            ),
            (
                -(head * 1.6).round() as i32,
                -(lift * 1.6).round() as i32 + (away * 90.0) as i32,
            ),
            (
                -(head * 2.0).round() as i32,
                -(lift * 2.0).round() as i32 - (away * 153.0) as i32,
            ),
        ];
        let sunlight = ((flight.yaw + 0.7).sin() * 75.0 + 160.0) as i32;
        for (layer, &(dx, dy)) in shifts.iter().enumerate() {
            for y in 0..170_i32 {
                let sy = y - dy + 11;
                if !(0..192).contains(&sy) {
                    continue;
                }
                for x in 0..320_i32 {
                    let sx = x - dx + 11;
                    if !(0..342).contains(&sx) {
                        continue;
                    }
                    let color_index =
                        self.layers[layer].pixels[sy as usize * 342 + sx as usize] as usize;
                    if color_index == 0 {
                        continue;
                    }
                    let lit = usize::from(
                        flight.weather == crate::content::weather::Weather::Clear
                            && (x - sunlight + (y - 95) / 2).abs() < 40,
                    );
                    let color = self.layers[layer].colors
                        [flight.weather as usize * 6 + theme * 2 + lit][color_index];
                    if color == 0 {
                        continue;
                    }
                    self.native[y as usize * 320 + x as usize] = color;
                }
            }
            let mut paint = Paint {
                pixels: &mut self.native,
                dx,
                dy,
            };
            if layer == 1 {
                instruments(&mut paint, flight, world);
            }
            if layer == 2 {
                levers(&mut paint, flight);
            }
        }
        self.pixels.resize(w * h, 0);
        if w == 320 && h == 170 {
            self.pixels.copy_from_slice(&self.native);
            return;
        }
        for y in 0..h {
            for x in 0..w {
                self.pixels[y * w + x] = self.native[(y * 170 / h) * 320 + x * 320 / w];
            }
        }
    }

    pub fn glass(&mut self, output: &mut [u32], w: usize, h: usize, flight: &Flight) {
        let (s, c) = (flight.bank + flight.spin).sin_cos();
        for y in 40..96 {
            let half = 49 - (y - 40) / 8;
            for x in 160 - half..=160 + half {
                if w == 320 && h == 170 {
                    let i = y * 320 + x;
                    if self.pixels[i] == 0 {
                        output[i] = mix(
                            output[i],
                            rgb(112, 204, 181),
                            if x == 160 - half || x == 160 + half || y == 40 {
                                0.5
                            } else {
                                0.06
                            },
                        );
                    }
                    continue;
                }
                let sx = x * w / 320;
                let sy = y * h / 170;
                for yy in sy..((y + 1) * h / 170).max(sy + 1).min(h) {
                    for xx in sx..((x + 1) * w / 320).max(sx + 1).min(w) {
                        let i = yy * w + xx;
                        if self.pixels[i] == 0 {
                            output[i] = mix(
                                output[i],
                                rgb(112, 204, 181),
                                if x == 160 - half || x == 160 + half || y == 40 {
                                    0.5
                                } else {
                                    0.06
                                },
                            );
                        }
                    }
                }
            }
        }
        if !flight.hud {
            return;
        }
        self.hud.fill(0);
        let mut p = Paint {
            pixels: &mut self.hud,
            dx: 0,
            dy: 0,
        };
        for side in [-1, 1] {
            p.line(160 + side * 5, 70, 160 + side * 3, 70, GREEN);
            p.line(160 + side * 5, 76, 160 + side * 3, 76, GREEN);
            p.line(160 + side * 5, 70, 160 + side * 5, 72, GREEN);
            p.line(160 + side * 5, 76, 160 + side * 5, 74, GREEN);
            p.line(160 + side * 35, 56, 160 + side * 35, 90, GREEN);
            let value = if side < 0 {
                flight.speed * 0.6
            } else {
                flight.position.z * 0.8
            };
            for tick in 0..12 {
                let y = 56 + ((tick * 3 + (value as i32) % 3) % 35);
                p.line(160 + side * 35, y, 160 + side * 32, y, GREEN);
            }
        }
        for deg in [-20.0_f32, -10.0, 0.0, 10.0, 20.0] {
            let dy = (crate::flight::angle(flight.pitch - deg.to_radians()))
                * 192.0
                * crate::flight::VIEW_SCALE;
            for side in [-1.0, 1.0] {
                let (a, b) = if deg == 0.0 { (9.0, 30.0) } else { (4.0, 11.0) };
                p.line(
                    (160.0 + c * side * a + s * dy) as i32,
                    (73.0 - s * side * a + c * dy) as i32,
                    (160.0 + c * side * b + s * dy) as i32,
                    (73.0 - s * side * b + c * dy) as i32,
                    GREEN,
                );
            }
        }
        p.number(119, 47, (flight.speed * 4.2) as i32, 3, GREEN);
        p.number(185, 47, (flight.position.z * 120.0) as i32, 5, GREEN);
        p.number(
            154,
            86,
            flight.yaw.to_degrees().rem_euclid(360.0) as i32,
            3,
            GREEN,
        );
        for y in 42..94 {
            for x in 114..207 {
                let color = self.hud[y * 320 + x];
                if color == 0 {
                    continue;
                }
                if w == 320 && h == 170 {
                    let i = y * 320 + x;
                    if self.pixels[i] == 0 {
                        output[i] = color;
                    }
                    continue;
                }
                for yy in y * h / 170..((y + 1) * h / 170).min(h) {
                    for xx in x * w / 320..((x + 1) * w / 320).min(w) {
                        let i = yy * w + xx;
                        if self.pixels[i] == 0 {
                            output[i] = color;
                        }
                    }
                }
            }
        }
    }
}

struct Paint<'a> {
    pixels: &'a mut [u32],
    dx: i32,
    dy: i32,
}
impl Paint<'_> {
    fn put(&mut self, x: i32, y: i32, color: u32) {
        let (x, y) = (x + self.dx, y + self.dy);
        if (0..320).contains(&x) && (0..170).contains(&y) {
            self.pixels[y as usize * 320 + x as usize] = color;
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, color);
            }
        }
    }
    fn line(&mut self, x: i32, y: i32, xx: i32, yy: i32, color: u32) {
        let n = (xx - x).abs().max((yy - y).abs()).max(1);
        for t in 0..=n {
            self.put(x + (xx - x) * t / n, y + (yy - y) * t / n, color);
        }
    }
    fn number(&mut self, x: i32, y: i32, value: i32, digits: usize, color: u32) {
        for (i, ch) in format!("{value:0digits$}").bytes().enumerate() {
            if ch.is_ascii_digit() {
                for (yy, row) in GLYPHS[(ch - b'0') as usize].iter().enumerate() {
                    for xx in 0..3 {
                        if row & (4 >> xx) != 0 {
                            self.put(x + i as i32 * 4 + xx, y + yy as i32, color);
                        }
                    }
                }
            }
        }
    }
}

fn instruments(p: &mut Paint<'_>, f: &Flight, world: &World) {
    let crt = [
        rgb(3, 16, 17),
        rgb(5, 27, 25),
        rgb(13, 56, 41),
        rgb(24, 91, 54),
        rgb(71, 158, 82),
        rgb(169, 242, 142),
    ];
    let (sy, cy) = f.yaw.sin_cos();
    let (sweep_s, sweep_c) = (f.time * 1.6).sin_cos();
    for y in 0..29 {
        for x in 0..49 {
            let dx = (x - 24) as f32;
            let dy = (14 - y) as f32;
            let at = index(
                (f.position.x + (dx * cy + dy * sy) * 15.0) as i32,
                (f.position.y + (dx * sy - dy * cy) * 15.0) as i32,
                SIZE,
            );
            let material = if world.kind == crate::world::MapKind::Space {
                0
            } else {
                world.materials[at]
            };
            let mut tone = if world.kind == crate::world::MapKind::Moon {
                1 + (world.heights[at] as usize / 14).min(3)
            } else if material >= 6 {
                3
            } else if material >= 4 {
                2
            } else {
                y as usize & 1
            };
            let d = dx.hypot(dy);
            if (d - 4.5).abs() < 0.45
                || (d - 9.0).abs() < 0.45
                || (d - 13.5).abs() < 0.45
                || x == 24
                || y == 14
            {
                tone = tone.max(2);
            }
            if d < 14.0
                && (dx * sweep_c - dy * sweep_s).abs() < 0.7
                && dx * sweep_s + dy * sweep_c > 0.0
            {
                tone = 5;
            }
            p.put(47 + x, 112 + y, crt[tone]);
        }
    }
    if world.kind.vacuum() {
        for object in &world.scenery {
            let d = object.position - f.position;
            let x = ((d.x * cy + d.y * sy) / 15.0).round() as i32;
            let y = -((d.x * sy - d.y * cy) / 15.0).round() as i32;
            if x.abs() < 23 && y.abs() < 13 {
                p.rect(71 + x, 126 + y, 2, 2, crt[4]);
            }
        }
        for portal in &world.portals {
            let d = portal.center - f.position;
            let x = ((d.x * cy + d.y * sy) / 15.0).round() as i32;
            let y = -((d.x * sy - d.y * cy) / 15.0).round() as i32;
            if x.abs() < 22 && y.abs() < 12 {
                p.rect(70 + x, 125 + y, 3, 3, rgb(105, 211, 242));
                p.put(71 + x, 126 + y, crt[0]);
            }
        }
    }
    for wing in f.wingmen.iter().take(f.wing_count) {
        let d = wing.position - f.position;
        p.rect(
            71 + ((d.x * cy + d.y * sy) / 15.0).round() as i32,
            126 - ((d.x * sy - d.y * cy) / 15.0).round() as i32,
            2,
            2,
            crt[5],
        );
    }
    p.line(69, 127, 71, 124, crt[5]);
    p.line(71, 124, 73, 127, crt[5]);
    for y in 0..29 {
        p.rect(224, 112 + y, 49, 1, crt[y as usize & 1]);
    }
    for y in 0..20 {
        for x in 0..15 {
            let mx = x * 34 / 15;
            let my = y * 46 / 20;
            if (0..13).any(|z| MODEL[(z * 46 + my) * 34 + mx] != 0) {
                p.put(232 + x as i32, 117 + y as i32, crt[4]);
            }
        }
    }
    for engine in 0..2 {
        p.rect(255 + engine * 8, 118, 5, 14, crt[2]);
        let level = (7.0 + f.boost * 6.0) as i32;
        for y in 0..level {
            if y % 3 != 2 {
                p.rect(
                    256 + engine * 8,
                    130 - y,
                    3,
                    1,
                    if y > 11 { AMBER } else { crt[4] },
                );
            }
        }
    }
    p.number(227, 114, (f.position.z * 120.0) as i32, 5, crt[4]);
    p.number(227, 135, (f.speed * 4.2) as i32, 3, crt[4]);
    for i in 0..5 {
        if i < 2 || f.boost > 0.25 || ((f.time * 2.0) as usize + i).is_multiple_of(3) {
            p.rect(143 + i as i32 * 7, 105, 6, 3, AMBER);
            p.rect(143 + i as i32 * 7, 105, 6, 1, rgb(255, 222, 155));
        }
    }
    let (s, c) = (f.bank + f.spin).sin_cos();
    for y in -14..=14 {
        for x in -14..=14 {
            let d = x * x + y * y;
            if d > 196 {
                continue;
            }
            let yy = s * x as f32 + c * y as f32 - f.pitch.sin() * 23.0;
            let xx = c * x as f32 - s * y as f32;
            let mut color = if yy > 0.0 {
                rgb(153, 82, 38)
            } else {
                rgb(70, 157, 201)
            };
            if yy.abs() < 0.6 || ((yy / 4.0).round() * 4.0 - yy).abs() < 0.5 && xx.abs() < 4.0 {
                color = rgb(225, 232, 215);
            }
            p.put(
                160 + x,
                129 + y,
                mix(
                    color,
                    INK,
                    ((d as f32 / 196.0 - 0.5).max(0.0) * 0.8).min(0.65),
                ),
            );
        }
    }
    p.line(151, 129, 157, 129, AMBER);
    p.line(163, 129, 169, 129, AMBER);
    p.rect(159, 128, 3, 3, AMBER);
    let needles = [
        -225.0 + f.speed / 166.0 * 270.0,
        -225.0 + f.position.z / 300.0 * 270.0,
        -90.0 + f.pitch.sin() * 240.0,
        -90.0 + f.rate / 1.25 * 120.0,
    ];
    for ((x, y), degrees) in [(119, 120), (119, 139), (201, 120), (201, 139)]
        .into_iter()
        .zip(needles)
    {
        let (s, c) = degrees.to_radians().sin_cos();
        p.line(x, y, x + (c * 4.0) as i32, y + (s * 4.0) as i32, AMBER);
        p.put(x, y, rgb(210, 217, 226));
    }
}

fn levers(p: &mut Paint<'_>, f: &Flight) {
    let y = 162 - (f.boost * 9.0) as i32;
    p.rect(71, y + 3, 2, 20, rgb(112, 118, 134));
    p.rect(81, y + 3, 2, 20, rgb(112, 118, 134));
    p.rect(68, y - 1, 24, 7, INK);
    p.rect(69, y, 22, 5, rgb(240, 88, 14));
    p.rect(70, y, 20, 2, AMBER);
    let top = 143 - (f.pitch.sin() * 6.0) as i32;
    for y in top..181 {
        let d = y - top;
        let x = 160 + (f.rate / 1.25 * 5.0 * (1.0 - d as f32 / 50.0)) as i32;
        let half = if d < 8 {
            6
        } else if d < 18 {
            4
        } else {
            7 + (d - 18) / 3 * 2
        };
        p.rect(x - half - 1, y, half * 2 + 3, 1, INK);
        let color = if d < 2 {
            rgb(150, 156, 170)
        } else if d < 8 {
            rgb(61, 65, 80)
        } else if d % 3 == 0 {
            rgb(29, 31, 42)
        } else {
            rgb(84, 89, 105)
        };
        p.rect(x - half, y, half * 2 + 1, 1, color);
        if d == 4 || d == 5 {
            p.rect(x + 2, y, 3, 1, AMBER);
            p.rect(x - 4, y, 3, 1, rgb(22, 26, 33));
        }
    }
}
