mod aircraft;
mod celestial;
mod clouds;
mod cockpit;
mod feel;
mod geology;
mod landmarks;
mod lighting;
mod monuments;
mod objects;
mod orbit;
mod palette;
mod portals;
mod primitives;
mod settlements;
mod synthwave;
mod terrain;
mod terrain_cache;
mod tilted;
mod weather;
use crate::gameplay::{Mission, Mode, PickupKind, Weapon};
use glam::{Vec2, Vec3, vec2, vec3};
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

use crate::content::weather::Weather;
use crate::flight::{Camera, Flight, MODEL_SCALE};
use crate::render::palette::JET;
use crate::world::{MATERIAL_COUNT, SIZE, World, hash, index};

const MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/falcon.bin"));
pub(crate) const TOP: [[u8; 3]; MATERIAL_COUNT] = [
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

pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
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
    light: lighting::Lighting,
    scenery_fog: Option<(f32, f32, u32)>,
    warp: Option<crate::world::orbit::SurfaceWarp>,
    terrain_cache: terrain_cache::TerrainCache,
    workers: Vec<Renderer>,
    portal_view: Option<Box<Renderer>>,
    portal_samples: Vec<(u32, u32, f32)>,
    /// 虫洞合成时读取的原画面副本，跨帧复用
    portal_base: Vec<u32>,
    /// 实心三角形写入的遮罩值；敌机用独立值以便描边
    mark: u8,
    near_depth: f32,
    cockpit: crate::render::cockpit::Cockpit,
    visible: Vec<(i32, i32)>,
    mapping: Vec<usize>,
    mapping_key: Option<(usize, usize, usize, usize, u32, u64)>,
    /// 座舱画布尺寸与是否旋转；变化时才整张重置遮挡与深度
    canvas: Option<(usize, usize, bool)>,
    /// 上次映射中由遮挡变为可见的画布位置
    /// 画布像素的揭开印记，等于 `reveal_stamp` 表示可见；并行映射时各线程直接写入
    reveal: Vec<AtomicU8>,
    reveal_stamp: u8,
    previous_visible: Vec<(i32, i32)>,
    remap_bands: Vec<RemapBand>,
    pub stages: [f64; 3],
    pub triangles: u32,
    x_origin: usize,
    full_width: usize,
    w: usize,
    h: usize,
    pixels: Vec<u32>,
    depth: Vec<f32>,
    mask: Vec<u8>,
    cloud_pixels: Vec<clouds::CloudPixel>,
    cloud_touched: Vec<usize>,
    cloud_vertices: Vec<Option<clouds::CachedVertex>>,
    column: Vec<i32>,
    pub rgba: Vec<u8>,
    output: Vec<u32>,
    bloom: Vec<[u16; 3]>,
    voxels: Vec<Vec<(f32, f32, usize, bool)>>,
    pub detail: [f64; 4],
}

/// 当前光照的黄昏、夜晚与合成器浪潮程度，供配乐同步过渡
pub fn time_of_day(flight: &Flight, world: &World, theme: usize) -> (f32, f32, f32) {
    let light = lighting::Lighting::new(flight, world, theme);
    (light.twilight(), light.night(), light.synthwave)
}

/// 列的可见行区间，裁剪到画布高度
fn span(top: i32, bottom: i32, h: usize) -> std::ops::Range<usize> {
    let top = top.clamp(0, h as i32) as usize;
    let bottom = bottom.clamp(0, h as i32) as usize;
    top..bottom.max(top)
}

/// 按每列可见像素数的前缀和切分列区间，使各线程的绘制量接近
fn strips(visible: &[(i32, i32)], count: usize, h: usize) -> Vec<usize> {
    let w = visible.len();
    // 每列另计少量固定开销，避免全遮挡区域被并成一条
    let weight = |x: usize| span(visible[x].0, visible[x].1, h).len() + 4;
    let total: usize = (0..w).map(weight).sum();
    let mut bounds = Vec::with_capacity(count + 1);
    bounds.push(0);
    let mut acc = 0;
    let mut x = 0;
    for i in 1..count {
        let goal = total * i / count;
        while x < w && acc + weight(x) / 2 < goal {
            acc += weight(x);
            x += 1;
        }
        bounds.push(x);
    }
    bounds.push(w);
    bounds
}

