use super::*;

/// 敌机像素的遮罩值，描边时只识别敌机自身
const ENEMY_MARK: u8 = 3;

impl Renderer {
    pub(super) fn mission(&mut self, mission: &Mission, flight: &Flight, camera: &Camera) {
        for (index, gate) in mission
            .gates
            .iter()
            .enumerate()
            .skip(mission.gate_index)
            .take(4)
            .rev()
        {
            let right = vec3(-gate.normal.y, gate.normal.x, 0.0);
            let active = index == mission.gate_index;
            let color = if active {
                rgb(100, 255, 201)
            } else {
                rgb(105, 154, 178)
            };
            for i in 0..40 {
                let a = i as f32 * std::f32::consts::TAU / 40.0;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 40.0;
                for r in [gate.radius, gate.radius + if active { 2.0 } else { 0.0 }] {
                    self.world_line(
                        camera,
                        gate.position + (right * a.cos() + Vec3::Z * a.sin()) * r,
                        gate.position + (right * b.cos() + Vec3::Z * b.sin()) * r,
                        color,
                    );
                }
            }
            if active {
                let r = gate.radius;
                for side in [-1.0, 1.0] {
                    self.world_line(
                        camera,
                        gate.position + right * side * (r + 8.0),
                        gate.position + right * side * (r + 3.0),
                        rgb(255, 243, 155),
                    );
                }
            }
        }
        for pickup in &mission.pickups {
            if pickup.collected || pickup.position.distance_squared(flight.position) > 650.0 * 650.0
            {
                continue;
            }
            let Some((p, d, scale)) = camera.project(pickup.position) else {
                continue;
            };
            let color = match pickup.kind {
                PickupKind::Coin => rgb(255, 218, 71),
                PickupKind::Upgrade(w) => {
                    let c = w.color();
                    rgb(c[0], c[1], c[2])
                }
                PickupKind::Shield => rgb(109, 255, 165),
                PickupKind::Energy => rgb(103, 222, 255),
                PickupKind::Bomb => rgb(255, 112, 91),
                PickupKind::Support => rgb(216, 174, 255),
                PickupKind::Time => rgb(200, 255, 240),
            };
            let radius = (scale * 6.0).round().clamp(2.0, 18.0) as i32;
            let rotate = (flight.time * 3.0).cos().abs() * 0.7 + 0.3;
            for y in -radius..=radius {
                for x in -radius..=radius {
                    let nx = x as f32
                        / (radius as f32
                            * if pickup.kind == PickupKind::Coin {
                                rotate
                            } else {
                                1.0
                            });
                    let ny = y as f32 / radius as f32;
                    let distance = nx * nx + ny * ny;
                    if distance < 1.0 {
                        self.point(
                            p + vec2(x as f32, y as f32),
                            d,
                            if distance > 0.55 {
                                color
                            } else {
                                mix(color, rgb(30, 35, 62), 0.5)
                            },
                            0,
                            1,
                        );
                    }
                }
            }
            if pickup.kind != PickupKind::Coin {
                use font8x8::UnicodeFonts;
                let label = match pickup.kind {
                    PickupKind::Upgrade(w) => match w {
                        Weapon::Gun => 'G',
                        Weapon::Missile => 'M',
                        Weapon::Laser => 'L',
                        Weapon::Plasma => 'P',
                        Weapon::Orb => 'O',
                    },
                    PickupKind::Shield => 'S',
                    PickupKind::Energy => 'A',
                    PickupKind::Bomb => 'B',
                    PickupKind::Support => 'W',
                    PickupKind::Time => 'T',
                    PickupKind::Coin => 'C',
                };
                if let Some(glyph) = font8x8::BASIC_FONTS.get(label) {
                    self.point(p, d - 1.0, color, 5, 0);
                    self.point(p, d - 1.1, rgb(20, 32, 45), 4, 0);
                    for (y, row) in glyph.iter().enumerate() {
                        for x in 0..8 {
                            if row & (1 << x) != 0 {
                                self.point(
                                    p + vec2(x as f32 - 3.0, y as f32 - 3.0),
                                    d - 1.2,
                                    rgb(244, 251, 240),
                                    0,
                                    0,
                                );
                            }
                        }
                    }
                }
            }
        }

        if let Some(convoy) = &mission.convoy {
            let right = vec3(flight.yaw.cos(), flight.yaw.sin(), 0.0);
            let forward = vec3(flight.yaw.sin(), -flight.yaw.cos(), 0.0);
            let local =
                |x: f32, y: f32, z: f32| convoy.position + right * x + forward * y + Vec3::Z * z;
            for side in [-1.0, 1.0] {
                self.triangle(
                    camera,
                    [
                        local(0.0, 20.0, 0.0),
                        local(side * 5.0, -15.0, 0.0),
                        local(0.0, -7.0, 6.0),
                    ],
                    rgb(146, 222, 238),
                );
                self.triangle(
                    camera,
                    [
                        local(side * 3.0, 7.0, 0.0),
                        local(side * 27.0, -8.0, 0.0),
                        local(side * 4.0, -12.0, 0.0),
                    ],
                    rgb(53, 121, 164),
                );
                self.triangle(
                    camera,
                    [
                        local(side * 3.0, -8.0, 4.0),
                        local(side * 11.0, -18.0, 4.0),
                        local(side * 4.0, -16.0, 8.0),
                    ],
                    rgb(210, 238, 240),
                );
                if let Some((p, d, _)) = camera.project(local(side * 13.0, -5.0, 0.0)) {
                    self.point(p, d, rgb(255, 211, 124), 1, 0);
                }
            }
            if flight.hud
                && let Some((p, d, _)) = camera.project(convoy.position)
            {
                self.stroke(
                    p + vec2(-12.0, -10.0),
                    p + vec2(-12.0 + 24.0 * convoy.health / convoy.max_health, -10.0),
                    d - 20.0,
                    rgb(83, 241, 217),
                );
            }
        }
        for enemy in &mission.enemies {
            if enemy.is_ground() {
                let p = enemy.position;
                let color = if enemy.hit > 0.1 {
                    rgb(255, 240, 180)
                } else {
                    rgb(157, 143, 112)
                };
                for side in [-1.0, 1.0] {
                    self.triangle(
                        camera,
                        [
                            p + vec3(-12.0, side * 10.0, 0.0),
                            p + vec3(12.0, side * 10.0, 0.0),
                            p + vec3(0.0, 0.0, 12.0),
                        ],
                        color,
                    );
                    self.triangle(
                        camera,
                        [
                            p + vec3(side * 12.0, -10.0, 0.0),
                            p + vec3(side * 12.0, 10.0, 0.0),
                            p + vec3(0.0, 0.0, 12.0),
                        ],
                        mix(color, rgb(30, 37, 46), 0.4),
                    );
                    let gun = p + Vec3::Z * 10.0 + Vec3::X * side * 3.0;
                    self.world_line(
                        camera,
                        gun,
                        gun + (flight.position - gun).normalize_or_zero() * 17.0,
                        rgb(227, 133, 85),
                    );
                }
                continue;
            }
            let scale = if enemy.is_boss() {
                3.2
            } else {
                match enemy.kind {
                    7 => 0.7,
                    8 => 1.2,
                    _ => 0.75 + enemy.kind as f32 * 0.18,
                }
            };
            let forward = vec3(enemy.yaw.sin(), -enemy.yaw.cos(), 0.0);
            let right = vec3(enemy.yaw.cos(), enemy.yaw.sin(), 0.0);
            let bank = enemy.phase.cos() * 0.25;
            let right = right * bank.cos() + Vec3::Z * bank.sin();
            let local = |x: f32, y: f32, z: f32| {
                enemy.position + (right * x + forward * y + Vec3::Z * z) * scale
            };
            let color = if enemy.hit > 0.1 {
                rgb(255, 252, 232)
            } else {
                match enemy.kind {
                    0 => rgb(239, 98, 112),
                    1 => rgb(171, 119, 235),
                    2 => rgb(209, 161, 101),
                    3 => rgb(113, 133, 172),
                    4 => rgb(141, 208, 123),
                    5 => rgb(189, 135, 215),
                    7 => rgb(120, 221, 193),
                    _ => rgb(255, 172, 76),
                }
            };
            let dark = mix(color, rgb(22, 24, 42), 0.55);
            self.mark = ENEMY_MARK;
            for side in [-1.0, 1.0] {
                self.triangle(
                    camera,
                    [
                        local(0.0, 14.0, 1.0),
                        local(side * 5.0, -11.0, 0.0),
                        local(0.0, -8.0, 5.0),
                    ],
                    color,
                );
                self.triangle(
                    camera,
                    [
                        local(side * 3.0, 4.0, 1.0),
                        local(side * 19.0, -7.0, 0.0),
                        local(side * 4.0, -10.0, 2.0),
                    ],
                    dark,
                );
                self.triangle(
                    camera,
                    [
                        local(side * 3.0, -6.0, 3.0),
                        local(side * 8.0, -14.0, 2.0),
                        local(side * 4.0, -12.0, 8.0),
                    ],
                    color,
                );
                if enemy.is_boss() {
                    self.triangle(
                        camera,
                        [
                            local(side * 7.0, 8.0, 1.0),
                            local(side * 15.0, -8.0, 1.0),
                            local(side * 10.0, -6.0, 5.0),
                        ],
                        color,
                    );
                }
                if let Some((p, d, s)) = camera.project(local(side * 3.0, -11.0, 1.0)) {
                    self.point(
                        p,
                        d,
                        rgb(255, 92, 74),
                        (s * scale).round().clamp(1.0, 4.0) as i32,
                        0,
                    );
                }
            }
            self.triangle(
                camera,
                [
                    local(0.0, 9.0, 3.0),
                    local(-2.0, 0.0, 5.2),
                    local(2.0, 0.0, 5.2),
                ],
                rgb(144, 238, 255),
            );
            self.mark = 1;
            // 深色描边，让敌机从地形和云层中跳出来
            let reach = 20.0 * scale;
            if let Some((p, d, s)) = camera.project(enemy.position) {
                self.outline(p, reach * s + 2.0, d, ENEMY_MARK, rgb(20, 18, 36));
            }
        }
        if flight.hud && mission.mode == Mode::Combat {
            for enemy in &mission.enemies {
                if let Some((p, d, scale)) = camera.project(enemy.position) {
                    let r = (enemy.radius() * scale + 3.0).clamp(4.0, 24.0);
                    let color = if enemy.is_boss() {
                        rgb(255, 181, 97)
                    } else {
                        rgb(255, 112, 137)
                    };
                    for side in [-1.0, 1.0] {
                        for top in [-1.0, 1.0] {
                            let corner = p + vec2(side * r, top * r * 0.6);
                            self.stroke(corner, corner - vec2(side * 3.0, 0.0), d - 30.0, color);
                            self.stroke(corner, corner - vec2(0.0, top * 3.0), d - 30.0, color);
                        }
                    }
                }
            }
            if flight.weights[2] < 0.5
                && let Some((p, d, _)) = camera.project(flight.position + flight.axes()[1] * 180.0)
            {
                for side in [-1.0, 1.0] {
                    self.stroke(
                        p + vec2(side * 3.0, 0.0),
                        p + vec2(side * 6.0, 0.0),
                        d - 30.0,
                        rgb(173, 255, 233),
                    );
                }
            }
        }
        for enemy in &mission.enemies {
            if enemy.cooldown > 0.0
                && enemy.cooldown < 0.6
                && (enemy.is_ground() || enemy.is_boss())
            {
                let vector = enemy.aim_point - enemy.position;
                for i in (0..12).step_by(2) {
                    self.world_line(
                        camera,
                        enemy.position + vector * (i as f32 / 12.0),
                        enemy.position + vector * ((i + 1) as f32 / 12.0),
                        rgb(178, 100, 95),
                    );
                }
            }
        }
        for shot in &mission.shots {
            let color = if shot.hostile {
                rgb(255, 91, 116)
            } else {
                let c = shot.weapon.color();
                rgb(c[0], c[1], c[2])
            };
            if shot.weapon == Weapon::Laser && !shot.hostile {
                self.world_line(camera, shot.previous, shot.position, color);
                self.world_line(
                    camera,
                    shot.previous + Vec3::Z,
                    shot.position + Vec3::Z,
                    rgb(238, 255, 255),
                );
            } else if let Some((p, d, s)) = camera.project(shot.position) {
                let radius = (shot.radius * s).round().clamp(1.0, 8.0) as i32;
                let tracer = !shot.hostile && matches!(shot.weapon, Weapon::Gun | Weapon::Missile);
                let tail = camera.project(shot.position - shot.velocity * 0.03);
                if let (true, Some((tail, _, _))) = (tracer, tail) {
                    // 曳光弹：沿飞行方向的武器色外沿与亮芯，区别于方形拾取物
                    let scale = self.full_width.max(320) as f32 / 320.0;
                    let tail = p + (tail - p).clamp_length(4.0 * scale, 12.0 * scale);
                    let normal = (p - tail).normalize_or_zero().perp() * scale;
                    self.stroke(tail, p, d, color);
                    self.stroke(tail + normal, p + normal, d, color);
                    self.stroke(p.lerp(tail, 0.5), p, d - 0.1, rgb(255, 250, 225));
                    self.point(p, d - 0.2, rgb(255, 255, 255), radius.min(2) - 1, 0);
                } else {
                    self.disc(p, d, rgb(44, 24, 53), radius + 1);
                    self.disc(p, d - 0.1, color, radius);
                    self.disc(p, d - 0.2, rgb(255, 250, 225), (radius - 1).max(0));
                }
                if !shot.hostile && shot.weapon != Weapon::Gun {
                    self.world_line(
                        camera,
                        shot.position,
                        shot.position - shot.velocity * 0.045,
                        color,
                    );
                }
            }
        }
        for effect in &mission.effects {
            self.explosion(effect, camera);
        }
        // 枪口焰：开火后短暂的十字星芒
        if mission.muzzle > 0.0 && flight.weights[2] < 0.5 {
            let glow = mission.muzzle / 0.06;
            for side in [-1.0, 1.0] {
                if let Some((p, d, s)) = camera.project(flight.local(vec3(side * 4.5, 25.0, 0.0))) {
                    let arm = (s * 3.0 * glow).clamp(1.0, 5.0);
                    let hot = mix(rgb(255, 196, 92), rgb(255, 255, 236), glow);
                    self.stroke(p - vec2(arm, 0.0), p + vec2(arm, 0.0), d - 2.0, hot);
                    self.stroke(p - vec2(0.0, arm), p + vec2(0.0, arm), d - 2.0, hot);
                    self.point(p, d - 3.0, rgb(255, 255, 255), (arm * 0.35) as i32, 0);
                }
            }
        }
        if mission.mode == Mode::Combat
            && mission.invulnerable > 0.6
            && flight.weights[2] < 0.5
            && let Some((p, d, s)) = camera.project(flight.position)
        {
            let r = s * 15.0;
            for i in 0..24 {
                let a = i as f32 * std::f32::consts::TAU / 24.0;
                self.point(
                    p + vec2(a.cos(), a.sin() * 0.6) * r,
                    d - 15.0,
                    rgb(129, 231, 255),
                    0,
                    0,
                );
            }
        }
    }

