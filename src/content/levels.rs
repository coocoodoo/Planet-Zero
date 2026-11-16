use crate::content::weather::Weather;
use crate::gameplay::progression::Phase;
use crate::world::MapKind;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub name: String,
    pub map: MapKind,
    pub weather: Weather,
    pub phases: [Phase; 3],
    pub durations: [f32; 3],
    pub patrol_quota: u32,
    pub strike_quota: u32,
    pub reinforcement_seconds: f32,
}

pub fn operation(level: u32) -> &'static Operation {
    static LEVELS: OnceLock<Vec<Operation>> = OnceLock::new();
    let levels = LEVELS.get_or_init(|| {
        [
            include_str!("../../content/levels/01-coast-guard.toml"),
            include_str!("../../content/levels/02-ice-convoy.toml"),
            include_str!("../../content/levels/03-canyon-raid.toml"),
            include_str!("../../content/levels/04-sand-front.toml"),
            include_str!("../../content/levels/05-marsh-rescue.toml"),
            include_str!("../../content/levels/06-caldera.toml"),
        ]
        .into_iter()
        .map(|source| toml::from_str(source).expect("关卡配置格式错误"))
        .collect()
    });
    &levels[level.saturating_sub(1) as usize % levels.len()]
}
