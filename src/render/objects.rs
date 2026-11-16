use super::*;

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
                self.point(p, d, rgb(44, 24, 53), radius + 1, 0);
                self.point(p, d - 0.1, color, radius, 0);
                self.point(p, d - 0.2, rgb(255, 250, 225), (radius - 1).max(0), 0);
                if !shot.hostile {
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
            let t = 1.0 - effect.life / effect.duration;
            let c = effect.color;
            let color = mix(rgb(c[0], c[1], c[2]), rgb(85, 60, 79), t * 0.7);
            let right = vec3(camera.yaw.cos(), -camera.yaw.sin(), 0.0);
            for i in 0..20 {
                let a = i as f32 * std::f32::consts::TAU / 20.0;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 20.0;
                self.world_line(
                    camera,
                    effect.position + (right * a.cos() + Vec3::Z * a.sin()) * effect.radius * t,
                    effect.position + (right * b.cos() + Vec3::Z * b.sin()) * effect.radius * t,
                    color,
                );
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
}
