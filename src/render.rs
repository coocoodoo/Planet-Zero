use glam::{Vec2, vec2, vec3};

use crate::flight::{Camera, Flight, MODEL_SCALE};
use crate::palette::JET;
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
            self.sky(&camera, theme, height);
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
                        worker.sky(&view, theme, height);
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
        let top = TOP.map(|v| grade(rgb(v[0], v[1], v[2]), theme));
        let sides = SIDE.map(|v| grade(rgb(v[0], v[1], v[2]), theme));
        let shadow_color = rgb(13, 26, 56);
        let haze = match theme {
            1 => rgb(250, 182, 134),
            2 => rgb(28, 44, 92),
            _ => rgb(188, 224, 246),
        };
        let mut colors = [[[[0_u32; 2]; MATERIAL_COUNT + 2]; 2]; 11];
        let mut clouds = [[0_u32; 9]; 11];
        for fog in 0..=10 {
            let amount = fog as f32 / 10.0;
            for (shade, materials) in colors[fog].iter_mut().enumerate() {
                for (material, faces) in materials.iter_mut().enumerate() {
                    let color = match material {
                        MATERIAL_COUNT => grade(rgb(240, 252, 255), theme),
                        m if m == MATERIAL_COUNT + 1 => grade(rgb(160, 211, 244), theme),
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
                *cloud_color = mix(grade(base, theme), haze, amount);
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
        let view_distance = 1500.0 + 3.0 * altitude;
        let fog_start = 300.0 + 2.0 * altitude;
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
            let fog_index = (fog * 10.0) as usize;
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

    fn sky(&mut self, camera: &Camera, theme: usize, height: usize) {
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
        for y in 0..self.h {
            let gradient = ((horizon - y as f32) / (height as f32 * 0.75)).clamp(0.0, 1.0);
            let color = mix(low, high, (gradient * 12.0).floor() / 12.0);
            for x in 0..self.w {
                let at = x * self.h + y;
                if (y as i32) >= self.visible[x].0
                    && (y as i32) < self.visible[x].1
                    && self.pixels[at] == 0
                {
                    self.pixels[at] = if theme == 2
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
                    let color = grade(mix(rgb(255, 255, 255), rgb(120, 180, 218), age), theme);
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
            palette[i] = mix(rgb(r, g, b), grade(rgb(r, g, b), theme), 0.4);
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
