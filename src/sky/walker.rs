//! On-foot first-person movement over the engine heightmap: walking, slope limit, gravity and a
//! jetpack. Units are engine world units (one heightmap cell is one unit; the aircraft is ~17 long).
use crate::flight::{Eye, heading};
use crate::world::{SIZE, World};
use glam::{Vec3, vec3};

pub const EYE_HEIGHT: f32 = 2.2;

/// Ground height that matches what the renderer draws. See `Walker::ground`.
pub fn ground(world: &World, x: f32, y: f32) -> f32 {
    let (gx, gy) = (x / 4.0, y / 4.0);
    let (x0, y0) = (gx.floor(), gy.floor());
    let (fx, fy) = (gx - x0, gy - y0);
    let h = |i: f32, j: f32| world.mesh_height(((x0 + i) * 4.0) as i32, ((y0 + j) * 4.0) as i32);
    let top = h(0.0, 0.0) + (h(1.0, 0.0) - h(0.0, 0.0)) * fx;
    let bottom = h(0.0, 1.0) + (h(1.0, 1.0) - h(0.0, 1.0)) * fx;
    let mesh = top + (bottom - top) * fy;
    let solid = world.height(x, y);
    if solid > mesh + 3.0 { solid } else { mesh }
}
pub const WALK_SPEED: f32 = 7.5;
pub const RUN_SPEED: f32 = 13.0;
const GRAVITY: f32 = 26.0;
const JET_ACCEL: f32 = 44.0;
const MAX_RISE: f32 = 2.4;
/// Largest ground rise (units per unit moved) that can be walked up.
const MAX_SLOPE: f32 = 1.1;

#[derive(Clone, Copy, Debug, Default)]
pub struct WalkInput {
    /// -1 back … 1 forward
    pub forward: f32,
    /// -1 left … 1 right
    pub strafe: f32,
    /// -1 left … 1 right (radians per second are applied inside)
    pub turn: f32,
    /// -1 down … 1 up
    pub look: f32,
    pub jet: bool,
    pub run: bool,
    /// Ease the view back toward the horizon when not looking up or down
    pub level: bool,
}

