use pax_kit::math::Point2;
use pax_kit::*;
use std::f64::consts::FRAC_PI_2;

/// Scroll coordinates are observed, never written by decorative motion.
pub struct ScrollMotion {
    pub x: Property<f64>,
    pub y: Property<f64>,
    pub paused: Property<bool>,
}
impl Store for ScrollMotion {}

/// A passive page-level pointer sample, shared without adding a hit-test layer.
#[derive(Default)]
pub struct PointerMotion {
    pub x: f64,
    pub y: f64,
    last_ms: Option<f64>,
    speed: f64,
}
impl Store for PointerMotion {}

impl PointerMotion {
    pub fn record(&mut self, x: f64, y: f64, now: f64) {
        // Several native events can share the engine's frame timestamp. Keep
        // their displacement for the next sample instead of zeroing velocity.
        if self.last_ms.is_some_and(|last| now <= last) {
            return;
        }
        self.speed = self.last_ms.map_or(0.0, |last| {
            let dt = (now - last) / 1000.0;
            if dt > 0.2 {
                0.0
            } else {
                ((x - self.x).hypot(y - self.y) / dt).min(1800.0)
            }
        });
        self.x = x;
        self.y = y;
        self.last_ms = Some(now);
    }

    fn energy(&self, now: f64) -> f64 {
        self.last_ms.map_or(0.0, |last| {
            (self.speed / 1000.0).min(1.0) * (-(now - last).max(0.0) / 150.0).exp()
        })
    }
}

/// A reusable, non-interactive CMY underlay for a rounded rectangular surface.
/// Place after the opaque surface in Pax source order; no mask is needed.
#[pax]
#[file("chromatic_contour.pax")]
pub struct ChromaticContour {
    pub radius: Property<f64>,
    pub strength: Property<f64>,
    pub seed: Property<f64>,
    pub cyan_path: Property<Vec<PathElement>>,
    pub magenta_path: Property<Vec<PathElement>>,
    pub yellow_path: Property<Vec<PathElement>>,
    pub cyan_alpha: Property<f64>,
    pub magenta_alpha: Property<f64>,
    pub yellow_alpha: Property<f64>,
    pub cyan_width: Property<f64>,
    pub magenta_width: Property<f64>,
    pub yellow_width: Property<f64>,
    pub _scroll_x: Property<f64>,
    pub _scroll_y: Property<f64>,
    pub _paused: Property<bool>,
    pub _previous_x: Property<f64>,
    pub _previous_y: Property<f64>,
    pub _last_ms: Property<f64>,
    pub _noise_time: Property<f64>,
    pub _scratch: Property<f64>,
    pub _lag_x: Property<f64>,
    pub _lag_y: Property<f64>,
}

