use super::*;
use celestial::{Scenery, Structure};
use glam::Vec3;

impl World {
    pub(super) fn add_landmarks(&mut self) {
        for landmark in &crate::content::maps::definition(self.kind).landmarks {
            let mut position = Vec3::from_array(landmark.position);
            if landmark.kind == Structure::Fuji {
                self.mount_fuji(position.x, position.y);
                continue;
            }
            position.z += if let Some(size) = landmark.kind.landmark_size() {
                self.landmark_site(position.x, position.y, size * landmark.scale)
            } else {
                self.height(position.x, position.y)
            };
            self.place_landmark(Scenery {
                position,
                yaw: landmark.yaw,
                scale: landmark.scale,
                kind: landmark.kind,
            });
        }
        let scenery = std::mem::take(&mut self.scenery);
        self.obstacles.clear();
        for object in scenery {
            self.place_landmark(object);
        }
    }
    fn landmark_site(&mut self, x: f32, y: f32, size: Vec3) -> f32 {
        let radius = size.truncate().length() + 8.0;
        let height = (self.heights[index(x as i32, y as i32, SIZE)] as f32)
            .max(6.0)
            .min(254.0 - size.z);
        self.scenery.retain(|object| {
            let delta = object.position.truncate() - glam::vec2(x, y);
            delta.length() > radius + 45.0 * object.scale
        });
        let material = match self.kind {
            MapKind::Desert => 17,
            MapKind::City => 33,
            MapKind::Icebergs => 15,
            MapKind::Canyon => 19,
            MapKind::Volcanic => 21,
            _ => 12,
        };
        let extent = (radius + 24.0).ceil() as i32;
        for dy in -extent..=extent {
            for dx in -extent..=extent {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                let blend = ((radius + 24.0 - distance) / 24.0).clamp(0.0, 1.0);
                if blend == 0.0 {
                    continue;
                }
                let at = index(x as i32 + dx, y as i32 + dy, SIZE);
                let old = self.heights[at] as f32;
                self.heights[at] = (old + (height - old) * blend).round() as u8;
                if distance <= radius {
                    self.materials[at] = material;
                }
            }
        }
        height
    }
    fn mount_fuji(&mut self, x: f32, y: f32) {
        let radius = 400;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let r = ((dx * dx + dy * dy) as f32).sqrt() / radius as f32;
                if r >= 1.0 {
                    continue;
                }
                let at = index(x as i32 + dx, y as i32 + dy, SIZE);
                let angle = (dy as f32).atan2(dx as f32);
                let cone = 42.0 + 194.0 * (1.0 - r).powf(1.25) - (1.0 - r / 0.06).max(0.0) * 12.0;
                let blend = ((1.0 - r) * 4.0).min(1.0);
                self.heights[at] = (self.heights[at] as f32 * (1.0 - blend) + cone * blend) as u8;
                if r < 0.75 {
                    self.materials[at] = if cone > 171.0 + (angle * 7.0).sin() * 9.0 {
                        15 + u8::from(dx + dy > 0)
                    } else {
                        12 + u8::from(dx + dy > 0)
                    };
                }
            }
        }
    }
    pub(super) fn place_landmark(&mut self, object: Scenery) {
        let Some(size) = object.kind.footprint() else {
            self.scenery.push(object);
            return;
        };
        let (rx, ry, height) = (size.x, size.y, size.z);
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
