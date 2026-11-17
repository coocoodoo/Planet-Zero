use super::*;
use crate::world::celestial::{Scenery, Structure};
impl Renderer {
    pub(super) fn settlement(&mut self, camera: &Camera, object: Scenery) {
        let (s, c) = object.yaw.sin_cos();
        let local = |p: Vec3| {
            object.position + vec3(c * p.x - s * p.y, s * p.x + c * p.y, p.z) * object.scale
        };
        let house = object.kind == Structure::House;
        let (w, l, h) = if house {
            (10.0, 12.0, 9.0)
        } else {
            (20.0, 28.0, 14.0)
        };
        let variant = hash((object.position.x as u32).wrapping_add(object.position.y as u32));
        let walls = if variant < 85 {
            rgb(224, 210, 174)
        } else if variant < 170 {
            rgb(202, 202, 189)
        } else {
            rgb(189, 153, 120)
        };
        self.box_mesh(
            camera,
            &local,
            vec3(0.0, 0.0, h * 0.5),
            vec3(w, l, h * 0.5),
            walls,
        );
        let roof = if house {
            rgb(151, 76, 56)
        } else {
            rgb(99, 126, 135)
        };
        let p = [
            vec3(-w - 1.0, -l - 1.0, h),
            vec3(w + 1.0, -l - 1.0, h),
            vec3(0.0, -l - 1.0, h + 7.0),
            vec3(-w - 1.0, l + 1.0, h),
            vec3(w + 1.0, l + 1.0, h),
            vec3(0.0, l + 1.0, h + 7.0),
        ]
        .map(local);
        for face in [
            [0, 1, 2],
            [3, 5, 4],
            [0, 2, 5],
            [0, 5, 3],
            [2, 1, 4],
            [2, 4, 5],
        ] {
            self.triangle(camera, face.map(|i| p[i]), roof);
        }
        let Some((_, _, scale)) = camera.project(object.position) else {
            return;
        };
        if scale * object.scale < 0.25 {
            return;
        }
        self.box_mesh(
            camera,
            &local,
            vec3(0.0, -l - 0.2, 3.0),
            vec3(if house { 2.0 } else { 7.0 }, 0.2, 3.0),
            rgb(59, 76, 87),
        );
        for side in [-1.0, 1.0] {
            self.box_mesh(
                camera,
                &local,
                vec3(side * w * 0.6, -l - 0.3, 6.0),
                vec3(1.7, 0.2, 1.7),
                rgb(159, 214, 224),
            );
            if house {
                self.world_line(
                    camera,
                    local(vec3(side * (w + 5.0), -l - 6.0, 2.0)),
                    local(vec3(side * (w + 5.0), l + 5.0, 2.0)),
                    rgb(221, 210, 173),
                );
            }
        }
        if house && variant > 128 {
            self.box_mesh(
                camera,
                &local,
                vec3(4.0, 5.0, h + 5.0),
                vec3(1.5, 1.7, 5.0),
                rgb(146, 131, 111),
            );
        }
    }
}
