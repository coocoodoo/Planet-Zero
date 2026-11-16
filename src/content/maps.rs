use crate::world::MapKind;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapDefinition {
    pub name: String,
    pub seed: u32,
    pub water_coverage: f32,
    pub height_scale: f32,
    pub tree_threshold: u8,
    pub cloud_clusters: usize,
    pub elevation_octaves: Vec<(usize, f32)>,
}

pub fn definition(kind: MapKind) -> &'static MapDefinition {
    static MAPS: OnceLock<Vec<MapDefinition>> = OnceLock::new();
    &MAPS.get_or_init(|| {
        [
            include_str!("../../content/maps/islands.toml"),
            include_str!("../../content/maps/alpine.toml"),
            include_str!("../../content/maps/canyon.toml"),
            include_str!("../../content/maps/desert.toml"),
            include_str!("../../content/maps/wetlands.toml"),
            include_str!("../../content/maps/volcanic.toml"),
        ]
        .into_iter()
        .map(|source| toml::from_str(source).expect("地图配置格式错误"))
        .collect()
    })[kind as usize]
}
