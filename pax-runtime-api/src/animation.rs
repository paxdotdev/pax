//! Animation primitives: interpolation, easing curves, transition queues, and timelines. See also: properties.ease_to and properties.ease_to_later.

use super::*;

// Represents an atomic segment of an imperative transition timeline.
//
// This segment is defined with length `duration_frames`, is interpolated according to `curve`, and settles at
// `ending_value` at the end of its interval.
pub struct TransitionQueueEntry<T> {
    // Length of this transition segment in runtime frames.
    pub duration_frames: u64,
    // Easing curve applied over the segment.
    pub curve: EasingCurve,
    // Value reached when the segment completes.
    pub ending_value: T,
}

#[cfg(debug_assertions)]
impl<T> std::fmt::Debug for TransitionQueueEntry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionQueueEntry")
            .field("duration_frames", &self.duration_frames)
            // .field("ending_value", &self.ending_value)
            .finish()
    }
}

impl<T: Interpolatable> Interpolatable for Option<T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        match &self {
            Self::Some(s) => match other {
                Self::Some(o) => Some(s.interpolate(o, t)),
                _ => None,
            },
            Self::None => None,
        }
    }
}

// Manages transition queues for property interpolation, e.g. for `ease_to_later`.
pub struct TransitionManager<T> {
    queue: VecDeque<TransitionQueueEntry<T>>,
    /// The value we are currently transitioning from
    transition_checkpoint_value: T,
    /// The time the current transition started
    origin_frames_elapsed: u64,
}

#[cfg(debug_assertions)]
impl<T> std::fmt::Debug for TransitionManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionManager")
            .field("queue", &self.queue)
            // .field("value", &self.transition_checkpoint_value)
            .finish()
    }
}

impl<T: Interpolatable> TransitionManager<T> {
    // Creates an empty transition queue starting from `value`.
    pub fn new(value: T, current_time: u64) -> Self {
        Self {
            queue: VecDeque::new(),
            transition_checkpoint_value: value,
            origin_frames_elapsed: current_time,
        }
    }

    // Appends one transition segment to the queue.
    pub fn push_transition(&mut self, transition: TransitionQueueEntry<T>) {
        self.queue.push_back(transition);
    }

    // Clears queued transitions after first checkpointing the current eased value.
    pub fn reset_transitions(&mut self, current_time: u64) {
        let eased_value = self.compute_eased_value(current_time);
        self.transition_checkpoint_value =
            eased_value.unwrap_or(self.transition_checkpoint_value.clone());
        self.queue.clear();
        self.origin_frames_elapsed = current_time;
    }

    // Computes the current eased value and advances completed transition segments.
    pub fn compute_eased_value(&mut self, frames_elapsed: u64) -> Option<T> {
        let global_fe = frames_elapsed;
        let origin_fe = &mut self.origin_frames_elapsed;
        let mut completed_transition = false;

        // Fast-forward transitions that have already completed, including
        // zero-duration transitions that should take effect immediately.
        while let Some(current_transition) = self.queue.front() {
            let local_fe = global_fe.saturating_sub(*origin_fe);
            if local_fe < current_transition.duration_frames {
                break;
            }
            let curr = self.queue.pop_front()?;
            *origin_fe += curr.duration_frames;
            self.transition_checkpoint_value = curr.ending_value;
            completed_transition = true;
        }

        let current_transition = match self.queue.front() {
            Some(current_transition) => current_transition,
            None => {
                return if completed_transition {
                    Some(self.transition_checkpoint_value.clone())
                } else {
                    None
                };
            }
        };

        let local_fe = global_fe.saturating_sub(*origin_fe);
        let progress =
            (local_fe as f64 / current_transition.duration_frames as f64).clamp(0.0, 1.0);
        let interpolated_val = current_transition.curve.interpolate(
            &self.transition_checkpoint_value,
            &current_transition.ending_value,
            progress,
        );
        Some(interpolated_val)
    }
}

/// Pre-built easing curves for use in transitions, as well as a `Custom` variant that can be used to
/// specify an arbitrary easing curve via a function `f: f64 -> f64` mapping a time on the unit interval to a multiplier on the unit interval.
pub enum EasingCurve {
    /// A linear easing curve, where the interpolated value changes at a constant rate over time.
    Linear,
    /// A hold easing curve, where the interpolated value remains constant until the end of the transition, at which point it jumps to the final value.
    Hold,
    /// A quadratic easing curve where the interpolated value starts slow and accelerates towards the end of the transition.
    InQuad,
    /// A quadratic easing curve where the interpolated value starts fast and decelerates towards the end of the transition.
    OutQuad,
    /// A quadratic easing curve where the interpolated value starts slow, accelerates towards the middle of the transition, and then decelerates towards the end of the transition.
    InOutQuad,
    /// A back easing curve where the interpolated value starts by briefly moving in the opposite direction before accelerating towards the final value.
    InBack,
    /// A back easing curve where the interpolated value overshoots the final value before settling back to it.
    OutBack,
    /// A back easing curve where the interpolated value starts by briefly moving in the opposite direction, then accelerates towards the final value, overshooting it before settling back to it.
    InOutBack,
    /// A custom easing curve defined by a user-provided function that maps a time on the unit interval to a multiplier on the unit interval.
    Custom(Box<dyn Fn(f64) -> f64>),
}

