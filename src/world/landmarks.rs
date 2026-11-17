use super::*;
use celestial::{Scenery, Structure};
use glam::Vec3;

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
    }
    pub(super) fn place_landmark(&mut self, object: Scenery) {
        let (rx, ry, height) = match object.kind {
            Structure::Building => (11.0, 14.0, 91.0),
            Structure::House => (10.0, 12.0, 16.0),
            Structure::Warehouse => (20.0, 28.0, 20.0),
            Structure::IceSpire => (10.0, 10.0, 45.0),
            Structure::Boulder => (9.0, 7.0, 10.0),
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
        let (s, c) = object.yaw.sin_cos();
        let (rx, ry) = (
            (c.abs() * rx + s.abs() * ry) * object.scale,
            (s.abs() * rx + c.abs() * ry) * object.scale,
        );
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
