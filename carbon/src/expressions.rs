use std::cell::RefCell;
use std::rc::Rc;

use crate::{CarbonEngine, RenderTreeContext};
use crate::runtime::{StackFrame};

/// An abstract PropertyValue that may be either Literal or
/// a dynamic runtime Expression, or a Timeline-bound value
pub trait PropertyValue<T> {
    //either unwrap T
    //or provide a fn -> T
    fn compute_in_place(&mut self, _rtc: &RenderTreeContext) {}
    fn read(&self) -> &T;
}

/// The Literal form of a Property: a bare literal value
pub struct PropertyValueLiteral<T> {
    pub value: T,
}

impl<T> PropertyValue<T> for PropertyValueLiteral<T> {
    fn read(&self) -> &T {
        &self.value
    }
}


/// The Timeline form of a PropertyValue
pub struct PropertyValueTimeline {
    pub starting_value: Box<dyn PropertyValue<f64>>,
    pub timeline_segments: Vec<TimelineSegment>,
    pub cached_evaluated_value: f64,
}

pub struct TimelineSegment {
    pub curve_in: Box<dyn Easible>,
    pub ending_value: Box<dyn PropertyValue<f64>>,
    pub ending_frame_inclusive: usize,
}

// Author's note:  zb deliberated between "Easable," "Easeable", and "Easible".
// Given that none of the above could be found in written reference, zb decided
// on Easible for 'able to be eased' c.f. 'audible', 'able to be heard'
pub trait Easible {
    /// Map the domain x [0,1] to the range y [all f64]
    fn project(&self, x: f64) -> f64;
}

pub struct LinearEasingCurve {}
impl Easible for LinearEasingCurve {
    fn project(&self, x: f64) -> f64 {
        x
    }
}

pub struct EasingCurve {}
impl EasingCurve {
    pub fn linear() -> Box<dyn Easible> {
        Box::new(LinearEasingCurve {})
    }
}


//TODO: create an Interpolatable trait that allows us
//      to ease between values beyond f64 (e.g. a discrete interpolator
//      for ints or vecs; an interpolator for Colors)
impl PropertyValue<f64> for PropertyValueTimeline {

    fn compute_in_place(&mut self, rtc: &RenderTreeContext) {
        let timeline_playhead_position = rtc.timeline_playhead_position;
        let mut starting_frame : usize = 0;
        let mut starting_value = self.starting_value.read();
        let mut segments_iter = self.timeline_segments.iter().peekable();
        let mut active_segment = match segments_iter.next() {
            Some(seg) => seg,
            None => {panic!("Timeline properties must define at least one TimelineSegment.")}
        };

        //Scan through our list of timeline segments to find our active segment
        //TODO:  this lookup could be optimized to constant-time with something like
        //       a tree-map, or a "hashmap with ranges for keys => pointers-to-segments for values"
        while timeline_playhead_position > active_segment.ending_frame_inclusive
            && segments_iter.peek().is_some()
        {
            starting_frame = active_segment.ending_frame_inclusive;
            starting_value = active_segment.ending_value.read();
            active_segment = segments_iter.next().unwrap();
        };

        // Determine how far along the playhead is between starting_frame and
        // the current segment's ending_frame.  That ratio [0,100%] (capped)
        // is the number to pass into our easing curve
        let progress = (
            (active_segment.ending_frame_inclusive - starting_frame) as f64
            /
            (timeline_playhead_position - starting_frame) as f64
        ).min(1.0); //cap at 1.0 to satisfy domain expectations of easing functions [0,1]

        let progress_eased = active_segment.curve_in.project(progress);

        //the computed value is a function of the magnitude of difference
        //between val_last and val_next.  Keep in mind that progress_eased is NOT
        //bound to [0,1], because some easing curves can "hyperextend" their
        //interpolation, e.g. a standard elastic curve.  Such hyperextension, too,
        //is a function of the magnitude of the difference between val_last and val_next.
        let ending_value = active_segment.ending_value.read();
        self.cached_evaluated_value = starting_value + (progress_eased * (ending_value - starting_value));
    }


    fn read(&self) -> &f64 {
        //TODO:
        //  [x] pass in frame t
        //  [ ] evaluate in the context of t

        &self.cached_evaluated_value
        // unimplemented!()

        // &self.starting_value.read()
    }
}






/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct InjectionContext<'a> {
    //TODO: add scope tree, etc.
    pub engine: &'a CarbonEngine,
    pub stack_frame: Rc<RefCell<StackFrame>>,
}

/// An abstract wrapper around a function (`inject_and_evaluate`) that can take an `InjectionContext`,
/// and return a value `T` from an evaluated Expression.
pub trait Evaluator<T> {
    /// calls (variadic) self.evaluate and returns its value
    fn inject_and_evaluate(&self, ic: &InjectionContext) -> T;
}

/// The `Expression` form of a property — stores a function
/// that evaluates the value itself, as well as a "register" of
/// the memoized value (`cached_value`) that can be referred to
/// via calls to `read()`
pub struct PropertyValueExpression<T, E: Evaluator<T>>
{
    pub evaluator: E,
    pub cached_value: T,
}

impl<T, E: Evaluator<T>> PropertyValueExpression<T, E>
{

}

impl<T, E: Evaluator<T>> PropertyValue<T> for PropertyValueExpression<T, E> {
    fn compute_in_place(&mut self, rtc: &RenderTreeContext) {

        let ic = InjectionContext {
            engine: rtc.engine,
            stack_frame: Rc::clone(&rtc.runtime.borrow_mut().peek_stack_frame().unwrap())
        };
        self.cached_value = self.evaluator.inject_and_evaluate(&ic);
    }
    fn read(&self) -> &T {
        &self.cached_value
    }
}
