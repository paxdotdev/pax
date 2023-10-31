use std::cell::RefCell;
use std::collections::HashMap;

use pax_core::pax_properties_coproduct::TypesCoproduct;
use pax_core::{ComponentInstance, ExpressionContext, NodeRegistry, InstantiationArgs};
use piet_common::RenderContext;
use std::rc::Rc;

const PLACEHOLDER_ERROR : &str = "Fatal: the development placeholder cartridge is still attached -- a defined cartridge must be attached during compilation.  This means that Pax compilation failed -- please try again with `pax build` or `pax run`.";

pub fn instantiate_expression_table<R: 'static + RenderContext>(
) -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_component_stacker<R: 'static + RenderContext>(
    _node_registry: Rc<RefCell<NodeRegistry<R>>>,
    _args: InstantiationArgs<R>,
) -> Rc<RefCell<ComponentInstance<R>>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_main_component<R: 'static + RenderContext>(
    _node_registry: Rc<RefCell<NodeRegistry<R>>>,
) -> Rc<RefCell<ComponentInstance<R>>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}
