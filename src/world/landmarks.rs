use super::*;
use celestial::{Scenery, Structure};
use glam::{Vec3, vec3};

impl World {
    pub(super) fn add_landmarks(&mut self) {
        for landmark in &crate::content::maps::definition(self.kind).landmarks {
            let mut position = Vec3::from_array(landmark.position);
            position.z += self.height(position.x, position.y);
            self.place_landmark(Scenery {
                position,
                yaw: landmark.yaw,
                scale: landmark.scale,
                kind: landmark.kind,
            });
        }
        if self.kind == MapKind::City {
            for y in 4..18 {
                for x in 4..18 {
                    let n = hash((x * 131 + y * 977) as u32 ^ self.seed);
                    if n < 30 {
                        continue;
                    }
                    let (wx, wy) = (x as f32 * 192.0 + 96.0, y as f32 * 192.0 + 96.0);
                    let scale = if (x - 11_i32).abs() + (y - 11_i32).abs() < 3 {
                        2.4 + n as f32 / 340.0
                    } else {
                        0.45 + n as f32 / 160.0
                    };
                    let position = vec3(wx, wy, self.height(wx, wy));
                    self.place_landmark(Scenery {
                        position,
                        scale,
                        yaw: 0.0,
                        kind: Structure::Building,
                    });
                }
            }
        }
    }
    fn place_landmark(&mut self, object: Scenery) {
        let (rx, ry, height) = match object.kind {
            Structure::Building => (11.0, 14.0, 76.0),
            Structure::Windmill => (3.0, 3.0, 70.0),
            Structure::Lighthouse => (6.0, 6.0, 36.0),
            Structure::RockStack => (10.0, 10.0, 50.0),
            _ => {
                self.scenery.push(object);
                return;
            }
        };
        if self.obstacles.is_empty() {
            self.obstacles = vec![0; 256 * 256];
        }
        let rx = rx * object.scale;
        let ry = ry * object.scale;
        let x0 = ((object.position.x - rx) / 16.0).floor() as i32;
        let x1 = ((object.position.x + rx) / 16.0).floor() as i32;
        let y0 = ((object.position.y - ry) / 16.0).floor() as i32;
        let y1 = ((object.position.y + ry) / 16.0).floor() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let at = index(x, y, 256);
                let top = (object.position.z + height * object.scale).clamp(0.0, 254.0) as u8;
                self.obstacles[at] = self.obstacles[at].max(top);
            }
        }
        self.scenery.push(object);
    }
}
