use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{CarbonEngine, Runtime, StackFrame, RenderTreeContext};

pub trait Property<T> {
    //either unwrap T
    //or provide a fn -> T
    fn eval_in_place(&mut self, rtc: &RenderTreeContext) {}
    fn read(&self) -> &T;
}

pub struct PropertyLiteral<T> {
    pub value: T,
}

impl<T> Property<T> for PropertyLiteral<T> {
    fn read(&self) -> &T {
        &self.value
    }
}

pub struct InjectionContext<'a> {
    //TODO: add scope tree, etc.
    pub engine: &'a CarbonEngine,
    pub stack_frame: Rc<RefCell<StackFrame>>,
}

pub trait Evaluator<T> {
    //calls (variadic) self.evaluate and returns its value
    fn inject_and_evaluate(&self, ic: &InjectionContext) -> T;
}



//TODO:  can we genericize the signature of the FnMut?
//          1. it should always return `T`
//          2. it should support dynamic, variadic signatures
//       See: https://github.com/rust-lang/rfcs/issues/376
//          If not through vanilla generics, this might be achievable through a macro?
//       Given the lack of variadic support (at time of authoring,) YES a macro
//       seems to be the only viable approach.  For PoC, proceeding with a "hand-unrolled"
//       PoC with the aim to "roll" that logic into a macro
pub struct PropertyExpression<T, E: Evaluator<T>>
{
    pub evaluator: E,
    pub cached_value: T,
}

impl<T, E: Evaluator<T>> PropertyExpression<T, E>
{

}

impl<T, E: Evaluator<T>> Property<T> for PropertyExpression<T, E> {
    fn eval_in_place(&mut self, rtc: &RenderTreeContext) {

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