impl ChromaticContour {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let _ = ctx.peek_local_store(|motion: &mut ScrollMotion| self.observe_motion(motion));
        self._previous_x.set(self._scroll_x.get());
        self._previous_y.set(self._scroll_y.get());
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
        self._noise_time.set(self.seed.get() * 0.37);
    }

    fn observe_motion(&self, motion: &ScrollMotion) {
        // replace_with copies an evaluator; it does not alias a literal
        // Property. Keep an explicit dependency on the live scroll inputs.
        let x = motion.x.clone();
        let y = motion.y.clone();
        let paused = motion.paused.clone();
        self._scroll_x
            .replace_with(Property::computed(move || x.get(), &[motion.x.untyped()]));
        self._scroll_y
            .replace_with(Property::computed(move || y.get(), &[motion.y.untyped()]));
        self._paused.replace_with(Property::computed(
            move || paused.get(),
            &[motion.paused.untyped()],
        ));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        // A repeated timestamp is not a blank frame. Retain the last picture,
        // including accumulated scroll input, until the clock advances.
        let Some((elapsed, dt)) = frame_delta(self._last_ms.get(), now) else {
            return;
        };
        self._last_ms.set(now);
        let delta = (
            self._scroll_x.get() - self._previous_x.get(),
            self._scroll_y.get() - self._previous_y.get(),
        );
        self._previous_x.set(self._scroll_x.get());
        self._previous_y.set(self._scroll_y.get());
        let (width, height) = ctx.bounds_self.get();
        // local_point includes native scroller presentation offsets. Cheap
        // visibility checks avoid tessellating offscreen decorative paths.
        let origin = ctx.local_point(Point2::new(0.0, 0.0));
        let viewport = ctx.viewport.get();
        let visible = -origin.x * width < viewport.width + 100.0
            && -origin.y * height < viewport.height + 100.0
            && width - origin.x * width > -100.0
            && height - origin.y * height > -100.0;
        if self._paused.get() {
            self.cyan_alpha.set_if_neq(0.0);
            self.magenta_alpha.set_if_neq(0.0);
            self.yellow_alpha.set_if_neq(0.0);
            return;
        }
        if !visible || width <= 0.0 || height <= 0.0 {
            // Culling is not an opacity animation. Retain the last geometry
            // instead of flashing off at the visibility boundary.
            self._lag_x.set(0.0);
            self._lag_y.set(0.0);
            self._scratch.set(0.0);
            return;
        }
        // A loop recenter or a scrollbar jump must not kick the outline across
        // the page. Ordinary native momentum remains fully directional.
        let dx = if delta.0.abs() < 500.0 && elapsed < 0.5 {
            delta.0
        } else {
            0.0
        };
        let dy = if delta.1.abs() < 500.0 && elapsed < 0.5 {
            delta.1
        } else {
            0.0
        };
        let lag_x = follow(self._lag_x.get(), scroll_pull(dx, elapsed, 52.0), dt);
        let lag_y = follow(self._lag_y.get(), scroll_pull(dy, elapsed, 64.0), dt);
        self._lag_x.set(lag_x);
        self._lag_y.set(lag_y);
        let (pointer, energy) = ctx
            .peek_local_store(|motion: &mut PointerMotion| {
                let p = ctx.local_point(Point2::new(motion.x, motion.y));
                ((p.x * width, p.y * height), motion.energy(now))
            })
            .unwrap_or(((width * 0.5, height * 0.5), 0.0));
        let outside = (pointer.0 - pointer.0.clamp(0.0, width))
            .hypot(pointer.1 - pointer.1.clamp(0.0, height));
        let scratch = follow(self._scratch.get(), energy * (-outside / 100.0).exp(), dt);
        self._scratch.set(scratch);
        // Integrate phase; multiplying absolute time by pointer speed would
        // jump the waveform every time the mouse accelerated.
        let noise_time = self._noise_time.get() + dt * (1.2 + scratch * 5.0);
        self._noise_time.set(noise_time);
        for channel in 0..3 {
            let sample = contour_sample(
                width,
                height,
                self.radius.get(),
                self.strength.get(),
                self.seed.get(),
                channel,
                noise_time,
                (lag_x, lag_y),
                scratch,
                pointer,
            );
            let (path, alpha, stroke) = match channel {
                0 => (&self.cyan_path, &self.cyan_alpha, &self.cyan_width),
                1 => (&self.magenta_path, &self.magenta_alpha, &self.magenta_width),
                _ => (&self.yellow_path, &self.yellow_alpha, &self.yellow_width),
            };
            alpha.set_if_neq(sample.1);
            stroke.set_if_neq(sample.2);
            path.set(sample.0);
        }
    }
}

fn follow(current: f64, target: f64, dt: f64) -> f64 {
    current + (target - current) * (1.0 - (-8.0 * dt).exp())
}

fn scroll_pull(delta: f64, elapsed: f64, limit: f64) -> f64 {
    // The visual resists the scroll axis: down pulls up, right pulls left.
    (-delta / elapsed * 0.055).clamp(-limit, limit)
}

fn frame_delta(previous: f64, now: f64) -> Option<(f64, f64)> {
    let elapsed = (now - previous) / 1000.0;
    (elapsed > 0.0).then_some((elapsed, elapsed.min(0.05)))
}

fn noise(seed: f64) -> f64 {
    ((seed * 12.9898).sin() * 43758.5453).fract().abs()
}

// Periodic spatial white-noise samples, band-limited in space and time. No
// frame-wise RNG, resets, or discontinuity where the closed contour rejoins.
fn wave_noise(progress: f64, time: f64, seed: f64, cells: usize) -> f64 {
    let u = progress.rem_euclid(1.0) * cells as f64;
    let x = u.floor() as usize;
    let t = time.floor();
    let smooth = |v: f64| v * v * v * (v * (v * 6.0 - 15.0) + 10.0);
    let mix = |a: f64, b: f64, v: f64| a + (b - a) * v;
    let sample =
        |x: usize, t: f64| noise(seed + (x % cells) as f64 * 17.31 + t * 103.7) * 2.0 - 1.0;
    let a = mix(sample(x, t), sample(x + 1, t), smooth(u.fract()));
    let b = mix(
        sample(x, t + 1.0),
        sample(x + 1, t + 1.0),
        smooth(u.fract()),
    );
    mix(a, b, smooth(time.fract()))
}