/// Turn rate ramps from slow (precise aim) to fast (turn around) while a turn key is held.
pub const TURN_SLOW: f32 = 0.9;
pub const TURN_FAST: f32 = 2.8;
const TURN_RAMP: f32 = 0.35;
/// A tap of a turn key nudges the view this much
const TURN_NUDGE: f32 = 0.05;
/// Seconds without look input before the view levels
const LEVEL_AFTER: f32 = 1.8;
pub const LEVEL_PITCH: f32 = -0.07;
const QUICK_TURN_SECONDS: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walker {
    /// Feet position
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: Vec3,
    pub grounded: bool,
    /// Seconds the current turn has been held
    pub turn_hold: f32,
    /// Radians left of a quick 180° turn
    pub quick_turn: f32,
    /// Seconds since the last look input
    pub look_idle: f32,
}
impl Walker {
    pub fn new(world: &World, x: f32, y: f32, yaw: f32) -> Self {
        Self {
            position: vec3(x, y, ground(world, x, y)),
            yaw,
            pitch: -0.08,
            velocity: Vec3::ZERO,
            grounded: true,
            turn_hold: 0.0,
            quick_turn: 0.0,
            look_idle: 0.0,
        }
    }
    /// A walker on a flat deck (station hangar) at `position`.
    pub fn new_flat(position: Vec3, yaw: f32) -> Self {
        Self {
            position,
            yaw,
            pitch: -0.08,
            velocity: Vec3::ZERO,
            grounded: true,
            turn_hold: 0.0,
            quick_turn: 0.0,
            look_idle: 0.0,
        }
    }
    /// Start turning around (double tap back).
    pub fn turn_around(&mut self) {
        if self.quick_turn == 0.0 {
            self.quick_turn = std::f32::consts::PI;
        }
    }
    pub fn eye(&self) -> Eye {
        Eye {
            position: self.position + Vec3::Z * EYE_HEIGHT,
            yaw: self.yaw,
            pitch: self.pitch,
        }
    }
    /// Ground under a point: the rendered terrain mesh (vertices every 4 units, bilinear),
    /// or a building or obstacle when one stands there.
    pub fn ground(world: &World, x: f32, y: f32) -> f32 {
        ground(world, x, y)
    }
    /// Advance one step. `jet_fuel` says whether the jetpack has charge. Returns true while
    /// the jetpack fires.
    pub fn step(&mut self, world: &World, input: WalkInput, dt: f32, jet_fuel: bool) -> bool {
        let limit = 8.0;
        let bounds = (
            glam::Vec2::splat(limit),
            glam::Vec2::splat(SIZE as f32 - limit),
        );
        self.step_on(&|x, y| ground(world, x, y), bounds, input, dt, jet_fuel)
    }
    /// Walk over any ground (a planet's mesh, a station's flat deck) inside `bounds` (x, y).
    pub fn step_on(
        &mut self,
        ground_at: &dyn Fn(f32, f32) -> f32,
        bounds: (glam::Vec2, glam::Vec2),
        input: WalkInput,
        dt: f32,
        jet_fuel: bool,
    ) -> bool {
        if input.turn != 0.0 {
            if self.turn_hold == 0.0 {
                self.yaw += input.turn.signum() * TURN_NUDGE;
            }
            self.turn_hold += dt;
            let ramp = (self.turn_hold / TURN_RAMP).min(1.0);
            self.yaw += input.turn * (TURN_SLOW + (TURN_FAST - TURN_SLOW) * ramp * ramp) * dt;
        } else {
            self.turn_hold = 0.0;
        }
        if self.quick_turn > 0.0 {
            let step = (std::f32::consts::PI / QUICK_TURN_SECONDS * dt).min(self.quick_turn);
            self.yaw += step;
            self.quick_turn -= step;
        }
        if input.look != 0.0 {
            self.look_idle = 0.0;
            self.pitch = (self.pitch + input.look * 1.4 * dt).clamp(-1.1, 0.9);
        } else {
            self.look_idle += dt;
            if input.level && self.look_idle > LEVEL_AFTER {
                self.pitch += (LEVEL_PITCH - self.pitch) * (3.0 * dt).min(1.0);
            }
        }
        let (forward, right) = heading(self.yaw);
        let speed = if input.run { RUN_SPEED } else { WALK_SPEED };
        let wish = (forward * input.forward + right * input.strafe).clamp_length_max(1.0) * speed;
        // on the ground movement is direct; in the air only partial control
        let control = if self.grounded { 1.0 } else { 0.25 };
        let blend = (control * 12.0 * dt).min(1.0);
        self.velocity.x += (wish.x - self.velocity.x) * blend;
        self.velocity.y += (wish.y - self.velocity.y) * blend;
        let jetting = input.jet && jet_fuel;
        if jetting {
            self.velocity.z = (self.velocity.z + JET_ACCEL * dt).min(14.0);
            self.grounded = false;
        }
        self.velocity.z -= GRAVITY * dt;
        let horizontal = vec3(self.velocity.x, self.velocity.y, 0.0) * dt;
        let here = ground_at(self.position.x, self.position.y);
        let mut next = self.position;
        // move each axis separately so the walker slides along walls and steep slopes
        for axis in [Vec3::X, Vec3::Y] {
            let step = horizontal * axis;
            if step.length_squared() == 0.0 {
                continue;
            }
            let candidate = next + step;
            let (cx, cy) = (
                candidate.x.clamp(bounds.0.x, bounds.1.x),
                candidate.y.clamp(bounds.0.y, bounds.1.y),
            );
            let ground = ground_at(cx, cy);
            let rise = ground - next.z;
            let steep = rise > MAX_RISE.max(step.length() * MAX_SLOPE) && ground > here + 0.05;
            if !steep || !self.grounded && next.z > ground {
                next.x = cx;
                next.y = cy;
            }
        }
        next.z += self.velocity.z * dt;
        let ground = ground_at(next.x, next.y);
        if next.z <= ground {
            next.z = ground;
            self.velocity.z = 0.0;
            self.grounded = true;
        } else if self.grounded && next.z - ground < 0.9 && !jetting {
            // stay on the ground walking downhill
            next.z = ground;
            self.velocity.z = 0.0;
        } else {
            self.grounded = false;
        }
        self.position = next;
        jetting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapKind;
    #[test]
    fn walker_follows_the_ground_and_the_jetpack_climbs() {
        let world = World::generate(MapKind::Highlands, 77);
        let (x, y) = world.spawn;
        let mut walker = Walker::new(&world, x, y, 0.4);
        let start = walker.position;
        let input = WalkInput {
            forward: 1.0,
            ..Default::default()
        };
        for _ in 0..90 {
            walker.step(&world, input, 1.0 / 30.0, false);
            let ground = ground(&world, walker.position.x, walker.position.y);
            assert!(walker.position.z >= ground - 0.001);
            assert!(walker.position.z - ground < 3.0);
        }
        let moved = (walker.position - start).truncate().length();
        assert!(moved > 4.0, "{moved}");
        let before = walker.position.z;
        for _ in 0..30 {
            walker.step(
                &world,
                WalkInput {
                    jet: true,
                    ..Default::default()
                },
                1.0 / 30.0,
                true,
            );
        }
        assert!(walker.position.z > before + 4.0);
        assert!(!walker.grounded);
        for _ in 0..200 {
            walker.step(&world, WalkInput::default(), 1.0 / 30.0, false);
        }
        assert!(walker.grounded);
    }
    #[test]
    fn turning_ramps_up_quick_turn_spins_round_and_the_view_levels() {
        let world = World::generate(MapKind::Highlands, 77);
        let (x, y) = world.spawn;
        let mut walker = Walker::new(&world, x, y, 0.0);
        let dt = 1.0 / 30.0;
        let turn = WalkInput {
            turn: 1.0,
            ..Default::default()
        };
        walker.step(&world, turn, dt, false);
        let first = walker.yaw;
        assert!(
            first > TURN_NUDGE && first < TURN_NUDGE + 0.05,
            "a tap nudges"
        );
        for _ in 0..30 {
            walker.step(&world, turn, dt, false);
        }
        let before = walker.yaw;
        walker.step(&world, turn, dt, false);
        assert!(
            (walker.yaw - before - TURN_FAST * dt).abs() < 0.001,
            "full speed when held"
        );
        let start = walker.yaw;
        walker.step(&world, WalkInput::default(), dt, false);
        walker.turn_around();
        for _ in 0..12 {
            walker.step(&world, WalkInput::default(), dt, false);
        }
        assert!((walker.yaw - start - std::f32::consts::PI).abs() < 0.01);
        walker.pitch = 0.6;
        let level = WalkInput {
            level: true,
            ..Default::default()
        };
        for _ in 0..150 {
            walker.step(&world, level, dt, false);
        }
        assert!((walker.pitch - LEVEL_PITCH).abs() < 0.05);
    }
}
