pub mod levels;
pub mod maps;
pub mod vehicles;
pub mod weather;

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_level_and_map_data_is_valid() {
        use crate::gameplay::progression::Phase;
        for level in 1..=6 {
            let op = super::levels::operation(level);
            assert!(!op.name.is_empty() && op.name.len() < 20);
            assert_eq!(op.phases[2], Phase::Boss);
            assert!(op.durations[..2].iter().all(|v| *v >= 60.0 && *v < 180.0));
            assert!(op.reinforcement_seconds >= 5.0);
        }
        for kind in crate::world::MapKind::ALL {
            let map = super::maps::definition(kind);
            assert!(map.water_coverage >= 0.0 && map.water_coverage < 1.0);
            assert_eq!(kind.vacuum(), map.cosmic.is_some());
            if let Some(layout) = &map.cosmic {
                assert!(
                    layout
                        .landmarks
                        .iter()
                        .all(|p| p.scale > 0.0 && p.position.iter().all(|n| n.is_finite()))
                );
                assert!(layout.portals.is_empty() || layout.portals.len() == 2);
                assert!(
                    layout
                        .portals
                        .iter()
                        .all(|p| p.radius > 4.0 && p.yaw.is_finite())
                );
            }
            assert!(map.height_scale > 0.0 && map.height_scale <= 1.5);
            assert!(
                map.elevation_octaves
                    .iter()
                    .all(|(cell, amp)| cell.is_power_of_two()
                        && *cell <= 1024
                        && amp.is_finite()
                        && *amp > 0.0)
            );
        }
    }
}
