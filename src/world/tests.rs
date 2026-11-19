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
        assert!(world.height(world.spawn.0, world.spawn.1) < 200.0);
        for object in &world.scenery {
            if let Some(size) = object.kind.landmark_size() {
                let ground = world.heights
                    [index(object.position.x as i32, object.position.y as i32, SIZE)]
                    as f32;
                assert!((object.position.z - ground).abs() < 1.0);
                assert!(
                    world.height(object.position.x, object.position.y)
                        >= object.position.z + size.z * object.scale - 1.0
                );
            }
        }
        if kind == MapKind::Alpine {
            assert!(world.height(2048.0, 2200.0) > 220.0);
            assert_eq!(world.materials[index(2048, 2200, SIZE)], 15);
        }
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

#[test]
fn canyon_terrain_wraps_across_the_y_boundary() {
    let world = World::generate(MapKind::Canyon, 7);
    let mut worst = 0_i32;
    for x in 0..SIZE {
        let edge = world.heights[(SIZE - 1) * SIZE + x] as i32;
        let seam = world.heights[x] as i32;
        worst = worst.max((edge - seam).abs());
    }
    assert!(worst <= 40, "canyon y seam jumps by {worst}");
}
