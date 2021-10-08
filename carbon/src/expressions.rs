use std::collections::HashMap;
use crate::{CarbonEngine, Runtime, StackFrame};
use std::rc::Rc;
use std::cell::RefCell;
use std::marker::PhantomData;

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
    fn eval_in_place(&mut self, ptc: &PropertyTreeContext) -> &T; //TODO:  maybe this doesn't need to return
    fn read(&self) -> &T;
}

pub struct PropertyLiteral<T> {
    pub value: T,
}

impl<T> Property<T> for PropertyLiteral<T> {
    fn eval_in_place(&mut self, _ctx: &PropertyTreeContext) -> &T {
        &self.value
    }
    fn read(&self) -> &T {
        &self.value
    }
}

pub struct InjectionContext<'a> {
    //TODO: add scope tree, etc.
    pub engine: &'a CarbonEngine,
    pub stack_frame: Option<Rc<RefCell<StackFrame>>>,
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

pub struct PropertyTreeContext<'a> {
    pub engine: &'a CarbonEngine,
    pub runtime: Rc<RefCell<Runtime>>,
}

impl<T, E: Evaluator<T>> Property<T> for PropertyExpression<T, E> {
    fn eval_in_place(&mut self, ptc: &PropertyTreeContext) -> &T {
        //first: derive values
        //  - iterate through dependencies
        //  - parse dep string into a value; cast as PolymorphicType
        //  - future: track use of dependency in dep graph
        //then: call the evaluator, passing the derived values

        let mut dep_values : HashMap<String, PolymorphicValue> = HashMap::new();
        //
        // for (key, value) in self.dependencies.iter() {
        //
        //     //  this value needs to be evaluated from a combination of:
        //     //  - engine, for globals like current frame count
        //     //  - local component, for locals like vars and descendents
        //
        //     match value {
        //         PolymorphicType::Float => {
        //             // let val = &self.resolve_dependency(key, ptc.engine);
        //             // dep_values.insert(key.to_owned(), PolymorphicValue::Float(*val));
        //         }
        //         PolymorphicType::Integer => {
        //             panic!("Integer types not implemented for expression dependencies")
        //         }
        //         PolymorphicType::Boolean => {
        //             panic!("Boolean types not implemented for expression dependencies")
        //         }
        //     }
        // }


        let ic = InjectionContext {
            engine: ptc.engine,
            stack_frame: ptc.runtime.borrow_mut().peek_stack_frame().clone()
        };
        self.cached_value = self.evaluator.inject_and_evaluate(&ic);
        &self.cached_value
    }
    fn read(&self) -> &T {
        &self.cached_value
    }
}
