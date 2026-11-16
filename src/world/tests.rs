use super::*;

#[test]
fn maps_have_distinct_terrain_and_valid_periodic_samples() {
    let mut signatures = std::collections::HashSet::new();
    for kind in MapKind::ALL {
        let world = World::generate(kind, kind.seed());
        assert_eq!(world.heights.len(), SIZE * SIZE);
        assert!(world.heights.iter().all(|&h| (h as usize) <= MAX_HEIGHT));
        assert!(
            world
                .materials
                .iter()
                .all(|&m| (m as usize) < MATERIAL_COUNT)
        );
        let signature = world
            .heights
            .iter()
            .zip(&world.materials)
            .step_by(997)
            .fold(0_u64, |sum, (&h, &m)| {
                sum.wrapping_mul(31).wrapping_add(h as u64 * 32 + m as u64)
            });
        assert!(signatures.insert(signature));
        assert_eq!(
            world.height(-17.0, 85.0),
            world.height(SIZE as f32 - 17.0, 85.0)
        );
        assert_eq!(
            world.height(124.0, SIZE as f32 + 71.0),
            world.height(124.0, 71.0)
        );
        assert!(world.height(world.spawn.0, world.spawn.1) < 80.0);
    }
}
