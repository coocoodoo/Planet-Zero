use super::*;

impl Mission {
    pub(super) fn combat(
        &mut self,
        dt: f32,
        flight: &mut Flight,
        world: &World,
        previous: Vec3,
        fire: bool,
    ) {
        self.energy = (self.energy
            + dt * (9.0 * (1.0 + self.upgrades[2] as f32 * 0.2)
                + if self.overdrive_time > 0.0 { 25.0 } else { 0.0 }))
        .min(100.0);
        self.support = (self.support - dt).max(0.0);
        flight.wing_count = if self.support > 0.0 { 2 } else { 0 };
        self.direct_combat(dt, flight, world);
        if self.outcome.is_some() {
            return;
        }
        self.fire_timer -= dt;
        if fire && self.fire_timer <= 0.0 {
            self.fire(flight);
        }
        self.wing_support(dt, flight);
        let collision = self.enemy_turn(dt, flight, world, previous);
        self.jammer_repair(dt);
        if collision {
            self.damage(18.0 * self.difficulty.factor());
        }
        self.move_shots(dt, flight, world, previous);
        self.resolve_kills();
        if self.health <= 0.0 {
            if self.outcome.is_none() {
                self.sounds.push(crate::audio::Sound::Destroyed);
            }
            self.outcome = Some(false);
            self.announce("AIRCRAFT LOST");
        }
    }
    /// 僚机支援：每 0.5 秒向视野内的敌机发射追踪弹
    fn wing_support(&mut self, dt: f32, flight: &Flight) {
        self.support_timer -= dt;
        if self.support > 0.0 && self.support_timer <= 0.0 {
            self.support_timer = 0.5;
            for wing in flight
                .wingmen
                .iter()
                .take(2)
                .filter(|wing| wing.portal_wait <= 0.0)
            {
                if let Some((id, target)) = self.aim(wing.position, flight.axes()[1], -0.2)
                    && self.shots.len() < 180
                {
                    self.shots.push(Shot {
                        position: wing.position,
                        previous: wing.position,
                        velocity: (target - wing.position).normalize_or_zero() * 330.0,
                        life: 3.0,
                        damage: 12.0,
                        weapon: Weapon::Missile,
                        hostile: false,
                        grazed: false,
                        target: Some(id),
                        radius: 1.5,
                    });
                }
            }
        }
    }
    /// 敌机机动与射击；返回本帧是否与玩家相撞
    fn enemy_turn(&mut self, dt: f32, flight: &Flight, world: &World, previous: Vec3) -> bool {
        let (forward, right) = heading(flight.yaw);
        let difficulty = self.difficulty.factor();
        let mut collision = false;
        for enemy in &mut self.enemies {
            if enemy.health <= 0.0 {
                continue;
            }
            let distance_before = enemy.position.distance(previous);
            enemy.hit = (enemy.hit - dt).max(0.0);
            let spec = enemy.kind.spec();
            enemy.phase += dt * spec.phase_rate;
            let enraged = enemy.is_boss() && enemy.health < enemy.max_health * 0.5;
            let distance = if enemy.is_boss() {
                235.0
            } else {
                145.0 + enemy.id as f32 % 4.0 * 30.0
            };
            let lateral = enemy.phase.sin() * spec.lateral;
            let mut goal = flight.position + forward * distance + right * lateral;
            goal.z = (flight.position.z + (enemy.phase * 0.7).sin() * 24.0).clamp(100.0, 260.0);
            goal.z = goal.z.max(world.height(goal.x, goal.y) + 45.0);
            enemy.position += (goal - enemy.position) * (dt * spec.agility).min(1.0);
            enemy.yaw += angle(flight.yaw - enemy.yaw) * (dt * 2.0).min(1.0);
            // 敌机贴近到掠过距离时播放呼啸声
            if !enemy.is_ground()
                && distance_before >= 45.0
                && enemy.position.distance(flight.position) < 45.0
            {
                self.sounds.push(crate::audio::Sound::Flyby);
            }
            collision |=
                segment_distance(enemy.position, previous, flight.position) < enemy.radius() + 7.0;
            let previous_cooldown = enemy.cooldown;
            enemy.cooldown -= dt;
            if previous_cooldown > 0.6 && enemy.cooldown <= 0.6 {
                enemy.aim_point = if enemy.id % 3 == 0 {
                    self.convoy.as_ref().map_or(flight.position, |c| c.position)
                } else {
                    flight.position
                };
            }
            if enemy.cooldown <= 0.0 {
                enemy.cooldown = if enraged { 0.7 } else { spec.cooldown } / difficulty;
                let aim = (enemy.aim_point - enemy.position).normalize_or_zero();
                self.sounds.push(crate::audio::Sound::EnemyFire(
                    enemy.is_boss() || enemy.is_ground() || enemy.kind == EnemyKind::Striker,
                ));
                let count = if enraged { 11 } else { spec.burst as i32 };
                for i in 0..count {
                    if self.shots.len() >= 180 {
                        break;
                    }
                    let offset = i as f32 - (count - 1) as f32 * 0.5;
                    let (horizontal, vertical) = match enemy.kind {
                        EnemyKind::Serpent => (
                            offset * 0.025 + (enemy.phase * 2.0).sin() * 0.16,
                            offset * 0.045,
                        ),
                        EnemyKind::Tempest => {
                            let a = i as f32 * std::f32::consts::TAU / count as f32 + enemy.phase;
                            (a.cos() * 0.24, a.sin() * 0.24)
                        }
                        _ => (
                            offset * 0.075,
                            if enraged {
                                (i as f32 * 1.7 + enemy.phase).sin() * 0.16
                            } else {
                                0.0
                            },
                        ),
                    };
                    let velocity = (aim + right * horizontal + Vec3::Z * vertical).normalize()
                        * (95.0 + 20.0 * difficulty);
                    self.shots.push(Shot {
                        position: enemy.position,
                        previous: enemy.position,
                        velocity,
                        life: 5.0,
                        damage: 6.0 * difficulty,
                        weapon: Weapon::Orb,
                        hostile: true,
                        grazed: false,
                        target: None,
                        radius: if enemy.is_boss() { 3.5 } else { 2.5 },
                    });
                }
            }
        }
        collision
    }
    /// 维修无人机修复附近仍存活的敌机
    fn jammer_repair(&mut self, dt: f32) {
        let jammers: Vec<_> = self
            .enemies
            .iter()
            .filter(|e| e.kind == EnemyKind::Jammer && e.health > 0.0)
            .map(|e| e.position)
            .collect();
        for enemy in &mut self.enemies {
            // 只修复仍存活的敌机，避免把本帧被击毁的目标救回
            if enemy.kind != EnemyKind::Jammer
                && enemy.health > 0.0
                && jammers.iter().any(|p| p.distance(enemy.position) < 150.0)
            {
                enemy.health = (enemy.health + dt * 8.0).min(enemy.max_health);
            }
        }
    }
    /// 推进弹体并结算命中、擦弹与溅射
    fn move_shots(&mut self, dt: f32, flight: &Flight, world: &World, previous: Vec3) {
        let mut explosions = Vec::new();
        let mut damage = 0.0_f32;
        for shot in &mut self.shots {
            shot.life -= dt;
            let spec = shot.weapon.spec();
            if shot.life <= 0.0 || spec.speed == 0.0 {
                continue;
            }
            shot.previous = shot.position;
            if !shot.hostile && spec.homing > 0.0 {
                let target = shot
                    .target
                    .and_then(|id| self.enemies.iter().find(|e| e.id == id && e.health > 0.0));
                if let Some(target) = target {
                    let speed = shot.velocity.length();
                    shot.velocity = shot.velocity.lerp(
                        (target.position - shot.position).normalize_or_zero() * speed,
                        (dt * spec.homing).min(1.0),
                    );
                }
            }
            shot.position += shot.velocity * dt;
            if shot.hostile {
                let relative_before = shot.previous - previous;
                let relative_after = shot.position - flight.position;
                if segment_distance(Vec3::ZERO, relative_before, relative_after) < shot.radius + 6.0
                {
                    damage = damage.max(shot.damage);
                    shot.life = 0.0;
                } else if !shot.grazed
                    && segment_distance(Vec3::ZERO, relative_before, relative_after) < 20.0
                {
                    shot.grazed = true;
                    self.sounds.push(crate::audio::Sound::Graze);
                    self.grazes += 1;
                    self.score += 25;
                    self.overdrive = (self.overdrive + 3.0).min(100.0);
                }
                if shot.life > 0.0
                    && let Some(convoy) = &mut self.convoy
                    && segment_distance(
                        Vec3::ZERO,
                        shot.previous - convoy.previous,
                        shot.position - convoy.position,
                    ) < shot.radius + 13.0
                {
                    shot.life = 0.0;
                    if convoy.cooldown <= 0.0 {
                        convoy.health = (convoy.health - shot.damage).max(0.0);
                        convoy.cooldown = 0.7;
                    }
                }
            } else {
                for enemy in &mut self.enemies {
                    // 穿透弹用 target 记录上一个命中目标，避免同一目标连续两帧重复结算
                    if enemy.health > 0.0
                        && !(spec.pierce && shot.target == Some(enemy.id))
                        && segment_distance(enemy.position, shot.previous, shot.position)
                            < enemy.radius() + shot.radius
                    {
                        enemy.health -= shot.damage;
                        enemy.hit = 0.16;
                        self.hits += 1;
                        self.overdrive = (self.overdrive + 0.35).min(100.0);
                        explosions.push((enemy.position, shot.weapon, shot.damage, enemy.id));
                        if !spec.pierce {
                            shot.life = 0.0;
                            break;
                        }
                        shot.target = Some(enemy.id);
                    }
                }
            }
            if shot.position.z < world.height(shot.position.x, shot.position.y) {
                shot.life = 0.0;
            }
        }
        if damage > 0.0 {
            self.damage(damage);
        }
        if !explosions.is_empty() {
            self.sounds.push(crate::audio::Sound::Impact);
        }
        for (position, weapon, damage, id) in explosions {
            let radius = weapon.spec().splash;
            if radius > 10.0 {
                for enemy in &mut self.enemies {
                    if enemy.id != id && enemy.position.distance(position) < radius {
                        enemy.health -= damage * 0.6;
                        enemy.hit = 0.15;
                    }
                }
            }
            self.effect(position, radius, weapon.color());
        }
        self.shots.retain(|s| s.life > 0.0);
    }
    /// 移除被击毁的敌机，结算得分、连击与掉落
    fn resolve_kills(&mut self) {
        let dead: Vec<_> = self
            .enemies
            .iter()
            .filter(|e| e.health <= 0.0)
            .map(|e| (e.position, e.kind))
            .collect();
        self.enemies.retain(|e| e.health > 0.0);
        for (position, kind) in dead {
            let boss = kind.is_boss();
            self.kills += 1;
            self.sounds.push(crate::audio::Sound::Explosion(boss));
            self.overdrive = (self.overdrive + 4.0).min(100.0);
            self.boss_defeated |= boss;
            if self.phase() == Phase::Patrol
                || (self.phase() == Phase::Strike && kind == EnemyKind::Battery)
            {
                self.objective += 1;
            }
            self.combo = (self.combo + 1).min(10);
            self.combo_time = 5.0;
            self.best_combo = self.best_combo.max(self.combo);
            self.score += kind.spec().score * (1 + self.combo / 3);
            self.effect(position, if boss { 90.0 } else { 23.0 }, [255, 160, 66]);
            let roll = self.random() * 10.0;
            let choice = roll as usize;
            let kind = match choice {
                // 一半概率掉落武器升级，在全部弹药中均匀抽取
                0..=4 => {
                    PickupKind::Upgrade(Weapon::ALL[(roll / 5.0 * Weapon::COUNT as f32) as usize])
                }
                5 => PickupKind::Energy,
                6 => PickupKind::Shield,
                7 => PickupKind::Bomb,
                8 => PickupKind::Support,
                _ => PickupKind::Shield,
            };
            if self.pickups.len() < 32 && self.random() < 0.45 {
                self.pickups.push(Pickup {
                    position,
                    kind,
                    life: 18.0,
                    collected: false,
                });
            }
        }
    }
    pub(super) fn damage(&mut self, amount: f32) {
        if self.invulnerable <= 0.0 {
            self.health = (self.health - amount).max(0.0);
            self.invulnerable = 0.9;
            self.hurt = 0.35;
            self.announce("SHIELD HIT / ROLL TO EVADE");
            self.sounds.push(crate::audio::Sound::Damage);
        }
    }
}
