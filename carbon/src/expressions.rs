use std::cell::RefCell;
use std::rc::Rc;

use crate::{CarbonEngine, RenderTreeContext};
use crate::runtime::{StackFrame};

/// An abstract Property that may be either Literal or
/// a dynamic runtime Expression
pub trait Property<T> {
    //either unwrap T
    //or provide a fn -> T
    fn compute_in_place(&mut self, _rtc: &RenderTreeContext) {}
    fn read(&self) -> &T;
}

/// The Literal form of a property: simply a value
pub struct PropertyLiteral<T> {
    pub value: T,
}

impl<T> Property<T> for PropertyLiteral<T> {
    fn read(&self) -> &T {
        &self.value
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
pub struct PropertyExpression<T, E: Evaluator<T>>
{
    pub evaluator: E,
    pub cached_value: T,
}

impl<T, E: Evaluator<T>> PropertyExpression<T, E>
{

}

impl<T, E: Evaluator<T>> Property<T> for PropertyExpression<T, E> {
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