    /// 爆炸：抖动的火球、上升的烟团与抛物线碎片，小型命中只有火花
    fn explosion(&mut self, effect: &crate::gameplay::Effect, camera: &Camera) {
        const BAYER: [f32; 16] = [
            0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
        ];
        let Some((p, d, s)) = camera.project(effect.position) else {
            return;
        };
        let t = 1.0 - effect.life / effect.duration;
        let scale = self.full_width.max(320) as f32 / 320.0;
        let size = (effect.radius * s).min(26.0 * scale);
        let tint = rgb(effect.color[0], effect.color[1], effect.color[2]);
        let seed = effect.position.x.to_bits() ^ effect.position.y.to_bits().rotate_left(11);
        let mut blob = |r: f32, center: Vec2, color: u32, hot: u32, density: f32, depth: f32| {
            let reach = r.ceil() as i32;
            for y in -reach..=reach {
                for x in -reach..=reach {
                    let distance = ((x * x + y * y) as f32).sqrt() / r.max(0.5);
                    if distance > 1.0 {
                        continue;
                    }
                    let at = center + vec2(x as f32, y as f32);
                    let cell = ((at.y as i32 & 3) << 2 | (at.x as i32 & 3)) as usize;
                    // 核心实心，只在外缘用抖动过渡，避免整团变成网格
                    let edge = ((distance - 0.55) / 0.45).max(0.0);
                    if BAYER[cell] < density * (1.0 - edge) * 16.0 {
                        let color = if distance < 0.45 { hot } else { color };
                        self.point(at, depth, color, 0, 0);
                    }
                }
            }
        };
        if effect.radius > 10.0 {
            if t > 0.25 {
                let k = (t - 0.25) / 0.75;
                for i in 0..3 {
                    let side = (i as f32 - 1.0) * 0.45;
                    let center = p + vec2(side * size, -k * size * (0.7 + i as f32 * 0.15));
                    let smoke = mix(rgb(84, 78, 86), tint, 0.12);
                    blob(
                        size * (0.3 + 0.25 * k),
                        center,
                        smoke,
                        mix(smoke, rgb(120, 112, 116), 0.4),
                        (1.0 - k) * 0.55,
                        d + 1.0,
                    );
                }
            }
            if t < 0.4 {
                let k = t / 0.4;
                let color = if k < 0.33 {
                    mix(rgb(255, 250, 220), rgb(255, 214, 92), k * 3.0)
                } else if k < 0.66 {
                    mix(rgb(255, 214, 92), rgb(255, 128, 48), (k - 0.33) * 3.0)
                } else {
                    mix(rgb(255, 128, 48), rgb(150, 44, 36), (k - 0.66) * 3.0)
                };
                let hot = mix(color, rgb(255, 255, 240), 0.6 * (1.0 - k));
                blob(
                    size * (0.3 + 0.55 * k),
                    p,
                    color,
                    hot,
                    1.0 - k * 0.5,
                    d - 1.0,
                );
            }
        }
        let sparks = if effect.radius > 10.0 { 8 } else { 4 };
        let reach = size.max(6.0 * scale) * 1.6;
        for i in 0..sparks {
            let h = crate::world::hash(seed.wrapping_add(i * 7919)) as f32 / 255.0;
            let angle = (i as f32 + h) * std::f32::consts::TAU / sparks as f32;
            let travel = reach * t * (0.6 + 0.4 * h);
            let at =
                p + vec2(angle.cos(), angle.sin() * 0.8) * travel + vec2(0.0, t * t * reach * 0.5);
            let color = mix(
                mix(rgb(255, 245, 200), tint, t.min(0.5) * 2.0),
                rgb(90, 40, 30),
                t,
            );
            self.point(at, d - 2.0, color, 0, 0);
            if t < 0.6 {
                let tail = p + vec2(angle.cos(), angle.sin() * 0.8) * travel * 0.8;
                self.point(tail, d - 2.0, mix(color, rgb(60, 30, 30), 0.4), 0, 0);
            }
        }
    }
    /// 在包围范围内给指定遮罩的像素外缘描一圈深色
    fn outline(&mut self, center: Vec2, reach: f32, depth: f32, mark: u8, color: u32) {
        let (w, h) = (self.w as i32, self.h as i32);
        let reach = reach.min(80.0);
        let x0 = ((center.x - reach) as i32).max(1);
        let x1 = ((center.x + reach) as i32).min(w - 2);
        let y0 = ((center.y - reach) as i32).max(1);
        let y1 = ((center.y + reach) as i32).min(h - 2);
        let h = self.h;
        for x in x0..=x1 {
            for y in y0..=y1 {
                let at = x as usize * h + y as usize;
                if self.mask[at] != mark
                    && self.depth[at] >= depth - 30.0
                    && [at - 1, at + 1, at - h, at + h]
                        .iter()
                        .any(|&n| self.mask[n] == mark)
                {
                    self.pixels[at] = color;
                }
            }
        }
        // 描边完成后归还遮罩，避免与下一架敌机混在一起
        for x in x0..=x1 {
            for y in y0..=y1 {
                let at = x as usize * h + y as usize;
                if self.mask[at] == mark {
                    self.mask[at] = 1;
                }
            }
        }
    }
}
