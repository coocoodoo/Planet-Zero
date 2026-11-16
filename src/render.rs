use crate::arcade::{Mission, Mode, PickupKind, Weapon};
use glam::{Vec2, Vec3, vec2, vec3};

use crate::flight::{Camera, Flight, MODEL_SCALE};
use crate::palette::JET;
use crate::weather::Weather;
use crate::world::{CLOUD_ALT, MATERIAL_COUNT, MAX_HEIGHT, SIZE, World, hash, index};

const MODEL: &[u8] = include_bytes!("../assets/jet.bin");
const TOP: [[u8; 3]; MATERIAL_COUNT] = [
    [36, 104, 180],
    [44, 124, 198],
    [66, 160, 216],
    [104, 204, 214],
    [238, 220, 164],
    [222, 200, 140],
    [96, 178, 52],
    [70, 148, 44],
    [134, 206, 76],
    [70, 158, 64],
    [42, 122, 54],
    [28, 92, 46],
    [146, 140, 134],
    [118, 112, 110],
    [178, 172, 162],
    [246, 250, 255],
    [214, 226, 244],
    [206, 155, 76],
    [242, 194, 112],
    [172, 101, 58],
    [219, 144, 78],
    [43, 45, 53],
    [86, 76, 80],
    [255, 98, 23],
    [58, 126, 89],
    [107, 99, 64],
    [119, 194, 215],
];
const SIDE: [[u8; 3]; MATERIAL_COUNT] = [
    [36, 104, 180],
    [44, 124, 198],
    [66, 160, 216],
    [104, 204, 214],
    [204, 180, 122],
    [204, 180, 122],
    [56, 120, 44],
    [50, 108, 42],
    [60, 128, 46],
    [24, 78, 42],
    [22, 72, 40],
    [20, 64, 38],
    [96, 90, 92],
    [86, 82, 86],
    [104, 98, 98],
    [186, 202, 228],
    [176, 192, 220],
    [166, 116, 58],
    [198, 149, 80],
    [116, 64, 41],
    [159, 96, 53],
    [28, 30, 39],
    [53, 48, 57],
    [181, 41, 13],
    [39, 83, 59],
    [74, 67, 45],
    [76, 133, 163],
];

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    u32::from_le_bytes([r, g, b, 255])
}

pub(crate) fn mix(a: u32, b: u32, amount: f32) -> u32 {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();
    rgb(
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * amount) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * amount) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * amount) as u8,
    )
}

fn grade(c: u32, theme: usize) -> u32 {
    let [r, g, b, _] = c.to_le_bytes();
    let (r, g, b) = (r as f32, g as f32, b as f32);
    match theme {
        1 => rgb(
            (r * 0.95 + g * 0.1 + 12.0) as u8,
            (g * 0.72 + r * 0.05 + 4.0) as u8,
            (b * 0.7 + r * 0.08 + 10.0) as u8,
        ),
        2 => rgb(
            (r * 0.22 + 2.0) as u8,
            (g * 0.3 + 6.0) as u8,
            (b * 0.5 + 22.0) as u8,
        ),
        _ => c,
    }
}

