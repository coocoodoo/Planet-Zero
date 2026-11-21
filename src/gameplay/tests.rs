use super::*;
use crate::content::vehicles::Vehicle;
use crate::flight::Controls;
use std::sync::OnceLock;

fn world() -> &'static World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| World::new(7))
}

#[test]
fn gates_require_forward_crossing_and_preserve_order() {
    let world = world();
    let mut flight = Flight::new(world);
    let mut m = Mission::new(Mode::Race, Difficulty::Cadet, &mut flight, world);
    m.countdown = 0.0;
    let gate = m.gates[0].clone();
    let before = gate.position - gate.normal * 80.0;
    flight.position = gate.position + gate.normal * 80.0;
    m.step(1.0 / 30.0, &mut flight, world, before, false);
    assert_eq!((m.gate_index, m.perfect, m.score), (1, 1, 750));
    let gate = m.gates[1].clone();
    flight.position = gate.position - gate.normal * 5.0;
    m.step(
        1.0 / 30.0,
        &mut flight,
        world,
        gate.position + gate.normal * 5.0,
        false,
    );
    assert_eq!(m.gate_index, 1);
    flight.position = gate.position + gate.normal * 5.0 + Vec3::Z * 80.0;
    m.step(
        1.0 / 30.0,
        &mut flight,
        world,
        gate.position - gate.normal * 5.0 + Vec3::Z * 80.0,
        false,
    );
    assert_eq!(m.gate_index, 1);
}

#[test]
fn coin_chain_expires_and_pickups_apply_upgrades() {
    let world = world();
    let mut f = Flight::new(world);
    let mut m = Mission::new(Mode::Coins, Difficulty::Cadet, &mut f, world);
    m.countdown = 0.0;
    for i in 0..2 {
        let p = m.pickups[i].position;
        f.position = p;
        m.step(0.01, &mut f, world, p, false);
    }
    assert_eq!((m.combo, m.score), (2, 300));
    let p = f.position;
    m.step(5.1, &mut f, world, p, false);
    assert_eq!(m.combo, 0);
    m.pickups.clear();
    for kind in [
        PickupKind::Upgrade(Weapon::Laser),
        PickupKind::Upgrade(Weapon::Laser),
        PickupKind::Upgrade(Weapon::Laser),
        PickupKind::Energy,
        PickupKind::Shield,
        PickupKind::Bomb,
        PickupKind::Support,
    ] {
        m.pickups.push(Pickup {
            position: p,
            kind,
            life: 10.0,
            collected: false,
        });
    }
    m.health = 10.0;
    m.energy = 0.0;
    m.collect(0.1, &f, p);
    assert_eq!(m.levels[Weapon::Laser as usize], 3);
    assert_eq!(m.weapon, Weapon::Laser);
    assert_eq!(m.energy, 100.0);
    assert_eq!(m.health, 50.0);
    assert_eq!(m.bombs, 3);
    assert_eq!(m.support, 30.0);
}

#[test]
fn weapons_hit_and_boss_enters_enraged_phase() {
    let world = world();
    for weapon in Weapon::ALL {
        let mut f = Flight::new(world);
        let mut m = Mission::new(Mode::Combat, Difficulty::Cadet, &mut f, world);
        m.countdown = 0.0;
        m.wave = 4;
        m.phase_index = 2;
        m.spawn_wave(&f, world);
        m.phase_started = true;
        m.weapon = weapon;
        m.enemies[0].position = f.position + f.axes()[1] * 180.0;
        let before = m.enemies[0].health;
        for _ in 0..180 {
            let p = f.position;
            m.step(1.0 / 30.0, &mut f, world, p, true);
        }
        assert!(
            m.enemies[0].health < before,
            "{} must damage target",
            weapon.name()
        );
        assert!(m.energy >= 0.0 && m.shots.len() <= 180 && m.effects.len() <= 48);
    }
    let mut f = Flight::new(world);
    let mut m = Mission::new(Mode::Combat, Difficulty::Ace, &mut f, world);
    m.countdown = 0.0;
    m.wave = 4;
    m.phase_index = 2;
    m.spawn_wave(&f, world);
    m.phase_started = true;
    m.enemies[0].health = m.enemies[0].max_health * 0.4;
    m.enemies[0].cooldown = 0.0;
    let p = f.position;
    m.step(0.01, &mut f, world, p, false);
    assert_eq!(m.shots.iter().filter(|s| s.hostile).count(), 11);
    let before = m.enemies[0].health;
    m.bomb();
    assert!(m.shots.iter().all(|s| !s.hostile));
    assert_eq!(m.enemies[0].health, before - 130.0);
}

