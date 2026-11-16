use super::*;

#[test]
fn maps_have_distinct_terrain_and_valid_periodic_samples() {
    let mut signatures = std::collections::HashSet::new();
    for kind in MapKind::ALL {
        let world = World::generate(kind, kind.seed());
        assert_eq!(
            world.heights.len(),
            if kind == MapKind::Space {
                0
            } else {
                SIZE * SIZE
            }
        );
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

#[test]
fn lunar_sites_and_space_landmarks_are_present_without_atmosphere() {
    use celestial::Structure;
    let moon = World::generate(MapKind::Moon, 1969);
    assert_eq!(
        moon.scenery
            .iter()
            .filter(|s| s.kind == Structure::Base)
            .count(),
        4
    );
    assert!(moon.clouds.iter().all(|c| *c == 0));
    assert!(moon.materials.iter().all(|m| *m >= 27));
    assert!(moon.height(578.0 * 4.0, 600.0 * 4.0) < moon.height(578.0 * 4.0 + 370.0, 600.0 * 4.0));
    let space = World::generate(MapKind::Space, 1999);
    assert_eq!(space.portals.len(), 2);
    assert!(space.scenery.iter().any(|s| s.kind == Structure::Cruiser));
    assert!(space.scenery.iter().any(|s| s.kind == Structure::Satellite));
}