struct EasingEvaluators {}
impl EasingEvaluators {
    fn linear(t: f64) -> f64 {
        t
    }
    #[allow(dead_code)]
    fn none(t: f64) -> f64 {
        if t == 1.0 {
            1.0
        } else {
            0.0
        }
    }
    fn in_quad(t: f64) -> f64 {
        t * t
    }
    fn out_quad(t: f64) -> f64 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
    fn in_out_quad(t: f64) -> f64 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }
    fn in_back(t: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        C3 * t * t * t - C1 * t * t
    }
    fn out_back(t: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
    }

    fn in_out_back(t: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C2: f64 = C1 * 1.525;
        if t < 0.5 {
            ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
        } else {
            ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
        }
    }
}

impl EasingCurve {
    /// Interpolates between `v0` and `v1` using `t` as time on the unit interval.
    pub fn interpolate<T: Interpolatable>(&self, v0: &T, v1: &T, t: f64) -> T /*vt*/ {
        let multiplier = match self {
            EasingCurve::Linear => EasingEvaluators::linear(t),
            EasingCurve::Hold => EasingEvaluators::none(t),
            EasingCurve::InQuad => EasingEvaluators::in_quad(t),
            EasingCurve::OutQuad => EasingEvaluators::out_quad(t),
            EasingCurve::InOutQuad => EasingEvaluators::in_out_quad(t),
            EasingCurve::InBack => EasingEvaluators::in_back(t),
            EasingCurve::OutBack => EasingEvaluators::out_back(t),
            EasingCurve::InOutBack => EasingEvaluators::in_out_back(t),
            EasingCurve::Custom(evaluator) => (*evaluator)(t),
        };

        v0.interpolate(v1, multiplier)
    }
}

impl<I: Clone + 'static> ImplToFromPaxAny for std::ops::Range<I> {}
impl<T: 'static> ImplToFromPaxAny for Rc<T> {}
impl<T: Clone + 'static> ImplToFromPaxAny for Weak<T> {}
impl<T: Clone + 'static> ImplToFromPaxAny for Option<T> {}

impl<T1: Clone + 'static, T2: Clone + 'static> ImplToFromPaxAny for (T1, T2) {}

/// Marks a value as able to participate in Pax property transitions.
pub trait Interpolatable
where
    Self: Sized + Clone,
{
    /// Interpolates this value toward `other` at time `t` on the unit interval.
    ///
    /// The default implementation acts like a `None` ease: the first value is
    /// simply retained until the transition completes.
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        self.clone()
    }
}

impl<I: Interpolatable> Interpolatable for std::ops::Range<I> {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        self.start.interpolate(&_other.start, _t)..self.end.interpolate(&_other.end, _t)
    }
}
impl Interpolatable for () {}

impl<T: ?Sized + Clone> Interpolatable for HashSet<T> {}
impl<T: ?Sized + Clone> Interpolatable for VecDeque<T> {}
impl<T: ?Sized> Interpolatable for Rc<T> {}
impl<T: Interpolatable> Interpolatable for Weak<T> {}
impl<T1: Interpolatable, T2: Interpolatable> Interpolatable for (T1, T2) {}
impl<I: Interpolatable> Interpolatable for Vec<I> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        //FUTURE: could revisit the following assertion/constraint, perhaps with a "don't-care" approach to disjoint vec elements
        assert_eq!(
            self.len(),
            other.len(),
            "cannot interpolate between vecs of different lengths"
        );

        self.iter()
            .enumerate()
            .map(|(i, elem)| elem.interpolate(other.get(i).unwrap(), t))
            .collect()
    }
}

impl Interpolatable for kurbo::BezPath {}

impl Interpolatable for Instant {}

impl Interpolatable for char {}

impl Interpolatable for f64 {
    fn interpolate(&self, other: &f64, t: f64) -> f64 {
        self + (*other - self) * t
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EasingCurve, Interpolatable, Numeric, Rotation, TransitionManager, TransitionQueueEntry,
    };

    #[test]
    fn zero_duration_transition_applies_immediately() {
        let mut tm = TransitionManager::new(0.0, 0);
        tm.push_transition(TransitionQueueEntry {
            duration_frames: 0,
            curve: EasingCurve::Linear,
            ending_value: 10.0,
        });

        assert_eq!(tm.compute_eased_value(0), Some(10.0));
        assert_eq!(tm.compute_eased_value(1), None);
    }

    #[test]
    fn completed_transition_returns_final_value_when_polled_late() {
        let mut tm = TransitionManager::new(0.0, 0);
        tm.push_transition(TransitionQueueEntry {
            duration_frames: 10,
            curve: EasingCurve::Linear,
            ending_value: 10.0,
        });

        assert_eq!(tm.compute_eased_value(11), Some(10.0));
        assert_eq!(tm.compute_eased_value(12), None);
    }

