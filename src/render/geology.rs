use super::*;
use crate::world::celestial::{Scenery, Structure};
impl Renderer {
    fn rock_box(
        &mut self,
        camera: &Camera,
        local: &impl Fn(Vec3) -> Vec3,
        center: Vec3,
        half: Vec3,
        color: u32,
    ) {
        let p: [Vec3; 8] = std::array::from_fn(|i| {
            local(
                center
                    + half
                        * vec3(
                            if i & 1 == 0 { -1.0 } else { 1.0 },
                            if i & 2 == 0 { -1.0 } else { 1.0 },
                            if i & 4 == 0 { -1.0 } else { 1.0 },
                        ),
            )
        });
        for face in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            self.rock_triangle(camera, [p[face[0]], p[face[1]], p[face[2]]], color);
            self.rock_triangle(camera, [p[face[0]], p[face[2]], p[face[3]]], color);
        }
    }
    pub(super) fn geological_structure(&mut self, camera: &Camera, object: Scenery) {
        let local = object.to_world();
        match object.kind {
            Structure::Arch | Structure::Cave => {
                let cave = object.kind == Structure::Cave;
                let (width, height, base, length) = if cave {
                    (48.0, 50.0, 24.0, 110.0)
                } else {
                    (74.0, 74.0, 20.0, 12.0)
                };
                let rock = if cave {
                    rgb(111, 103, 93)
                } else {
                    rgb(186, 130, 83)
                };
                for i in 0..12 {
                    let a = i as f32 * std::f32::consts::PI / 12.0;
                    let b = (i + 1) as f32 * std::f32::consts::PI / 12.0;
                    let inner = |angle: f32, y: f32| {
                        let irregular = (angle * 3.0 + y * 0.027).sin() * angle.sin() * 3.0;
                        vec3(
                            angle.cos() * (width + irregular),
                            y,
                            base + angle.sin() * (height + irregular),
                        )
                    };
                    let outer = |angle: f32, y: f32| {
                        vec3(
                            angle.cos() * (width + 12.0),
                            y,
                            base + angle.sin() * (height + 16.0),
                        )
                    };
                    for y in [-length, length] {
                        let p = [
                            local(inner(a, y)),
                            local(inner(b, y)),
                            local(outer(b, y)),
                            local(outer(a, y)),
                        ];
                        self.rock_triangle(camera, [p[0], p[1], p[2]], rock);
                        self.rock_triangle(camera, [p[0], p[2], p[3]], rock);
                        if cave && i % 3 == 0 {
                            self.rock_box(
                                camera,
                                &local,
                                inner(a, y * 0.65) - Vec3::Z * 3.0,
                                vec3(1.0, 1.5, 5.0),
                                mix(rock, rgb(57, 61, 59), 0.3),
                            );
                        }
                    }
                    for (is_inner, shade) in [(true, 0.45), (false, 0.05)] {
                        let point = |angle, y| {
                            if is_inner {
                                inner(angle, y)
                            } else {
                                outer(angle, y)
                            }
                        };
                        let segments = if cave { 6 } else { 1 };
                        for segment in 0..segments {
                            let y0 = -length + 2.0 * length * segment as f32 / segments as f32;
                            let y1 =
                                -length + 2.0 * length * (segment + 1) as f32 / segments as f32;
                            let p = [
                                local(point(a, y0)),
                                local(point(b, y0)),
                                local(point(b, y1)),
                                local(point(a, y1)),
                            ];
                            let color = mix(rock, rgb(23, 29, 35), shade + (i % 3) as f32 * 0.035);
                            self.rock_triangle(camera, [p[0], p[1], p[2]], color);
                            self.rock_triangle(camera, [p[0], p[2], p[3]], color);
                        }
                    }
                }
                for side in [-1.0, 1.0] {
                    self.rock_box(
                        camera,
                        &local,
                        vec3(side * (width + 6.0), 0.0, base * 0.5),
                        vec3(6.0, length, base * 0.5),
                        rock,
                    );
                }
            }
            Structure::Stairs => {
                for i in 0..24 {
                    self.box_mesh(
                        camera,
                        &local,
                        vec3(0.0, i as f32 * 4.0, i as f32 * 4.0 + 2.0),
                        vec3(8.0, 2.0, 2.0),
                        rgb(183, 169, 135),
                    );
                }
                for side in [-1.0, 1.0] {
                    self.world_line(
                        camera,
                        local(vec3(side * 8.0, 0.0, 8.0)),
                        local(vec3(side * 8.0, 96.0, 104.0)),
                        rgb(114, 91, 61),
                    );
                }
            }
            Structure::IceSpire | Structure::Boulder => {
                let ice = object.kind == Structure::IceSpire;
                let h = if ice { 45.0 } else { 10.0 };
                for i in 0..6 {
                    let a = i as f32 * std::f32::consts::TAU / 6.0;
                    let b = (i + 1) as f32 * std::f32::consts::TAU / 6.0;
                    let color = if ice {
                        if i % 2 == 0 {
                            rgb(211, 243, 251)
                        } else {
                            rgb(99, 178, 208)
                        }
                    } else {
                        rgb(128, 132, 119)
                    };
                    self.rock_triangle(
                        camera,
                        [
                            local(vec3(a.cos() * 10.0, a.sin() * 8.0, 0.0)),
                            local(vec3(b.cos() * 10.0, b.sin() * 8.0, 0.0)),
                            local(vec3(3.0, -2.0, h)),
                        ],
                        color,
                    );
                }
            }
            _ => {}
        }
    }
}