fn contour_sample(
    w: f64,
    h: f64,
    radius: f64,
    strength: f64,
    seed: f64,
    channel: usize,
    time: f64,
    lag: (f64, f64),
    scratch: f64,
    pointer: (f64, f64),
) -> (Vec<PathElement>, f64, f64) {
    let channel = channel as f64;
    let travel = lag.0.hypot(lag.1);
    if w <= 0.0 || h <= 0.0 {
        return (vec![], 0.0, 1.0);
    }
    let intensity = (travel / 55.0).min(1.0);
    let alpha = (0.58 + intensity * 0.14 + scratch * 0.1) * (1.0 - channel * 0.08);
    let stroke = 1.05 + intensity * 0.25 + scratch * 0.15;
    let radius = radius.max(2.0).min(w.min(h) * 0.48);
    let channel_scale = 1.0 - channel * 0.24;
    let mut points = Vec::with_capacity(128);
    for index in 0..128 {
        let progress = index as f64 / 128.0;
        let (x, y) = rounded_perimeter(w, h, radius, progress);
        let normal = (
            x - x.clamp(radius, w - radius),
            y - y.clamp(radius, h - radius),
        );
        let normal_length = normal.0.hypot(normal.1);
        let (nx, ny) = (normal.0 / normal_length, normal.1 / normal_length);
        let wave = wave_noise(progress, time, seed + channel * 27.7, 64) * 0.75
            + wave_noise(progress, time * 0.61, seed + channel * 41.1 + 7.0, 16) * 0.25;
        let local_scratch =
            scratch * (0.3 + 0.7 * (-(x - pointer.0).hypot(y - pointer.1) / 160.0).exp());
        let tail = if travel > 0.001 {
            ((nx * lag.0 + ny * lag.1) / travel).max(0.0).powi(2)
        } else {
            0.0
        };
        // Stable outlines stay outside the opaque surface. Only the trailing
        // side stretches; the leading edge cannot disappear underneath it.
        let expansion = 3.0
            + (2.0 - channel) * 1.75
            + strength
                * (local_scratch * 6.0
                    + wave * (1.1 + local_scratch * 6.0 + intensity * 0.35)
                    + intensity * 5.0 * tail);
        let drift = channel_scale * strength * tail;
        points.push((
            x + nx * expansion + lag.0 * drift,
            y + ny * expansion + lag.1 * drift,
        ));
    }
    (closed_spline(&points), alpha, stroke)
}

// Arc-length sampling prevents long straight sides from pulling the adjacent
// cubic handles into loops around a tightly rounded corner.
fn rounded_perimeter(w: f64, h: f64, r: f64, progress: f64) -> (f64, f64) {
    let horizontal = w - 2.0 * r;
    let vertical = h - 2.0 * r;
    let arc = FRAC_PI_2 * r;
    let mut distance = progress * (2.0 * horizontal + 2.0 * vertical + 4.0 * arc);
    for side in 0..4 {
        let line = if side % 2 == 0 { horizontal } else { vertical };
        if distance <= line {
            return match side {
                0 => (r + distance, 0.0),
                1 => (w, r + distance),
                2 => (w - r - distance, h),
                _ => (0.0, h - r - distance),
            };
        }
        distance -= line;
        if distance <= arc {
            let angle = -FRAC_PI_2 + side as f64 * FRAC_PI_2 + distance / r;
            let (x, y) = match side {
                0 => (w - r, r),
                1 => (w - r, h - r),
                2 => (r, h - r),
                _ => (r, r),
            };
            return (x + r * angle.cos(), y + r * angle.sin());
        }
        distance -= arc;
    }
    (r, 0.0)
}

