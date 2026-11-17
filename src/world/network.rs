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
    entrance: Option<Portal>,
}
#[derive(Clone, Copy)]
pub struct Route {
    pub id: u32,
    pub aperture: Portal,
    pub target: MapKey,
}
pub struct Network {
    links: Vec<Link>,
    cache: Vec<Arc<World>>,
    job: Option<(MapKey, std::thread::JoinHandle<World>)>,
    random: Random,
    pub active: bool,
}
impl Network {
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
            active: false,
        };
        for i in 0..kinds.len() {
            network.connect(kinds[i], kinds[(i + 1) % kinds.len()]);
            if i < kinds.len() / 2 {
                network.connect(kinds[i], kinds[i + kinds.len() / 2]);
            }
        }
        network
    }
    pub fn snapshot(&self) -> Self {
        Self {
            links: self.links.clone(),
            cache: self.cache.clone(),
            job: None,
            random: Random(self.random.0),
            active: self.active,
        }
    }
    fn connect(&mut self, a: MapKind, b: MapKind) {
        self.links.push(Link {
            id: self.random.0,
            ends: [
                MapKey {
                    kind: a,
                    seed: a.seed(),
                },
                MapKey {
                    kind: b,
                    seed: b.seed(),
                },
            ],
            entrance: None,
        });
        self.random.next();
    }
    pub fn ensure(&mut self, world: &World) {
        let key = MapKey::of(world);
        if self.links.iter().any(|l| l.ends.contains(&key)) {
            return;
        }
        for offset in [1, 4, 7] {
            let kind = MapKind::ALL[(world.kind as usize + offset) % MapKind::ALL.len()];
            self.links.push(Link {
                id: self.random.0,
                ends: [
                    key,
                    MapKey {
                        kind,
                        seed: kind.seed(),
                    },
                ],
                entrance: None,
            });
            self.random.next();
        }
    }
    pub fn routes(&self, world: &World) -> Vec<Route> {
        let key = MapKey::of(world);
        self.links
            .iter()
            .filter_map(|link| {
                let side = link.ends.iter().position(|k| *k == key)?;
                Some(Route {
                    id: link.id,
                    aperture: if side == 0 {
                        link.entrance.unwrap_or_else(|| endpoint(world, link.id))
                    } else {
                        endpoint(world, link.id)
                    },
                    target: link.ends[1 - side],
                })
            })
            .collect()
    }
    pub fn exit(&self, world: &World, id: u32) -> Portal {
        self.links
            .iter()
            .find(|l| l.id == id && l.ends[0] == MapKey::of(world))
            .and_then(|l| l.entrance)
            .unwrap_or_else(|| endpoint(world, id))
    }
    pub fn summon(&mut self, world: &World, position: Vec3, forward: Vec3) {
        self.ensure(world);
        let offset = 1 + (self.random.next() * 11.0) as usize;
        let kind = MapKind::ALL[(world.kind as usize + offset) % 12];
        let mut aperture = Portal {
            center: position + forward * 260.0 + Vec3::Z * (self.random.next() * 180.0 - 70.0),
            yaw: forward.x.atan2(-forward.y) + std::f32::consts::PI,
            radius: 42.0 + self.random.next() * 15.0,
        };
        safe_aperture(world, &mut aperture);
        let key = MapKey::of(world);
        if self
            .links
            .iter()
            .filter(|l| l.ends[0] == key && l.entrance.is_some())
            .count()
            >= 3
        {
            let at = self
                .links
                .iter()
                .position(|l| l.ends[0] == key && l.entrance.is_some())
                .unwrap();
            self.links.remove(at);
        }
        self.links.push(Link {
            id: self.random.0,
            ends: [
                key,
                MapKey {
                    kind,
                    seed: kind.seed(),
                },
            ],
            entrance: Some(aperture),
        });
        self.random.next();
        self.active = true;
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
            self.job = Some((
                key,
                std::thread::spawn(move || World::generate(key.kind, key.seed)),
            ));
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
        .max(floor + aperture.radius + crate::flight::MIN_CLEARANCE);
}
