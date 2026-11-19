use super::portal::Portal;
use super::{MapKind, Random, World};
use glam::{Vec3, vec3};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MapKey {
    pub kind: MapKind,
    pub seed: u32,
}
impl MapKey {
    pub fn of(world: &World) -> Self {
        Self {
            kind: world.kind,
            seed: world.seed,
        }
    }
}
#[derive(Clone, Copy)]
struct Link {
    id: u32,
    ends: [MapKey; 2],
    entrances: [Option<Portal>; 2],
}
#[derive(Clone, Copy)]
pub struct Route {
    pub id: u32,
    pub aperture: Portal,
    pub target: MapKey,
}
#[derive(Clone, Copy)]
pub struct Environment {
    pub weather: crate::content::weather::Weather,
    pub theme: usize,
}
pub struct Network {
    links: Vec<Link>,
    cache: Vec<Arc<World>>,
    job: Option<(MapKey, std::thread::JoinHandle<World>)>,
    random: Random,
    environment_seed: u32,
    summoned: std::collections::HashMap<MapKey, u32>,
    pub active: bool,
}
impl Network {
    pub fn random_earth(&mut self) -> MapKey {
        self.random_earth_except(None)
    }
    pub fn random_earth_except(&mut self, exclude: Option<MapKind>) -> MapKey {
        let kinds: Vec<_> = MapKind::ALL
            .into_iter()
            .filter(|kind| !kind.vacuum() && Some(*kind) != exclude)
            .collect();
        let kind = kinds[(self.random.next() * kinds.len() as f32) as usize];
        MapKey {
            kind,
            seed: kind.seed(),
        }
    }
    pub fn park_portals(&mut self, world: &World, position: Vec3, forward: Vec3) {
        self.ensure(world);
        let direction = forward.normalize_or_zero();
        let right = vec3(-direction.y, direction.x, 0.0).normalize_or_zero();
        let key = MapKey::of(world);
        for (i, link) in self
            .links
            .iter_mut()
            .filter(|link| link.ends.contains(&key))
            .enumerate()
        {
            let side = usize::from(link.ends[1] == key);
            let mut center = position - direction * (3200.0 + i as f32 * 500.0)
                + right * (i as f32 * 2.0 - 1.0) * 450.0;
            if world.kind == MapKind::Space {
                while super::orbit::Body::ALL
                    .into_iter()
                    .any(|body| body.altitude(center) < body.orbit_altitude() + 500.0)
                {
                    center -= direction * 1200.0;
                }
            }
            let mut portal = Portal {
                center,
                yaw: forward.x.atan2(-forward.y),
                radius: 48.0,
            };
            safe_aperture(world, &mut portal);
            link.entrances[side] = Some(portal);
        }
    }
    pub fn new(seed: u32) -> Self {
        let mut random = Random(seed);
        let mut kinds = MapKind::ALL;
        for i in (1..kinds.len()).rev() {
            kinds.swap(i, (random.next() * (i + 1) as f32) as usize);
        }
        let mut network = Self {
            links: vec![],
            cache: vec![],
            job: None,
            random,
            environment_seed: seed,
            summoned: Default::default(),
            active: false,
        };
        for i in 0..kinds.len() {
            network.connect(kinds[i], kinds[(i + 1) % kinds.len()]);
        }
        network
    }
    pub fn snapshot(&self) -> Self {
        Self {
            links: self.links.clone(),
            cache: self.cache.clone(),
            job: None,
            random: Random(self.random.0),
            environment_seed: self.environment_seed,
            summoned: self.summoned.clone(),
            active: self.active,
        }
    }
    fn add_link(&mut self, ends: [MapKey; 2], entrances: [Option<Portal>; 2]) {
        self.links.push(Link {
            id: self.random.0,
            ends,
            entrances,
        });
        self.random.next();
    }
    fn connect(&mut self, a: MapKind, b: MapKind) {
        self.add_link(
            [
                MapKey {
                    kind: a,
                    seed: a.seed(),
                },
                MapKey {
                    kind: b,
                    seed: b.seed(),
                },
            ],
            [None; 2],
        );
    }
    pub fn ensure(&mut self, world: &World) {
        let key = MapKey::of(world);
        if self.links.iter().any(|l| l.ends.contains(&key)) {
            return;
        }
        let at = self.random_link(world.kind);
        let link = self.links.remove(at);
        self.add_link([link.ends[0], key], [link.entrances[0], None]);
        self.add_link([key, link.ends[1]], [None, link.entrances[1]]);
    }
    fn random_link(&mut self, exclude: MapKind) -> usize {
        let choices: Vec<_> = self
            .links
            .iter()
            .enumerate()
            .filter(|(_, l)| l.ends.iter().all(|k| k.kind != exclude))
            .map(|(i, _)| i)
            .collect();
        choices[(self.random.next() * choices.len() as f32) as usize]
    }
    pub fn routes(&self, world: &World) -> Vec<Route> {
        let key = MapKey::of(world);
        self.links
            .iter()
            .filter_map(|link| {
                let side = link.ends.iter().position(|k| *k == key)?;
                Some(Route {
                    id: link.id,
                    aperture: link.entrances[side].unwrap_or_else(|| endpoint(world, link.id)),
                    target: link.ends[1 - side],
                })
            })
            .collect()
    }
    pub fn exit(&self, world: &World, id: u32) -> Portal {
        self.links
            .iter()
            .find(|link| link.id == id)
            .and_then(|link| {
                link.ends
                    .iter()
                    .position(|key| *key == MapKey::of(world))
                    .and_then(|side| link.entrances[side])
            })
            .unwrap_or_else(|| endpoint(world, id))
    }
    pub fn summon(&mut self, world: &World, position: Vec3, forward: Vec3) {
        self.ensure(world);
        let key = MapKey::of(world);
        let old = self.summoned.get(&key).copied();
        let at = self
            .links
            .iter()
            .position(|link| link.ends.contains(&key) && Some(link.id) == old)
            .unwrap_or_else(|| {
                self.links
                    .iter()
                    .position(|link| link.ends.contains(&key))
                    .unwrap()
            });
        let removed = self.links.remove(at);
        let side = usize::from(removed.ends[1] == key);
        let neighbor = removed.ends[1 - side];
        let at = self
            .links
            .iter()
            .enumerate()
            .filter(|(_, link)| !link.ends.contains(&key) && !link.ends.contains(&neighbor))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let at = at[(self.random.next() * at.len() as f32) as usize];
        let remote = self.links.remove(at);
        let mut reached = std::collections::HashSet::new();
        let mut pending = vec![key];
        while let Some(k) = pending.pop() {
            if reached.insert(k) {
                for link in self.links.iter().filter(|link| link.ends.contains(&k)) {
                    pending.extend(link.ends);
                }
            }
        }
        let target_side = usize::from(reached.contains(&remote.ends[0]));
        self.add_link(
            [neighbor, remote.ends[1 - target_side]],
            [
                removed.entrances[1 - side],
                remote.entrances[1 - target_side],
            ],
        );
        let yaw = forward.x.atan2(-forward.y);
        let direction = vec3(yaw.sin(), -yaw.cos(), 0.0);
        let mut aperture = Portal {
            center: position + direction * (680.0 + self.random.next() * 120.0),
            yaw: yaw + std::f32::consts::PI,
            radius: 48.0 + self.random.next() * 8.0,
        };
        safe_aperture(world, &mut aperture);
        self.add_link(
            [key, remote.ends[target_side]],
            [Some(aperture), remote.entrances[target_side]],
        );
        self.summoned.insert(key, self.links.last().unwrap().id);
        self.request(remote.ends[target_side]);
        self.active = true;
    }
    pub fn environment(&self, key: MapKey) -> Environment {
        let value = super::hash(key.seed ^ self.environment_seed ^ key.kind as u32) as u32;
        let mut random = Random(key.seed ^ self.environment_seed ^ value.wrapping_mul(7919));
        let weather = (random.next() * 100.0) as usize;
        let time = (random.next() * 100.0) as usize;
        use crate::content::weather::Weather;
        Environment {
            weather: if key.kind.vacuum() {
                Weather::Clear
            } else {
                match weather {
                    0..48 => Weather::Clear,
                    48..78 => Weather::Cloudy,
                    78..86 => Weather::Rainbow,
                    86..91 => Weather::Rain,
                    91..94 => Weather::Snow,
                    94..96 => Weather::Storm,
                    96..99 => Weather::Fog,
                    _ => Weather::Sandstorm,
                }
            },
            theme: match time {
                0..85 => 0,
                85..95 => 1,
                _ => 2,
            },
        }
    }
    pub fn take(&mut self, key: MapKey) -> Option<Arc<World>> {
        let at = self.cache.iter().position(|w| MapKey::of(w) == key)?;
        Some(self.cache.remove(at))
    }
    pub fn stage_portal(
        &mut self,
        world: &World,
        id: u32,
        position: Vec3,
        forward: Vec3,
    ) -> Portal {
        let yaw = forward.x.atan2(-forward.y);
        let mut portal = Portal {
            center: position + vec3(yaw.sin(), -yaw.cos(), 0.0) * 650.0,
            yaw: yaw + std::f32::consts::PI,
            radius: 62.0,
        };
        safe_aperture(world, &mut portal);
        if let Some(link) = self.links.iter_mut().find(|link| link.id == id) {
            let side = usize::from(link.ends[1] == MapKey::of(world));
            link.entrances[side] = Some(portal);
        }
        portal
    }
    pub fn get(&self, key: MapKey) -> Option<&Arc<World>> {
        self.cache.iter().find(|w| MapKey::of(w) == key)
    }
    pub fn remember(&mut self, world: Arc<World>) {
        let key = MapKey::of(&world);
        self.cache.retain(|w| MapKey::of(w) != key);
        self.cache.push(world);
        while self.cache.len() > 2 {
            self.cache.remove(0);
        }
    }
    pub fn poll(&mut self) {
        if self.job.as_ref().is_some_and(|(_, job)| job.is_finished()) {
            let (_, job) = self.job.take().unwrap();
            self.remember(Arc::new(job.join().expect("虫洞目的地图生成失败")));
        }
    }
    pub fn request(&mut self, key: MapKey) {
        if self.get(key).is_none() && self.job.is_none() {
            self.job = Some((key, super::loader::spawn_world(key.kind, key.seed)));
        }
    }
    pub fn preload(&mut self, world: &World, position: Vec3, forward: Vec3) {
        self.ensure(world);
        self.poll();
        let mut routes = self.routes(world);
        routes.sort_by(|a, b| {
            let rank = |r: &Route| {
                let delta = r.aperture.center - position;
                delta.length_squared()
                    * if delta.normalize_or_zero().dot(forward) > 0.3 {
                        0.12
                    } else {
                        1.0
                    }
            };
            rank(a).total_cmp(&rank(b))
        });
        self.cache
            .retain(|world| routes.iter().any(|r| r.target == MapKey::of(world)));
        for route in routes.iter().take(2) {
            if self.get(route.target).is_none() {
                self.request(route.target);
                break;
            }
        }
    }
}
fn endpoint(world: &World, id: u32) -> Portal {
    let mut random = Random(id ^ (world.kind as u32).wrapping_mul(7919));
    let azimuth = random.next() * std::f32::consts::TAU;
    let distance = 340.0 + random.next() * 1000.0;
    let center = vec3(
        world.spawn.0 + azimuth.sin() * distance,
        world.spawn.1 - azimuth.cos() * distance,
        105.0 + random.next() * 330.0,
    );
    let mut aperture = Portal {
        center,
        yaw: azimuth + std::f32::consts::PI,
        radius: 38.0 + random.next() * 23.0,
    };
    safe_aperture(world, &mut aperture);
    aperture
}
fn safe_aperture(world: &World, aperture: &mut Portal) {
    if world.kind == MapKind::Space {
        for body in super::orbit::Body::ALL {
            let delta = aperture.center - body.center();
            if delta.length() < body.radius() + body.orbit_altitude() + 500.0 {
                aperture.center = body.center()
                    + delta.try_normalize().unwrap_or(Vec3::Y)
                        * (body.radius() + body.orbit_altitude() + 500.0);
            }
        }
        return;
    }
    let mut floor = 0.0_f32;
    for x in [-1.0, 0.0, 1.0] {
        for y in [-1.0, 0.0, 1.0] {
            floor = floor.max(world.height(
                aperture.center.x + x * aperture.radius,
                aperture.center.y + y * aperture.radius,
            ));
        }
    }
    aperture.center.z = aperture
        .center
        .z
        .max(floor + aperture.radius + crate::flight::MIN_CLEARANCE)
        .max(world.airspace.floor(world, aperture.center) + aperture.radius);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacing_portals_preserves_a_connected_two_exit_network() {
        let mut network = Network::new(7109);
        let mut world = World::generate(MapKind::City, MapKind::City.seed());
        for iteration in 0..60 {
            if iteration % 10 == 0 {
                world.seed += 1;
                network.ensure(&world);
            }
            network.summon(&world, vec3(2200.0, 2700.0, 350.0), -Vec3::Y);
            let keys: std::collections::HashSet<_> =
                network.links.iter().flat_map(|l| l.ends).collect();
            for key in &keys {
                assert_eq!(
                    network
                        .links
                        .iter()
                        .filter(|l| l.ends.contains(key))
                        .count(),
                    2
                );
            }
            let mut visited = std::collections::HashSet::new();
            let mut pending = vec![MapKey::of(&world)];
            while let Some(key) = pending.pop() {
                if !visited.insert(key) {
                    continue;
                }
                for link in network.links.iter().filter(|l| l.ends.contains(&key)) {
                    pending.extend(link.ends);
                }
            }
            assert_eq!(visited, keys);
        }
    }
}