fn closed_spline(points: &[(f64, f64)]) -> Vec<PathElement> {
    let px = |x: f64| Size::Pixels(x.into());
    let mut path = vec![PathElement::Point(px(points[0].0), px(points[0].1))];
    for i in 0..points.len() {
        let a = points[(i + points.len() - 1) % points.len()];
        let b = points[i];
        let c = points[(i + 1) % points.len()];
        let d = points[(i + 2) % points.len()];
        path.push(PathElement::Cubic(
            px(b.0 + (c.0 - a.0) / 6.0),
            px(b.1 + (c.1 - a.1) / 6.0),
            px(c.0 - (d.0 - b.0) / 6.0),
            px(c.1 - (d.1 - b.1) / 6.0),
        ));
        path.push(PathElement::Point(px(c.0), px(c.1)));
    }
    path.push(PathElement::Close);
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_inputs_remain_live_after_mount() {
        let contour = ChromaticContour::default();
        let motion = ScrollMotion {
            x: Property::new(0.0),
            y: Property::new(0.0),
            paused: Property::new(false),
        };
        contour.observe_motion(&motion);
        assert_eq!(contour._scroll_y.get(), 0.0);
        motion.x.set(45.0);
        motion.y.set(900.0);
        motion.paused.set(true);
        assert_eq!(contour._scroll_x.get(), 45.0);
        assert_eq!(contour._scroll_y.get(), 900.0);
        assert!(contour._paused.get());
    }
    #[test]
    fn response_is_directional_and_frame_rate_independent() {
        assert!(scroll_pull(12.0, 0.016, 64.0) < 0.0);
        assert!(scroll_pull(-12.0, 0.016, 64.0) > 0.0);
        assert_eq!(scroll_pull(0.0, 0.016, 64.0), 0.0);
        assert!(follow(0.0, 40.0, 0.016) > 0.0);
        assert!(follow(0.0, -40.0, 0.016) < 0.0);
        let one = follow(0.0, 40.0, 0.032);
        let two = follow(follow(0.0, 40.0, 0.016), 40.0, 0.016);
        assert!((one - two).abs() < 1e-9);
    }
    #[test]
    fn all_idle_channels_are_continuously_visible_and_quiet() {
        for frame in 0..600 {
            for channel in 0..3 {
                let (path, alpha, width) = contour_sample(
                    300.0,
                    500.0,
                    32.0,
                    1.0,
                    7.0,
                    channel,
                    frame as f64 / 60.0,
                    (0.0, 0.0),
                    0.0,
                    (150.0, 250.0),
                );
                assert!((0.48..=0.58).contains(&alpha));
                assert_eq!(width, 1.05);
                assert!(matches!(path.last(), Some(PathElement::Close)));
            }
        }
    }

    #[test]
    fn waveform_has_no_temporal_or_spatial_reset_seam() {
        for channel in 0..3 {
            for frame in 0..600 {
                let time = frame as f64 / 60.0;
                let seed = 7.0 + channel as f64 * 27.7;
                assert!(
                    (wave_noise(0.0, time, seed, 64) - wave_noise(1.0, time, seed, 64)).abs()
                        < 1e-9
                );
                for i in 0..128 {
                    let p = i as f64 / 128.0;
                    assert!(
                        (wave_noise(p, time, seed, 64)
                            - wave_noise(p, time + 1.0 / 60.0, seed, 64))
                        .abs()
                            < 0.065
                    );
                }
            }
        }
    }

    #[test]
    fn stopping_and_reversing_scroll_never_blanks_the_contours() {
        let mut lag = 0.0;
        for frame in 0..240 {
            let target = match frame {
                0..60 => 60.0,
                60..120 => 0.0,
                120..180 => -60.0,
                _ => 0.0,
            };
            lag = follow(lag, target, 1.0 / 60.0);
            for channel in 0..3 {
                let (path, alpha, _) = contour_sample(
                    300.0,
                    500.0,
                    32.0,
                    1.0,
                    7.0,
                    channel,
                    frame as f64 / 60.0,
                    (0.0, lag),
                    0.0,
                    (0.0, 0.0),
                );
                assert!(alpha >= 0.4);
                assert_eq!(path.len(), 258);
            }
        }
    }

    #[test]
    fn duplicate_frames_hold_and_slow_frames_do_not_blank() {
        assert_eq!(frame_delta(100.0, 100.0), None);
        assert_eq!(frame_delta(100.0, 90.0), None);
        assert_eq!(frame_delta(100.0, 500.0), Some((0.4, 0.05)));
    }

    #[test]
    fn pointer_scratch_requires_movement_and_decays_at_rest() {
        let mut pointer = PointerMotion::default();
        pointer.record(100.0, 100.0, 1000.0);
        assert_eq!(pointer.energy(1000.0), 0.0);
        pointer.record(125.0, 100.0, 1020.0);
        assert_eq!(pointer.energy(1020.0), 1.0);
        pointer.record(140.0, 100.0, 1020.0);
        assert_eq!(pointer.energy(1020.0), 1.0);
        assert_eq!(pointer.x, 125.0);
        assert!(pointer.energy(1520.0) < 0.04);
        pointer.record(125.0, 100.0, 1540.0);
        assert_eq!(pointer.energy(1540.0), 0.0);
    }

    #[test]
    fn generated_points_stay_finite_outside_the_opaque_surface() {
        for (w, h, radius) in [(300.0, 500.0, 43.0), (350.0, 474.0, 8.0)] {
            for channel in 0..3 {
                for frame in 0..120 {
                    let time = frame as f64 / 60.0;
                    let (path, _, _) = contour_sample(
                        w,
                        h,
                        radius,
                        1.0,
                        7.0,
                        channel,
                        time,
                        (time.sin() * 52.0, time.cos() * 64.0),
                        1.0,
                        (w * 0.5, h * 0.5),
                    );
                    for element in path {
                        if let PathElement::Point(Size::Pixels(x), Size::Pixels(y)) = element {
                            let (x, y) = (x.to_float(), y.to_float());
                            assert!(x.is_finite() && y.is_finite());
                            let distance = (x - x.clamp(radius, w - radius))
                                .hypot(y - y.clamp(radius, h - radius));
                            assert!(distance > radius + 1.0);
                        }
                    }
                }
            }
        }
    }
}
