use super::*;

impl Mission {
    pub fn phase(&self) -> Phase {
        campaign::operation(self.level).phases[self.phase_index]
    }
    pub fn phase_duration(&self) -> f32 {
        campaign::operation(self.level).durations[self.phase_index]
    }
    pub fn objective_goal(&self) -> u32 {
        let op = campaign::operation(self.level);
        match self.phase() {
            Phase::Patrol => op.patrol_quota,
            Phase::Strike => op.strike_quota,
            _ => 0,
        }
    }
    pub fn prepare_benchmark(&mut self, boss: bool) {
        self.phase_index = if boss { 2 } else { 0 };
        self.phase_started = false;
        self.enemies.clear();
        self.countdown = 0.0;
    }
    pub(super) fn spawn_enemy(
        &mut self,
        kind: EnemyKind,
        flight: &Flight,
        world: &World,
        lane: f32,
    ) {
        if self.enemies.len() >= 12 {
            return;
        }
        let (forward, right) = heading(flight.yaw);
        let phase = self.random() * std::f32::consts::TAU;
        let boss = kind.is_boss();
        let health = kind.spec().health
            * self.difficulty.factor()
            * (1.0 + self.level.saturating_sub(1).min(20) as f32 * 0.08);
        let mut position = flight.position
            + forward * (if boss { 270.0 } else { 220.0 } + self.random() * 75.0)
            + right * lane;
        if kind == EnemyKind::Battery {
            position += forward * 220.0;
            position.z = world.height(position.x, position.y) + 5.0;
        }
        self.next_id += 1;
        self.enemies.push(Enemy {
            id: self.next_id,
            position,
            yaw: flight.yaw,
            health,
            max_health: health,
            kind,
            phase,
            cooldown: 1.5 + self.random_fraction(kind),
            hit: 0.0,
            aim_point: flight.position,
        });
    }
    pub(super) fn random_fraction(&self, kind: EnemyKind) -> f32 {
        (self.next_id % 5) as f32 * 0.25 + kind as u8 as f32 * 0.03
    }
    pub(super) fn spawn_wave(&mut self, flight: &Flight, world: &World) {
        self.wave += 1;
        if self.phase() == Phase::Boss
            && !self.enemies.iter().any(Enemy::is_boss)
            && !self.boss_defeated
        {
            let kind = EnemyKind::BOSSES[((self.random() * 3.0) as usize).min(2)];
            self.spawn_enemy(kind, flight, world, 0.0);
            for side in [-1.0, 1.0] {
                self.spawn_enemy(EnemyKind::Interceptor, flight, world, side * 110.0);
            }
            self.announce("WARNING / BOSS APPROACH");
            self.sounds.push(crate::audio::Sound::Boss);
            return;
        }
        self.announce("ENEMY WAVE INCOMING");
        self.sounds.push(crate::audio::Sound::Wave);
        let count =
            (3 + (self.level - 1).min(3)).min(8_u32.saturating_sub(self.enemies.len() as u32));
        for i in 0..count {
            let kind = if self.level >= 3 && i == 0 && self.wave.is_multiple_of(3) {
                EnemyKind::Jammer
            } else if self.level >= 2 && i == 1 && self.wave.is_multiple_of(2) {
                EnemyKind::Striker
            } else {
                EnemyKind::FIGHTERS[(self.random() * 3.0) as usize]
            };
            self.spawn_enemy(
                kind,
                flight,
                world,
                (i as f32 - (count - 1) as f32 * 0.5) * 65.0,
            );
        }
        if self.phase() == Phase::Strike
            && self.objective < self.objective_goal()
            && self.enemies.iter().filter(|e| e.is_ground()).count() < 2
        {
            self.spawn_enemy(
                EnemyKind::Battery,
                flight,
                world,
                if self.wave.is_multiple_of(2) {
                    65.0
                } else {
                    -65.0
                },
            );
        }
    }
    pub(super) fn direct_combat(&mut self, dt: f32, flight: &Flight, world: &World) {
        if !self.phase_started {
            self.phase_started = true;
            self.phase_elapsed = 0.0;
            self.objective = 0;
            self.spawn_timer = 0.0;
            self.convoy = if self.phase() == Phase::Escort {
                Some(Convoy {
                    position: flight.position + flight.axes()[1] * 85.0,
                    previous: flight.position,
                    health: 220.0,
                    max_health: 220.0,
                    cooldown: 0.0,
                })
            } else {
                None
            };
        }
        self.phase_elapsed += dt;
        if self.convoy.as_ref().is_some_and(|c| c.health <= 0.0) {
            self.outcome = Some(false);
            self.announce("CONVOY LOST");
            return;
        }
        if let Some(convoy) = &mut self.convoy {
            convoy.previous = convoy.position;
            let (forward, right) = heading(flight.yaw);
            let goal = flight.position + forward * 95.0 + right * 25.0;
            convoy.position = convoy.position.lerp(goal, (dt * 2.0).min(1.0));
            convoy.position.z = convoy
                .position
                .z
                .max(world.height(convoy.position.x, convoy.position.y) + 55.0);
            convoy.cooldown = (convoy.cooldown - dt).max(0.0);
            convoy.health = (convoy.health + dt * 0.8).min(convoy.max_health);
        }
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 && (self.phase() != Phase::Boss || !self.boss_defeated) {
            self.spawn_wave(flight, world);
            self.spawn_timer = if self.phase() == Phase::Boss {
                22.0
            } else {
                campaign::operation(self.level).reinforcement_seconds
            };
        }
        self.supply_timer -= dt;
        if self.supply_timer <= 0.0 {
            self.supply_timer = 24.0;
            let forward = flight.axes()[1];
            for (offset, kind) in [
                (0.0, PickupKind::Shield),
                (18.0, PickupKind::Energy),
                (
                    36.0,
                    if self.wave.is_multiple_of(2) {
                        PickupKind::Support
                    } else {
                        PickupKind::Upgrade(self.weapon)
                    },
                ),
            ] {
                if self.pickups.len() < 32 {
                    self.pickups.push(Pickup {
                        position: flight.position + forward * (70.0 + offset),
                        kind,
                        life: 20.0,
                        collected: false,
                    });
                }
            }
        }
        let completed = match self.phase() {
            Phase::Boss => self.boss_defeated,
            Phase::Patrol | Phase::Strike => {
                self.phase_elapsed >= self.phase_duration()
                    && self.objective >= self.objective_goal()
            }
            _ => self.phase_elapsed >= self.phase_duration(),
        };
        if completed {
            if self.phase() == Phase::Boss {
                self.score += (self.health * 30.0 + self.remaining * 10.0) as u32;
                self.outcome = Some(true);
                return;
            }
            self.phase_index += 1;
            self.phase_started = false;
            self.enemies.clear();
            self.shots.clear();
            self.convoy = None;
            self.health = (self.health + 25.0).min(self.max_health);
            self.energy = 100.0;
            self.support = self.support.max(18.0);
            self.score += 2500;
            self.announce("OBJECTIVE COMPLETE");
            self.sounds.push(crate::audio::Sound::Phase);
        }
    }
}