#[test]
fn all_vehicles_can_finish_race_and_coin_routes() {
    let world = world();
    for vehicle in Vehicle::ALL {
        for mode in [Mode::Race, Mode::Coins] {
            let mut f = Flight::new(world);
            f.vehicle = vehicle;
            let mut m = Mission::new(mode, Difficulty::Ace, &mut f, world);
            for _ in 0..4500 {
                let Some(target) = m.target(&f) else {
                    break;
                };
                let delta = target - f.position;
                let controls = Controls {
                    turn: (angle(delta.x.atan2(-delta.y) - f.yaw) * 2.2 / vehicle.turn())
                        .clamp(-1.0, 1.0),
                    climb: (delta.z * 2.0 / (f.speed * 0.55)).clamp(-1.0, 1.0),
                    manual: true,
                    ..Default::default()
                };
                let p = f.position;
                if m.countdown <= 0.0 {
                    f.step(1.0 / 30.0, &controls, world);
                }
                m.step(1.0 / 30.0, &mut f, world, p, false);
                if m.outcome.is_some() {
                    break;
                }
            }
            println!(
                "{} {}: {:?}, {:.1}s score={} gates={} coins={}",
                vehicle.name(),
                mode.name(),
                m.outcome,
                m.elapsed,
                m.score,
                m.gate_index,
                m.collected
            );
            assert_eq!(m.outcome, Some(true), "{} {}", vehicle.name(), mode.name());
        }
    }
}

#[test]
fn full_combat_can_be_won_with_bounded_entities() {
    let world = world();
    let mut f = Flight::new(world);
    let mut m = Mission::new(Mode::Combat, Difficulty::Cadet, &mut f, world);
    for i in 0..9000 {
        let target = m.target(&f).unwrap_or(f.position + f.axes()[1] * 200.0);
        let delta = target - f.position;
        let controls = Controls {
            turn: (angle(delta.x.atan2(-delta.y) - f.yaw) * 2.0 / f.vehicle.turn())
                .clamp(-1.0, 1.0),
            climb: (delta.z * 1.5 / (f.speed * 0.55)).clamp(-1.0, 1.0),
            manual: true,
            ..Default::default()
        };
        let p = f.position;
        if m.countdown <= 0.0 {
            f.step(1.0 / 30.0, &controls, world);
        }
        if i % 150 == 0 {
            f.roll();
            m.dodge();
        }
        if m.wave == 5 && i % 450 == 0 {
            m.bomb();
        }
        m.step(1.0 / 30.0, &mut f, world, p, true);
        assert!(
            m.enemies.len() <= 16
                && m.shots.len() <= 180
                && m.effects.len() <= 48
                && m.pickups.len() <= 32
        );
        if m.outcome.is_some() {
            break;
        }
    }
    println!(
        "combat {:?}: {:.1}s kills={} score={} shield={}",
        m.outcome, m.elapsed, m.kills, m.score, m.health
    );
    assert_eq!(m.outcome, Some(true));
    assert_eq!(m.phase_index, 2);
    assert!(m.elapsed >= 130.0 && m.elapsed < 300.0);
    assert!(m.wave >= 12 && m.kills >= 30);
}