pub struct Renderer {
    workers: Vec<Renderer>,
    cockpit: crate::cockpit::Cockpit,
    visible: Vec<(i32, i32)>,
    mapping: Vec<usize>,
    pub stages: [f64; 3],
    x_origin: usize,
    w: usize,
    h: usize,
    pixels: Vec<u32>,
    depth: Vec<f32>,
    mask: Vec<u8>,
    column: Vec<i32>,
    pub rgba: Vec<u8>,
    output: Vec<u32>,
    voxels: Vec<Vec<(f32, f32, usize)>>,
    silhouette: Vec<bool>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        let mut renderer = Self::single();
        if cfg!(target_os = "linux") {
            renderer.workers = (0..3).map(|_| Self::single()).collect();
        }
        renderer
    }

    fn single() -> Self {
        let mut voxels = vec![Vec::new(); 13];
        let mut silhouette = vec![false; 34 * 46];
        for z in 0..13 {
            for y in 0..46 {
                for x in 0..34 {
                    let color = MODEL[(z * 46 + y) * 34 + x] as usize;
                    if color != 0 {
                        voxels[z].push((x as f32 - 16.5, 25.3 - y as f32, color));
                        silhouette[y * 34 + x] = true;
                    }
                }
            }
        }
        Self {
            workers: vec![],
            cockpit: crate::cockpit::Cockpit::new(),
            visible: vec![],
            mapping: vec![],
            stages: [0.0; 3],
            x_origin: 0,
            w: 0,
            h: 0,
            pixels: vec![],
            depth: vec![],
            mask: vec![],
            column: vec![],
            rgba: vec![],
            output: vec![],
            voxels,
            silhouette,
        }
    }

    pub fn render(
        &mut self,
        world: &World,
        flight: &Flight,
        width: usize,
        height: usize,
        theme: usize,
    ) {
        self.render_mission(world, flight, (width, height), theme, None);
    }

    pub fn render_mission(
        &mut self,
        world: &World,
        flight: &Flight,
        size: (usize, usize),
        theme: usize,
        mission: Option<&Mission>,
    ) {
        let (width, height) = size;
        let cockpit = flight.weights[2];
        let rolled = cockpit > 0.01;
        // 座舱围绕画面上方的视线中心旋转，外围画布覆盖完整翻滚
        let reach_y = height as f32 * 1.28;
        let side = ((width * width) as f32 + reach_y * reach_y).sqrt().ceil() as usize + 4;
        self.w = if rolled { side } else { width };
        self.h = if rolled { side } else { height };
        self.pixels.resize(self.w * self.h, 0);
        self.pixels.fill(0);
        self.depth.resize(self.pixels.len(), f32::INFINITY);
        self.depth.fill(f32::INFINITY);
        self.mask.resize(self.pixels.len(), 0);
        self.mask.fill(0);
        self.column.resize(self.w, self.h as i32);
        self.column.fill(self.h as i32);
        let offset = vec2(
            (self.w - width) as f32 * 0.5,
            (self.h - height) as f32 * 0.5,
        );
        let mut camera = Camera::new(flight, width, height);
        camera.center += offset;
        let started = std::time::Instant::now();
        let has_panel = cockpit > 0.55;
        if has_panel {
            self.cockpit.prepare(flight, world, width, height, theme);
        }
        self.visible.resize(self.w, (self.h as i32, 0));
        self.visible.fill((self.h as i32, 0));
        self.mapping.resize(width * height, 0);
        self.output.resize(width * height, 0);
        let roll = (flight.bank + flight.spin) * cockpit;
        let (s, c) = roll.sin_cos();
        let pivot = vec2(width as f32 * 0.5, height as f32 * 0.43);
        if !rolled {
            self.visible.fill((0, height as i32));
            for y in 0..height {
                for x in 0..width {
                    self.mapping[y * width + x] = x * height + y;
                }
            }
        } else {
            for y in 0..height {
                let py = y as f32 - pivot.y;
                let mut q = vec2(-pivot.x * c - py * s, -pivot.x * s + py * c) + pivot + offset;
                for x in 0..width {
                    let sx = (q.x.round() as usize).min(self.w - 1);
                    let sy = (q.y.round() as usize).min(self.h - 1);
                    self.mapping[y * width + x] = sx * self.h + sy;
                    if !has_panel || self.cockpit.pixels[y * width + x] == 0 {
                        self.visible[sx].0 = self.visible[sx].0.min(sy as i32);
                        self.visible[sx].1 = self.visible[sx].1.max(sy as i32 + 1);
                    }
                    q.x += c;
                    q.y += s;
                }
            }
        }
        for (column, visible) in self.column.iter_mut().zip(&self.visible) {
            *column = visible.1;
        }
        self.stages[0] = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        if self.workers.is_empty() {
            self.world(world, flight, &camera, theme);
            self.sky(&camera, flight, theme, height);
        } else {
            let strip = self.w.div_ceil(self.workers.len());
            let w = self.w;
            let h = self.h;
            let visible = &self.visible;
            std::thread::scope(|scope| {
                for (i, worker) in self.workers.iter_mut().enumerate() {
                    let start = i * strip;
                    let end = ((i + 1) * strip).min(w);
                    worker.x_origin = start;
                    worker.w = end - start;
                    worker.h = h;
                    worker.pixels.resize(worker.w * h, 0);
                    worker.pixels.fill(0);
                    worker.depth.resize(worker.w * h, f32::INFINITY);
                    worker.depth.fill(f32::INFINITY);
                    worker.visible.clear();
                    worker.visible.extend_from_slice(&visible[start..end]);
                    worker.column.clear();
                    worker
                        .column
                        .extend(visible[start..end].iter().map(|v| v.1));
                    let mut view = camera;
                    view.center.x -= start as f32;
                    scope.spawn(move || {
                        worker.world(world, flight, &view, theme);
                        worker.sky(&view, flight, theme, height);
                    });
                }
            });
            for (i, worker) in self.workers.iter().enumerate() {
                let start = i * strip * h;
                let end = start + worker.pixels.len();
                self.pixels[start..end].copy_from_slice(&worker.pixels);
                self.depth[start..end].copy_from_slice(&worker.depth);
            }
        }
        self.stages[1] = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        let mut jets: Vec<&Flight> = flight.wingmen.iter().take(flight.wing_count).collect();
        if cockpit < 0.6 {
            jets.push(flight);
        }
        jets.sort_by(|a, b| {
            let da = camera.project(a.position).map_or(f32::INFINITY, |p| p.1);
            let db = camera.project(b.position).map_or(f32::INFINITY, |p| p.1);
            db.total_cmp(&da)
        });
        for jet in &jets {
            self.trails(jet, &camera, theme);
        }
        for jet in jets {
            self.jet(jet, &camera, theme);
        }
        if let Some(mission) = mission {
            self.mission(mission, flight, &camera);
        }
        self.precipitation(flight, &camera, theme);
        for i in 0..width * height {
            self.output[i] = if has_panel && self.cockpit.pixels[i] != 0 {
                self.cockpit.pixels[i]
            } else {
                self.pixels[self.mapping[i]]
            };
        }
        if has_panel {
            self.cockpit.glass(&mut self.output, width, height, flight);
        }
        self.stages[2] = started.elapsed().as_secs_f64() * 1000.0;
        self.rgba.resize(width * height * 4, 0);
        for (dest, color) in self.rgba.chunks_exact_mut(4).zip(&self.output) {
            dest.copy_from_slice(&color.to_le_bytes());
        }
    }

    fn world_line(&mut self, camera: &Camera, a: Vec3, b: Vec3, color: u32) {
        if let (Some((a, da, _)), Some((b, db, _))) = (camera.project(a), camera.project(b)) {
            if a.distance(b) > 1200.0 {
                return;
            }
            let steps = a.distance(b).ceil().clamp(1.0, 400.0) as usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                self.point(a.lerp(b, t), da + (db - da) * t, color, 0, 0);
            }
        }
    }

    fn triangle(&mut self, camera: &Camera, points: [Vec3; 3], color: u32) {
        let [Some((a, da, _)), Some((b, db, _)), Some((c, dc, _))] =
            points.map(|p| camera.project(p))
        else {
            return;
        };
        let area = (b - a).perp_dot(c - a);
        if area.abs() < 0.1 {
            return;
        }
        let min = a.min(b).min(c).max(Vec2::ZERO);
        let max = a
            .max(b)
            .max(c)
            .min(vec2(self.w as f32 - 1.0, self.h as f32 - 1.0));
        for y in min.y as i32..=max.y as i32 {
            for x in min.x as i32..=max.x as i32 {
                let p = vec2(x as f32, y as f32);
                let u = (b - p).perp_dot(c - p) / area;
                let v = (c - p).perp_dot(a - p) / area;
                let w = 1.0 - u - v;
                if u >= 0.0 && v >= 0.0 && w >= 0.0 {
                    self.point(p, u * da + v * db + w * dc, color, 0, 1);
                }
            }
        }
    }

    fn mission(&mut self, mission: &Mission, flight: &Flight, camera: &Camera) {
        for (index, gate) in mission
            .gates
            .iter()
            .enumerate()
            .skip(mission.gate_index)
            .take(4)
            .rev()
        {
            let right = vec3(-gate.normal.y, gate.normal.x, 0.0);
            let active = index == mission.gate_index;
            let color = if active {
                rgb(100, 255, 201)
            } else {
                rgb(105, 154, 178)
            };
            for i in 0..40 {
                let a = i as f32 * std::f32::consts::TAU / 40.0;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 40.0;
                for r in [gate.radius, gate.radius + if active { 2.0 } else { 0.0 }] {
                    self.world_line(
                        camera,
                        gate.position + (right * a.cos() + Vec3::Z * a.sin()) * r,
                        gate.position + (right * b.cos() + Vec3::Z * b.sin()) * r,
                        color,
                    );
                }
            }
            if active {
                let r = gate.radius;
                for side in [-1.0, 1.0] {
                    self.world_line(
                        camera,
                        gate.position + right * side * (r + 8.0),
                        gate.position + right * side * (r + 3.0),
                        rgb(255, 243, 155),
                    );
                }
            }
        }
        for pickup in &mission.pickups {
            if pickup.collected || pickup.position.distance_squared(flight.position) > 650.0 * 650.0
            {
                continue;
            }
            let Some((p, d, scale)) = camera.project(pickup.position) else {
                continue;
            };
            let color = match pickup.kind {
                PickupKind::Coin => rgb(255, 218, 71),
                PickupKind::Upgrade(w) => {
                    let c = w.color();
                    rgb(c[0], c[1], c[2])
                }
                PickupKind::Shield => rgb(109, 255, 165),
                PickupKind::Energy => rgb(103, 222, 255),
                PickupKind::Bomb => rgb(255, 112, 91),
                PickupKind::Support => rgb(216, 174, 255),
                PickupKind::Time => rgb(200, 255, 240),
            };
            let radius = (scale * 6.0).round().clamp(2.0, 18.0) as i32;
            let rotate = (flight.time * 3.0).cos().abs() * 0.7 + 0.3;
            for y in -radius..=radius {
                for x in -radius..=radius {
                    let nx = x as f32
                        / (radius as f32
                            * if pickup.kind == PickupKind::Coin {
                                rotate
                            } else {
                                1.0
                            });
                    let ny = y as f32 / radius as f32;
                    let distance = nx * nx + ny * ny;
                    if distance < 1.0 {
                        self.point(
                            p + vec2(x as f32, y as f32),
                            d,
                            if distance > 0.55 {
                                color
                            } else {
                                mix(color, rgb(30, 35, 62), 0.5)
                            },
                            0,
                            1,
                        );
                    }
                }
            }
            if pickup.kind != PickupKind::Coin {
                self.stroke(
                    p - vec2(0.0, 2.0),
                    p + vec2(0.0, 2.0),
                    d - 1.0,
                    rgb(255, 255, 255),
                );
                self.stroke(
                    p - vec2(2.0, 0.0),
                    p + vec2(2.0, 0.0),
                    d - 1.0,
                    rgb(255, 255, 255),
                );
            }
        }
        for enemy in &mission.enemies {
            let scale = if enemy.kind == 3 {
                3.2
            } else {
                0.75 + enemy.kind as f32 * 0.18
            };
            let forward = vec3(enemy.yaw.sin(), -enemy.yaw.cos(), 0.0);
            let right = vec3(enemy.yaw.cos(), enemy.yaw.sin(), 0.0);
            let bank = enemy.phase.cos() * 0.25;
            let right = right * bank.cos() + Vec3::Z * bank.sin();
            let local = |x: f32, y: f32, z: f32| {
                enemy.position + (right * x + forward * y + Vec3::Z * z) * scale
            };
            let color = if enemy.hit > 0.0 {
                rgb(255, 252, 232)
            } else {
                match enemy.kind {
                    0 => rgb(239, 98, 112),
                    1 => rgb(171, 119, 235),
                    2 => rgb(209, 161, 101),
                    _ => rgb(113, 133, 172),
                }
            };
            let dark = mix(color, rgb(22, 24, 42), 0.55);
            for side in [-1.0, 1.0] {
                self.triangle(
                    camera,
                    [
                        local(0.0, 14.0, 1.0),
                        local(side * 5.0, -11.0, 0.0),
                        local(0.0, -8.0, 5.0),
                    ],
                    color,
                );
                self.triangle(
                    camera,
                    [
                        local(side * 3.0, 4.0, 1.0),
                        local(side * 19.0, -7.0, 0.0),
                        local(side * 4.0, -10.0, 2.0),
                    ],
                    dark,
                );
                self.triangle(
                    camera,
                    [
                        local(side * 3.0, -6.0, 3.0),
                        local(side * 8.0, -14.0, 2.0),
                        local(side * 4.0, -12.0, 8.0),
                    ],
                    color,
                );
                if enemy.kind == 3 {
                    self.triangle(
                        camera,
                        [
                            local(side * 7.0, 8.0, 1.0),
                            local(side * 15.0, -8.0, 1.0),
                            local(side * 10.0, -6.0, 5.0),
                        ],
                        color,
                    );
                }
                if let Some((p, d, s)) = camera.project(local(side * 3.0, -11.0, 1.0)) {
                    self.point(
                        p,
                        d,
                        rgb(255, 92, 74),
                        (s * scale).round().clamp(1.0, 4.0) as i32,
                        0,
                    );
                }
            }
            self.triangle(
                camera,
                [
                    local(0.0, 9.0, 3.0),
                    local(-2.0, 0.0, 5.2),
                    local(2.0, 0.0, 5.2),
                ],
                rgb(144, 238, 255),
            );
        }
        for shot in &mission.shots {
            let color = if shot.hostile {
                rgb(255, 91, 116)
            } else {
                let c = shot.weapon.color();
                rgb(c[0], c[1], c[2])
            };
            if shot.weapon == Weapon::Laser && !shot.hostile {
                self.world_line(camera, shot.previous, shot.position, color);
                self.world_line(
                    camera,
                    shot.previous + Vec3::Z,
                    shot.position + Vec3::Z,
                    rgb(238, 255, 255),
                );
            } else if let Some((p, d, s)) = camera.project(shot.position) {
                let radius = (shot.radius * s).round().clamp(1.0, 8.0) as i32;
                self.point(p, d, rgb(44, 24, 53), radius + 1, 0);
                self.point(p, d - 0.1, color, radius, 0);
                self.point(p, d - 0.2, rgb(255, 250, 225), (radius - 1).max(0), 0);
                if !shot.hostile {
                    self.world_line(
                        camera,
                        shot.position,
                        shot.position - shot.velocity * 0.045,
                        color,
                    );
                }
            }
        }
        for effect in &mission.effects {
            let t = 1.0 - effect.life / effect.duration;
            let c = effect.color;
            let color = mix(rgb(c[0], c[1], c[2]), rgb(85, 60, 79), t * 0.7);
            let right = vec3(camera.yaw.cos(), -camera.yaw.sin(), 0.0);
            for i in 0..20 {
                let a = i as f32 * std::f32::consts::TAU / 20.0;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 20.0;
                self.world_line(
                    camera,
                    effect.position + (right * a.cos() + Vec3::Z * a.sin()) * effect.radius * t,
                    effect.position + (right * b.cos() + Vec3::Z * b.sin()) * effect.radius * t,
                    color,
                );
            }
        }
        if mission.mode == Mode::Combat
            && mission.invulnerable > 0.6
            && flight.weights[2] < 0.5
            && let Some((p, d, s)) = camera.project(flight.position)
        {
            let r = s * 15.0;
            for i in 0..24 {
                let a = i as f32 * std::f32::consts::TAU / 24.0;
                self.point(
                    p + vec2(a.cos(), a.sin() * 0.6) * r,
                    d - 15.0,
                    rgb(129, 231, 255),
                    0,
                    0,
                );
            }
        }
    }

    fn span(&mut self, x: usize, y0: i32, y1: i32, color: u32, depth: f32, checker: bool) {
        for y in y0.max(self.visible[x].0)..y1.min(self.visible[x].1) {
            let i = x * self.h + y as usize;
            if self.pixels[i] == 0 && !(checker && (x + self.x_origin + y as usize) & 1 == 0) {
                self.pixels[i] = color;
                self.depth[i] = depth;
            }
        }
    }

    fn world(&mut self, world: &World, flight: &Flight, camera: &Camera, theme: usize) {
        let (s, c) = camera.yaw.sin_cos();
        let q = camera.q;
        let scale = camera.scale;
        let slope = camera.slope;
        let altitude = camera.position.z;
        let cx = camera.center.x;
        let cy = camera.center.y;
        let weather = flight.weather;
        let flash = weather.flash(flight.time);
        let lighting = |color| {
            mix(
                weather.shade(grade(color, theme)),
                rgb(225, 234, 255),
                flash,
            )
        };
        let top = TOP.map(|v| lighting(rgb(v[0], v[1], v[2])));
        let sides = SIDE.map(|v| lighting(rgb(v[0], v[1], v[2])));
        let shadow_color = rgb(13, 26, 56);
        let haze = mix(weather.haze(theme), rgb(225, 234, 255), flash);
        let mut colors = [[[[0_u32; 2]; MATERIAL_COUNT + 2]; 2]; 11];
        let mut clouds = [[0_u32; 9]; 11];
        for fog in 0..=10 {
            let amount = fog as f32 / 10.0;
            for (shade, materials) in colors[fog].iter_mut().enumerate() {
                for (material, faces) in materials.iter_mut().enumerate() {
                    let color = match material {
                        MATERIAL_COUNT => lighting(rgb(240, 252, 255)),
                        m if m == MATERIAL_COUNT + 1 => lighting(rgb(160, 211, 244)),
                        _ => top[material],
                    };
                    let side = if !(4..MATERIAL_COUNT).contains(&material) {
                        color
                    } else {
                        sides[material]
                    };
                    for (face, base) in [color, side].into_iter().enumerate() {
                        faces[face] = mix(
                            if shade == 1 {
                                mix(base, shadow_color, 0.33)
                            } else {
                                base
                            },
                            haze,
                            amount,
                        );
                    }
                }
            }
            for (k, cloud_color) in clouds[fog].iter_mut().enumerate() {
                let f = k as f32 / 7.0;
                let base = match k {
                    8 => rgb(255, 255, 255),
                    7 => rgb(138, 160, 200),
                    _ => rgb(
                        (158.0 + 44.0 * f) as u8,
                        (182.0 + 36.0 * f) as u8,
                        (222.0 + 18.0 * f) as u8,
                    ),
                };
                *cloud_color = mix(lighting(base), haze, amount);
            }
        }
        let below = self.h as f32 - cy;
        let nearest = if q > 0.00001 { -0.85 / q } else { -1e8 };
        let camera_height = if q > 0.00001 {
            altitude + slope / q
        } else {
            1e8
        };
        let dn = scale * slope + q * below;
        let mut distance = if camera_height > MAX_HEIGHT as f32 + 1.0 && dn > 0.00001 {
            ((scale * (altitude - MAX_HEIGHT as f32) - below) / dn - 2.0).max(nearest)
        } else {
            nearest
        };
        let (fog_start, view_distance) = weather.visibility(altitude);
        let above = cy + 2.0;
        let end = if scale * slope > q * above {
            view_distance.min((scale * altitude + above) / (scale * slope - q * above))
        } else {
            view_distance
        };
        let mut rows = [0_i32; MAX_HEIGHT + 1];
        for (z, row) in rows.iter_mut().enumerate() {
            *row = (cy - scale / (1.0 + q * distance) * (slope * distance - altitude + z as f32))
                .floor() as i32;
        }
        let shadows: Vec<_> = std::iter::once(flight)
            .chain(flight.wingmen.iter().take(flight.wing_count))
            .map(|f| {
                let (js, jc) = f.yaw.sin_cos();
                (
                    f.position + vec3(0.5, 0.32, 0.0) * f.position.z,
                    js,
                    jc,
                    1.0 / (MODEL_SCALE * (f.bank + f.spin).cos().abs().max(0.3)),
                )
            })
            .collect();
        for _ in 0..4000 {
            if distance >= end {
                break;
            }
            let denominator = 1.0 + q * distance;
            let step = (denominator * denominator / (scale * (slope + q * altitude)))
                .min(2.5 * denominator / scale)
                .max(0.5);
            distance += step;
            let denominator = 1.0 + q * distance;
            let m = scale / denominator;
            let previous = rows;
            for (z, row) in rows.iter_mut().enumerate() {
                *row = (cy - m * (slope * distance - altitude + z as f32) + 0.00001).floor() as i32;
            }
            let inverse = denominator / scale;
            let ax = c * inverse;
            let ay = -s * inverse;
            let mut wx = camera.position.x - cx * ax - distance * s;
            let mut wy = camera.position.y - cx * ay - distance * c;
            let fog = if distance > fog_start {
                (((distance - fog_start) / (view_distance - fog_start)).sqrt() * 10.0).round()
                    / 10.0
            } else {
                0.0
            };
            let fog_index = ((fog * 10.0) as usize).min(10);
            for x in 0..self.w {
                if self.visible[x].0 >= self.visible[x].1 || self.column[x] <= self.visible[x].0 {
                    wx += ax;
                    wy += ay;
                    continue;
                }
                let cloud = world.cloud(wx, wy, flight.time);
                if cloud > 0 {
                    let z = CLOUD_ALT + cloud - 1;
                    let color = clouds[fog_index][8];
                    self.span(x, rows[z], previous[z], color, distance, cloud == 1);
                    self.span(
                        x,
                        previous[CLOUD_ALT],
                        rows[CLOUD_ALT],
                        clouds[fog_index][7],
                        distance,
                        cloud == 1,
                    );
                    for k in (0..cloud - 1).rev() {
                        self.span(
                            x,
                            previous[CLOUD_ALT + k + 1],
                            previous[CLOUD_ALT + k],
                            clouds[fog_index][k],
                            distance,
                            false,
                        );
                    }
                }
                let gx = wx.floor() as i32;
                let gy = wy.floor() as i32;
                let at = index(gx, gy, SIZE);
                let h = world.heights[at] as usize;
                let mut highest = self.column[x];
                if rows[h] < highest || previous[h] < highest {
                    let mut material = world.materials[at] as usize;
                    let mut shade = world.cloud(wx + 20.0, wy + 12.8, flight.time) > 1;
                    for &(shadow, js, jc, bank_scale) in &shadows {
                        let dx = wx - shadow.x + 0.5 * h as f32;
                        let dy = wy - shadow.y + 0.32 * h as f32;
                        if dx.abs() < 35.0 && dy.abs() < 35.0 {
                            let a = (dx * jc + dy * js) * bank_scale;
                            let b = (dx * js - dy * jc) / MODEL_SCALE;
                            let ix = (a + 17.0).floor() as i32;
                            let iy = (25.8 - b).floor() as i32;
                            if (0..34).contains(&ix) && (0..46).contains(&iy) {
                                shade |= self.silhouette[iy as usize * 34 + ix as usize];
                            }
                        }
                    }
                    if material == 3
                        && hash((at as u32).wrapping_add((flight.time * 36.0) as u32)) < 120
                    {
                        material = MATERIAL_COUNT;
                    } else if material < 3
                        && inverse < 1.8
                        && hash(index((gx + (flight.time * 6.0) as i32) >> 2, gy, 256) as u32) > 247
                    {
                        material = MATERIAL_COUNT + 1;
                    }
                    let [color, side] = colors[fog_index][usize::from(shade)][material];
                    if previous[h] < highest {
                        self.span(x, previous[h], highest, side, distance, false);
                        highest = previous[h].max(self.visible[x].0);
                    }
                    if rows[h] < highest {
                        self.span(x, rows[h], highest, color, distance, false);
                        highest = rows[h];
                    }
                    self.column[x] = highest;
                }
                wx += ax;
                wy += ay;
            }
        }
    }

    fn sky(&mut self, camera: &Camera, flight: &Flight, theme: usize, height: usize) {
        let horizon = if camera.q > 0.00001 {
            camera.center.y - camera.scale * camera.slope / camera.q
        } else {
            -1e8
        };
        let (low, high) = match theme {
            1 => (rgb(255, 198, 132), rgb(84, 68, 150)),
            2 => (rgb(36, 56, 110), rgb(7, 10, 34)),
            _ => (rgb(198, 232, 250), rgb(62, 142, 222)),
        };
        let weather = flight.weather;
        let (low, high) = if weather == Weather::Clear {
            (low, high)
        } else {
            let cover = match weather {
                Weather::Fog => 0.92,
                Weather::Sandstorm => 0.85,
                Weather::Snow => 0.68,
                Weather::Rain => 0.55,
                _ => 0.4,
            };
            (
                weather.haze(theme),
                mix(weather.shade(high), weather.haze(theme), cover),
            )
        };
        let flash = weather.flash(flight.time);
        for y in 0..self.h {
            let gradient = ((horizon - y as f32) / (height as f32 * 0.75)).clamp(0.0, 1.0);
            let color = mix(
                mix(low, high, (gradient * 12.0).floor() / 12.0),
                rgb(225, 234, 255),
                flash,
            );
            for x in 0..self.w {
                let at = x * self.h + y;
                if (y as i32) >= self.visible[x].0
                    && (y as i32) < self.visible[x].1
                    && self.pixels[at] == 0
                {
                    self.pixels[at] = if theme == 2
                        && weather == Weather::Clear
                        && gradient > 0.2
                        && hash(
                            ((x + self.x_origin) as u32)
                                .wrapping_add((camera.yaw * 300.0) as i32 as u32)
                                ^ (y as u32 * 8191),
                        ) == 255
                        && (x + self.x_origin).is_multiple_of(3)
                    {
                        rgb(182, 202, 232)
                    } else {
                        color
                    };
                }
            }
        }
    }

    fn precipitation(&mut self, flight: &Flight, camera: &Camera, theme: usize) {
        let weather = flight.weather;
        let (count, fall, wind) = weather.particles();
        let time = flight.time;
        for i in 0..count {
            let seed = i as u32 * 7919 + 31;
            let rx = hash(seed) as f32 / 255.0;
            let ry = hash(seed + 171) as f32 / 255.0;
            let rz = hash(seed + 947) as f32 / 255.0;
            let flutter = if weather == Weather::Snow {
                (time * 1.7 + rx * 20.0).sin() * 4.0
            } else {
                0.0
            };
            let position = vec3(
                (rx * 256.0 + time * wind + flutter - flight.position.x).rem_euclid(256.0)
                    + flight.position.x
                    - 128.0,
                (ry * 256.0 + time * wind * 0.3 - flight.position.y).rem_euclid(256.0)
                    + flight.position.y
                    - 128.0,
                (rz * 180.0 - time * fall - flight.position.z).rem_euclid(180.0)
                    + flight.position.z
                    - 80.0,
            );
            if let Some((a, depth, scale)) = camera.project(position) {
                if a.x < -20.0
                    || a.y < -20.0
                    || a.x > self.w as f32 + 20.0
                    || a.y > self.h as f32 + 20.0
                {
                    continue;
                }
                let base = match weather {
                    Weather::Snow => rgb(239, 247, 250),
                    Weather::Sandstorm => {
                        if i % 3 == 0 {
                            rgb(239, 194, 108)
                        } else {
                            rgb(132, 91, 48)
                        }
                    }
                    _ => rgb(149, 187, 216),
                };
                let color = mix(
                    grade(base, theme),
                    weather.haze(theme),
                    weather.fog(depth, flight.position.z) * 0.5,
                );
                if weather == Weather::Snow {
                    self.point(a, depth, color, 0, 0);
                    if scale > 1.4 && i % 9 == 0 {
                        self.point(a + vec2(1.0, 0.0), depth, color, 0, 0);
                        self.point(a + vec2(0.0, 1.0), depth, color, 0, 0);
                    }
                } else {
                    let tail = position - vec3(wind, wind * 0.3, -fall) * 0.035;
                    if let Some((b, _, _)) = camera.project(tail) {
                        self.stroke(a, b, depth, color);
                    }
                }
            }
        }
        if weather.flash(time) > 0.06 {
            let (s, c) = camera.yaw.sin_cos();
            let right = vec3(c, -s, 0.0);
            let forward = vec3(-s, -c, 0.0);
            let seed = (time / 11.0) as u32;
            let base = flight.position
                + forward * 420.0
                + right * ((hash(seed + 451) as f32 / 255.0 - 0.5) * 180.0);
            let mut previous = base + vec3(0.0, 0.0, 160.0);
            for segment in 1..=9 {
                let point = base
                    + right * ((hash(seed + segment * 71) as f32 / 255.0 - 0.5) * 22.0)
                    + vec3(0.0, 0.0, 160.0 - segment as f32 * 25.0);
                if let (Some((a, depth, _)), Some((b, _, _))) =
                    (camera.project(previous), camera.project(point))
                {
                    self.stroke(a, b, depth, rgb(241, 244, 255));
                    if (segment == 3 || segment == 5)
                        && let Some((branch, _, _)) =
                            camera.project(point + right * 28.0 - vec3(0.0, 0.0, 30.0))
                    {
                        self.stroke(b, branch, depth, rgb(174, 195, 234));
                    }
                }
                previous = point;
            }
        }
    }

    fn stroke(&mut self, a: Vec2, b: Vec2, depth: f32, color: u32) {
        let steps = a.distance(b).ceil().clamp(1.0, 48.0) as usize;
        for step in 0..=steps {
            self.point(a.lerp(b, step as f32 / steps as f32), depth, color, 0, 0);
        }
    }

    fn point(&mut self, p: Vec2, depth: f32, color: u32, radius: i32, mark: u8) {
        let x = p.x.round() as i32;
        let y = p.y.round() as i32;
        for yy in y - radius..=y + radius {
            for xx in x - radius..=x + radius {
                if xx >= 0 && yy >= 0 && xx < self.w as i32 && yy < self.h as i32 {
                    let at = xx as usize * self.h + yy as usize;
                    if self.depth[at] >= depth - 1.0 {
                        self.pixels[at] = color;
                        self.mask[at] = mark;
                        if mark == 1 {
                            self.depth[at] = self.depth[at].min(depth);
                        }
                    }
                }
            }
        }
    }

    fn trails(&mut self, flight: &Flight, camera: &Camera, theme: usize) {
        for i in (1..flight.trails.len()).rev() {
            for side in 0..2 {
                if let (Some((a, da, _)), Some((b, db, _))) = (
                    camera.project(flight.trails[i - 1][side]),
                    camera.project(flight.trails[i][side]),
                ) {
                    let steps = a.distance(b).ceil().clamp(1.0, 100.0) as usize;
                    let age = i as f32 / flight.trails.len() as f32;
                    if age > 0.7 && i % 2 == 0 {
                        continue;
                    }
                    let color = mix(
                        flight.weather.shade(grade(
                            mix(rgb(255, 255, 255), rgb(120, 180, 218), age),
                            theme,
                        )),
                        flight.weather.haze(theme),
                        flight.weather.fog((da + db) * 0.5, flight.position.z),
                    );
                    for step in 0..=steps {
                        let t = step as f32 / steps as f32;
                        self.point(a.lerp(b, t), da + (db - da) * t, color, 0, 0);
                    }
                }
            }
        }
    }

    fn jet(&mut self, flight: &Flight, camera: &Camera, theme: usize) {
        let [right, forward, up] = flight.axes();
        let shape = flight.vehicle.shape();
        let (right, forward, up) = (right * shape.x, forward * shape.y, up * shape.z);
        let (s, c) = camera.yaw.sin_cos();
        let up_depth = -(up.x * s + up.y * c);
        let eye = if camera.q < 0.00001 {
            if camera.slope * up.z - up_depth >= 0.0 {
                1e9
            } else {
                -1e9
            }
        } else {
            let eye = camera.position + vec3(s / camera.q, c / camera.q, camera.slope / camera.q);
            (eye - flight.position).dot(up) / MODEL_SCALE
        };
        let mut order: Vec<usize> = (0..13).collect();
        order.sort_by(|a, b| {
            (((*b as f32 - 5.0) * flight.spread - eye).abs())
                .total_cmp(&((*a as f32 - 5.0) * flight.spread - eye).abs())
        });
        let mut palette = [0_u32; 96];
        for &(i, r, g, b) in JET {
            palette[i] = flight
                .weather
                .shade(mix(rgb(r, g, b), grade(rgb(r, g, b), theme), 0.4));
        }
        if flight.vehicle != crate::vehicle::Vehicle::Falcon {
            let [r, g, b] = flight.vehicle.tint();
            for color in &mut palette {
                *color = mix(*color, rgb(r, g, b), 0.28);
            }
        }
        let jet_depth = camera.project(flight.position).map_or(0.0, |p| p.1);
        let fog = flight.weather.fog(jet_depth, flight.position.z);
        for color in &mut palette {
            *color = mix(*color, flight.weather.haze(theme), fog);
        }
        self.mask.fill(0);
        let center_scale = camera.project(flight.position).map_or(1.0, |p| p.2);
        let samples = (center_scale * MODEL_SCALE * 1.7).ceil().clamp(2.0, 6.0) as usize;
        let view_axis = |a: glam::Vec3| {
            vec2(a.x * c - a.y * s, -a.z + camera.slope * (a.x * s + a.y * c)) * MODEL_SCALE
        };
        let r = view_axis(right);
        let f = view_axis(forward);
        let mut offsets = [Vec2::ZERO; 36];
        for iy in 0..samples {
            for ix in 0..samples {
                offsets[iy * samples + ix] = r * ((ix as f32 + 0.5) / samples as f32 - 0.5)
                    + f * ((iy as f32 + 0.5) / samples as f32 - 0.5);
            }
        }
        let mut bounds = [self.w as i32, self.h as i32, 0, 0];
        for z in order {
            for voxel in 0..self.voxels[z].len() {
                let (a, b, color) = self.voxels[z][voxel];
                let center = flight.position
                    + (right * a + forward * b + up * ((z as f32 - 5.0) * flight.spread))
                        * MODEL_SCALE;
                if let Some((p, depth, scale)) = camera.project(center) {
                    bounds[0] = bounds[0].min(p.x as i32 - 4);
                    bounds[1] = bounds[1].min(p.y as i32 - 4);
                    bounds[2] = bounds[2].max(p.x as i32 + 4);
                    bounds[3] = bounds[3].max(p.y as i32 + 4);
                    for offset in &offsets[..samples * samples] {
                        self.point(p + *offset * scale, depth, palette[color], 0, 1);
                    }
                }
            }
        }
        let center_depth = camera.project(flight.position).map_or(0.0, |p| p.1);
        for y in bounds[1].max(1) as usize..bounds[3].clamp(1, self.h as i32 - 1) as usize {
            for x in bounds[0].max(1) as usize..bounds[2].clamp(1, self.w as i32 - 1) as usize {
                let at = x * self.h + y;
                if self.mask[at] == 0
                    && self.depth[at] >= center_depth - 2.0
                    && [at - 1, at + 1, at - self.h, at + self.h]
                        .iter()
                        .any(|&n| self.mask[n] == 1)
                {
                    self.pixels[at] = rgb(13, 15, 26);
                }
            }
        }
        if flight.vehicle == crate::vehicle::Vehicle::Wasp {
            for side in [-1.0, 1.0] {
                let hub = flight.local(vec3(side * 17.0, 0.0, 5.0));
                for blade in 0..3 {
                    let a = flight.time * 42.0 + blade as f32 * std::f32::consts::TAU / 3.0;
                    let tip = hub + (right * a.cos() + forward * a.sin()) * 7.0;
                    if let (Some((p, d, _)), Some((q, _, _))) =
                        (camera.project(hub), camera.project(tip))
                    {
                        self.stroke(p, q, d - 1.0, rgb(202, 225, 187));
                    }
                }
            }
        }
        if flight.spread < 2.5 {
            let length = (9.0 + flight.boost * 19.0) * (0.9 + 0.1 * (flight.time * 47.0).sin());
            for engine in [-1.0, 1.0] {
                for step in (0..(length * 3.0) as usize).rev() {
                    let along = step as f32 / 3.0;
                    let f = along / length;
                    let center = flight.local(vec3(
                        engine * 1.96 - flight.rate * along * along * 0.015,
                        -18.3 - along,
                        -0.9,
                    ));
                    if let Some((p, depth, scale)) = camera.project(center) {
                        let radius = (1.1 * (1.0 - f).sqrt() * scale * MODEL_SCALE).round() as i32;
                        self.point(
                            p,
                            depth,
                            mix(rgb(255, 238, 158), rgb(242, 77, 17), f),
                            radius,
                            2,
                        );
                        if f < 0.55 {
                            self.point(p, depth - 0.1, rgb(255, 253, 224), (radius - 1).max(0), 2);
                        }
                    }
                }
            }
        }
    }
}
