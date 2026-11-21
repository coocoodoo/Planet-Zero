use super::*;
use crate::world::portal::Portal;

#[derive(Clone, Copy)]
pub struct PortalTransit {
    pub entry: Portal,
    pub elapsed: f32,
    pub overhead: bool,
    wing_entered: [bool; 2],
}
impl PortalTransit {
    pub fn matches(&self, portal: Portal) -> bool {
        self.entry.center.distance_squared(portal.center) < 0.01
    }
    pub fn expansion(&self) -> f32 {
        if self.overhead {
            crate::world::orbit::smooth(self.elapsed / 0.75)
        } else {
            0.0
        }
    }
    fn completed(&self, camera: &Camera) -> bool {
        if self.overhead {
            self.elapsed >= 0.75
        } else {
            let (eye, _) = camera.ray(camera.center);
            (self.entry.center - eye).dot(camera.forward) <= 0.14 / camera.q.max(0.00001)
        }
    }
}

impl Flight {
    pub(crate) fn hides_portal(&self, portal: Portal) -> bool {
        (self.portal_cooldown > 0.0 || self.portal_exit_pending)
            && self
                .portal_exit
                .is_some_and(|exit| exit.center.distance_squared(portal.center) < 0.01)
    }

    pub(super) fn update_portal_visibility(&mut self) {
        if !self.portal_exit_pending {
            return;
        }
        let camera = Camera::new(self, 320, 170);
        let visible = self.portal_exit.is_some_and(|exit| {
            camera
                .project(exit.center)
                .is_some_and(|(center, _, scale)| {
                    let radius =
                        (exit.radius * scale).max(1.0) * crate::world::portal::LENS_RADIUS + 2.0;
                    center.x + radius >= 0.0
                        && center.x - radius <= 320.0
                        && center.y + radius >= 0.0
                        && center.y - radius <= 170.0
                })
        });
        self.portal_exit_pending = visible;
        if !visible {
            self.portal_exit = None;
        }
    }

    pub fn approach_portal(&mut self, portal: Portal, previous: Vec3) -> Option<Portal> {
        if !self.can_warp() || self.hides_portal(portal) {
            return None;
        }
        let camera = Camera::new(self, 320, 170);
        if self.portal_transit.is_none() && camera.enters_portal(portal, previous, self.position) {
            self.portal_transit = Some(PortalTransit {
                entry: camera.portal_frame(portal),
                elapsed: 0.0,
                overhead: self.weights[0] > 0.5,
                wing_entered: [false; 2],
            });
        }
        let transit = self.portal_transit.as_mut().filter(|t| t.matches(portal))?;
        for (entered, wing) in transit.wing_entered.iter_mut().zip(&self.wingmen) {
            let offset = wing.position - transit.entry.center;
            let depth = offset.dot(transit.entry.normal());
            let radial = offset - transit.entry.normal() * depth;
            *entered |= depth < 0.0 && radial.length() < portal.radius * 0.82;
        }
        transit.completed(&camera).then_some(transit.entry)
    }
    pub fn traverse_portal(&mut self, world: &World, previous: Vec3) -> bool {
        if world.portals.len() != 2 || self.portal_cooldown > 0.0 {
            return false;
        }
        for (i, entry) in world.portals.iter().enumerate() {
            if let Some(entry) = self.approach_portal(*entry, previous) {
                let exit = world.portals[1 - i];
                self.warp(entry, exit);
                return true;
            }
        }
        false
    }

    pub fn warp(
        &mut self,
        entry: crate::world::portal::Portal,
        exit: crate::world::portal::Portal,
    ) {
        let camera = Camera::new(self, 320, 170);
        let rotation = entry.rotation_to(exit);
        let overhead = self
            .portal_transit
            .map_or(self.weights[0] > 0.5, |t| t.overhead);
        self.position = entry.transform(exit, self.position);
        self.follow = entry.transform(exit, self.follow);
        self.yaw += rotation;
        self.space_attitude = self
            .space_attitude
            .map(|q| glam::Quat::from_rotation_z(rotation) * q);
        self.camera_yaw -= rotation;
        self.camera_offset -= rotation * 0.4;
        for trail in &mut self.trails {
            *trail = trail.map(|point| entry.transform(exit, point));
        }
        for (i, wing) in self.wingmen.iter_mut().enumerate() {
            let entered = overhead
                || self.portal_transit.is_some_and(|t| t.wing_entered[i])
                || camera
                    .project(entry.center)
                    .zip(camera.project(wing.position))
                    .is_some_and(|((center, _, scale), (point, _, _))| {
                        point.distance(center) < entry.radius * scale * 0.82
                            && wing.position.distance(entry.center) < entry.radius * 1.6
                    });
            if entered {
                wing.position = entry.transform(exit, wing.position);
                wing.follow = entry.transform(exit, wing.follow);
                for trail in &mut wing.trails {
                    *trail = trail.map(|point| entry.transform(exit, point));
                }
            } else {
                wing.portal_wait = 0.45 + i as f32 * 0.3;
                wing.rejoining = true;
                wing.position =
                    exit.center + exit.right() * (i as f32 * 2.0 - 1.0) * exit.radius * 0.25;
                wing.follow = wing.position;
                wing.trails.clear();
            }
            wing.yaw += rotation;
            wing.space_attitude = wing
                .space_attitude
                .map(|q| glam::Quat::from_rotation_z(rotation) * q);
            wing.camera_yaw -= rotation;
        }
        self.portal_exit = Some(exit);
        self.portal_exit_pending = overhead;
        self.portal_cooldown = 0.8;
        self.portal_transit = None;
        self.warp_flash = 0.08;
    }
}
