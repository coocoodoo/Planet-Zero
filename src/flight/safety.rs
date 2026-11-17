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

pub(super) fn loop_lift(
    world: &World,
    position: Vec3,
    yaw: f32,
    pitch: f32,
    rate: f32,
    speed: f32,
) -> f32 {
    if world.kind == MapKind::Space {
        return 0.0;
    }
    let heading = vec3(yaw.sin(), -yaw.cos(), 0.0);
    let radius = speed / rate;
    let mut lift = 0.0_f32;
    for t in [0.4, 0.8, 1.3, 1.9, 2.6, 3.4] {
        let a = pitch + rate * t;
        let probe = position
            + heading * radius * (a.sin() - pitch.sin())
            + Vec3::Z * radius * (pitch.cos() - a.cos());
        lift = lift.max(clearance_floor(world, probe) + 6.0 - probe.z);
    }
    lift
}
