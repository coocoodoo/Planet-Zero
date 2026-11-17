use super::{App, Page};
use crate::world::{MapKind, SIZE, World, index, network::MapKey};
use glam::{Vec2, Vec3, vec2};

pub(crate) const CHART_SIZE: usize = 136;

#[derive(Default)]
pub(crate) struct Chart {
    pub zoom: usize,
    pub space: bool,
    pub focus: Option<Vec2>,
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

pub(crate) fn map_offset(kind: MapKind, position: Vec2, center: Vec2) -> Vec2 {
    if kind == MapKind::Space {
        position - center
    } else {
        offset(position, center)
    }
}

impl Chart {
    pub fn prepare(&mut self, world: &World, position: Vec3) {
        self.space = world.kind == MapKind::Space;
        let mut low = position.truncate();
        let mut high = low;
        if self.space {
            for body in crate::world::orbit::Body::ALL {
                low = low.min(body.center().truncate() - Vec2::splat(body.radius()));
                high = high.max(body.center().truncate() + Vec2::splat(body.radius()));
            }
        }
        let full_span = if self.space {
            2.0_f32.powf(
                ((high - low).max_element() * 1.3)
                    .max(32768.0)
                    .log2()
                    .ceil(),
            )
        } else {
            SIZE as f32
        };
        let center = if let Some(center) = self.focus {
            center
        } else if self.zoom == 0 {
            if self.space {
                (low + high) * 0.5
            } else {
                Vec2::splat(SIZE as f32 * 0.5)
            }
        } else {
            position.truncate()
        };
        let key = (MapKey::of(world), self.zoom, center);
        if self.cached == Some(key) {
            return;
        }
        self.center = center;
        self.span = full_span / (1 << self.zoom) as f32;
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
                self.pixels[y * CHART_SIZE + x] = color;
            }
        }
        self.cached = Some(key);
    }
    pub fn pan(&mut self, world: &World, position: Vec3, movement: Vec2) {
        self.prepare(world, position);
        let position = self.center + movement * self.span;
        self.focus = Some(if self.space {
            position
        } else {
            position.rem_euclid(Vec2::splat(SIZE as f32))
        });
    }
    pub fn zoom_by(&mut self, direction: i32, world: &World, position: Vec3) {
        self.prepare(world, position);
        self.focus = Some(self.center);
        self.zoom = (self.zoom as i32 + direction).clamp(0, 9) as usize;
    }
    pub fn point(&self, position: Vec3) -> Vec2 {
        Vec2::splat(CHART_SIZE as f32 * 0.5)
            + (if self.space {
                position.truncate() - self.center
            } else {
                offset(position.truncate(), self.center)
            }) * CHART_SIZE as f32
                / self.span
    }
}

impl App {
    pub(super) fn toggle_chart(&mut self) {
        if self.page == Page::Chart {
            self.page = self.chart_return;
        } else {
            self.chart_return = self.page;
            self.network.ensure(&self.world);
            if self
                .chart
                .cached
                .is_some_and(|(key, _, _)| key != MapKey::of(&self.world))
            {
                self.chart.focus = None;
                self.chart.zoom = 0;
            }
            self.chart.selected = 0;
            self.page = Page::Chart;
        }
        self.release_keys();
    }
}