fn stone_texture() -> &'static Vec<u8> {
    static TEXTURE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    TEXTURE.get_or_init(|| {
        let noise = |x: f32, y: f32, cell: i32| {
            let u = x / cell as f32;
            let v = y / cell as f32;
            let (ix, iy) = (u.floor() as i32, v.floor() as i32);
            let (tx, ty) = (u - u.floor(), v - v.floor());
            let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
            let sample =
                |dx, dy| hash(index(ix + dx, iy + dy, (256 / cell) as usize) as u32) as f32 / 255.0;
            let (a, b, c, d) = (sample(0, 0), sample(1, 0), sample(0, 1), sample(1, 1));
            a + (b - a) * tx + (c - a + (d - c - b + a) * tx) * ty
        };
        (0..256 * 256)
            .map(|i| {
                let (x, y) = ((i % 256) as f32, (i / 256) as f32);
                let broad = noise(x, y, 32);
                let veins = noise(x + broad * 5.0, y, 8);
                let grain = hash(i as u32) as f32 / 255.0;
                let layer = (y * 0.125 + broad * 1.4 + veins * 0.2).fract();
                let fissure = (x * 0.0625 + broad * 2.0 + veins * 0.5).fract();
                let mut shade = 4.0 + broad * 5.0 + veins * 3.0 + grain * 2.0;
                if layer < 0.075 {
                    shade -= 3.0;
                }
                if fissure < 0.04 && veins < 0.65 {
                    shade -= 5.0;
                }
                shade.clamp(0.0, 15.0) as u8
            })
            .collect()
    })
}

fn stone_mips() -> &'static Vec<Vec<u8>> {
    static MIPS: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    MIPS.get_or_init(|| {
        let base = stone_texture();
        let mut levels = vec![base.clone()];
        let mut size = 256usize;
        while size > 4 {
            let src = levels.last().unwrap();
            let next = size / 2;
            let mut level = vec![0u8; next * next];
            for y in 0..next {
                for x in 0..next {
                    let sum = src[(y * 2) * size + x * 2] as u32
                        + src[(y * 2) * size + x * 2 + 1] as u32
                        + src[(y * 2 + 1) * size + x * 2] as u32
                        + src[(y * 2 + 1) * size + x * 2 + 1] as u32;
                    level[y * next + x] = (sum / 4) as u8;
                }
            }
            levels.push(level);
            size = next;
        }
        levels
    })
}

pub(super) fn stone_texel(p: Vec3) -> usize {
    let u = (p.x * 0.8 + p.y * 0.6).floor() as i32;
    let v = (p.z * 1.2 + p.y * 0.3).floor() as i32;
    stone_texture()[index(u, v, 256)] as usize
}

/// 按视距在相邻层级间插值采样，越远纹样越平缓
pub(super) fn stone_texels_lod<const N: usize>(points: [Vec3; N], lod: f32) -> [f32; N] {
    if lod <= 0.0 {
        return points.map(|p| stone_texel(p) as f32);
    }
    let levels = stone_mips();
    let lod = lod.clamp(0.0, levels.len() as f32 - 1.0);
    let lower = lod.floor() as usize;
    let blend = lod - lower as f32;
    points.map(|p| {
        let u = (p.x * 0.8 + p.y * 0.6).floor() as i32;
        let v = (p.z * 1.2 + p.y * 0.3).floor() as i32;
        let sample =
            |level: usize| levels[level][index(u >> level, v >> level, 256usize >> level)] as f32;
        let value = sample(lower);
        if blend == 0.0 {
            value
        } else {
            value * (1.0 - blend) + sample(lower + 1) * blend
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn variance(lod: f32) -> f32 {
        let mut values = Vec::new();
        for y in 0..24 {
            for x in 0..24 {
                values.push(stone_texels_lod([vec3(x as f32 * 7.0, y as f32 * 7.0, 20.0)], lod)[0]);
            }
        }
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32
    }
    #[test]
    fn distant_levels_flatten_stone_detail() {
        let near = variance(0.0);
        let far = variance(3.0);
        assert!(far < near * 0.6, "near {near} far {far}");
    }
}
