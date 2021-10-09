use std::collections::HashMap;
use crate::{CarbonEngine, Runtime, StackFrame};
use std::rc::Rc;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::any::Any;

pub struct Variable {
    pub name: String,
    pub value: Box<PolymorphicValue>,
    pub access: VariableAccessLevel,
}

pub enum VariableAccessLevel {
    Public,
    Private,
}

pub enum PolymorphicValue
{
    Float(f64),
    Integer(i64),
    Boolean(bool),
}

pub enum PolymorphicType {
    Float,
    Integer,
    Boolean,
}

pub trait Property<T> {
    //either unwrap T
    //or provide a fn -> T
    fn eval_in_place(&mut self, ptc: &PropertyTreeContext) {}
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

pub struct InjectionContext<'a, D> {
    //TODO: add scope tree, etc.
    pub engine: &'a CarbonEngine<D>,
    pub stack_frame: Rc<RefCell<StackFrame<D>>>,
}

pub trait Evaluator<T, D> {
    //calls (variadic) self.evaluate and returns its value
    fn inject_and_evaluate(&self, ic: &InjectionContext<D>) -> T;
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
    pub dependencies : Vec<String>,
    pub cached_value: T,
}

impl<T, E: Evaluator<T>> PropertyExpression<T, E>
{
    //TODO:  support types other than f64
    fn resolve_dependency(&self, name: &str, engine: &CarbonEngine) -> f64 {
        // Turn a string like `"this.property_name"` or `"engine.frames_elapsed"`
        // into the appropriate underlying value.
        // TODO:  determine if there's a better, with-the-type-system way to handle this
        //        (perhaps through macros.)  Keep an eye on support for a future bolt-on JS runtime.
        match name {
            "engine.frames_elapsed" => {
                engine.frames_elapsed as f64
            }
            _ => {
                //TODO:  since this is not a hard-coded dependency,
                //       now perform dynamic evaluation
                //    1. handle `this`
                //    2. handle property access; `this.height`
                //       [do we allow endless ref loops here? and trust a pre-processor to avoid them?]
                //    3. collect ids of children, handle e.g. `rect_1`

                panic!("unsupported dependency")
            }
        }
    }
}

pub struct PropertyTreeContext<'a, D> {
    pub engine: &'a CarbonEngine<D>,
    pub runtime: Rc<RefCell<Runtime<D>>>,
}

impl<T, E: Evaluator<T>> Property<T> for PropertyExpression<T, E> {
    fn eval_in_place(&mut self, ptc: &PropertyTreeContext) {

        let ic = InjectionContext {
            engine: ptc.engine,
            stack_frame: Rc::clone(&ptc.runtime.borrow_mut().peek_stack_frame().unwrap())
        };
        self.cached_value = self.evaluator.inject_and_evaluate(&ic);
    }
    fn read(&self) -> &T {
        &self.cached_value
    }
}
