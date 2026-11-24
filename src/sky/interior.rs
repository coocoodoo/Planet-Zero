//! The inside of a space station: a hangar you walk around after docking, your ship on the pad,
//! and people at counters along the back wall: a mission agent, a trader, an outfitter, a
//! technician, and a traveller who wanders about. The hangar is built from extra triangles at a
//! fixed spot of the system's space map, where the floor is the map's flat zero plane.
use super::galaxy::StarSystem;
use super::names;
use super::props::{cuboid, spindle};
use super::rng::Rng;
use crate::render::{ExtraTriangle, mix, rgb};
use glam::{Vec2, Vec3, vec3};

/// Hangar centre in the space map (far from every planet, station and asteroid field)
pub const ORIGIN: Vec3 = Vec3::new(-30000.0, -30000.0, 0.0);
/// Walkable area relative to ORIGIN
pub const WALK_MIN: Vec2 = Vec2::new(-27.0, -30.0);
pub const WALK_MAX: Vec2 = Vec2::new(27.0, 16.0);
/// The parked ship relative to ORIGIN, and how close you can walk to it
pub const PAD: Vec2 = Vec2::new(0.0, 0.0);
const PAD_RADIUS: f32 = 8.0;
const CEILING: f32 = 14.0;
/// Counter front line (people stand behind it)
const COUNTER_Y: f32 = -31.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Agent,
    Trader,
    Outfitter,
    Technician,
    Traveller,
}
impl Role {
    pub fn title(self) -> &'static str {
        match self {
            Self::Agent => "MISSION AGENT",
            Self::Trader => "TRADER",
            Self::Outfitter => "OUTFITTER",
            Self::Technician => "TECHNICIAN",
            Self::Traveller => "TRAVELLER",
        }
    }
    /// Label of key 4 in the conversation
    pub fn service(self) -> &'static str {
        match self {
            Self::Agent => "MISSIONS",
            Self::Trader => "TRADE",
            Self::Outfitter => "ARMORY",
            Self::Technician => "UPGRADE",
            Self::Traveller => "ASK",
        }
    }
    pub fn sign(self) -> u32 {
        match self {
            Self::Agent => rgb(255, 200, 80),
            Self::Trader => rgb(110, 230, 140),
            Self::Outfitter => rgb(255, 110, 90),
            Self::Technician => rgb(110, 200, 255),
            Self::Traveller => rgb(220, 160, 255),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Npc {
    pub role: Role,
    pub name: String,
    /// 0–2: body plan of the species running this station
    pub species: u8,
    pub position: Vec3,
    pub skin: u32,
    pub cloth: u32,
    pub seed: u64,
}

/// Species body plans (original designs): tall and crested, stout and broad, and a floating
/// lantern-headed folk.
pub const SPECIES: [&str; 3] = ["TALLFOLK", "STONEKIN", "LANTERNS"];

/// The station's people. The traveller walks a loop; the rest stay at their counters.
pub fn people(system: &StarSystem, visit: u32, time: f32) -> Vec<Npc> {
    let mut rng = Rng::new(system.seed ^ 0x57a7_1011);
    let species = rng.below(3) as u8;
    let skin = [rgb(120, 170, 150), rgb(170, 130, 110), rgb(150, 140, 200)][species as usize];
    let skin = mix(skin, rgb(rng.below(255) as u8, 120, 160), 0.2);
    let roles = [Role::Agent, Role::Trader, Role::Outfitter, Role::Technician];
    let mut out: Vec<Npc> = roles
        .iter()
        .enumerate()
        .map(|(i, &role)| Npc {
            role,
            name: names::name(system.seed ^ i as u64, 70),
            species,
            position: ORIGIN + vec3(-18.0 + i as f32 * 12.0, COUNTER_Y - 2.5, 0.0),
            skin,
            cloth: mix(role.sign(), rgb(40, 44, 60), 0.45),
            seed: system.seed ^ ((i as u64 + 1) * 0x9e37),
        })
        .collect();
    // a visitor from elsewhere, different each docking, strolling around the hangar
    let mut v = Rng::new(system.seed ^ (visit as u64).wrapping_mul(0x77));
    let around = time * 0.08 + v.range(0.0, 6.0);
    out.push(Npc {
        role: Role::Traveller,
        name: names::name(system.seed ^ visit as u64, 71),
        species: v.below(3) as u8,
        position: ORIGIN + vec3(around.cos() * 17.0, around.sin() * 9.0 - 14.0, 0.0),
        skin: [rgb(200, 180, 120), rgb(110, 150, 190), rgb(190, 120, 150)][v.below(3) as usize],
        cloth: mix(Role::Traveller.sign(), rgb(30, 30, 40), 0.5),
        seed: system.seed ^ visit as u64 ^ 0x7e,
    });
    out
}

/// Where you stand to talk to someone.
pub fn talk_spot(npc: &Npc) -> Vec3 {
    if npc.role == Role::Traveller {
        npc.position
    } else {
        npc.position + Vec3::Y * 4.5
    }
}

/// Keep a walker inside the hangar and off the landing pad.
pub fn clamp(p: Vec3) -> Vec3 {
    let mut local = (p - ORIGIN).truncate().clamp(WALK_MIN, WALK_MAX);
    let from_pad = local - PAD;
    if from_pad.length() < PAD_RADIUS {
        local = PAD + from_pad.normalize_or(Vec2::X) * PAD_RADIUS;
    }
    (ORIGIN.truncate() + local).extend(p.z.clamp(0.0, CEILING - 3.0))
}

/// Where you stand after docking: beside the ship, facing the counters.
pub fn arrival() -> (Vec3, f32) {
    (ORIGIN + vec3(10.0, 2.0, 0.0), std::f32::consts::PI * 0.95)
}

/// A flat-shaded quad: brightness from which way it faces (no dependence on the sun).
fn quad(out: &mut Vec<ExtraTriangle>, p: [Vec3; 4], color: u32) {
    let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
    let shade = 0.55 + 0.35 * normal.z.max(0.0) + 0.12 * normal.x.abs();
    let c = mix(rgb(0, 0, 0), color, shade.min(1.0));
    out.push(ExtraTriangle {
        points: [p[0], p[1], p[2]],
        color: c,
        glow: true,
    });
    out.push(ExtraTriangle {
        points: [p[0], p[2], p[3]],
        color: c,
        glow: true,
    });
}

/// The hangar: floor tiles, walls, ceiling beams, counters with signs, the pad, the open door.
pub fn hangar(system: &StarSystem, time: f32, out: &mut Vec<ExtraTriangle>) {
    let o = ORIGIN;
    let at = |x: f32, y: f32, z: f32| o + vec3(x, y, z);
    let accent = system.class.color();
    let floor = rgb(58, 64, 78);
    // floor in 12×12 tiles with a checker tone and lit lanes
    for ty in -4..2 {
        for tx in -3..3 {
            let (x0, y0) = (tx as f32 * 12.0, ty as f32 * 12.0 - 2.0);
            let tone = if (tx + ty) % 2 == 0 {
                floor
            } else {
                mix(floor, rgb(90, 96, 112), 0.35)
            };
            quad(
                out,
                [
                    at(x0, y0, 0.0),
                    at(x0 + 12.0, y0, 0.0),
                    at(x0 + 12.0, y0 + 12.0, 0.0),
                    at(x0, y0 + 12.0, 0.0),
                ],
                tone,
            );
        }
    }
    for x in [-24.0_f32, 24.0] {
        quad(
            out,
            [
                at(x - 0.3, -30.0, 0.02),
                at(x + 0.3, -30.0, 0.02),
                at(x + 0.3, 20.0, 0.02),
                at(x - 0.3, 20.0, 0.02),
            ],
            mix(accent, rgb(255, 255, 255), 0.3),
        );
    }
    // walls and ceiling
    let wall = rgb(84, 90, 108);
    quad(
        out,
        [
            at(-36.0, -38.0, 0.0),
            at(36.0, -38.0, 0.0),
            at(36.0, -38.0, CEILING),
            at(-36.0, -38.0, CEILING),
        ],
        wall,
    );
    for (x, sign) in [(-36.0_f32, 1.0_f32), (36.0, -1.0)] {
        let _ = sign;
        quad(
            out,
            [
                at(x, -38.0, 0.0),
                at(x, 22.0, 0.0),
                at(x, 22.0, CEILING),
                at(x, -38.0, CEILING),
            ],
            mix(wall, rgb(0, 0, 0), 0.15),
        );
    }
    for y in [-30.0_f32, -14.0, 2.0, 18.0] {
        cuboid(
            out,
            at(0.0, y, CEILING),
            vec3(36.0, 0.8, 0.6),
            0.0,
            rgb(70, 74, 90),
            true,
        );
        cuboid(
            out,
            at(0.0, y, CEILING - 0.7),
            vec3(30.0, 0.15, 0.1),
            0.0,
            mix(accent, rgb(255, 255, 255), 0.6),
            true,
        );
    }
    for x in [-34.0_f32, 34.0] {
        for y in [-30.0_f32, -10.0, 10.0] {
            cuboid(
                out,
                at(x, y, CEILING * 0.5),
                vec3(1.0, 1.0, CEILING * 0.5),
                0.0,
                rgb(96, 100, 116),
                true,
            );
        }
    }
    // open hangar door: frame and a shimmering field
    let pulse = 0.5 + 0.5 * super::rng::fast_sin(time * 1.7);
    let field = mix(rgb(40, 90, 140), rgb(90, 200, 255), pulse);
    for x in [-36.0_f32, 36.0] {
        cuboid(
            out,
            at(x, 22.0, CEILING * 0.5),
            vec3(1.2, 1.2, CEILING * 0.5),
            0.0,
            rgb(110, 116, 132),
            true,
        );
    }
    cuboid(
        out,
        at(0.0, 22.0, CEILING),
        vec3(37.0, 1.2, 0.8),
        0.0,
        rgb(110, 116, 132),
        true,
    );
    for k in 0..4 {
        let z = 1.5 + k as f32 * 3.2;
        cuboid(
            out,
            at(0.0, 22.0, z),
            vec3(35.0, 0.05, 0.06),
            0.0,
            field,
            true,
        );
    }
    // landing pad ring
    for k in 0..12 {
        let a0 = k as f32 / 12.0 * std::f32::consts::TAU;
        let a1 = (k as f32 + 0.6) / 12.0 * std::f32::consts::TAU;
        let r = PAD_RADIUS + 1.0;
        let p0 = at(PAD.x + a0.cos() * r, PAD.y + a0.sin() * r, 0.04);
        let p1 = at(PAD.x + a1.cos() * r, PAD.y + a1.sin() * r, 0.04);
        let glow = if (k + (time * 3.0) as usize).is_multiple_of(3) {
            rgb(255, 210, 90)
        } else {
            rgb(90, 200, 255)
        };
        quad(
            out,
            [
                p0,
                p1,
                p1 + (p1 - o).with_z(0.0).normalize_or_zero() * 0.6,
                p0 + (p0 - o).with_z(0.0).normalize_or_zero() * 0.6,
            ],
            glow,
        );
    }
    // counters with role signs
    for (i, role) in [Role::Agent, Role::Trader, Role::Outfitter, Role::Technician]
        .iter()
        .enumerate()
    {
        let x = -18.0 + i as f32 * 12.0;
        cuboid(
            out,
            at(x, COUNTER_Y, 0.6),
            vec3(4.5, 0.8, 0.6),
            0.0,
            rgb(70, 76, 94),
            true,
        );
        cuboid(
            out,
            at(x, COUNTER_Y, 1.25),
            vec3(4.6, 0.9, 0.06),
            0.0,
            rgb(150, 156, 172),
            true,
        );
        cuboid(
            out,
            at(x + 2.6, COUNTER_Y, 1.7),
            vec3(0.6, 0.3, 0.4),
            0.0,
            mix(role.sign(), rgb(255, 255, 255), 0.4),
            true,
        );
        cuboid(
            out,
            at(x, -37.6, 7.5),
            vec3(4.0, 0.2, 0.9),
            0.0,
            role.sign(),
            true,
        );
    }
    // cargo crates near the walls
    for (x, y, s) in [
        (-30.0, 12.0, 1.6),
        (-27.0, 13.5, 1.0),
        (30.0, -20.0, 1.4),
        (29.0, -16.5, 1.0),
        (31.0, 8.0, 1.2),
    ] {
        cuboid(
            out,
            at(x, y, s),
            Vec3::splat(s),
            0.3,
            rgb(120, 96, 64),
            true,
        );
        cuboid(
            out,
            at(x, y, s),
            vec3(s + 0.05, 0.12, s + 0.05),
            0.3,
            mix(accent, rgb(0, 0, 0), 0.3),
            true,
        );
    }
}

/// A person: body, head by species, arms, idle bob; faces `look` when it is near.
pub fn person(npc: &Npc, time: f32, look: Vec3, out: &mut Vec<ExtraTriangle>) {
    let p = npc.position;
    let to = (look - p).truncate();
    let yaw = if to.length() < 14.0 {
        to.y.atan2(to.x)
    } else {
        -std::f32::consts::FRAC_PI_2
    };
    let bob = super::rng::fast_sin(time * 2.2 + npc.seed as f32) * 0.05;
    let (s, c) = yaw.sin_cos();
    let forward = vec3(c, s, 0.0);
    let side = vec3(-s, c, 0.0);
    let (tall, wide) = match npc.species {
        0 => (1.25, 0.8),
        1 => (0.9, 1.25),
        _ => (1.05, 0.9),
    };
    let hip = p + Vec3::Z * (1.0 * tall + bob);
    // legs and body
    for k in [-1.0, 1.0] {
        cuboid(
            out,
            p + side * 0.22 * k * wide + Vec3::Z * 0.5 * tall,
            vec3(0.16, 0.16, 0.5 * tall),
            yaw,
            mix(npc.cloth, rgb(0, 0, 0), 0.35),
            true,
        );
    }
    spindle(
        out,
        hip - Vec3::Z * 0.1,
        hip + Vec3::Z * 0.55 * tall,
        hip + Vec3::Z * 1.0 * tall,
        0.42 * wide,
        4,
        yaw + 0.78,
        (npc.cloth, mix(npc.cloth, rgb(0, 0, 0), 0.3)),
        true,
    );
    // arms
    let wave = super::rng::fast_sin(time * 1.3 + npc.seed as f32 * 0.1) * 0.15;
    for k in [-1.0_f32, 1.0] {
        let shoulder = hip + Vec3::Z * 0.85 * tall + side * 0.5 * wide * k;
        let hand = shoulder - Vec3::Z * 0.75 * tall + forward * (0.15 + wave * k);
        spindle(
            out,
            hand,
            shoulder.lerp(hand, 0.5),
            shoulder,
            0.1,
            3,
            yaw,
            (npc.skin, npc.skin),
            true,
        );
    }
    let neck = hip + Vec3::Z * 1.05 * tall;
    match npc.species {
        0 => {
            // tall head with a crest
            spindle(
                out,
                neck,
                neck + Vec3::Z * 0.35,
                neck + Vec3::Z * 0.8,
                0.28,
                4,
                yaw,
                (npc.skin, mix(npc.skin, rgb(0, 0, 0), 0.25)),
                true,
            );
            spindle(
                out,
                neck + Vec3::Z * 0.5 - forward * 0.1,
                neck + Vec3::Z * 0.75 - forward * 0.3,
                neck + Vec3::Z * 1.1 - forward * 0.5,
                0.08,
                3,
                yaw,
                (npc.cloth, npc.cloth),
                true,
            );
            cuboid(
                out,
                neck + Vec3::Z * 0.45 + forward * 0.24,
                vec3(0.04, 0.18, 0.04),
                yaw,
                rgb(255, 240, 200),
                true,
            );
        }
        1 => {
            // broad, flat head with a heavy brow
            cuboid(
                out,
                neck + Vec3::Z * 0.3,
                vec3(0.32, 0.36, 0.26),
                yaw,
                npc.skin,
                true,
            );
            cuboid(
                out,
                neck + Vec3::Z * 0.42 + forward * 0.3,
                vec3(0.05, 0.3, 0.05),
                yaw,
                rgb(40, 30, 20),
                true,
            );
            cuboid(
                out,
                neck + Vec3::Z * 0.33 + forward * 0.33,
                vec3(0.03, 0.2, 0.03),
                yaw,
                rgb(255, 200, 120),
                true,
            );
        }
        _ => {
            // a lantern: a glowing orb floating above the shoulders
            let float = neck + Vec3::Z * (0.55 + bob * 4.0);
            spindle(
                out,
                float - Vec3::Z * 0.3,
                float,
                float + Vec3::Z * 0.3,
                0.3,
                5,
                time,
                (mix(npc.skin, rgb(255, 255, 220), 0.5), npc.skin),
                true,
            );
        }
    }
}

/// What a person says when you walk up.
pub fn greeting(npc: &Npc, visit: u32) -> String {
    let mut rng = Rng::new(npc.seed ^ visit as u64);
    let lines: &[&str] = match npc.role {
        Role::Agent => &[
            "WORK IS WAITING, PILOT",
            "THE GUILD PAYS ON TIME",
            "PICK A JOB, ANY JOB",
            "BUSY SKIES TODAY",
        ],
        Role::Trader => &[
            "BUYING AND SELLING",
            "FAIR PRICES, MOSTLY",
            "WHAT HAVE YOU BROUGHT",
            "CREDITS TALK HERE",
        ],
        Role::Outfitter => &[
            "NEW STOCK IN THE RACK",
            "LOOKING FOR A BIGGER GUN",
            "EVERY BARREL TESTED",
            "ARMS FOR THE ROAD",
        ],
        Role::Technician => &[
            "YOUR SHIP SOUNDS TIRED",
            "I CAN TUNE THAT DRIVE",
            "UPGRADES WHILE YOU WAIT",
            "LET ME LOOK AT THAT SUIT",
        ],
        Role::Traveller => &[
            "LONG WAY FROM HOME",
            "HAVE YOU SEEN THE CORE",
            "QUIET STATION TODAY",
            "I TRADE IN STORIES",
        ],
    };
    lines[rng.below(lines.len() as u32) as usize].to_owned()
}

/// Small talk: tips, rumours and lore, cycling with each CHAT.
pub fn chat(npc: &Npc, index: u32) -> String {
    const TIPS: [&str; 16] = [
        "PIRATES HATE A FULL SHIELD",
        "TILT GENTLY WHEN YOU AIM",
        "RICH ROCKS GLOW BLUE",
        "GOLDEN ROCKS PAY THE BEST",
        "CACHES WAKE THEIR GUARDS",
        "BEACONS SHOW THE WAY",
        "RUINS REMEMBER THINGS",
        "DRONES CANNOT SEE THROUGH HILLS",
        "SWAP WEAPONS WHEN HOT",
        "A STEADY PERK HELPS YOUR AIM",
        "SODIUM KEEPS THE HEAT OUT",
        "JUMP CELLS NEED TRITIUM",
        "THE GUILD REMEMBERS YOU",
        "RAIDERS STRIKE WHEN YOU ROAM",
        "THE MOON IS NEVER FAR",
        "EVERY STAR HAS A STATION",
    ];
    let mut rng = Rng::new(npc.seed);
    let start = rng.below(TIPS.len() as u32);
    TIPS[((start + index) % TIPS.len() as u32) as usize].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::galaxy::Galaxy;
    #[test]
    fn hangar_has_its_people_and_keeps_walkers_inside() {
        let galaxy = Galaxy::default();
        let system = galaxy.system(galaxy.start());
        let people = people(&system, 3, 0.0);
        assert_eq!(people.len(), 5);
        assert_eq!(
            people.iter().filter(|n| n.role == Role::Traveller).count(),
            1
        );
        for npc in &people {
            let spot = clamp(talk_spot(npc));
            assert!(
                spot.distance(talk_spot(npc)) < 0.01 || npc.role == Role::Traveller,
                "{:?} reachable",
                npc.role
            );
            assert!(greeting(npc, 3).len() <= 34);
            assert!(chat(npc, 5).len() <= 34);
        }
        let outside = clamp(ORIGIN + vec3(200.0, -200.0, 50.0));
        assert!((outside - ORIGIN).truncate().x <= WALK_MAX.x && outside.z <= CEILING);
        let on_pad = clamp(ORIGIN + vec3(1.0, 1.0, 0.0));
        assert!((on_pad - ORIGIN).truncate().distance(PAD) >= PAD_RADIUS - 0.01);
        let mut tris = Vec::new();
        hangar(&system, 0.0, &mut tris);
        let hangar_tris = tris.len();
        for npc in &people {
            person(npc, 0.0, arrival().0, &mut tris);
        }
        // no terrain is drawn inside: ~1,000 triangles is a tenth of a planet-side frame
        assert!(tris.len() < 1200, "{} triangles", tris.len());
        assert!(hangar_tris > 50);
    }
}
