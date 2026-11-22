use crate::content::vehicles::Vehicle;
use crate::flight::Flight;
use crate::render::{mix, rgb};
use crate::world::{SIZE, World, index};

const HUD: u32 = 0xffdaed6f;
const AMBER: u32 = 0xff60b0ff;
const INK: u32 = 0xff160f0e;
const MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/falcon.bin"));
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
    runs: Vec<Vec<(usize, usize)>>,
}

#[derive(PartialEq, Eq)]
struct BaseKey {
    shifts: [(i32, i32); 3],
    palette: usize,
    weather: [u8; 8],
}
pub struct Cockpit {
    layers: [Layer; 3],
    vehicle: Vehicle,
    pub pixels: Vec<u32>,
    native: Vec<u32>,
    base: Vec<u32>,
    lit: Vec<u32>,
    owner: Vec<u8>,
    base_key: Option<BaseKey>,
    pub(super) revision: u64,
    hud: Vec<u32>,
}

impl Cockpit {
    pub fn new() -> Self {
        Self {
            layers: Self::load_layers(Vehicle::Falcon),
            vehicle: Vehicle::Falcon,
            pixels: Vec::new(),
            native: Vec::new(),
            base: Vec::new(),
            lit: Vec::new(),
            owner: Vec::new(),
            base_key: None,
            revision: 0,
            hud: Vec::new(),
        }
    }

