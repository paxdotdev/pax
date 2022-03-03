use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{PaxEngine, RenderTreeContext};
use crate::runtime::StackFrame;


use pax_runtime_api::{Property, PropertyLiteral};


// The `Expression` form of a property — stores a function
// that evaluates the value itself, as well as a "register" of
// the memoized value (`cached_value`) that can be referred to
// via calls to `read()`
pub struct PropertyExpression<T: Default>
{
    pub id: String,
    pub cached_value: T,
}
//
// impl<T, E: Fn(ExpressionContext) -> T> PropertyExpression<T, E> {
//     fn compute_in_place(&mut self, rtc: &RenderTreeContext) {
//         (*rtc.runtime).borrow().log("computing expression".into());
//         panic!("take that muthafucka");
//         let ec = ExpressionContext {
//              engine: &rtc.engine,
//              stack_frame: Rc::clone(&rtc.runtime.borrow_mut().peek_stack_frame().unwrap())
//         };
//         self.cached_value = ((self.evaluator)(ec));
//     }
// }

impl<T: Default> Property<T> for PropertyExpression<T> {
    fn get(&self) -> &T {
        &self.cached_value
    }

    fn _get_vtable_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn set(&mut self, value: T) {
        self.cached_value = value;
    }
}


//
// /// The Literal form of a Property: a bare literal value
// pub struct PropertyLiteral<T> {
//     pub value: T,
// }
//
// impl<T> Property<T> for PropertyLiteral<T> {
//     fn read(&self) -> &T {
//         &self.value
//     }
// }

/// The Timeline form of a Property
//
// trait Tweenable {
//
// }

//
// pub struct PropertyTimeline {
//     pub starting_value: Box<dyn Property<f64>>,
//     pub timeline_segments: Vec<TimelineSegment>,
//     pub cached_evaluated_value: f64,
// }

//TODO: create an Interpolatable trait that allows us
//      to ease between values beyond f64 (e.g. a discrete interpolator
// //      for ints or vecs; an interpolator for Colors)
// impl ComputableProperty for PropertyTimeline {
//
//     fn compute_in_place(&mut self, rtc: &RenderTreeContext) {
//         let timeline_playhead_position = rtc.timeline_playhead_position;
//         let mut starting_frame : usize = 0;
//         let mut starting_value = self.starting_value.get();
//         let mut segments_iter = self.timeline_segments.iter().peekable();
//         let mut active_segment = match segments_iter.next() {
//             Some(seg) => seg,
//             None => {panic!("Timeline properties must define at least one TimelineSegment.")}
//         };
//
//         //Scan through our list of timeline segments to find our active segment
//         //TODO:  this lookup could be optimized to constant-time with something like
//         //       a tree-map, or a "'''hashmap''' with ranges-of-frames for keys => pointers-to-segments for values"
//         while timeline_playhead_position > active_segment.ending_frame_inclusive
//             && segments_iter.peek().is_some()
//         {
//             starting_frame = active_segment.ending_frame_inclusive;
//             starting_value = active_segment.ending_value.get();
//             active_segment = segments_iter.next().unwrap();
//         };
//
//         // Determine how far along the playhead is between starting_frame and
//         // the current segment's ending_frame.  That ratio [0,100%] (capped)
//         // is the number to pass into our easing curve
//         let progress = (
//             (timeline_playhead_position - starting_frame) as f64
//             /
//             (active_segment.ending_frame_inclusive - starting_frame) as f64
//         ).min(1.0); //cap at 1.0 to satisfy domain expectations of easing functions [0,1]
//
//         let progress_eased = active_segment.curve_in.map(progress);
//
//         //the computed value is a function of the magnitude of difference
//         //between val_last and val_next.  Keep in mind that progress_eased is NOT
//         //bound to [0,1], because some easing curves can "hyperextend" their
//         //interpolation, e.g. a standard elastic curve.  Such hyperextension, too,
//         //is a function of the magnitude of the difference between val_last and val_next.
//         let ending_value = active_segment.ending_value.get();
//
//         self.cached_evaluated_value = starting_value + (progress_eased * (ending_value - starting_value));
//     }
//
//
// }

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext<'a> {
    //TODO: add scope tree, etc.
    pub engine: &'a PaxEngine,
    pub stack_frame: Rc<RefCell<StackFrame>>,
}


//TODO: come back & figure out implementation of expressions with the Property dependency graph split
// An abstract wrapper around a function (`inject_and_evaluate`) that can take an `InjectionContext`,
// and return a value `T` from an evaluated Expression.
// pub trait Evaluator<T> {
//     /// calls (variadic) self.evaluate and returns its value
//     fn inject_and_evaluate(&self, ic: &InjectionContext) -> T;
// }

// The `Expression` form of a property — stores a function
// that evaluates the value itself, as well as a "register" of
// the memoized value (`cached_value`) that can be referred to
// via calls to `read()`
// pub struct PropertyExpression<T, E: Evaluator<T>>
// {
//     pub evaluator: E,
//     pub cached_value: T,
// }
//
// // impl<T, E: Evaluator<T>> PropertyExpression<T, E> {}
//
// impl<T, E: Evaluator<T>> Property<T> for PropertyExpression<T, E> {
//     fn compute_in_place(&mut self, rtc: &RenderTreeContext) {
//
//         let ic = InjectionContext {
//             engine: rtc.engine,
//             stack_frame: Rc::clone(&rtc.runtime.borrow_mut().peek_stack_frame().unwrap())
//         };
//
//         self.cached_value = self.evaluator.inject_and_evaluate(&ic);
//     }
//     fn read(&self) -> &T {
//         &self.cached_value
//     }
// }

