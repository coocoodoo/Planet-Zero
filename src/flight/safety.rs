use super::*;
use crate::world::MapKind;

pub(super) fn clearance_floor(world: &World, position: Vec3) -> f32 {
    if world.kind == MapKind::Space {
        return -240.0;
    }
    let mut height = 0.0_f32;
    for y in [-8.0, 0.0, 8.0] {
        for x in [-8.0, 0.0, 8.0] {
            height = height.max(world.height(position.x + x, position.y + y));
        }
    }
    height + MIN_CLEARANCE
}

pub(super) fn flight_floor(world: &World, position: Vec3, forward: Vec3, speed: f32) -> (f32, f32) {
    let floor = clearance_floor(world, position);
    let mut climb = (floor - position.z) * 2.5;
    if world.kind == MapKind::Space {
        return (floor, climb);
    }
    let right = vec3(-forward.y, forward.x, 0.0).normalize_or_zero();
    for seconds in [0.3, 0.65, 1.1, 1.8, 2.4] {
        for side in [-1.0, 0.0, 1.0] {
            let probe = position + forward * speed * seconds + right * side * 12.0;
            let target = clearance_floor(world, probe);
            if target > floor + 6.0 {
                climb = climb.max((target + 3.0 - position.z) / seconds);
            }
        }
    }
    (floor, climb)
}
