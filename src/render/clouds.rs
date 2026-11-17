use super::*;

impl Renderer {
    pub(super) fn draw_clouds(
        &mut self,
        world: &World,
        flight: &Flight,
        camera: &Camera,
        theme: usize,
    ) {
        if world.kind.vacuum() || crate::content::maps::definition(world.kind).cloud_clusters == 0 {
            return;
        }
        let far = flight.weather.visibility(camera.position.z).1;
        let base = world.cloud_height(camera.position.x, camera.position.y);
        let upper = flight.weather.shade(grade(rgb(240, 248, 255), theme));
        let lower = flight.weather.shade(grade(rgb(153, 180, 214), theme));
        for x in 0..self.w {
            for y in self.visible[x].0..self.visible[x].1 {
                let (origin, ray) = camera.ray(vec2(x as f32, y as f32));
                if ray.z.abs() < 0.0001 {
                    continue;
                }
                let mut t = (base + 4.0 - origin.z) / ray.z;
                for _ in 0..2 {
                    let p = origin + ray * t;
                    t = (world.cloud_height(p.x, p.y) + 4.0 - origin.z) / ray.z;
                }
                let p = origin + ray * t;
                let depth = (p - camera.position).dot(camera.forward);
                let at = x * self.h + y as usize;
                if 1.0 + camera.q * depth < 0.12
                    || depth > far
                    || depth >= self.depth[at]
                    || self.clip_depth.get(at).is_some_and(|d| depth < *d)
                {
                    continue;
                }
                let cloud = world.cloud(p.x, p.y, flight.time);
                if cloud == 0 || (cloud == 1 && (x + y as usize).is_multiple_of(2)) {
                    continue;
                }
                let ceiling = world.height(p.x, p.y);
                if p.z - ceiling < 12.0 {
                    continue;
                }
                self.pixels[at] = if ray.z < 0.0 { upper } else { lower };
                self.depth[at] = depth;
            }
        }
    }
}
