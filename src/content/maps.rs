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
    #[serde(default)]
    pub cosmic: Option<CosmicDefinition>,
    #[serde(default)]
    pub landmarks: Vec<Landmark>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CosmicDefinition {
    pub spawn: [f32; 2],
    #[serde(default)]
    pub crater_count: usize,
    #[serde(default)]
    pub featured_crater: [f32; 3],
    #[serde(default)]
    pub debris_count: usize,
    #[serde(default)]
    pub landmarks: Vec<Landmark>,
    #[serde(default)]
    pub portals: Vec<PortalDefinition>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Landmark {
    pub position: [f32; 3],
    pub yaw: f32,
    pub scale: f32,
    pub kind: crate::world::celestial::Structure,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalDefinition {
    pub position: [f32; 3],
    pub yaw: f32,
    pub radius: f32,
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
            include_str!("../../content/maps/moon.toml"),
            include_str!("../../content/maps/space.toml"),
            include_str!("../../content/maps/fjords.toml"),
            include_str!("../../content/maps/icebergs.toml"),
            include_str!("../../content/maps/city.toml"),
            include_str!("../../content/maps/highlands.toml"),
        ]
        .into_iter()
        .map(|source| toml::from_str(source).expect("地图配置格式错误"))
        .collect()
    })[kind as usize]
}
