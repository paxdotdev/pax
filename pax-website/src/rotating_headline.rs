use crate::SiteTheme;
use pax_kit::math::Point2;
use pax_kit::*;

const WORDS: [&str; 4] = ["creative", "performant", "native", "portable"];
const HOLD_MS: f64 = 4200.0;
const CHANGE_MS: f64 = 900.0;
const CYCLE_MS: f64 = HOLD_MS + CHANGE_MS;
const WRITE_MS: f64 = 1600.0;

/// A fixed three-line signboard. Creative uses Handwriter's native text
/// alternative; other words use native Text. Only the current word is mounted.
#[pax]
#[file("rotating_headline.pax")]
pub struct RotatingHeadline {
    pub font_px: Property<f64>,
    pub playing: Property<bool>,
    pub word: Property<String>,
    pub word_x: Property<f64>,
    pub word_y: Property<f64>,
    pub word_scale_x: Property<f64>,
    pub word_scale_y: Property<f64>,
    pub word_rotate: Property<f64>,
    pub word_opacity: Property<f64>,
    pub writing_progress: Property<f64>,
    pub _elapsed_ms: Property<f64>,
    pub _last_ms: Property<f64>,
}

impl RotatingHeadline {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
        self.apply_pose(pose_at(0.0));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self._last_ms.get()).clamp(0.0, 50.0);
        self._last_ms.set(now);
        let origin = ctx.local_point(Point2::new(0.0, 0.0));
        let height = ctx.bounds_self.get().1;
        if -origin.y * height >= ctx.viewport.get().height || (1.0 - origin.y) * height <= 0.0 {
            return;
        }
        if self.playing.get() {
            self._elapsed_ms
                .set((self._elapsed_ms.get() + dt).rem_euclid(CYCLE_MS * 4.0));
        } else {
            // Pausing resolves a half-flipped word to a legible resting pose.
            self._elapsed_ms
                .set_if_neq(rest_time(self._elapsed_ms.get()));
        }
        let pose = pose_at(self._elapsed_ms.get());
        self.writing_progress.set_if_neq(writing_progress(
            self.writing_progress.get(),
            dt,
            pose.index == 0,
            self.playing.get(),
        ));
        self.apply_pose(pose);
    }

    fn apply_pose(&mut self, pose: WordPose) {
        self.word.set_if_neq(WORDS[pose.index].to_string());
        self.word_x.set_if_neq(pose.x);
        self.word_y.set_if_neq(pose.y);
        self.word_scale_x.set_if_neq(pose.sx * 100.0);
        self.word_scale_y.set_if_neq(pose.sy * 100.0);
        self.word_rotate.set_if_neq(pose.angle);
        self.word_opacity.set_if_neq(pose.opacity);
    }
}

fn writing_progress(previous: f64, dt: f64, creative: bool, playing: bool) -> f64 {
    if !creative {
        0.0
    } else if !playing {
        1.0
    } else {
        (previous + dt / WRITE_MS).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct WordPose {
    index: usize,
    x: f64,
    y: f64,
    sx: f64,
    sy: f64,
    angle: f64,
    opacity: f64,
}

fn rest_time(time: f64) -> f64 {
    pose_at(time).index as f64 * CYCLE_MS
}

fn pose_at(time: f64) -> WordPose {
    let time = time.rem_euclid(CYCLE_MS * 4.0);
    let index = (time / CYCLE_MS).floor() as usize;
    let phase = time % CYCLE_MS;
    let mut pose = WordPose {
        index,
        x: 0.0,
        y: 0.0,
        sx: 1.0,
        sy: 1.0,
        angle: 0.0,
        opacity: 1.0,
    };
    if phase <= HOLD_MS {
        return pose;
    }
    let t = (phase - HOLD_MS) / CHANGE_MS;
    let incoming = t >= 0.5;
    let half = if incoming { (1.0 - t) * 2.0 } else { t * 2.0 };
    let excursion = half * half * (3.0 - 2.0 * half);
    if incoming {
        pose.index = (index + 1) % WORDS.len();
    }
    let direction = if incoming { 1.0 } else { -1.0 };
    // The outgoing handoff is hidden. Creative writes in instead of fading in.
    pose.opacity = (1.0 - excursion * 1.35).clamp(0.0, 1.0);
    match index {
        0 => {
            pose.x = direction * excursion * 1.65;
        } // Express slide → performant.
        1 => {
            pose.sy = (1.0 - excursion).max(0.015);
        } // Split-flap → native.
        2 => {
            pose.y = direction * excursion * 1.15;
        } // Vertical roll → portable.
        _ => {
            if incoming {
                pose.opacity = 1.0;
            } else {
                // Portable rocks away; the next word is revealed by its strokes.
                pose.angle = direction * excursion * 12.0;
                pose.y = excursion * 0.28;
                pose.sx = 1.0 - excursion * 0.14;
                pose.sy = 1.0 - excursion * 0.14;
            }
        }
    }
    pose
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_hold_in_requested_order_and_loop() {
        for index in 0..8 {
            let pose = pose_at(index as f64 * CYCLE_MS + HOLD_MS / 2.0);
            assert_eq!(pose.index, index % 4);
            assert_eq!(
                (pose.x, pose.y, pose.sx, pose.sy, pose.opacity),
                (0.0, 0.0, 1.0, 1.0, 1.0)
            );
        }
    }

    #[test]
    fn handoffs_are_hidden_and_pause_always_is_legible() {
        for ms in (0..(CYCLE_MS * 4.0) as usize).step_by(13) {
            let time = ms as f64;
            let pose = pose_at(time);
            assert!(pose.sx > 0.0 && pose.sy > 0.0);
            assert!((0.0..=1.0).contains(&pose.opacity));
            let rest = pose_at(rest_time(time));
            assert_eq!(rest.index, pose.index);
            assert_eq!(rest.opacity, 1.0);
        }
        for index in 0..4 {
            let midpoint = index as f64 * CYCLE_MS + HOLD_MS + CHANGE_MS / 2.0;
            assert_eq!(pose_at(midpoint - 0.01).opacity, 0.0);
            if index != 3 {
                assert_eq!(pose_at(midpoint).opacity, 0.0);
            }
        }
    }

    #[test]
    fn each_change_uses_a_distinct_affine_gesture() {
        let poses: Vec<_> = (0..4)
            .map(|i| pose_at(i as f64 * CYCLE_MS + HOLD_MS + 225.0))
            .collect();
        assert!(poses[0].x < 0.0 && poses[0].y == 0.0);
        assert!(poses[1].sy < 1.0 && poses[1].x == 0.0);
        assert!(poses[2].y < 0.0 && poses[2].angle == 0.0);
        assert!(poses[3].angle < 0.0 && poses[3].sx < 1.0);
    }

    #[test]
    fn writing_restarts_each_visit_and_pause_completes_it() {
        assert_eq!(writing_progress(1.0, 16.0, false, true), 0.0);
        assert_eq!(writing_progress(0.0, 800.0, true, true), 0.5);
        assert_eq!(writing_progress(0.5, 800.0, true, true), 1.0);
        assert_eq!(writing_progress(0.2, 0.0, true, false), 1.0);
        assert_eq!(writing_progress(1.0, 16.0, true, true), 1.0);
        let incoming = pose_at(CYCLE_MS * 4.0 - 100.0);
        assert_eq!(incoming.index, 0);
        assert_eq!(
            (incoming.x, incoming.y, incoming.angle, incoming.opacity),
            (0.0, 0.0, 0.0, 1.0)
        );
    }
}
