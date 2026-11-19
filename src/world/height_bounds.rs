use super::{SIZE, index};

const CELL: usize = 32;

#[derive(Clone, Default)]
pub(crate) struct HeightBounds {
    levels: Vec<Vec<[u8; 2]>>,
    vertices: Vec<u8>,
}

impl HeightBounds {
    pub fn build(heights: &super::MapData) -> Self {
        if heights.is_empty() {
            return Self::default();
        }
        let side = SIZE / CELL;
        let mut base = vec![[255, 0]; side * side];
        for y in 0..side {
            for x in 0..side {
                let range = &mut base[y * side + x];
                for dy in 0..=CELL {
                    let row = ((y * CELL + dy) % SIZE) * SIZE;
                    for dx in 0..=CELL {
                        let height = heights[row + (x * CELL + dx) % SIZE];
                        range[0] = range[0].min(height);
                        range[1] = range[1].max(height);
                    }
                }
            }
        }
        let mut levels = vec![base];
        for level in 1..=4 {
            let side = side >> level;
            let previous = &levels[level - 1];
            let mut bounds = vec![[255, 0]; side * side];
            for y in 0..side {
                for x in 0..side {
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let range = previous[(y * 2 + dy) * side * 2 + x * 2 + dx];
                        bounds[y * side + x][0] = bounds[y * side + x][0].min(range[0]);
                        bounds[y * side + x][1] = bounds[y * side + x][1].max(range[1]);
                    }
                }
            }
            levels.push(bounds);
        }
        let vertices = (0..SIZE / 4 * (SIZE / 4))
            .map(|i| heights[(i / (SIZE / 4) * 4) * SIZE + i % (SIZE / 4) * 4])
            .collect();
        Self { levels, vertices }
    }

    pub fn vertex(&self, x: i32, y: i32) -> f32 {
        self.vertices[index(x.div_euclid(4), y.div_euclid(4), SIZE / 4)] as f32
    }

    pub fn range(&self, x: i32, y: i32, size: i32) -> [u8; 2] {
        let level = size.max(CELL as i32).ilog2() as usize - CELL.ilog2() as usize;
        let cell = (CELL << level) as i32;
        self.levels[level][index(x.div_euclid(cell), y.div_euclid(cell), SIZE / cell as usize)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_include_shared_edges_and_periodic_neighbors() {
        let mut heights = vec![40; SIZE * SIZE];
        for (x, y, h) in [(32, 32, 250), (0, 0, 0), (4095, 4095, 220)] {
            heights[index(x, y, SIZE)] = h;
        }
        let bounds = HeightBounds::build(&heights.clone().into());
        for size in [8_i32, 16, 32, 64, 128, 256, 512] {
            for (x, y) in [(0, 0), (32, 32), (-size, -size), (4096, 4096)] {
                let x = x.div_euclid(size) * size;
                let y = y.div_euclid(size) * size;
                let [low, high] = bounds.range(x, y, size);
                for dy in 0..=size {
                    for dx in 0..=size {
                        assert!((low..=high).contains(&heights[index(x + dx, y + dy, SIZE)]));
                    }
                }
            }
        }
        assert_eq!(bounds.range(64, 64, 32), [40, 40]);
        for y in [-4100, -4, 0, 32, 4096] {
            for x in [-4100, -4, 0, 32, 4096] {
                assert_eq!(bounds.vertex(x, y), heights[index(x, y, SIZE)] as f32);
            }
        }
    }
}