    fn load_layers(vehicle: Vehicle) -> [Layer; 3] {
        macro_rules! assets {
            ($name:literal) => {
                [
                    include_bytes!(concat!(env!("OUT_DIR"), "/", $name, "-canopy.bin")) as &[u8],
                    include_bytes!(concat!(env!("OUT_DIR"), "/", $name, "-panel.bin")),
                    include_bytes!(concat!(env!("OUT_DIR"), "/", $name, "-deck.bin")),
                ]
            };
        }
        let sources = match vehicle {
            Vehicle::Falcon => assets!("falcon"),
            Vehicle::Dart => assets!("dart"),
            Vehicle::Atlas => assets!("atlas"),
            Vehicle::Wasp => assets!("wasp"),
            Vehicle::Paper => assets!("paper"),
        };
        sources.map(|bytes| {
            let count = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
            let palette: Vec<u32> = bytes[2..2 + count * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|v| u32::from_le_bytes(*v))
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
            let pixels: Vec<u8> = bytes[2 + count * 4..]
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|run| {
                    std::iter::repeat_n(run[2], u16::from_le_bytes([run[0], run[1]]) as usize)
                })
                .collect();
            let runs = pixels
                .as_chunks::<342>()
                .0
                .iter()
                .map(|row| {
                    let mut runs = Vec::new();
                    let mut x = 0;
                    while x < row.len() {
                        if palette[row[x] as usize] == 0 {
                            x += 1;
                            continue;
                        }
                        let start = x;
                        while x < row.len() && palette[row[x] as usize] != 0 {
                            x += 1;
                        }
                        runs.push((start, x));
                    }
                    runs
                })
                .collect();
            Layer {
                pixels,
                colors,
                runs,
            }
        })
    }

    pub(super) fn opaque(&self, x: usize, y: usize) -> bool {
        self.base[y * 320 + x] != 0
    }

    pub fn prepare(
        &mut self,
        flight: &Flight,
        world: &World,
        size: (usize, usize),
        theme: usize,
        network: Option<&crate::world::network::Network>,
    ) {
        if self.vehicle != flight.vehicle {
            self.layers = Self::load_layers(flight.vehicle);
            self.vehicle = flight.vehicle;
            self.base_key = None;
        }
        let (w, h) = size;
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
        ];
        let key = BaseKey {
            shifts,
            palette: theme * 2,
            weather: crate::content::weather::Weather::ALL
                .map(|w| (flight.weather.weight(w) * 64.0).round() as u8),
        };
        self.native.resize(320 * 170, 0);
        if self.base_key.as_ref() != Some(&key) {
            self.base.resize(320 * 170, 0);
            self.lit.resize(320 * 170, 0);
            self.owner.resize(320 * 170, 0);
            self.base.fill(0);
            self.lit.fill(0);
            self.owner.fill(0);
            for (layer, &(dx, dy)) in shifts.iter().enumerate() {
                let source = &self.layers[layer];
                let base: [u32; 256] = std::array::from_fn(|i| {
                    flight
                        .weather
                        .color(|w| source.colors[w as usize * 6 + key.palette][i])
                });
                let lit: [u32; 256] = std::array::from_fn(|i| {
                    flight
                        .weather
                        .color(|w| source.colors[w as usize * 6 + key.palette + 1][i])
                });
                for y in 0..170_i32 {
                    let sy = y - dy + 11;
                    if !(0..192).contains(&sy) {
                        continue;
                    }
                    for &(start, end) in &source.runs[sy as usize] {
                        let x0 = (start as i32 + dx - 11).max(0);
                        let x1 = (end as i32 + dx - 11).min(320);
                        for x in x0..x1 {
                            let sx = x - dx + 11;
                            let index = source.pixels[sy as usize * 342 + sx as usize] as usize;
                            let at = y as usize * 320 + x as usize;
                            self.base[at] = base[index];
                            self.lit[at] = lit[index];
                            self.owner[at] = layer as u8 + 1;
                        }
                    }
                }
            }
            self.base_key = Some(key);
            self.revision += 1;
        }
        self.native.copy_from_slice(&self.base);
        let sunlight_weight = flight
            .weather
            .weight(crate::content::weather::Weather::Clear);
        if sunlight_weight > 0.0 && theme < 2 {
            let sunlight = ((flight.yaw + 0.7).sin() * 75.0 + 160.0) as i32;
            for y in 0..170_i32 {
                let center = sunlight - (y - 95) / 2;
                let start = (center - 39).clamp(0, 320) as usize;
                let end = (center + 40).clamp(0, 320) as usize;
                let row = y as usize * 320;
                for at in row + start..row + end {
                    if self.base[at] != 0 {
                        self.native[at] = mix(self.base[at], self.lit[at], sunlight_weight);
                    }
                }
            }
        }
        for layer in [1, 2] {
            let (dx, dy) = shifts[layer];
            let mut paint = Paint {
                pixels: &mut self.native,
                dx,
                dy,
                occlusion: Some((&self.owner, layer as u8 + 1)),
            };
            if layer == 1 {
                instruments(&mut paint, flight, world, network);
            } else {
                if flight.vehicle == Vehicle::Falcon {
                    levers(&mut paint, flight);
                } else if flight.vehicle == Vehicle::Paper {
                    paint.number(108, 164, flight.speed as i32, 3, rgb(83, 103, 94));
                    paint.number(207, 164, flight.position.z as i32, 3, rgb(83, 103, 94));
                }
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
        let (hud, tint) = match flight.vehicle {
            Vehicle::Falcon => (HUD, rgb(112, 204, 181)),
            Vehicle::Dart => (rgb(132, 230, 250), rgb(92, 173, 214)),
            Vehicle::Atlas => (rgb(242, 204, 125), rgb(188, 159, 95)),
            Vehicle::Wasp => (rgb(164, 237, 139), rgb(124, 193, 136)),
            Vehicle::Paper => (rgb(128, 150, 133), rgb(0, 0, 0)),
        };
        if flight.vehicle != Vehicle::Paper {
            for y in 40..96 {
                let half = 49 - (y - 40) / 8;
                for x in 160 - half..=160 + half {
                    if w == 320 && h == 170 {
                        let i = y * 320 + x;
                        if self.pixels[i] == 0 {
                            output[i] = mix(
                                output[i],
                                tint,
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
                                    tint,
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
        }
        if !flight.hud {
            return;
        }
        self.hud.resize(320 * 170, 0);
        // 只读取准星区域（含一圈描边邻居），区域外的残留永远不会被用到
        for row in self.hud[41 * 320..95 * 320].as_chunks_mut::<320>().0 {
            row[113..208].fill(0);
        }
        let mut p = Paint {
            pixels: &mut self.hud,
            occlusion: None,
            dx: 0,
            dy: 0,
        };
        for side in [-1, 1] {
            p.line(160 + side * 5, 70, 160 + side * 3, 70, hud);
            p.line(160 + side * 5, 76, 160 + side * 3, 76, hud);
            p.line(160 + side * 5, 70, 160 + side * 5, 72, hud);
            p.line(160 + side * 5, 76, 160 + side * 5, 74, hud);
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
                    hud,
                );
            }
        }
        for y in 42..94 {
            for x in 114..207 {
                let color = self.hud[y * 320 + x];
                let color = if color != 0 {
                    color
                } else if [
                    y * 320 + x - 1,
                    y * 320 + x + 1,
                    (y - 1) * 320 + x,
                    (y + 1) * 320 + x,
                ]
                .iter()
                .any(|&at| self.hud[at] != 0)
                {
                    rgb(3, 8, 13)
                } else {
                    continue;
                };
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
    occlusion: Option<(&'a [u8], u8)>,
    dx: i32,
    dy: i32,
}
impl Paint<'_> {
    fn put(&mut self, x: i32, y: i32, color: u32) {
        let (x, y) = (x + self.dx, y + self.dy);
        if (0..320).contains(&x) && (0..170).contains(&y) {
            let at = y as usize * 320 + x as usize;
            if self
                .occlusion
                .is_none_or(|(owner, layer)| owner[at] <= layer)
            {
                self.pixels[at] = color;
            }
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

fn instruments(
    p: &mut Paint<'_>,
    f: &Flight,
    world: &World,
    network: Option<&crate::world::network::Network>,
) {
    if f.vehicle == Vehicle::Falcon {
        falcon_instruments(p, f, world, network);
        return;
    }
    let (x, y, accent) = match f.vehicle {
        Vehicle::Dart => (42, 129, rgb(98, 216, 245)),
        Vehicle::Atlas => (36, 127, rgb(236, 183, 86)),
        Vehicle::Wasp => (62, 137, rgb(148, 226, 126)),
        Vehicle::Paper => return,
        Vehicle::Falcon => unreachable!(),
    };
    let crt = [
        rgb(3, 12, 17),
        mix(INK, accent, 0.08),
        mix(INK, accent, 0.2),
        mix(INK, accent, 0.4),
        mix(INK, accent, 0.65),
        accent,
    ];
    p.dx += x - 47;
    p.dy += y - 112;
    radar(p, f, world, network, crt);
    p.dx -= x - 47;
    p.dy -= y - 112;
    match f.vehicle {
        Vehicle::Dart => {
            horizon(p, f, (160, 128), (47, 11), false, accent);
            p.line(114, 141, 205, 141, crt[3]);
            p.number(115, 146, f.speed as i32, 3, accent);
            p.number(177, 146, f.position.z as i32, 4, accent);
            for engine in 0..2 {
                let x = 236 + engine * 19;
                let fill = (17.0 * (0.55 + f.boost * 0.45)) as i32;
                p.rect(x, 135, 9, 18, crt[1]);
                p.rect(x, 153 - fill, 9, fill, accent);
                for y in (135..153).step_by(4) {
                    p.line(x, y, x + 8, y, INK);
                }
            }
        }
        Vehicle::Atlas => {
            horizon(p, f, (160, 127), (16, 16), true, accent);
            gauge(p, (122, 129), 13, f.speed / 160.0, accent);
            gauge(p, (199, 129), 13, f.position.z / 600.0, accent);
            gauge(p, (121, 157), 6, 0.6 + f.boost * 0.4, accent);
            gauge(
                p,
                (199, 157),
                6,
                (f.rate / f.vehicle.turn() + 1.0) * 0.5,
                accent,
            );
            p.number(113, 138, f.speed as i32, 3, accent);
            p.number(190, 138, f.position.z as i32, 4, accent);
            for engine in 0..4 {
                let x = 240 + engine * 10;
                let fill = (15.0 * (0.6 + f.boost * 0.4)) as i32;
                p.rect(x, 134, 6, 17, crt[1]);
                p.rect(x, 151 - fill, 6, fill, crt[4]);
                p.put(x + 2, 131, accent);
            }
            p.number(246, 153, (f.speed * 4.2) as i32, 3, accent);
        }
        Vehicle::Wasp => {
            horizon(p, f, (160, 138), (31, 10), false, accent);
            p.number(129, 153, f.speed as i32, 3, accent);
            p.number(177, 153, f.position.z as i32, 4, accent);
            for rotor in 0..2 {
                gauge(p, (221 + rotor * 24, 148), 7, 0.65 + f.boost * 0.3, accent);
            }
            p.number(213, 159, (85.0 + f.boost * 15.0) as i32, 3, accent);
            p.number(239, 159, (85.0 + f.boost * 15.0) as i32, 3, accent);
        }
        Vehicle::Falcon | Vehicle::Paper => unreachable!(),
    }
}

fn horizon(
    p: &mut Paint<'_>,
    f: &Flight,
    center: (i32, i32),
    half: (i32, i32),
    round: bool,
    accent: u32,
) {
    let (s, c) = (f.bank + f.spin).sin_cos();
    let pitch = f.pitch.sin() * 19.0;
    for y in -half.1..=half.1 {
        for x in -half.0..=half.0 {
            if round && x * x + y * y > half.0 * half.0 {
                continue;
            }
            let height = s * x as f32 + c * y as f32 - pitch;
            let color = if height.abs() < 0.7 {
                accent
            } else if height > 0.0 {
                rgb(91, 66, 42)
            } else {
                rgb(24, 75, 98)
            };
            p.put(center.0 + x, center.1 + y, color);
        }
    }
    for side in [-1, 1] {
        p.line(
            center.0 + side * 3,
            center.1,
            center.0 + side * 10,
            center.1,
            INK,
        );
        p.line(
            center.0 + side * 3,
            center.1 - 1,
            center.0 + side * 10,
            center.1 - 1,
            accent,
        );
    }
    p.rect(center.0 - 1, center.1 - 1, 3, 3, INK);
}

fn gauge(p: &mut Paint<'_>, center: (i32, i32), radius: i32, value: f32, accent: u32) {
    for step in 0..7 {
        let (s, c) = (-225.0 + step as f32 * 45.0).to_radians().sin_cos();
        p.put(
            center.0 + (c * radius as f32) as i32,
            center.1 + (s * radius as f32) as i32,
            accent,
        );
    }
    let (s, c) = (-225.0 + value.clamp(0.0, 1.0) * 270.0)
        .to_radians()
        .sin_cos();
    p.line(
        center.0,
        center.1,
        center.0 + (c * (radius - 2) as f32) as i32,
        center.1 + (s * (radius - 2) as f32) as i32,
        accent,
    );
    p.rect(center.0, center.1, 2, 2, rgb(220, 225, 210));
}

fn radar(
    p: &mut Paint<'_>,
    f: &Flight,
    world: &World,
    network: Option<&crate::world::network::Network>,
    crt: [u32; 6],
) {
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
    }
    let connected = network.is_some_and(|network| network.active);
    let routes = network
        .filter(|network| network.active)
        .map(|network| network.routes(world))
        .unwrap_or_default();
    for portal in routes
        .iter()
        .map(|route| &route.aperture)
        .chain(world.portals.iter().filter(|_| !connected))
        .filter(|portal| !f.hides_portal(**portal))
    {
        let d = portal.center - f.position;
        let x = ((d.x * cy + d.y * sy) / 15.0).round() as i32;
        let y = -((d.x * sy - d.y * cy) / 15.0).round() as i32;
        if x.abs() < 22 && y.abs() < 12 {
            p.rect(70 + x, 125 + y, 3, 3, rgb(105, 211, 242));
            p.put(71 + x, 126 + y, crt[0]);
        }
    }
    for wing in f
        .wingmen
        .iter()
        .take(f.wing_count)
        .filter(|wing| wing.portal_wait <= 0.0)
    {
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
}

fn falcon_instruments(
    p: &mut Paint<'_>,
    f: &Flight,
    world: &World,
    network: Option<&crate::world::network::Network>,
) {
    let crt = [
        rgb(3, 16, 17),
        rgb(5, 27, 25),
        rgb(13, 56, 41),
        rgb(24, 91, 54),
        rgb(71, 158, 82),
        rgb(169, 242, 142),
    ];
    radar(p, f, world, network, crt);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::weather::Weather;

    #[test]
    fn vehicle_switch_refreshes_cockpit_and_occlusion() {
        let world = World::generate(crate::world::MapKind::Islands, 7);
        let mut flight = Flight::new(&world);
        flight.weights = [0.0, 0.0, 1.0];
        let mut cockpit = Cockpit::new();
        let mut frames = Vec::new();
        for vehicle in Vehicle::ALL.into_iter().chain([Vehicle::Falcon]) {
            flight.vehicle = vehicle;
            let revision = cockpit.revision;
            cockpit.prepare(&flight, &world, (320, 170), 0, None);
            assert!(cockpit.revision > revision);
            let mut fresh = Cockpit::new();
            fresh.prepare(&flight, &world, (320, 170), 0, None);
            assert_eq!(cockpit.pixels, fresh.pixels);
            assert_eq!(cockpit.base, fresh.base);
            frames.push(cockpit.pixels.clone());
        }
        assert_eq!(frames.first(), frames.last());
        for i in 0..5 {
            for j in i + 1..5 {
                assert_ne!(frames[i], frames[j]);
            }
        }
    }

    #[test]
    fn canopy_frames_share_one_layer() {
        let cockpit = Cockpit::new();
        let row = &cockpit.layers[0].runs[60];
        assert_eq!(row.len(), 4);
        assert!(row.iter().all(|&(start, end)| end > start));
    }

    #[test]
    fn outer_cockpit_windows_stay_clear_during_head_movement() {
        let world = World::generate(crate::world::MapKind::Islands, 7);
        let mut flight = Flight::new(&world);
        flight.weights = [0.0, 0.0, 1.0];
        let mut cockpit = Cockpit::new();
        for rate in [-1.25, 0.0, 1.25] {
            for pitch in [-0.6, 0.0, 0.6] {
                flight.rate = rate;
                flight.pitch = pitch;
                cockpit.prepare(&flight, &world, (320, 170), 0, None);
                for y in 0..12 {
                    for x in (0..20).chain(300..320) {
                        assert!(
                            !cockpit.opaque(x, y),
                            "rate={rate}, pitch={pitch}, ({x}, {y})"
                        );
                        assert_eq!(cockpit.pixels[y * 320 + x], 0);
                    }
                }
            }
        }
    }

    #[test]
    fn sunlight_preserves_transparent_cockpit_windows() {
        let world = World::generate(crate::world::MapKind::Islands, 7);
        let mut flight = Flight::new(&world);
        flight.weights = [0.0, 0.0, 1.0];
        let mut cockpit = Cockpit::new();
        for theme in 0..3 {
            for yaw in [0.0, 0.8, 2.4] {
                for pitch in [-0.6, 0.0, 0.6] {
                    flight.yaw = yaw;
                    flight.pitch = pitch;
                    for size in [(320, 170), (640, 340)] {
                        cockpit.prepare(&flight, &world, size, theme, None);
                        for y in 0..size.1 {
                            for x in 0..size.0 {
                                if !cockpit.opaque(x * 320 / size.0, y * 170 / size.1) {
                                    assert_eq!(cockpit.pixels[y * size.0 + x], 0);
                                }
                            }
                        }
                    }
                }
            }
            flight.weather.change(Weather::Cloudy);
            flight.weather.step(3.0, world.kind);
        }
    }
}
