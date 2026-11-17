mod aircraft;
mod celestial;
mod clouds;
mod cockpit;
mod geology;
mod landmarks;
mod objects;
mod palette;
mod portals;
mod primitives;
mod settlements;
mod terrain;
mod tilted;
mod weather;
use crate::gameplay::{Mission, Mode, PickupKind, Weapon};
use glam::{Vec2, Vec3, vec2, vec3};

use crate::content::weather::Weather;
use crate::flight::{Camera, Flight, MODEL_SCALE};
use crate::render::palette::JET;
use crate::world::{MATERIAL_COUNT, MAX_HEIGHT, SIZE, World, hash, index};

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
    [164, 167, 174],
    [205, 207, 211],
    [105, 109, 120],
    [38, 47, 59],
    [210, 188, 110],
    [111, 125, 142],
    [61, 82, 104],
    [111, 147, 171],
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
    [66, 67, 76],
    [99, 100, 108],
    [49, 52, 63],
    [26, 31, 40],
    [149, 137, 86],
    [73, 81, 96],
    [38, 51, 69],
    [67, 95, 116],
];

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    u32::from_le_bytes([r, g, b, 255])
}

pub(crate) fn mix(a: u32, b: u32, amount: f32) -> u32 {
    let t = (amount.clamp(0.0, 1.0) * 256.0) as u32;
    let rb = ((a & 0x00ff00ff) * (256 - t) + (b & 0x00ff00ff) * t) >> 8;
    let g = ((a & 0x0000ff00) * (256 - t) + (b & 0x0000ff00) * t) >> 8;
    0xff000000 | (rb & 0x00ff00ff) | (g & 0x0000ff00)
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
    portal_view: Option<Box<Renderer>>,
    clip_plane: Option<(Vec3, Vec3)>,
    clip_depth: Vec<f32>,
    near_depth: f32,
    cockpit: crate::render::cockpit::Cockpit,
    visible: Vec<(i32, i32)>,
    mapping: Vec<usize>,
    pub stages: [f64; 3],
    x_origin: usize,
    full_width: usize,
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
            portal_view: None,
            clip_plane: None,
            clip_depth: vec![],
            near_depth: f32::NEG_INFINITY,
            cockpit: crate::render::cockpit::Cockpit::new(),
            visible: vec![],
            mapping: vec![],
            stages: [0.0; 3],
            x_origin: 0,
            full_width: 0,
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
        self.render_connected(world, flight, size, theme, mission, None);
    }
    pub fn render_connected(
        &mut self,
        world: &World,
        flight: &Flight,
        size: (usize, usize),
        theme: usize,
        mission: Option<&Mission>,
        network: Option<&crate::world::network::Network>,
    ) {
        let (width, height) = size;
        if width > 320 && self.workers.is_empty() {
            self.workers = (0..3).map(|_| Self::single()).collect();
        }
        let cockpit = flight.weights[2];
        let rolled = cockpit > 0.01;
        // 座舱围绕画面上方的视线中心旋转，外围画布覆盖完整翻滚
        let reach_y = height as f32 * 1.28;
        let side = ((width * width) as f32 + reach_y * reach_y).sqrt().ceil() as usize + 4;
        self.w = if rolled { side } else { width };
        self.full_width = self.w;
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
        let has_panel =
            cockpit > 0.55 && flight.vehicle != crate::content::vehicles::Vehicle::Paper;
        if has_panel {
            self.cockpit.prepare(flight, world, size, theme, network);
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
        self.portal_mask(world, &camera, network);
        self.background(world, flight, &camera, theme, height);
        self.portals(world, flight, &camera, mission, theme, network);
        self.stages[1] = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        let mut jets: Vec<&Flight> = flight
            .wingmen
            .iter()
            .take(flight.wing_count)
            .filter(|wing| wing.portal_wait <= 0.0)
            .collect();
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
        if !world.kind.vacuum() {
            self.precipitation(flight, &camera, theme);
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
            let color = if flight.warp_flash > 0.0 {
                mix(*color, rgb(139, 220, 255), flight.warp_flash * 0.55)
            } else {
                *color
            };
            dest.copy_from_slice(&color.to_le_bytes());
        }
    }
    fn scene(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
        height: usize,
    ) {
        if world.kind != crate::world::MapKind::Space {
            self.world(world, flight, camera, theme);
        }
        if world.kind.vacuum() {
            self.cosmos(world, camera, flight.time);
        } else {
            self.sky(camera, flight, theme, height);
        }
        self.scenery(world, camera, flight.time);
    }

    fn background(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
        height: usize,
    ) {
        if self.workers.is_empty() {
            self.scene(world, flight, camera, theme, height);
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
                    worker.full_width = self.w;
                    worker.w = end - start;
                    worker.h = h;
                    worker.pixels.resize(worker.w * h, 0);
                    worker.pixels.fill(0);
                    worker.mask.resize(worker.w * h, 0);
                    worker.mask.fill(0);
                    worker.near_depth = self.near_depth;
                    worker.clip_plane = self.clip_plane;
                    worker.clip_depth.clear();
                    if !self.clip_depth.is_empty() {
                        worker
                            .clip_depth
                            .extend_from_slice(&self.clip_depth[start * h..end * h]);
                    }
                    worker.depth.resize(worker.w * h, f32::INFINITY);
                    worker
                        .depth
                        .copy_from_slice(&self.depth[start * h..end * h]);
                    worker.visible.clear();
                    worker.visible.extend_from_slice(&visible[start..end]);
                    worker.column.clear();
                    worker
                        .column
                        .extend(visible[start..end].iter().map(|v| v.1));
                    let mut view = *camera;
                    view.center.x -= start as f32;
                    scope.spawn(move || {
                        worker.scene(world, flight, &view, theme, height);
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
    }
}

#[cfg(test)]
mod parallel_tests {
    use super::*;
    #[test]
    fn column_workers_keep_scene_geometry_and_planets_consistent() {
        for kind in [
            crate::world::MapKind::City,
            crate::world::MapKind::Canyon,
            crate::world::MapKind::Space,
        ] {
            let world = World::generate(kind, kind.seed());
            let mut flight = Flight::new(&world);
            flight.position.z = 210.0;
            flight.follow = flight.position;
            flight.wing_count = 0;
            for pitch in [0.0, -0.55] {
                flight.pitch = pitch;
                for view in [1, 2] {
                    flight.weights = [0.0; 3];
                    flight.weights[view] = 1.0;
                    let mut single = Renderer::single();
                    let mut parallel = Renderer::single();
                    parallel.workers = (0..3).map(|_| Renderer::single()).collect();
                    single.render(&world, &flight, 320, 170, 0);
                    parallel.render(&world, &flight, 320, 170, 0);
                    let differences = single
                        .rgba
                        .chunks_exact(4)
                        .zip(parallel.rgba.chunks_exact(4))
                        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > 8))
                        .count();
                    assert!(
                        differences < 320 * 170 / 100,
                        "{} view {view} pitch {pitch}: {differences}",
                        kind.name()
                    );
                }
            }
        }
    }
}
