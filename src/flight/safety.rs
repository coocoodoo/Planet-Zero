use super::*;
use crate::world::MapKind;

pub(super) fn clearance_floor(world: &World, position: Vec3) -> f32 {
    if world.kind == MapKind::Space {
        return -f32::INFINITY;
    }
    let mut height = 0.0_f32;
    for y in [-8.0, 0.0, 8.0] {
        for x in [-8.0, 0.0, 8.0] {
            height = height.max(world.flight_height(position + vec3(x, y, 0.0)));
        }
    }
    height + MIN_CLEARANCE
}

pub(super) fn flight_floor(
    world: &World,
    position: Vec3,
    forward: Vec3,
    speed: f32,
    turn: f32,
) -> (f32, f32) {
    if world.kind == MapKind::Space {
        return (-f32::INFINITY, -f32::INFINITY);
    }
    let floor = clearance_floor(world, position);
    let surface = world.airspace.floor(world, position);
    let mut target = surface;
    let horizontal = forward.truncate();
    let horizon = if world.kind == MapKind::Canyon {
        5.0
    } else {
        8.0
    };
    let steps = (speed * horizon / 24.0).ceil() as usize;
    for step in 1..=steps {
        let seconds = step as f32 * horizon / steps as f32;
        let phase = turn * seconds;
        let (s, c) = phase.sin_cos();
        let offset = if turn.abs() > 0.01 {
            glam::vec2(
                horizontal.x * s + horizontal.y * (c - 1.0),
                horizontal.x * (1.0 - c) + horizontal.y * s,
            ) * (speed / turn)
        } else {
            horizontal * speed * seconds
        };
        let probe = position + offset.extend(0.0);
        let ahead = world
            .airspace
            .floor(world, probe)
            .max(clearance_floor(world, probe));
        if ahead > surface + 0.1 {
            let reserve = speed * crate::world::airspace::SLOPE * 1.8 + 8.0;
            target = target.max(ahead + reserve - seconds * speed * crate::world::airspace::SLOPE);
        }
    }
    let gap = position.z - floor;
    let stopping = if gap > 0.0 {
        -(2.0 * 24.0 * gap).sqrt() * 0.7
    } else {
        -gap * 2.0
    };
    let guard = ((target - position.z) * 0.8).max(stopping);
    (floor, guard)
}

pub(super) fn advance(world: &World, from: Vec3, to: Vec3, max_lift: f32) -> Vec3 {
    if world.kind == MapKind::Space {
        return to;
    }
    let collision = |p| clearance_floor(world, p) - MIN_CLEARANCE + 8.0;
    let floor = collision(from);
    let ceiling = world.flight_ceiling(from);
    // 已经低于地形时按允许速度逐步抬升，避免瞬间弹到地表
    let z = if to.z < floor && from.z < floor {
        (from.z + max_lift).min(floor)
    } else {
        to.z.clamp(floor, ceiling)
    };
    let candidate = vec3(to.x, to.y, z);
    let safe = |p: Vec3| p.z >= collision(p) && p.z <= world.flight_ceiling(p);
    if safe(candidate) {
        return candidate;
    }
    let origin = vec3(from.x, from.y, z);
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..10 {
        let middle = (lo + hi) * 0.5;
        if safe(origin.lerp(candidate, middle)) {
            lo = middle;
        } else {
            hi = middle;
        }
    }
    origin.lerp(candidate, lo)
}
