use super::SIZE;
use std::ops::{Index, IndexMut};

#[derive(Clone, Default)]
pub struct MapData {
    dense: Vec<u8>,
    tiles: Vec<u32>,
    data: Vec<u8>,
}

impl From<Vec<u8>> for MapData {
    fn from(dense: Vec<u8>) -> Self {
        Self {
            dense,
            ..Self::default()
        }
    }
}

impl MapData {
    pub(super) fn compact(&mut self) {
        if self.dense.is_empty() {
            return;
        }
        let mut tiles = Vec::with_capacity(SIZE * SIZE / 64);
        let mut data = Vec::new();
        for y in (0..SIZE).step_by(8) {
            for x in (0..SIZE).step_by(8) {
                let pixels: [u8; 64] =
                    std::array::from_fn(|i| self.dense[(y + i / 8) * SIZE + x + i % 8]);
                let mut palette = [0; 4];
                let mut codes = [0; 64];
                let mut count = 0;
                for (i, &pixel) in pixels.iter().enumerate() {
                    let color = match palette[..count].iter().position(|&p| p == pixel) {
                        Some(color) => color,
                        None => {
                            if count == 4 {
                                count = 5;
                                break;
                            }
                            palette[count] = pixel;
                            count += 1;
                            count - 1
                        }
                    };
                    codes[i] = color as u8;
                }
                let offset = data.len() as u32;
                match count {
                    1 => tiles.push(palette[0] as u32),
                    2..=4 => {
                        let bits = if count == 2 { 1 } else { 2 };
                        let colors = 1 << bits;
                        tiles.push((bits << 30) | offset);
                        data.extend_from_slice(&palette[..colors as usize]);
                        for group in codes.chunks_exact(8 / bits as usize) {
                            let mut packed = 0;
                            for (i, &color) in group.iter().enumerate() {
                                packed |= color << (i * bits as usize);
                            }
                            data.push(packed);
                        }
                    }
                    _ => {
                        tiles.push((3 << 30) | offset);
                        data.extend_from_slice(&pixels);
                    }
                }
            }
        }
        data.shrink_to_fit();
        if data.len() + tiles.len() * 4 < self.dense.len() {
            self.tiles = tiles;
            self.data = data;
            self.dense = Vec::new();
        }
    }

    fn expand(&mut self) {
        if !self.tiles.is_empty() {
            self.dense = self.iter().copied().collect();
            self.tiles = Vec::new();
            self.data = Vec::new();
        }
    }

    pub fn fill(&mut self, value: u8) {
        self.expand();
        self.dense.fill(value);
    }

    pub fn len(&self) -> usize {
        if self.tiles.is_empty() {
            self.dense.len()
        } else {
            SIZE * SIZE
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &u8> {
        (0..self.len()).map(|i| &self[i])
    }
}

impl Index<usize> for MapData {
    type Output = u8;

    #[inline]
    fn index(&self, at: usize) -> &u8 {
        if self.tiles.is_empty() {
            return &self.dense[at];
        }
        let x = at % SIZE;
        let y = at / SIZE;
        let tile = self.tiles[(y / 8) * (SIZE / 8) + x / 8];
        // 高两位区分单色、1 位索引、2 位索引和原始数据块
        let bits = tile >> 30;
        let offset = (tile & 0x3fff_ffff) as usize;
        if bits == 0 {
            static VALUES: [u8; 256] = {
                let mut values = [0; 256];
                let mut i = 0;
                while i < 256 {
                    values[i] = i as u8;
                    i += 1;
                }
                values
            };
            return &VALUES[offset];
        }
        let pixel = (y % 8) * 8 + x % 8;
        if bits == 3 {
            &self.data[offset + pixel]
        } else {
            let bit = pixel * bits as usize;
            let packed = self.data[offset + (1 << bits) + bit / 8];
            &self.data[offset + ((packed >> (bit % 8)) & ((1 << bits) - 1)) as usize]
        }
    }
}

impl IndexMut<usize> for MapData {
    fn index_mut(&mut self, at: usize) -> &mut u8 {
        self.expand();
        &mut self.dense[at]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_tiles_preserve_all_values_and_can_be_edited() {
        let source: Vec<u8> = (0..SIZE * SIZE)
            .map(|i| {
                let x = i % SIZE;
                match x / 8 % 4 {
                    0 => 255,
                    1 => (i % 2) as u8 + 128,
                    2 => (i % 3) as u8 + 250,
                    _ => ((i + i / SIZE) % 256) as u8,
                }
            })
            .collect();
        let mut map = MapData::from(source.clone());
        map.compact();
        assert!(map.data.len() + map.tiles.len() * 4 < source.len() / 2);
        assert!(map.iter().copied().eq(source.iter().copied()));
        map[123] = 34;
        assert_eq!(map[123], 34);
        assert_eq!(map[124], source[124]);
        map.compact();
        assert_eq!(map[123], 34);
    }
}
