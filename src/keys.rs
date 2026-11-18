pub const ESC: u16 = 1;
pub const FIRE: u16 = 2;
pub const WEAPON: u16 = 3;
pub const BOMB: u16 = 4;
pub const OVERDRIVE: u16 = 5;
pub const AUTOFIRE: u16 = 6;
pub const WEATHER: u16 = 2;
pub const TIME: u16 = 3;
pub const PORTAL: u16 = 4;
pub const VEHICLE: u16 = 5;
pub const WINGMEN: u16 = 6;
pub const MODEL: u16 = 7;
pub const AUTOPILOT: u16 = 8;
pub const DEMO: u16 = 9;
pub const MAP: u16 = 10;
pub const SEED: u16 = 11;
pub const RETRY: u16 = 19;
pub const HUD: u16 = 22;
pub const IMU: u16 = 23;
pub const PAUSE: u16 = 25;
pub const ENTER: u16 = 28;
pub const ROLL_LEFT: u16 = 16;
pub const ROLL_RIGHT: u16 = 18;
pub const LOOK_UP: u16 = 17;
pub const LOOK_LEFT: u16 = 30;
pub const LOOK_DOWN: u16 = 31;
pub const LOOK_RIGHT: u16 = 32;
pub const HOVER: u16 = 109;
pub const CALIBRATE: u16 = 37;
pub const BOOST: u16 = 42;
pub const VIEW: u16 = 57;
pub const CHART: u16 = 50;
pub const BOOST_RIGHT: u16 = 54;
pub const HELP: u16 = 59;
pub const SETTINGS: u16 = 60;
pub const SOUND: u16 = 68;
pub const UP: u16 = 103;
pub const LEFT: u16 = 105;
pub const RIGHT: u16 = 106;
pub const DOWN: u16 = 108;

pub fn cardputer_direction(code: u16) -> u16 {
    match code {
        33 => UP,
        45 => DOWN,
        44 => LEFT,
        46 => RIGHT,
        _ => code,
    }
}
