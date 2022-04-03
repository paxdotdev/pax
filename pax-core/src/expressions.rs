use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::{PaxEngine, RenderTreeContext};
use crate::runtime::StackFrame;


use pax_runtime_api::{EasingCurve, PropertyInstance, PropertyLiteral, TransitionManager, TransitionQueueEntry};


// The `Expression` form of a property — stores a function
// that evaluates the value itself, as well as a "register" of
// the memoized value (`cached_value`) that can be referred to
// via calls to `read()`
pub struct PropertyExpression<T: Default>
{
    pub id: String,
    pub cached_value: T,
    pub transition_manager: TransitionManager<T>,
}


impl<T: Default> PropertyExpression<T> {
    pub fn new(id: String) -> Self {
        Self {
            id,
            cached_value: Default::default(),
            transition_manager: TransitionManager::new(),
        }
    }
}

impl<T: Default + Clone> PropertyInstance<T> for PropertyExpression<T> {
    fn get(&self) -> &T {
        &self.cached_value
    }

    // fn is_fresh(&self) -> bool {
    //     self.is_fresh
    // }
    //
    // fn _mark_not_fresh(&mut self) {
    //     self.is_fresh = false;
    // }
    
    fn _get_vtable_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn set(&mut self, value: T) {
        self.cached_value = value;
    }

    //TODO: when trait fields land, DRY this implementation vs. other <T: PropertyInstance> implementations
    fn ease_to(&mut self, new_value: T, duration_frames: usize, curve: EasingCurve) {
        self.transition_manager.value = Some(self.get().clone());
        &self.transition_manager.queue.clear();
        &self.transition_manager.queue.push_back(TransitionQueueEntry {
            global_frame_started: None,
            duration_frames,
            curve,
            starting_value: self.cached_value.clone(),
            ending_value: new_value
        });
    }

    fn ease_to_later(&mut self, new_value: T, duration_frames: usize, curve: EasingCurve) {
        if let None = self.transition_manager.value {
            //handle case where transition queue is empty -- a None value gets skipped, so populate it with Some
            self.transition_manager.value = Some(self.get().clone());
        }
        &self.transition_manager.queue.push_back(TransitionQueueEntry {
            global_frame_started: None,
            duration_frames,
            curve,
            starting_value: self.cached_value.clone(),
            ending_value: new_value
        });
    }

    fn _get_transition_manager(&mut self) -> Option<&mut TransitionManager<T>> {
        if let None = self.transition_manager.value {
            None
        }else {
            Some(&mut self.transition_manager)
        }
    }

}

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext<'a> {
    //TODO: add scope tree, etc.
    pub engine: &'a PaxEngine,
    pub stack_frame: Rc<RefCell<StackFrame>>,
}
