use super::{App, Page};
use crate::world::{MapKind, SIZE, World, index, network::MapKey};
use glam::{Vec2, Vec3, vec2};

pub(crate) const CHART_SIZE: usize = 136;

#[derive(Default)]
pub(crate) struct Chart {
    pub zoom: usize,
    pub selected: usize,
    pub pixels: Vec<[u8; 3]>,
    pub center: Vec2,
    pub span: f32,
    cached: Option<(MapKey, usize, Vec2)>,
}

pub(crate) fn offset(position: Vec2, center: Vec2) -> Vec2 {
    let period = SIZE as f32;
    (position - center + Vec2::splat(period * 0.5)).rem_euclid(Vec2::splat(period))
        - Vec2::splat(period * 0.5)
}

impl Chart {
    pub fn prepare(&mut self, world: &World, position: Vec3) {
        let center = if self.zoom == 0 {
            Vec2::splat(SIZE as f32 * 0.5)
        } else {
            position.truncate()
        };
        let key = (MapKey::of(world), self.zoom, center);
        if self.cached == Some(key) {
            return;
        }
        self.center = center;
        self.span = SIZE as f32 / (1 << self.zoom) as f32;
        self.pixels.resize(CHART_SIZE * CHART_SIZE, [0; 3]);
        let step = self.span / CHART_SIZE as f32;
        for y in 0..CHART_SIZE {
            for x in 0..CHART_SIZE {
                let p = center
                    + (vec2(x as f32 + 0.5, y as f32 + 0.5) - Vec2::splat(CHART_SIZE as f32 * 0.5))
                        * step;
                let color = if world.kind == MapKind::Space {
                    [12, 22, 40]
                } else {
                    let i = index(p.x as i32, p.y as i32, SIZE);
                    let h = world.heights[i] as f32;
                    let neighbor = index((p.x + step) as i32, (p.y + step) as i32, SIZE);
                    let next = world.heights[neighbor] as f32;
                    let contour = h > 16.0 && (h / 24.0).floor() != (next / 24.0).floor();
                    let light = (0.62 + h / 550.0 + (h - next) * 0.012).clamp(0.35, 1.1)
                        * if contour { 0.75 } else { 1.0 };
                    crate::render::TOP[world.materials[i] as usize]
                        .map(|v| (v as f32 * light).min(255.0) as u8)
                };
                self.pixels[y * CHART_SIZE + x] = if x % 34 == 0 || y % 34 == 0 {
                    color.map(|v| v.saturating_add(13))
                } else {
                    color
                };
            }
        }
        self.cached = Some(key);
    }
    pub fn point(&self, position: Vec3) -> Vec2 {
        Vec2::splat(CHART_SIZE as f32 * 0.5)
            + offset(position.truncate(), self.center) * CHART_SIZE as f32 / self.span
    }
}

impl App {
    pub(super) fn toggle_chart(&mut self) {
        if self.page == Page::Chart {
            self.page = self.chart_return;
        } else {
            self.chart_return = self.page;
            self.network.ensure(&self.world);
            self.chart.selected = 0;
            self.page = Page::Chart;
        }
        self.release_keys();
    }
}
