use std::{collections::{HashSet, VecDeque}, rc::{Rc, Weak}};

pub trait Interpolatable
where
    Self: Sized + Clone,
{
    //default implementation acts like a `None` ease — that is,
    //the first value is simply returned.
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        self.clone()
    }
}

pub struct TransitionManager<T: Interpolatable> {
    queue: VecDeque<TransitionQueueEntry<T>>,
    /// The value we are currently transitioning from
    transition_checkpoint_value: T,
    /// The time the current transition started
    origin_frames_elapsed: u64,
}

#[cfg(debug_assertions)]
impl<T: Interpolatable> std::fmt::Debug for TransitionManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionManager")
            .field("queue", &self.queue)
            // .field("value", &self.transition_checkpoint_value)
            .finish()
    }
}

impl<T: Interpolatable> TransitionManager<T> {
    pub fn new(value: T, current_time: u64) -> Self {
        Self {
            queue: VecDeque::new(),
            transition_checkpoint_value: value,
            origin_frames_elapsed: current_time,
        }
    }

    pub fn push_transition(&mut self, transition: TransitionQueueEntry<T>) {
        self.queue.push_back(transition);
    }

    pub fn reset_transitions(&mut self, current_time: u64) {
        // update current value as to ease from this position
        self.compute_eased_value(current_time);
        self.queue.clear();
        self.origin_frames_elapsed = current_time;
    }

    pub fn compute_eased_value(&mut self, frames_elapsed: u64) -> Option<T> {
        let global_fe = frames_elapsed;
        let origin_fe = &mut self.origin_frames_elapsed;

        // Fast-forward transitions that have already passed
        while global_fe - *origin_fe > self.queue.front()?.duration_frames {
            let curr = self.queue.pop_front()?;
            *origin_fe += curr.duration_frames;
            self.transition_checkpoint_value = curr.ending_value;
        }
        let current_transition = self.queue.front()?;
        let local_fe = global_fe - *origin_fe;
        let progress = local_fe as f64 / current_transition.duration_frames as f64;
        let interpolated_val = current_transition.curve.interpolate(
            &self.transition_checkpoint_value,
            &current_transition.ending_value,
            progress,
        );
        Some(interpolated_val)
    }

    pub fn is_finished(&self) -> bool {
        self.queue.is_empty()
    }
}

pub enum EasingCurve {
    Linear,
    InQuad,
    OutQuad,
    InBack,
    OutBack,
    InOutBack,
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
    //for a time on the unit interval `t ∈ [0,1]`, given a value `t`,
    // find the interpolated value `vt` between `v0` and `v1` given the self-contained easing curve
    pub fn interpolate<T: Interpolatable>(&self, v0: &T, v1: &T, t: f64) -> T /*vt*/ {
        let multiplier = match self {
            EasingCurve::Linear => EasingEvaluators::linear(t),
            EasingCurve::InQuad => EasingEvaluators::in_quad(t),
            EasingCurve::OutQuad => EasingEvaluators::out_quad(t),
            EasingCurve::InBack => EasingEvaluators::in_back(t),
            EasingCurve::OutBack => EasingEvaluators::out_back(t),
            EasingCurve::InOutBack => EasingEvaluators::in_out_back(t),
            EasingCurve::Custom(evaluator) => (*evaluator)(t),
        };

        v0.interpolate(v1, multiplier)
    }
}

pub struct TransitionQueueEntry<T> {
    pub duration_frames: u64,
    pub curve: EasingCurve,
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



impl<I: Interpolatable> Interpolatable for std::ops::Range<I> {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        self.start.interpolate(&_other.start, _t)..self.end.interpolate(&_other.end, _t)
    }
}
impl Interpolatable for () {}

impl<T: ?Sized + Clone> Interpolatable for HashSet<T> {}
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

impl Interpolatable for char {}

impl Interpolatable for f64 {
    fn interpolate(&self, other: &f64, t: f64) -> f64 {
        self + (*other - self) * t
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