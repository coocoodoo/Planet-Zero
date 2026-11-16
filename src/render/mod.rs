mod aircraft;
mod cockpit;
mod objects;
mod palette;
mod primitives;
mod terrain;
mod weather;
use crate::gameplay::{Mission, Mode, PickupKind, Weapon};
use glam::{Vec2, Vec3, vec2, vec3};

use crate::content::weather::Weather;
use crate::flight::{Camera, Flight, MODEL_SCALE};
use crate::render::palette::JET;
use crate::world::{CLOUD_ALT, MATERIAL_COUNT, MAX_HEIGHT, SIZE, World, hash, index};

const MODEL: &[u8] = include_bytes!("../../assets/vehicles/falcon.bin");
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
    cockpit: crate::render::cockpit::Cockpit,
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
            cockpit: crate::render::cockpit::Cockpit::new(),
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
}