#[test]
fn all_operations_progress_through_every_phase() {
    for level in 1..=6 {
        let operation = campaign::operation(level);
        let world = World::generate(operation.map, operation.map.seed());
        let mut flight = Flight::new(&world);
        let mut mission =
            Mission::for_level(Mode::Combat, Difficulty::Cadet, &mut flight, &world, level);
        let mut phases = std::collections::HashSet::new();
        let mut boss_seen = false;
        for tick in 0..14400 {
            phases.insert(mission.phase_index);
            if mission.phase() == Phase::Strike {
                mission.weapon = Weapon::Missile;
            }
            boss_seen |= mission.enemies.iter().any(|enemy| enemy.is_boss());
            let target = mission
                .target(&flight)
                .unwrap_or(flight.position + flight.axes()[1] * 200.0);
            let delta = target - flight.position;
            let controls = Controls {
                turn: (angle(delta.x.atan2(-delta.y) - flight.yaw) * 2.0 / flight.vehicle.turn())
                    .clamp(-1.0, 1.0),
                climb: (delta.z * 1.5 / (flight.speed * 0.55)).clamp(-1.0, 1.0),
                manual: true,
                ..Default::default()
            };
            let previous = flight.position;
            if mission.countdown <= 0.0 {
                flight.step(1.0 / 30.0, &controls, &world);
            }
            if tick % 150 == 0 {
                flight.roll();
                mission.dodge();
            }
            if mission.phase() == Phase::Boss && tick % 450 == 0 {
                mission.bomb();
            }
            mission.activate_overdrive();
            mission.step(1.0 / 30.0, &mut flight, &world, previous, true);
            assert!(
                mission.enemies.len() <= 12
                    && mission.shots.len() <= 180
                    && mission.pickups.len() <= 32
            );
            if mission.outcome.is_some() {
                break;
            }
        }
        println!(
            "stage {} {}: {:?} {:.1}s phases={} objective={} kills={} shield={:.0}",
            level,
            operation.name,
            mission.outcome,
            mission.elapsed,
            phases.len(),
            mission.objective,
            mission.kills,
            mission.health
        );
        assert_eq!(mission.outcome, Some(true), "stage {level}");
        assert_eq!(phases.len(), 3);
        assert!(boss_seen, "stage {level}");
        assert!(mission.elapsed >= operation.durations[0] + operation.durations[1]);
    }
}

#[test]
fn convoy_loss_and_phase_objectives_control_progression() {
    let world = world();
    let mut flight = Flight::new(world);
    let mut mission = Mission::for_level(Mode::Combat, Difficulty::Cadet, &mut flight, world, 2);
    mission.direct_combat(0.1, &flight, world);
    mission.convoy.as_mut().unwrap().health = 0.0;
    mission.direct_combat(0.1, &flight, world);
    assert_eq!(mission.outcome, Some(false));
    assert_eq!(mission.message, "CONVOY LOST");
    let mut mission = Mission::for_level(Mode::Combat, Difficulty::Cadet, &mut flight, world, 3);
    mission.direct_combat(0.1, &flight, world);
    mission.phase_elapsed = 100.0;
    mission.direct_combat(0.1, &flight, world);
    assert_eq!(mission.phase(), Phase::Strike);
    mission.objective = mission.objective_goal();
    mission.direct_combat(0.1, &flight, world);
    assert_eq!(mission.phase_index, 1);
}

#[test]
fn all_three_bosses_are_spawnable() {
    let world = world();
    let mut kinds = std::collections::HashSet::new();
    for seed in 1_u32..=32 {
        let mut flight = Flight::new(world);
        let mut mission = Mission::new(Mode::Combat, Difficulty::Cadet, &mut flight, world);
        mission.shuffle_encounters(seed.wrapping_mul(0x9e3779b9));
        mission.prepare_benchmark(true);
        mission.direct_combat(0.1, &flight, world);
        kinds.insert(mission.enemies.iter().find(|e| e.is_boss()).unwrap().kind);
    }
    assert_eq!(kinds, std::collections::HashSet::from([3, 4, 5]));
}

#[test]
fn jammers_do_not_revive_destroyed_enemies() {
    let world = world();
    let mut f = Flight::new(world);
    let mut m = Mission::new(Mode::Combat, Difficulty::Cadet, &mut f, world);
    m.countdown = 0.0;
    m.enemies.clear();
    m.spawn_enemy(7, &f, world, 0.0);
    m.spawn_enemy(0, &f, world, 0.0);
    let jammer = m.enemies[0].position;
    m.enemies[1].position = jammer + glam::Vec3::X * 20.0;
    m.enemies[1].health = -0.1;
    let destroyed = m.enemies[1].id;
    let kills = m.kills;
    let p = f.position;
    m.combat(1.0 / 30.0, &mut f, world, p, false);
    assert_eq!(m.kills, kills + 1);
    assert!(m.enemies.iter().all(|e| e.id != destroyed));
}
