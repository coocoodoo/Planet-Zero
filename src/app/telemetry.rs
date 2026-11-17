#[derive(Clone, Copy, Debug, Default)]
pub struct Telemetry {
    pub battery_percent: Option<u8>,
    pub battery_voltage: Option<f32>,
    pub charging: bool,
    pub wifi_dbm: Option<i16>,
}
