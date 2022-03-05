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

impl<T: Default> Property<T> for PropertyExpression<T> {
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
}

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext<'a> {
    //TODO: add scope tree, etc.
    pub engine: &'a PaxEngine,
    pub stack_frame: Rc<RefCell<StackFrame>>,
}
