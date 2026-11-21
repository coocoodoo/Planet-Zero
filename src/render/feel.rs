use super::*;

impl Renderer {
    /// 低空时投在地面上的椭圆影子，帮助判断离地高度
    pub(super) fn shadow(&mut self, world: &World, flight: &Flight, camera: &Camera) {
        if world.kind == crate::world::MapKind::Space
            || flight.surface.is_some_and(|surface| surface.bend > 0.00001)
        {
            return;
        }
        let ground = world.height(flight.position.x, flight.position.y);
        let height = flight.position.z - ground;
        let strength = 1.0 - height / 120.0;
        if strength <= 0.0 {
            return;
        }
        let Some((p, d, s)) = camera.project(flight.position.with_z(ground)) else {
            return;
        };
        let rx = (8.0 * s).max(2.0);
        let ry = (rx * 0.45).max(1.0);
        let shade = rgb(10, 14, 22);
        let amount = 0.38 * strength.min(1.0);
        let (x0, x1) = ((p.x - rx) as i32, (p.x + rx) as i32);
        let (y0, y1) = ((p.y - ry) as i32, (p.y + ry) as i32);
        for x in x0.max(0)..=x1.min(self.w as i32 - 1) {
            let (top, bottom) = self.visible[x as usize];
            for y in y0.max(top)..=y1.min(bottom - 1) {
                let nx = (x as f32 - p.x) / rx;
                let ny = (y as f32 - p.y) / ry;
                if nx * nx + ny * ny > 1.0 {
                    continue;
                }
                let at = x as usize * self.h + y as usize;
                // 只压暗地面附近的像素，不落到飞机或云上
                if (self.depth[at] - d).abs() < 24.0 && self.mask[at] != 1 {
                    self.pixels[at] = mix(self.pixels[at], shade, amount);
                }
            }
        }
    }

    /// 加速时从画面中心向外的速度线，与背景混合
    pub(super) fn speed_lines(&mut self, flight: &Flight, camera: &Camera) {
        let boost = flight.boost.max(flight.orbital_thrust);
        if boost < 0.3 || flight.weights[1] < 0.6 {
            return;
        }
        let strength = (boost - 0.3) / 0.7;
        let diagonal = ((self.w * self.w + self.h * self.h) as f32).sqrt() * 0.5;
        let frame = (flight.time * 20.0) as u32;
        for i in 0..16_u32 {
            let h =
                crate::world::hash(frame.wrapping_mul(31).wrapping_add(i * 7919)) as f32 / 255.0;
            let g = crate::world::hash(i.wrapping_mul(131).wrapping_add(frame)) as f32 / 255.0;
            let angle = (i as f32 + h) * std::f32::consts::TAU / 16.0;
            let direction = vec2(angle.cos(), angle.sin());
            let start = diagonal * (0.42 + 0.3 * g);
            let length = diagonal * (0.1 + 0.15 * h) * strength;
            let steps = length as i32;
            for step in 0..steps {
                let t = step as f32 / steps.max(1) as f32;
                let at = camera.center + direction * (start + length * t);
                let (x, y) = (at.x as i32, at.y as i32);
                if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
                    break;
                }
                let index = x as usize * self.h + y as usize;
                let fade = strength * 0.6 * (1.0 - (t - 0.5).abs() * 2.0);
                self.pixels[index] = mix(self.pixels[index], rgb(236, 244, 255), fade);
            }
        }
    }

    /// 冲出或再入大气层时机头前方的等离子辉光
    pub(super) fn plasma(&mut self, flight: &Flight, camera: &Camera) {
        if flight.weights[2] > 0.5 {
            return;
        }
        let air = crate::world::orbit::thin_air(flight.position.z);
        let heat = flight.orbital_thrust * air * (1.0 - air) * 4.0;
        if heat < 0.05 {
            return;
        }
        let Some((p, d, s)) = camera.project(flight.local(vec3(0.0, 30.0, 0.0))) else {
            return;
        };
        let radius = (9.0 * s * heat).clamp(2.0, 18.0);
        let reach = radius.ceil() as i32;
        let flicker = 0.85 + 0.15 * (flight.time * 37.0).sin();
        for y in -reach..=reach {
            for x in -reach..=reach {
                let distance = ((x * x + y * y) as f32).sqrt() / radius;
                if distance > 1.0 {
                    continue;
                }
                let at = p + vec2(x as f32, y as f32);
                let cell = ((at.y as i32 & 3) << 2 | (at.x as i32 & 3)) as u32;
                // 外圈抖动透明，内圈由白热过渡到粉橙
                if (crate::world::hash(cell) as f32 / 255.0) > (1.0 - distance) * 1.6 * heat {
                    continue;
                }
                let color = mix(rgb(255, 250, 235), rgb(255, 112, 140), distance);
                let (px, py) = (at.x as i32, at.y as i32);
                if px >= 0 && py >= 0 && px < self.w as i32 && py < self.h as i32 {
                    let index = px as usize * self.h + py as usize;
                    if self.depth[index] >= d - 20.0 {
                        self.pixels[index] =
                            mix(self.pixels[index], color, (1.0 - distance) * flicker * 0.85);
                    }
                }
            }
        }
    }
}
