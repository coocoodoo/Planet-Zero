use super::*;

pub(super) fn shot(time: f32) -> usize {
    let phase = time.rem_euclid(96.0);
    match phase {
        14.0..24.0 | 58.0..68.0 => 0,
        36.0..44.0 => 2,
        80.0..86.0 => 3,
        _ => 1,
    }
}

impl Flight {
    pub(super) fn director_camera(&mut self, dt: f32) {
        let shot = if self.demo_target.is_some() {
            1
        } else {
            shot(self.demo_time)
        };
        if shot == 3 && self.director_shot != 3 {
            let [right, forward, _] = self.axes();
            self.director_anchor = self.position + forward * 260.0 + right * 165.0 + Vec3::Z * 80.0;
        }
        self.director_shot = shot;
        self.view = if shot == 3 { 1 } else { shot };
        self.fixed_weight += (f32::from(shot == 3) - self.fixed_weight) * ease(1.4, dt);
    }
}
