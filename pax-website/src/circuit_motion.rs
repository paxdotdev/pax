use std::f64::consts::{FRAC_PI_2, PI, TAU};

const EXTENT: f64 = 20.0;
const PERIOD_MS: f64 = 10_000.0;

#[derive(Clone, Copy, Debug)]
enum Playback {
    Idle {
        center: f64,
        phase: f64,
    },
    Returning {
        from: f64,
        target: f64,
        elapsed: f64,
        duration: f64,
        endpoint: f64,
    },
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    owner: Option<i64>,
    origin: (f64, f64),
    angle: f64,
    degrees_per_pixel: f64,
    horizontal: bool,
}

/// Keeps unwrapped angles so release never unwinds completed drag revolutions.
#[derive(Clone, Debug)]
pub struct Motion {
    pub angle: f64,
    playback: Playback,
    drag: Option<Drag>,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            angle: 0.0,
            playback: Playback::Idle {
                center: 0.0,
                phase: FRAC_PI_2,
            },
            drag: None,
        }
    }
}

impl Motion {
    pub fn advance(&mut self, dt: f64) {
        if self.drag.is_some() {
            return;
        }
        match self.playback {
            Playback::Idle { center, phase } => {
                let phase = (phase + dt * TAU / PERIOD_MS).rem_euclid(TAU);
                self.angle = center + EXTENT * phase.cos();
                self.playback = Playback::Idle { center, phase };
            }
            Playback::Returning {
                from,
                target,
                elapsed,
                duration,
                endpoint,
            } => {
                let elapsed = elapsed + dt;
                let t = (elapsed / duration).min(1.0);
                self.angle = from + (target - from) * (1.0 - (PI * t).cos()) * 0.5;
                if elapsed >= duration {
                    // Both curves have zero velocity at the shared endpoint.
                    self.playback = Playback::Idle {
                        center: target - endpoint,
                        phase: if endpoint > 0.0 { 0.0 } else { PI },
                    };
                    self.advance(elapsed - duration);
                } else {
                    self.playback = Playback::Returning {
                        from,
                        target,
                        elapsed,
                        duration,
                        endpoint,
                    };
                }
            }
        }
    }

    pub fn begin(&mut self, owner: Option<i64>, point: (f64, f64), width: f64) -> bool {
        if self.drag.is_some() {
            return false;
        }
        self.drag = Some(Drag {
            owner,
            origin: point,
            angle: self.angle,
            degrees_per_pixel: 360.0 / width.max(1.0),
            horizontal: owner.is_none(),
        });
        true
    }

    pub fn move_to(&mut self, owner: Option<i64>, point: (f64, f64)) -> bool {
        let Some(mut drag) = self.drag.filter(|drag| drag.owner == owner) else {
            return false;
        };
        let dx = point.0 - drag.origin.0;
        let dy = point.1 - drag.origin.1;
        if !drag.horizontal {
            if dx.hypot(dy) < 6.0 {
                return false;
            }
            if dy.abs() > dx.abs() {
                // Let vertical touch intent remain a page scroll.
                self.end(owner);
                return false;
            }
            drag.horizontal = true;
            self.drag = Some(drag);
        }
        self.angle = drag.angle + dx * drag.degrees_per_pixel;
        true
    }

    pub fn end(&mut self, owner: Option<i64>) -> bool {
        if !self.drag.is_some_and(|drag| drag.owner == owner) {
            return false;
        }
        self.drag = None;
        let positive = EXTENT + 360.0 * ((self.angle - EXTENT) / 360.0).round();
        let negative = -EXTENT + 360.0 * ((self.angle + EXTENT) / 360.0).round();
        let (target, endpoint) = if (positive - self.angle).abs() <= (negative - self.angle).abs() {
            (positive, EXTENT)
        } else {
            (negative, -EXTENT)
        };
        self.playback = Playback::Returning {
            from: self.angle,
            target,
            elapsed: 0.0,
            duration: (450.0 + (target - self.angle).abs() * 2.5).min(850.0),
            endpoint,
        };
        true
    }

    pub fn cancel(&mut self) -> bool {
        if let Some(drag) = self.drag {
            return self.end(drag.owner);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
    }

    #[test]
    fn idle_is_centered_face_on_and_spans_forty_degrees() {
        let mut motion = Motion::default();
        near(motion.angle, 0.0);
        motion.advance(PERIOD_MS / 4.0);
        near(motion.angle, -20.0);
        motion.advance(PERIOD_MS / 2.0);
        near(motion.angle, 20.0);
    }

    #[test]
    fn release_selects_nearest_equivalent_endpoint_without_unwinding() {
        for (angle, target) in [
            (10.0, 20.0),
            (-10.0, -20.0),
            (179.0, 20.0),
            (181.0, 340.0),
            (400.0, 380.0),
            (-800.0, -740.0),
        ] {
            let mut motion = Motion::default();
            motion.begin(None, (0.0, 0.0), 360.0);
            motion.move_to(None, (angle, 0.0));
            motion.advance(2000.0);
            near(motion.angle, angle);
            motion.end(None);
            let Playback::Returning { duration, .. } = motion.playback else {
                panic!()
            };
            motion.advance(duration);
            near(motion.angle, target);
            motion.advance(PERIOD_MS / 4.0);
            near(motion.angle, (target / 360.0).round() * 360.0);
        }
    }

    #[test]
    fn regrab_samples_the_current_return_pose() {
        let mut motion = Motion::default();
        motion.begin(None, (0.0, 0.0), 360.0);
        motion.move_to(None, (200.0, 0.0));
        motion.end(None);
        motion.advance(200.0);
        let sampled = motion.angle;
        assert!(sampled > 200.0 && sampled < 340.0);
        motion.begin(None, (50.0, 0.0), 360.0);
        motion.move_to(None, (60.0, 0.0));
        near(motion.angle, sampled + 10.0);
        motion.cancel();
        motion.advance(1000.0);
        assert!(motion.angle > 340.0 && motion.angle < 360.0);
    }

    #[test]
    fn touch_ownership_and_vertical_scroll_do_not_rotate_the_board() {
        let mut motion = Motion::default();
        assert!(motion.begin(Some(7), (0.0, 0.0), 360.0));
        assert!(!motion.begin(Some(8), (0.0, 0.0), 360.0));
        assert!(!motion.move_to(Some(8), (150.0, 0.0)));
        assert!(!motion.end(Some(8)));
        assert!(!motion.move_to(Some(7), (1.0, 2.0)));
        assert!(!motion.move_to(Some(7), (2.0, 20.0)));
        near(motion.angle, 0.0);
        assert!(!motion.move_to(Some(7), (80.0, 20.0)));
        assert!(motion.begin(Some(9), (0.0, 0.0), 360.0));
        assert!(motion.move_to(Some(9), (120.0, 2.0)));
        near(motion.angle, 120.0);
        assert!(motion.end(Some(9)));
        assert!(!motion.end(Some(9)));
    }
}
