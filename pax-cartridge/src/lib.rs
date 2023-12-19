use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

use pax_core::{ComponentInstance, ExpressionContext, InstantiationArgs, NodeRegistry};
use piet_common::RenderContext;
use std::rc::Rc;

const PLACEHOLDER_ERROR : &str = "Fatal: the development placeholder cartridge is still attached -- a defined cartridge must be attached during compilation.  This means that Pax compilation failed -- please try again with `pax build` or `pax run`.";

pub fn instantiate_expression_table(
) -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_component_stacker<R: 'static + RenderContext>(
    _node_registry: Rc<RefCell<NodeRegistry>>,
    _args: InstantiationArgs,
) -> Rc<RefCell<ComponentInstance>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_main_component(
    _node_registry: Rc<RefCell<NodeRegistry>>,
) -> Rc<ComponentInstance> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}
