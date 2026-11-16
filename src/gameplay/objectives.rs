use super::*;

impl Mission {
    pub(super) fn collect(&mut self, dt: f32, flight: &Flight, previous: Vec3) {
        let mut received = Vec::new();
        for p in &mut self.pickups {
            if p.collected {
                continue;
            }
            p.life -= dt;
            if p.life <= 0.0 {
                p.collected = true;
                continue;
            }
            let distance = segment_distance(p.position, previous, flight.position);
            if self.mode == Mode::Combat && distance < 100.0 {
                p.position = p.position.lerp(flight.position, (dt * 2.5).min(1.0));
            }
            if distance < if self.mode == Mode::Coins { 23.0 } else { 18.0 } {
                p.collected = true;
                received.push((p.kind, p.position));
            }
        }
        if self.mode == Mode::Combat {
            self.pickups.retain(|p| !p.collected);
        }
        for (kind, position) in received {
            self.collected += 1;
            match kind {
                PickupKind::Coin => {
                    self.combo = (self.combo + 1).min(10);
                    self.best_combo = self.best_combo.max(self.combo);
                    self.combo_time = 5.0 - self.difficulty as usize as f32;
                    self.score += 100 * self.combo;
                    self.announce("COIN CHAIN");
                }
                PickupKind::Time => {
                    self.remaining += 8.0;
                    self.combo_time = 5.0 - self.difficulty as usize as f32;
                    self.score += 250;
                    self.announce("TIME +8 SECONDS");
                }
                PickupKind::Upgrade(w) => {
                    self.levels[w as usize] = (self.levels[w as usize] + 1).min(3);
                    self.weapon = w;
                    self.energy = (self.energy + 25.0).min(100.0);
                    self.announce("WEAPON UPGRADE");
                }
                PickupKind::Energy => {
                    self.energy = 100.0;
                    self.announce("ENERGY REFILLED");
                }
                PickupKind::Shield => {
                    self.health = (self.health + 40.0).min(self.max_health);
                    self.announce("SHIELD REPAIRED");
                }
                PickupKind::Bomb => {
                    self.bombs = (self.bombs + 1).min(5);
                    self.announce("NOVA +1");
                }
                PickupKind::Support => {
                    self.support = 30.0;
                    self.announce("WINGMEN / 30 SECONDS");
                }
            }
            self.sounds.push(match kind {
                PickupKind::Coin => crate::audio::Sound::Coin(self.combo as u8),
                PickupKind::Upgrade(_) => crate::audio::Sound::Upgrade,
                PickupKind::Support => crate::audio::Sound::Support,
                _ => crate::audio::Sound::Pickup,
            });
            self.effect(position, 12.0, [255, 227, 104]);
        }
    }
    pub fn target(&self, flight: &Flight) -> Option<Vec3> {
        match self.mode {
            Mode::Race => self.gates.get(self.gate_index).map(|g| g.position),
            Mode::Coins => self
                .pickups
                .iter()
                .filter(|p| !p.collected)
                .min_by(|a, b| {
                    a.position
                        .distance_squared(flight.position)
                        .total_cmp(&b.position.distance_squared(flight.position))
                })
                .map(|p| p.position),
            Mode::Combat => self
                .enemies
                .iter()
                .filter(|e| e.health > 0.0)
                .min_by(|a, b| {
                    let score = |e: &Enemy| {
                        e.position.distance_squared(flight.position)
                            * if self.phase() == Phase::Strike && e.is_ground() {
                                0.08
                            } else {
                                1.0
                            }
                    };
                    score(a).total_cmp(&score(b))
                })
                .map(|e| e.position),
            Mode::Free => None,
        }
    }
    pub fn coin_goal(&self) -> u32 {
        [3000, 6000, 12000][self.difficulty as usize]
    }
}
