use crate::app::{HEIGHT, WIDTH};
use font8x8::UnicodeFonts;

pub(super) const WHITE: [u8; 3] = [225, 238, 244];
pub(super) const MUTED: [u8; 3] = [145, 171, 188];
pub(super) const CYAN: [u8; 3] = [111, 237, 218];
pub(super) const GOLD: [u8; 3] = [255, 208, 105];
pub(super) const RED: [u8; 3] = [255, 112, 130];

pub(super) struct Canvas<'a> {
    pub(super) pixels: &'a mut [u8],
}
impl Canvas<'_> {
    pub(super) fn pixel(&mut self, x: i32, y: i32, c: [u8; 3]) {
        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
            let i = (y as usize * WIDTH + x as usize) * 4;
            self.pixels[i..i + 3].copy_from_slice(&c);
            self.pixels[i + 3] = 255;
        }
    }
    pub(super) fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.pixel(xx, yy, c);
            }
        }
    }
    pub(super) fn shade(&mut self, x: usize, y: usize, w: usize, h: usize) {
        for yy in y..(y + h).min(HEIGHT) {
            for xx in x..(x + w).min(WIDTH) {
                let i = (yy * WIDTH + xx) * 4;
                let alpha = self.pixels[i + 3] as u32;
                if alpha == 255 {
                    for c in 0..3 {
                        self.pixels[i + c] = (self.pixels[i + c] as u16 / 4 + [7, 13, 22][c]) as u8;
                    }
                    continue;
                }
                let combined = 765 + alpha;
                for c in 0..3 {
                    self.pixels[i + c] = ((self.pixels[i + c] as u32 * alpha
                        + [7, 13, 22][c] * 1020)
                        / combined) as u8;
                }
                self.pixels[i + 3] = (combined / 4) as u8;
            }
        }
    }
    pub(super) fn text(&mut self, x: i32, y: i32, value: &str, c: [u8; 3]) {
        self.text_scale(x, y, value, c, 1);
    }
    pub(super) fn text_scale(&mut self, x: i32, y: i32, value: &str, c: [u8; 3], scale: i32) {
        for (i, ch) in value.chars().enumerate() {
            if let Some(glyph) = font8x8::BASIC_FONTS.get(ch).or(match ch {
                '▲' => Some([0, 8, 28, 62, 127, 0, 0, 0]),
                '▼' => Some([0, 127, 62, 28, 8, 0, 0, 0]),
                '◀' => Some([0, 16, 24, 28, 30, 28, 24, 16]),
                '▶' => Some([0, 4, 12, 28, 60, 28, 12, 4]),
                _ => None,
            }) {
                for (gy, row) in glyph.iter().enumerate() {
                    for gx in 0..8 {
                        if row & (1 << gx) != 0 {
                            self.rect(
                                x + (i as i32 * 8 + gx) * scale,
                                y + gy as i32 * scale,
                                scale,
                                scale,
                                c,
                            );
                        }
                    }
                }
            }
        }
    }
    pub(super) fn line(&mut self, x: i32, y: i32, xx: i32, yy: i32, c: [u8; 3]) {
        let steps = (xx - x).abs().max((yy - y).abs()).max(1);
        for i in 0..=steps {
            self.pixel(x + (xx - x) * i / steps, y + (yy - y) * i / steps, c);
        }
    }
    pub(super) fn bar(&mut self, x: i32, y: i32, w: i32, value: f32, c: [u8; 3]) {
        self.rect(x, y, w, 4, [38, 48, 65]);
        self.rect(x, y, (value.clamp(0.0, 1.0) * w as f32) as i32, 4, c);
    }
}

impl Canvas<'_> {
    pub(super) fn small(&mut self, x: i32, y: i32, value: &str, color: [u8; 3]) {
        const DIGITS: [[u8; 5]; 10] = [
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
        for (i, ch) in value.chars().enumerate() {
            let glyph = match ch {
                '0'..='9' => DIGITS[ch as usize - '0' as usize],
                '/' => [1, 1, 2, 4, 4],
                'd' => [1, 1, 7, 5, 7],
                'B' => [6, 5, 6, 5, 6],
                'm' => [0, 5, 7, 5, 5],
                '+' => [0, 2, 7, 2, 0],
                '-' => [0, 0, 7, 0, 0],
                _ => [0; 5],
            };
            for (yy, row) in glyph.iter().enumerate() {
                for xx in 0..3 {
                    if row & (1 << (2 - xx)) != 0 {
                        self.pixel(x + i as i32 * 4 + xx, y + yy as i32, color);
                    }
                }
            }
        }
    }
    pub(super) fn icon(&mut self, x: i32, y: i32, kind: char, color: [u8; 3]) {
        let rows = match kind {
            'u' => [0, 28, 34, 73, 34, 28, 0],
            'd' => [127, 65, 91, 85, 91, 65, 127],
            'i' => [28, 34, 73, 85, 73, 34, 28],
            'a' => [8, 28, 42, 73, 8, 8, 28],
            's' => [8, 42, 28, 127, 28, 42, 8],
            'c' => [0, 12, 18, 49, 65, 127, 0],
            'r' => [12, 18, 49, 127, 0, 42, 21],
            'n' => [8, 42, 28, 127, 28, 42, 8],
            'f' => [0, 119, 0, 62, 0, 119, 0],
            'b' => [28, 34, 93, 99, 65, 65, 0],
            'v' => [8, 0, 34, 8, 0, 65, 4],
            'S' => [62, 65, 65, 65, 34, 20, 8],
            'A' | 'Y' => [6, 12, 24, 62, 12, 24, 16],
            'B' => [12, 16, 28, 62, 62, 62, 28],
            'W' => [8, 8, 42, 127, 42, 8, 20],
            'V' | 'P' => [8, 28, 62, 127, 8, 20, 34],
            'X' => [62, 127, 85, 127, 62, 42, 42],
            '!' => [8, 28, 28, 8, 8, 0, 8],
            '+' => [0, 8, 8, 62, 8, 8, 0],
            'K' => [0, 1, 3, 70, 108, 56, 16],
            'F' => [62, 50, 44, 32, 32, 32, 32],
            'T' => [28, 8, 127, 62, 62, 127, 127],
            'C' => [8, 28, 127, 127, 28, 28, 62],
            'O' => [28, 34, 73, 73, 65, 34, 28],
            _ => [0; 7],
        };
        for (yy, row) in rows.iter().enumerate() {
            for xx in 0..7 {
                if row & (1 << (6 - xx)) != 0 {
                    self.pixel(x + xx, y + yy as i32, color);
                }
            }
        }
    }
}

impl Canvas<'_> {
    pub(super) fn weather(
        &mut self,
        x: i32,
        y: i32,
        weather: crate::content::weather::Weather,
        vacuum: bool,
    ) {
        use crate::content::weather::Weather;
        let (kind, color) = if vacuum {
            ('v', CYAN)
        } else {
            match weather {
                Weather::Clear => ('s', GOLD),
                Weather::Cloudy => ('c', WHITE),
                Weather::Rainbow => ('b', GOLD),
                Weather::Rain => ('r', CYAN),
                Weather::Snow => ('n', WHITE),
                Weather::Storm => ('A', GOLD),
                Weather::Fog => ('f', MUTED),
                Weather::Sandstorm => ('f', GOLD),
            }
        };
        self.icon(x, y, kind, color);
    }
}
