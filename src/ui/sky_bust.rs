//! Ray-traces a voxel bust (`sky::bust`) into the talk panel's portrait: an orthographic view
//! looking slightly down, swaying left and right, each cube face lit by its normal with a thin
//! dark seam between cubes so the voxels read on a 320×170 screen.
use super::canvas::Canvas;
use crate::sky::bust::{Bust, GLOW, SIZE, SOLID};
use glam::{Vec3, vec3};

/// Draw `bust` filling the rectangle (x, y, w, h); `time` drives the sway and the bob.
pub(super) fn portrait(c: &mut Canvas, bust: &Bust, time: f32, x: i32, y: i32, w: i32, h: i32) {
    let size = vec3(SIZE[0] as f32, SIZE[1] as f32, SIZE[2] as f32);
    let yaw = (time * 0.6).sin() * 0.5;
    let pitch = 0.22_f32;
    // camera axes in the bust's grid: looking toward +y (the face is at small y)
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let turn = |v: Vec3| vec3(v.x * cy - v.y * sy, v.x * sy + v.y * cy, v.z);
    let dir = turn(vec3(0.0, cp, -sp));
    let up = turn(vec3(0.0, sp, cp));
    let right = turn(Vec3::X);
    let bob = (time * 2.1).sin() * 0.25;
    let center = size * 0.5 + vec3(0.0, 0.0, 0.6 + bob);
    let scale = (h as f32 / 22.5).min(w as f32 / 17.0);
    let light = vec3(-0.45, -0.7, 0.75).normalize();
    for py in 0..h {
        for px in 0..w {
            let sx = (px as f32 + 0.5 - w as f32 * 0.5) / scale;
            let sz = (h as f32 * 0.5 - py as f32 - 0.5) / scale;
            let origin = center + right * sx + up * sz - dir * 40.0;
            if let Some(color) = trace(bust, origin, dir, size, light) {
                c.pixel(x + px, y + py, color);
            }
        }
    }
}

/// Walk the grid along the ray (Amanatides–Woo) and shade the first solid cube's face.
fn trace(bust: &Bust, origin: Vec3, dir: Vec3, size: Vec3, light: Vec3) -> Option<[u8; 3]> {
    // enter the grid's box
    let inv = dir.recip();
    let t0 = (Vec3::ZERO - origin) * inv;
    let t1 = (size - origin) * inv;
    let near = t0.min(t1);
    let far = t0.max(t1);
    let enter = near.max_element().max(0.0);
    let leave = far.min_element();
    if leave <= enter {
        return None;
    }
    let mut axis = if near.x >= near.y && near.x >= near.z {
        0
    } else if near.y >= near.z {
        1
    } else {
        2
    };
    let p = origin + dir * (enter + 1e-4);
    let mut cell = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
    let step = [
        dir.x.signum() as i32,
        dir.y.signum() as i32,
        dir.z.signum() as i32,
    ];
    let delta = inv.abs();
    let mut next = [0.0_f32; 3];
    for i in 0..3 {
        let boundary = cell[i] as f32 + if step[i] > 0 { 1.0 } else { 0.0 };
        next[i] = if dir[i] == 0.0 {
            f32::INFINITY
        } else {
            enter + (boundary - p[i]) / dir[i]
        };
    }
    let mut t = enter;
    for _ in 0..64 {
        let v = bust.get(cell[0], cell[1], cell[2]);
        if v & SOLID != 0 {
            let [r, g, b, _] = v.to_le_bytes();
            let base = [r as f32, g as f32, b as f32];
            let hit = origin + dir * t;
            // where on the face the ray landed, for the seams between cubes
            let (u, w) = match axis {
                0 => (hit.y, hit.z),
                1 => (hit.x, hit.z),
                _ => (hit.x, hit.y),
            };
            let seam = u
                .fract()
                .min(1.0 - u.fract())
                .min(w.fract().min(1.0 - w.fract()))
                < 0.07;
            let mut normal = Vec3::ZERO;
            normal[axis] = -step[axis] as f32;
            // glowing cubes keep most of their light but still show their shape
            let lit = if v & GLOW != 0 {
                0.78 + 0.3 * normal.dot(light).max(0.0)
            } else {
                0.42 + 0.58 * normal.dot(light).max(0.0)
            };
            let k = if seam { lit * 0.72 } else { lit };
            return Some(base.map(|c| (c * k).min(255.0) as u8));
        }
        // step to the next cube
        axis = if next[0] < next[1] && next[0] < next[2] {
            0
        } else if next[1] < next[2] {
            1
        } else {
            2
        };
        t = next[axis];
        if t > leave {
            return None;
        }
        cell[axis] += step[axis];
        next[axis] += delta[axis];
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::galaxy::Galaxy;
    use crate::sky::interior::people;

    #[test]
    fn the_portrait_shows_the_bust_with_lit_and_shaded_faces() {
        let galaxy = Galaxy::default();
        let system = galaxy.system(galaxy.start());
        let npc = &people(&system, 1, 0.0)[0];
        let bust = crate::sky::bust::bust(npc);
        let mut pixels = vec![0_u8; crate::app::WIDTH * crate::app::HEIGHT * 4];
        let mut c = Canvas {
            pixels: &mut pixels,
        };
        portrait(&mut c, &bust, 0.0, 8, 20, 72, 96);
        let mut drawn = 0;
        let mut shades = std::collections::HashSet::new();
        for y in 20..116 {
            for x in 8..80 {
                let i = (y * crate::app::WIDTH + x) * 4;
                if pixels[i + 3] == 255 {
                    drawn += 1;
                    shades.insert([pixels[i], pixels[i + 1], pixels[i + 2]]);
                }
            }
        }
        // the bust covers a good part of the portrait, in many shades (faces, seams, grain)
        assert!(drawn > 72 * 96 / 4, "{drawn}");
        assert!(shades.len() > 30, "{}", shades.len());
        // nothing outside the portrait
        assert!(pixels[(10 * crate::app::WIDTH + 100) * 4 + 3] == 0);
    }
}
