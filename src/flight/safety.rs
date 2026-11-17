use super::*;
use crate::world::MapKind;

pub(super) fn clearance_floor(world: &World, position: Vec3) -> f32 {
    if world.kind == MapKind::Space {
        return -240.0;
    }
    let mut height = 0.0_f32;
    for y in [-8.0, 0.0, 8.0] {
        for x in [-8.0, 0.0, 8.0] {
            height = height.max(world.flight_height(position + vec3(x, y, 0.0)));
        }
    }
    height + MIN_CLEARANCE
}

pub(super) fn flight_floor(world: &World, position: Vec3, forward: Vec3, speed: f32) -> (f32, f32) {
    let floor = clearance_floor(world, position);
    let gap = position.z - floor;
    let mut climb = if gap > 0.0 {
        -(2.0 * 60.0 * gap).sqrt() * 0.65
    } else {
        -gap * 2.5
    };
    if world.kind == MapKind::Space {
        return (floor, climb);
    }
    let right = vec3(-forward.y, forward.x, 0.0).normalize_or_zero();
    let steps = (speed * 3.0 / 8.0).ceil() as usize;
    for step in 1..=steps {
        let seconds = step as f32 * 3.0 / steps as f32;
        for side in [-1.0, 0.0, 1.0] {
            let probe = position + forward * speed * seconds + right * side * 12.0;
            let target = world.flight_height(probe) + MIN_CLEARANCE;
            if target > floor + 6.0 {
                climb = climb.max((target + 12.0 - position.z) / (seconds - 0.45).max(0.25));
            }
        }
    }
    (floor, climb)
}