    #[test]
    fn zero_duration_transition_chains_into_next_transition() {
        let mut tm = TransitionManager::new(0.0, 0);
        tm.push_transition(TransitionQueueEntry {
            duration_frames: 0,
            curve: EasingCurve::Linear,
            ending_value: 10.0,
        });
        tm.push_transition(TransitionQueueEntry {
            duration_frames: 10,
            curve: EasingCurve::Linear,
            ending_value: 20.0,
        });

        assert_eq!(tm.compute_eased_value(0), Some(10.0));
        assert_eq!(tm.compute_eased_value(5), Some(15.0));
        assert_eq!(tm.compute_eased_value(10), Some(20.0));
        assert_eq!(tm.compute_eased_value(11), None);
    }

    #[test]
    fn rotation_interpolates_linearly_in_degrees() {
        let start = Rotation::Degrees(Numeric::F64(-4.0));
        let end = Rotation::Degrees(Numeric::F64(8.0));

        let midpoint = start.interpolate(&end, 0.5);
        assert!((midpoint.get_as_degrees() - 2.0).abs() < 0.0001);
    }

    #[test]
    fn rotation_percent_normalizes_to_unit_interval() {
        let rotation = Rotation::Percent(Numeric::F64(100.0));
        assert!((rotation.to_float_0_1() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn in_out_quad_eases_symmetrically() {
        let start = 0.0f64;
        let end = 10.0f64;

        let first_quarter = EasingCurve::InOutQuad.interpolate(&start, &end, 0.25);
        let midpoint = EasingCurve::InOutQuad.interpolate(&start, &end, 0.5);
        let third_quarter = EasingCurve::InOutQuad.interpolate(&start, &end, 0.75);

        assert!((first_quarter - 1.25).abs() < 0.0001);
        assert!((midpoint - 5.0).abs() < 0.0001);
        assert!((third_quarter - 8.75).abs() < 0.0001);
    }

    #[test]
    fn option_interpolates_inner_value_when_both_sides_are_present() {
        let start = Some(0.0f64);
        let end = Some(10.0f64);

        assert_eq!(start.interpolate(&end, 0.0), Some(0.0));
        assert_eq!(start.interpolate(&end, 0.5), Some(5.0));
        assert_eq!(start.interpolate(&end, 1.0), Some(10.0));
    }
}

impl Interpolatable for bool {
    fn interpolate(&self, _other: &bool, _t: f64) -> bool {
        *self
    }
}

impl Interpolatable for usize {
    fn interpolate(&self, other: &usize, t: f64) -> usize {
        (*self as f64 + (*other - self) as f64 * t) as usize
    }
}

impl Interpolatable for isize {
    fn interpolate(&self, other: &isize, t: f64) -> isize {
        (*self as f64 + (*other - self) as f64 * t) as isize
    }
}

impl Interpolatable for i64 {
    fn interpolate(&self, other: &i64, t: f64) -> i64 {
        (*self as f64 + (*other - self) as f64 * t) as i64
    }
}

impl Interpolatable for i128 {
    fn interpolate(&self, other: &i128, t: f64) -> i128 {
        (*self as f64 + (*other - self) as f64 * t) as i128
    }
}

impl Interpolatable for u128 {
    fn interpolate(&self, other: &u128, t: f64) -> u128 {
        (*self as f64 + (*other - self) as f64 * t) as u128
    }
}

impl Interpolatable for u64 {
    fn interpolate(&self, other: &u64, t: f64) -> u64 {
        (*self as f64 + (*other - self) as f64 * t) as u64
    }
}

impl Interpolatable for u8 {
    fn interpolate(&self, other: &u8, t: f64) -> u8 {
        (*self as f64 + (*other - *self) as f64 * t) as u8
    }
}

impl Interpolatable for u16 {
    fn interpolate(&self, other: &u16, t: f64) -> u16 {
        (*self as f64 + (*other - *self) as f64 * t) as u16
    }
}

impl Interpolatable for u32 {
    fn interpolate(&self, other: &u32, t: f64) -> u32 {
        (*self as f64 + (*other - *self) as f64 * t) as u32
    }
}

impl Interpolatable for i8 {
    fn interpolate(&self, other: &i8, t: f64) -> i8 {
        (*self as f64 + (*other - *self) as f64 * t) as i8
    }
}

impl Interpolatable for i16 {
    fn interpolate(&self, other: &i16, t: f64) -> i16 {
        (*self as f64 + (*other - *self) as f64 * t) as i16
    }
}

impl Interpolatable for i32 {
    fn interpolate(&self, other: &i32, t: f64) -> i32 {
        (*self as f64 + (*other - *self) as f64 * t) as i32
    }
}

impl Interpolatable for String {}

/// Minimal timeline state used by animation-oriented controls.
pub struct Timeline {
    /// Current playhead frame.
    pub playhead_position: usize,
    /// Total number of frames.
    pub frame_count: usize,
    /// Whether the timeline is currently advancing.
    pub is_playing: bool,
}