fn render_pool() -> &'static rayon_core::ThreadPool {
    static POOL: std::sync::OnceLock<rayon_core::ThreadPool> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        rayon_core::ThreadPoolBuilder::new()
            .num_threads(3)
            .thread_name(|i| format!("render-{i}"))
            .build()
            .expect("创建渲染线程失败")
    })
}

/// 座舱翻滚时屏幕到外围画布的旋转映射参数
#[derive(Clone, Copy)]
struct Rotation {
    screen: (usize, usize),
    canvas: (usize, usize),
    pivot: Vec2,
    offset: Vec2,
    sin_cos: (f32, f32),
}

/// 一段屏幕行的旋转映射：给未被座舱遮挡的画布像素打上本次印记，并记录各画布列的可见区间
#[derive(Default)]
struct RemapBand {
    visible: Vec<(i32, i32)>,
}

impl Renderer {
    /// 画布像素是否被座舱遮挡（不在本次旋转映射的可见集合中）
    fn occluded(&self, at: usize) -> bool {
        self.reveal
            .get(at)
            .is_some_and(|r| r.load(Relaxed) != self.reveal_stamp)
    }
}

impl RemapBand {
    fn remap(
        &mut self,
        rotation: &Rotation,
        cockpit: Option<&cockpit::Cockpit>,
        (reveal, stamp): (&[AtomicU8], u8),
        first_row: usize,
        mapping: &mut [usize],
    ) {
        let (width, height) = rotation.screen;
        let (canvas_w, canvas_h) = rotation.canvas;
        let (s, c) = rotation.sin_cos;
        let pivot = rotation.pivot;
        self.visible.clear();
        self.visible.resize(canvas_w, (canvas_h as i32, 0));
        for (row, mapping) in mapping.chunks_mut(width).enumerate() {
            let y = first_row + row;
            let py = y as f32 - pivot.y;
            let mut q =
                vec2(-pivot.x * c - py * s, -pivot.x * s + py * c) + pivot + rotation.offset;
            for (x, target) in mapping.iter_mut().enumerate() {
                let sx = (q.x.round() as usize).min(canvas_w - 1);
                let sy = (q.y.round() as usize).min(canvas_h - 1);
                *target = sx * canvas_h + sy;
                if cockpit.is_none_or(|cockpit| {
                    !cockpit.opaque(
                        if width == 320 { x } else { x * 320 / width },
                        if height == 170 { y } else { y * 170 / height },
                    )
                }) {
                    reveal[*target].store(stamp, Relaxed);
                    let column = &mut self.visible[sx];
                    column.0 = column.0.min(sy as i32);
                    column.1 = column.1.max(sy as i32 + 1);
                }
                q.x += c;
                q.y += s;
            }
        }
    }
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
            renderer.workers = (0..4).map(|_| Self::single()).collect();
        }
        renderer
    }
    fn single() -> Self {
        let mut voxels = vec![Vec::new(); 13];
        for z in 0..13 {
            for y in 0..46 {
                for x in 0..34 {
                    let color = MODEL[(z * 46 + y) * 34 + x] as usize;
                    if color != 0 {
                        let exterior = x == 0
                            || x == 33
                            || y == 0
                            || y == 45
                            || z == 0
                            || z == 12
                            || [
                                ((z * 46 + y) * 34 + x) - 1,
                                ((z * 46 + y) * 34 + x) + 1,
                                (z * 46 + y - 1) * 34 + x,
                                (z * 46 + y + 1) * 34 + x,
                                ((z - 1) * 46 + y) * 34 + x,
                                ((z + 1) * 46 + y) * 34 + x,
                            ]
                            .iter()
                            .any(|&i| MODEL[i] == 0);
                        voxels[z].push((x as f32 - 16.5, 25.3 - y as f32, color, exterior));
                    }
                }
            }
        }
        Self {
            light: lighting::Lighting::default(),
            scenery_fog: None,
            warp: None,
            terrain_cache: terrain_cache::TerrainCache::default(),
            workers: vec![],
            portal_view: None,
            portal_samples: Vec::new(),
            portal_base: Vec::new(),
            mark: 1,
            near_depth: f32::NEG_INFINITY,
            cockpit: crate::render::cockpit::Cockpit::new(),
            visible: vec![],
            mapping: vec![],
            mapping_key: None,
            canvas: None,
            reveal: Vec::new(),
            reveal_stamp: 0,
            previous_visible: Vec::new(),
            remap_bands: Vec::new(),
            stages: [0.0; 3],
            triangles: 0,
            x_origin: 0,
            full_width: 0,
            w: 0,
            h: 0,
            pixels: vec![],
            depth: vec![],
            mask: vec![],
            cloud_pixels: vec![],
            cloud_touched: vec![],
            cloud_vertices: vec![],
            column: vec![],
            rgba: vec![],
            output: vec![],
            bloom: vec![],
            voxels,
            detail: [0.0; 4],
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
            self.cockpit.prepare(flight, world, size, theme, network);
            let strength = lighting::Lighting::new(flight, world, theme).synthwave;
            if strength > 0.0 {
                for color in &mut self.cockpit.pixels {
                    if *color == 0 {
                        continue;
                    }
                    let [r, g, b, _] = color.to_le_bytes();
                    if r.abs_diff(g) < 28 && g.abs_diff(b) < 35 {
                        *color = mix(*color, rgb(44, 20, 81), 0.42 * strength);
                    }
                }
            }
        }
        self.visible.resize(self.w, (self.h as i32, 0));
        self.mapping.resize(width * height, 0);
        self.output.resize(width * height, 0);
        let roll = flight.screen_roll() * cockpit;
        let (s, c) = roll.sin_cos();
        let pivot = vec2(width as f32 * 0.5, height as f32 * 0.43);
        let mapping_key = (
            self.w,
            self.h,
            width,
            height,
            roll.to_bits(),
            if has_panel { self.cockpit.revision } else { 0 },
        );
        if self.mapping_key != Some(mapping_key) {
            let canvas = (self.w, self.h, rolled);
            self.previous_visible.clear();
            if self.canvas != Some(canvas) || self.reveal.len() != self.pixels.len() {
                self.reveal.clear();
                self.reveal
                    .resize_with(self.pixels.len(), || AtomicU8::new(0));
                // 未翻滚的画布全部可见：印记 0 与清零后的像素一致
                self.reveal_stamp = 0;
                if rolled {
                    self.depth.fill(f32::NEG_INFINITY);
                }
                self.canvas = Some(canvas);
            } else if rolled {
                // 旧可见区间里新区间之外的部分恢复为不可绘制
                self.previous_visible.extend_from_slice(&self.visible);
            }
            if rolled {
                // 换一个印记即可让上次揭开的像素全部重新被遮挡；回绕时整体清零
                self.reveal_stamp = self.reveal_stamp.wrapping_add(1);
                if self.reveal_stamp == 0 {
                    self.reveal.iter().for_each(|r| r.store(0, Relaxed));
                    self.reveal_stamp = 1;
                }
            }
            self.visible.fill((self.h as i32, 0));
            if !rolled {
                self.visible.fill((0, height as i32));
                for y in 0..height {
                    for x in 0..width {
                        self.mapping[y * width + x] = x * height + y;
                    }
                }
            } else {
                // 旋转映射按行分段并行计算，各段直接给可见像素打印记，再合并各段的可见列区间
                let rotation = Rotation {
                    screen: (width, height),
                    canvas: (self.w, self.h),
                    pivot,
                    offset,
                    sin_cos: (s, c),
                };
                let cockpit = has_panel.then_some(&self.cockpit);
                let reveal = (self.reveal.as_slice(), self.reveal_stamp);
                let bands = &mut self.remap_bands;
                bands.resize_with(4, Default::default);
                let rows = height.div_ceil(bands.len());
                render_pool().in_place_scope(|scope| {
                    for (i, (band, mapping)) in bands
                        .iter_mut()
                        .zip(self.mapping.chunks_mut(rows * width))
                        .enumerate()
                        .rev()
                    {
                        let mut remap =
                            move || band.remap(&rotation, cockpit, reveal, i * rows, mapping);
                        if i == 0 {
                            remap();
                        } else {
                            scope.spawn(move |_| remap());
                        }
                    }
                });
                for band in &self.remap_bands[..height.div_ceil(rows)] {
                    for (merged, &(top, bottom)) in self.visible.iter_mut().zip(&band.visible) {
                        merged.0 = merged.0.min(top);
                        merged.1 = merged.1.max(bottom);
                    }
                }
                let h = self.h;
                for (x, (&old, &new)) in self.previous_visible.iter().zip(&self.visible).enumerate()
                {
                    let (old, new) = (span(old.0, old.1, h), span(new.0, new.1, h));
                    let column = &mut self.depth[x * h..(x + 1) * h];
                    column[old.start..new.start.clamp(old.start, old.end)].fill(f32::NEG_INFINITY);
                    column[new.end.clamp(old.start, old.end)..old.end].fill(f32::NEG_INFINITY);
                }
            }
            self.mapping_key = Some(mapping_key);
        }
        if rolled {
            // 可见区外的深度保持负无穷且不会被写入，只需重置每列的可见区间
            let h = self.h;
            for (x, &(top, bottom)) in self.visible.iter().enumerate() {
                let rows = span(top, bottom, h);
                let column = x * h;
                for (depth, reveal) in self.depth[column + rows.start..column + rows.end]
                    .iter_mut()
                    .zip(&self.reveal[column + rows.start..column + rows.end])
                {
                    *depth = if reveal.load(Relaxed) == self.reveal_stamp {
                        f32::INFINITY
                    } else {
                        f32::NEG_INFINITY
                    };
                }
            }
        } else {
            self.depth.fill(f32::INFINITY);
        }
        for (column, visible) in self.column.iter_mut().zip(&self.visible) {
            *column = visible.1;
        }
        self.stages[0] = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        self.portal_mask(world, flight, &camera, network, 0.81);
        self.draw_parallel(world, flight, &camera, theme, false);
        let portals_started = std::time::Instant::now();
        self.portals(world, flight, &camera, mission, theme, network);
        self.detail[3] = portals_started.elapsed().as_secs_f64() * 1000.0;
        self.stages[1] = started.elapsed().as_secs_f64() * 1000.0;
        let started = std::time::Instant::now();
        self.shadow(world, flight, &camera);
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
            self.trails(jet, &camera);
        }
        for jet in jets {
            self.jet(jet, &camera);
        }
        self.plasma(flight, &camera);
        if let Some(mission) = mission {
            self.mission(mission, flight, &camera);
        }
        // 当前世界的云和降水以传送门表面作为遮挡边界
        if flight.portal_transit.is_some() && !world.kind.vacuum() {
            self.portal_mask(world, flight, &camera, network, 1.06);
        }
        self.draw_parallel(world, flight, &camera, theme, true);
        if !world.kind.vacuum() {
            self.precipitation(world, flight, &camera);
        }
        self.speed_lines(flight, &camera);
        if world.kind == crate::world::MapKind::City && self.light.synthwave > 0.0 {
            self.synthwave_glow();
        }
        if rolled {
            // 按映射从画布随机取像素，访存占主导，分段并行
            let panel = has_panel.then_some(self.cockpit.pixels.as_slice());
            let canvas = self.pixels.as_slice();
            let chunk = self.output.len().div_ceil(4);
            render_pool().in_place_scope(|scope| {
                for (i, (output, mapping)) in self
                    .output
                    .chunks_mut(chunk)
                    .zip(self.mapping.chunks(chunk))
                    .enumerate()
                    .rev()
                {
                    let mut compose = move || {
                        let panel = panel.map(|panel| &panel[i * chunk..]);
                        for (j, (dest, &source)) in output.iter_mut().zip(mapping).enumerate() {
                            *dest = match panel {
                                Some(panel) if panel[j] != 0 => panel[j],
                                _ => canvas[source],
                            };
                        }
                    };
                    if i == 0 {
                        compose();
                    } else {
                        scope.spawn(move |_| compose());
                    }
                }
            });
        } else {
            for by in (0..height).step_by(16) {
                for bx in (0..width).step_by(16) {
                    for y in by..(by + 16).min(height) {
                        let row =
                            &mut self.output[y * width + bx..y * width + (bx + 16).min(width)];
                        for (x, dest) in row.iter_mut().enumerate() {
                            *dest = self.pixels[(bx + x) * height + y];
                        }
                    }
                }
            }
        }
        if has_panel {
            self.cockpit.glass(&mut self.output, width, height, flight);
        }
        self.stages[2] = started.elapsed().as_secs_f64() * 1000.0;
        self.rgba.resize(width * height * 4, 0);
        let hurt = mission.map_or(0.0, |m| m.hurt / 0.35);
        if hurt > 0.0 {
            self.hurt_frame(width, height, hurt, flight.time);
            return;
        }
        for (dest, color) in self
            .rgba
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&self.output)
        {
            let color = if flight.warp_flash > 0.0 {
                mix(*color, rgb(139, 220, 255), flight.warp_flash * 0.55)
            } else {
                *color
            };
            dest.copy_from_slice(&color.to_le_bytes());
        }
    }
    /// 受击画面：整体抖动一两个像素，边缘偏红并随时间淡去
    fn hurt_frame(&mut self, width: usize, height: usize, hurt: f32, time: f32) {
        let amplitude = hurt * 2.0 * width as f32 / 320.0;
        let dx = ((time * 97.0).sin() * amplitude).round() as isize;
        let dy = ((time * 83.0).cos() * amplitude).round() as isize;
        let red = rgb(255, 40, 36);
        for y in 0..height {
            let sy = (y as isize + dy).clamp(0, height as isize - 1) as usize;
            let ny = (y as f32 / height as f32 - 0.5).abs() * 2.0;
            for x in 0..width {
                let sx = (x as isize + dx).clamp(0, width as isize - 1) as usize;
                let nx = (x as f32 / width as f32 - 0.5).abs() * 2.0;
                let edge = nx.max(ny);
                let color = mix(
                    self.output[sy * width + sx],
                    red,
                    hurt * 0.55 * edge * edge * edge,
                );
                let at = (y * width + x) * 4;
                self.rgba[at..at + 4].copy_from_slice(&color.to_le_bytes());
            }
        }
    }
    fn layer(&mut self, world: &World, flight: &Flight, camera: &Camera, clouds: bool) {
        if clouds {
            let started = std::time::Instant::now();
            self.warp = flight.surface.filter(|surface| surface.bend > 0.00001);
            self.draw_clouds(world, flight, camera);
            self.warp = None;
            self.detail[1] = started.elapsed().as_secs_f64() * 1000.0;
            return;
        }
        self.detail = [0.0; 4];
        self.triangles = 0;
        self.warp = flight.surface.filter(|surface| surface.bend > 0.00001);
        if world.kind != crate::world::MapKind::Space {
            self.world(world, flight, camera);
        }
        if world.kind.vacuum() {
            self.cosmos(world, camera, flight);
        } else {
            self.sky(world, camera, flight);
            let bend = self.warp.map_or(0.0, |warp| warp.bend);
            let thin_air = 1.0
                - (1.0 - crate::world::orbit::thin_air(flight.position.z)) * (1.0 - bend).powi(2);
            if thin_air > 0.0 {
                self.stars(camera, thin_air, flight.sky_rotation, 1.0 - thin_air);
            }
            self.orbital_bodies(world, camera, flight);
        }
        let started = std::time::Instant::now();
        if !world.kind.vacuum() {
            let (start, end) = flight.weather.visibility(camera.position.z);
            let bend = self.warp.map_or(0.0, |warp| warp.bend) * 10000.0;
            let (start, end) = (start.min(3900.0) + bend, end.min(4200.0) + bend);
            let start = start + (650.0 - start) * self.light.synthwave;
            self.scenery_fog = Some((start, 1.0 / (end - start), self.light.haze(flight.weather)));
        }
        self.scenery(world, camera, flight.time, 1.0);
        self.scenery_fog = None;
        self.warp = None;
        if let (Some(space), Some(surface)) = (&flight.orbital_scenery, flight.surface) {
            let mut view = *camera;
            view.position = surface.space_origin()
                + surface.basis * (camera.position - surface.anchor.extend(0.0));
            view.right = surface.basis * camera.right;
            view.up = surface.basis * camera.up;
            view.forward = surface.basis * camera.forward;
            self.scenery(space, &view, flight.time, surface.bend);
        }
        self.detail[2] = started.elapsed().as_secs_f64() * 1000.0;
    }

    fn draw_parallel(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
        clouds: bool,
    ) {
        if clouds
            && (world.clouds.is_empty() || crate::world::orbit::thin_air(flight.position.z) >= 1.0)
        {
            return;
        }
        self.light = lighting::Lighting::new(flight, world, theme);
        if self.workers.is_empty() {
            self.layer(world, flight, camera, clouds);
        } else {
            let w = self.w;
            let h = self.h;
            let bounds = strips(&self.visible, self.workers.len(), h);
            let visible = &self.visible;
            let (pixels, depth) = (&self.pixels, &self.depth);
            render_pool().in_place_scope(|scope| {
                for (i, worker) in self.workers.iter_mut().enumerate().rev() {
                    let (start, end) = (bounds[i], bounds[i + 1]);
                    worker.light = self.light;
                    worker.x_origin = start;
                    worker.full_width = w;
                    worker.w = end - start;
                    worker.h = h;
                    worker.visible.clear();
                    worker.visible.extend_from_slice(&visible[start..end]);
                    if worker.visible.iter().all(|&(top, bottom)| top >= bottom) {
                        worker.detail = [0.0; 4];
                        worker.triangles = 0;
                        continue;
                    }
                    worker.pixels.resize(worker.w * h, 0);
                    worker.depth.resize(worker.w * h, f32::INFINITY);
                    worker.mask.resize(worker.w * h, 0);
                    worker.mask.fill(0);
                    if clouds {
                        worker.triangles = 0;
                    } else {
                        worker.pixels.fill(0);
                    }
                    // 只搬运可见行，座舱遮挡与虫洞透镜外的像素不会显示
                    for (x, &(top, bottom)) in worker.visible.iter().enumerate() {
                        let rows = span(top, bottom, h);
                        if rows.is_empty() {
                            continue;
                        }
                        let local = x * h + rows.start..x * h + rows.end;
                        let global = (start + x) * h + rows.start..(start + x) * h + rows.end;
                        if clouds {
                            worker.pixels[local.clone()].copy_from_slice(&pixels[global.clone()]);
                        }
                        worker.depth[local].copy_from_slice(&depth[global]);
                    }
                    worker.near_depth = self.near_depth;
                    worker.column.clear();
                    worker
                        .column
                        .extend(visible[start..end].iter().map(|v| v.1));
                    let mut view = *camera;
                    view.center.x -= start as f32;
                    let mut draw = move || worker.layer(world, flight, &view, clouds);
                    if i == 0 {
                        draw();
                    } else {
                        scope.spawn(move |_| draw());
                    }
                }
            });
            if clouds {
                self.detail[1] = self.workers.iter().map(|w| w.detail[1]).fold(0.0, f64::max);
                self.triangles += self.workers.iter().map(|w| w.triangles).sum::<u32>();
            } else {
                self.detail = std::array::from_fn(|i| {
                    self.workers.iter().map(|w| w.detail[i]).fold(0.0, f64::max)
                });
                self.triangles = self.workers.iter().map(|w| w.triangles).sum();
            }
            for (i, worker) in self.workers.iter().enumerate() {
                for (x, &(top, bottom)) in worker.visible.iter().enumerate() {
                    let rows = span(top, bottom, h);
                    if rows.is_empty() {
                        continue;
                    }
                    let local = x * h + rows.start..x * h + rows.end;
                    let global = (bounds[i] + x) * h + rows.start..(bounds[i] + x) * h + rows.end;
                    self.pixels[global.clone()].copy_from_slice(&worker.pixels[local.clone()]);
                    if !clouds {
                        self.depth[global].copy_from_slice(&worker.depth[local]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod parallel_tests {
    use super::*;
    #[test]
    fn strips_balance_visible_pixels_and_cover_every_column() {
        // 中间窄、两侧被座舱遮挡的可见区域
        let visible: Vec<(i32, i32)> = (0..400)
            .map(|x| {
                if (40..360).contains(&x) {
                    (60, 300)
                } else {
                    (400, 0)
                }
            })
            .collect();
        let bounds = strips(&visible, 4, 400);
        assert_eq!((bounds[0], bounds[4]), (0, 400));
        assert!(bounds.windows(2).all(|w| w[0] <= w[1]));
        let loads: Vec<usize> = bounds
            .windows(2)
            .map(|w| {
                (w[0]..w[1])
                    .map(|x| span(visible[x].0, visible[x].1, 400).len())
                    .sum()
            })
            .collect();
        let (min, max) = (loads.iter().min().unwrap(), loads.iter().max().unwrap());
        assert!(*max - *min <= 2 * 240, "{loads:?}");
    }
    #[test]
    fn column_workers_preserve_depth_for_foreground_occlusion() {
        let mut world = World::generate(crate::world::MapKind::Moon, 7);
        world.portals.clear();
        world.scenery.clear();
        let mut flight = Flight::new(&world);
        flight.position.z = 300.0;
        flight.follow = flight.position;
        flight.pitch = -0.45;
        flight.weights = [0.0, 1.0, 0.0];
        flight.wing_count = 0;
        let mut renderer = Renderer::single();
        renderer.workers = (0..4).map(|_| Renderer::single()).collect();
        renderer.render(&world, &flight, 320, 170, 0);
        let at = renderer.depth.iter().position(|d| d.is_finite()).unwrap();
        let color = renderer.pixels[at];
        renderer.point(
            vec2((at / renderer.h) as f32, (at % renderer.h) as f32),
            renderer.depth[at] + 100.0,
            rgb(255, 0, 255),
            0,
            1,
        );
        assert_eq!(renderer.pixels[at], color);
        for worker in &renderer.workers {
            for (i, depth) in worker.depth.iter().enumerate() {
                if depth.is_finite() {
                    assert!(renderer.depth[worker.x_origin * renderer.h + i] <= *depth + 0.01);
                }
            }
        }
    }
    #[test]
    fn curved_surface_matches_between_single_and_column_workers() {
        use crate::world::orbit::{Body, SurfaceWarp};
        let world = World::generate(crate::world::MapKind::Moon, 7);
        let mut flight = Flight::new(&world);
        flight.position.z = 900.0;
        flight.follow = flight.position;
        flight.pitch = -0.5;
        flight.wing_count = 0;
        for bend in [0.2, 0.7, 1.0] {
            flight.surface = Some(SurfaceWarp::new(
                Body::Moon,
                flight.position.truncate(),
                flight.position.truncate(),
                bend,
            ));
            for view in 0..3 {
                flight.weights = [0.0; 3];
                flight.weights[view] = 1.0;
                let mut single = Renderer::single();
                let mut parallel = Renderer::single();
                parallel.workers = (0..3).map(|_| Renderer::single()).collect();
                single.render(&world, &flight, 320, 170, 0);
                parallel.render(&world, &flight, 320, 170, 0);
                let differences = single
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(parallel.rgba.as_chunks::<4>().0.iter())
                    .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > 8))
                    .count();
                assert!(
                    differences < 320 * 170 / 100,
                    "bend {bend} view {view}: {differences}"
                );
            }
        }
    }
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
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(parallel.rgba.as_chunks::<4>().0.iter())
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
    #[test]
    fn aurora_preserves_moon_and_foreground_pixels() {
        let world = World::generate(crate::world::MapKind::Icebergs, 7);
        let mut flight = Flight::new(&world);
        flight.position = vec3(1024.0, 1024.0, 240.0);
        flight.follow = flight.position;
        flight.yaw = 0.0;
        flight.pitch = 0.2;
        flight.weights = [0.0, 1.0, 0.0];
        flight.wing_count = 0;
        let mut renderer = Renderer::single();
        renderer.render(&world, &flight, 320, 170, 2);
        let before = renderer.pixels.clone();
        assert!(renderer.depth.iter().any(|d| d.is_finite() && *d > 5000.0));
        let camera = Camera::new(&flight, 320, 170);
        renderer.aurora(&camera, &flight, 1.0);
        for (i, depth) in renderer.depth.iter().enumerate() {
            if depth.is_finite() {
                assert_eq!(renderer.pixels[i], before[i]);
            }
        }
        assert_ne!(renderer.pixels, before);
    }

    #[test]
    fn fog_hides_distant_buildings_and_keeps_nearby_geometry() {
        use crate::world::celestial::{Scenery, Structure};
        let mut world = World::generate(crate::world::MapKind::City, 7);
        world.scenery.clear();
        world.portals.clear();
        world.clouds.clear();
        let mut flight = Flight::new(&world);
        flight.position.z = 240.0;
        flight.follow = flight.position;
        flight.weights = [0.0, 1.0, 0.0];
        flight.wing_count = 0;
        flight.weather =
            crate::content::weather::WeatherState::new(Weather::Fog, world.kind, world.seed);
        let camera = Camera::new(&flight, 320, 170);
        let mut renderer = Renderer::single();
        renderer.render(&world, &flight, 320, 170, 0);
        let empty = renderer.rgba.clone();
        world.scenery.push(Scenery {
            position: camera.position + camera.forward * 600.0,
            yaw: 0.0,
            scale: 1.0,
            kind: Structure::Building,
        });
        renderer.render(&world, &flight, 320, 170, 0);
        assert_eq!(renderer.rgba, empty);
        world.scenery[0].position = camera.position + camera.forward * 100.0;
        renderer.render(&world, &flight, 320, 170, 0);
        assert_ne!(renderer.rgba, empty);
    }
}

#[cfg(test)]
mod equivalence_tests {
    use super::*;
    // 同一状态下单线程与按可见量分条的并行渲染应只在条带接缝处有零星差异
    #[test]
    fn single_and_parallel_render_the_same_frames() {
        use crate::app::App;
        use crate::world::MapKind;
        for (kind, seed) in [
            (MapKind::Islands, 7),
            (MapKind::City, 2049),
            (MapKind::Alpine, 31),
            (MapKind::Moon, MapKind::Moon.seed()),
            (MapKind::Space, MapKind::Space.seed()),
        ] {
            let mut app = App::new(World::generate(kind, seed));
            app.start();
            app.auto = true;
            app.flight.wing_count = 2;
            app.network.active = true;
            app.network.ensure(&app.world);
            app.network
                .summon(&app.world, app.flight.position, app.flight.axes()[1]);
            for _ in 0..2000 {
                app.prepare_scene();
                app.network.poll();
                if !app.network.loading() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            for frame in 0..240 {
                app.prepare_scene();
                app.network.poll();
                app.step(1.0 / 30.0, None);
                if frame % 40 != 39 {
                    continue;
                }
                for view in 0..3 {
                    let mut flight = app.flight.clone();
                    flight.weights = [0.0; 3];
                    flight.weights[view] = 1.0;
                    if view == 2 {
                        flight.bank = 0.6;
                    }
                    let mut parallel = Renderer::single();
                    parallel.workers = (0..4).map(|_| Renderer::single()).collect();
                    let mut single = Renderer::single();
                    for r in [&mut parallel, &mut single] {
                        r.render_connected(
                            &app.world,
                            &flight,
                            (320, 170),
                            app.theme,
                            Some(&app.mission),
                            Some(&app.network),
                        );
                    }
                    let diff = parallel
                        .rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(single.rgba.as_chunks::<4>().0.iter())
                        .filter(|(a, b)| a != b)
                        .count();
                    assert!(
                        diff < 320 * 170 / 100,
                        "{kind:?} frame {frame} view {view}: {diff}"
                    );
                }
            }
        }
    }
}
