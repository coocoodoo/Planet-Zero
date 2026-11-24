//! Extra scene content supplied by game modes that live outside the engine (Cardputer Sky):
//! lit or glowing triangles (surface props, stations, asteroids), parked aircraft, and a colour
//! grade for the finished 3D frame. The renderer draws them after the terrain with depth testing.
use super::*;

/// One world-space triangle. `glow` triangles ignore lighting (crystals, beacons, station lights).
#[derive(Clone, Copy, Debug)]
pub struct ExtraTriangle {
    pub points: [Vec3; 3],
    pub color: u32,
    pub glow: bool,
}

/// A 3×3 colour matrix plus offset applied to every pixel of the 3D frame (not the HUD).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    pub matrix: [[f32; 3]; 3],
    pub offset: [f32; 3],
}
impl Grade {
    pub const IDENTITY: Self = Self {
        matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        offset: [0.0; 3],
    };
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }
    /// Blend toward another grade; `t` = 0 keeps `self`.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut out = *self;
        for r in 0..3 {
            for c in 0..3 {
                out.matrix[r][c] += (other.matrix[r][c] - self.matrix[r][c]) * t;
            }
            out.offset[r] += (other.offset[r] - self.offset[r]) * t;
        }
        out
    }
    #[inline]
    pub fn apply(&self, color: u32) -> u32 {
        let [r, g, b, _] = color.to_le_bytes();
        let input = [r as f32, g as f32, b as f32];
        let channel = |row: usize| {
            let m = self.matrix[row];
            (m[0] * input[0] + m[1] * input[1] + m[2] * input[2] + self.offset[row])
                .clamp(0.0, 255.0) as u8
        };
        rgb(channel(0), channel(1), channel(2))
    }
}
impl Default for Grade {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Everything a game mode adds to the next frame. Buffers are reused between frames.
#[derive(Default)]
pub struct Extras {
    pub triangles: Vec<ExtraTriangle>,
    pub ships: Vec<Flight>,
    pub grade: Grade,
}
impl Extras {
    pub fn clear(&mut self) {
        self.triangles.clear();
        self.ships.clear();
        self.grade = Grade::IDENTITY;
    }
}

impl Renderer {
    pub(super) fn draw_extras(&mut self, camera: &Camera) {
        let triangles = std::mem::take(&mut self.extras.triangles);
        for triangle in &triangles {
            if triangle.glow {
                let color = self.light.color(triangle.color);
                self.surface(camera, triangle.points, color, false, None);
            } else {
                self.triangle(camera, triangle.points, triangle.color);
            }
        }
        self.extras.triangles = triangles;
        let ships = std::mem::take(&mut self.extras.ships);
        for ship in &ships {
            self.jet(ship, camera);
        }
        self.extras.ships = ships;
    }
    pub(super) fn grade_output(&mut self) {
        let grade = self.extras.grade;
        if grade.is_identity() {
            return;
        }
        for color in &mut self.output {
            *color = grade.apply(*color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_grade_keeps_colors_and_lerp_reaches_the_target() {
        let c = rgb(12, 200, 99);
        assert_eq!(Grade::IDENTITY.apply(c), c);
        let tint = Grade {
            matrix: [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            offset: [0.0; 3],
        };
        assert_eq!(tint.apply(c), rgb(200, 12, 99));
        assert_eq!(Grade::IDENTITY.lerp(&tint, 1.0), tint);
        assert_eq!(Grade::IDENTITY.lerp(&tint, 0.0), Grade::IDENTITY);
    }
}
