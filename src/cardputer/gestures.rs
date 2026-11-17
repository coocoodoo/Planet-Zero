#[derive(Default)]
pub(super) struct Gestures {
    held: f32,
    direction: f32,
    cooldown: f32,
}
impl Gestures {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn update(&mut self, dt: f32, tilt: f32) -> Option<f32> {
        self.cooldown = (self.cooldown - dt).max(0.0);
        if tilt.abs() < 0.78 {
            self.held = 0.0;
            self.direction = 0.0;
            return None;
        }
        if tilt.signum() != self.direction {
            self.held = 0.0;
            self.direction = tilt.signum();
        }
        self.held += dt;
        if self.held >= 0.8 && self.cooldown <= 0.0 {
            self.held = 0.0;
            self.cooldown = 1.8;
            Some(self.direction)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sustained_tilt_rolls_in_each_direction_without_gesture_chatter() {
        let mut g = Gestures::default();
        for _ in 0..60 {
            assert_eq!(g.update(1.0 / 60.0, 0.5), None);
        }
        let mut rolls = vec![];
        for _ in 0..180 {
            if let Some(d) = g.update(1.0 / 60.0, -0.95) {
                rolls.push(d);
            }
        }
        assert_eq!(rolls, vec![-1.0, -1.0]);
        g.reset();
        for _ in 0..10 {
            assert_eq!(g.update(0.05, 0.9), None);
        }
        for _ in 0..10 {
            assert_eq!(g.update(0.05, -0.9), None);
        }
        g.reset();
        assert!(
            (0..60)
                .filter_map(|_| g.update(1.0 / 60.0, 1.0))
                .any(|d| d == 1.0)
        );
    }
}
