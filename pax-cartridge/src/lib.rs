use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

use pax_runtime::{ComponentInstance, ExpressionContext, InstantiationArgs};
use pax_runtime_api::pax_value::PaxValue;
use piet_common::RenderContext;
use std::rc::Rc;

const PLACEHOLDER_ERROR : &str = "Fatal: the development placeholder cartridge is still attached -- a defined cartridge must be attached during compilation.  This means that Pax compilation failed -- please try again with `pax build` or `pax run`.";

pub fn instantiate_expression_table() -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxValue>>
{
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_component_stacker<R: 'static + RenderContext>(
    _args: InstantiationArgs,
) -> Rc<RefCell<ComponentInstance>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_main_component() -> Rc<ComponentInstance> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub struct DefinitionToInstanceTraverser {}

impl DefinitionToInstanceTraverser {
    pub fn new() -> Self {
        unreachable!("{}", PLACEHOLDER_ERROR)
    }

    pub fn get_main_component(&mut self) -> Rc<ComponentInstance> {
        unreachable!("{}", PLACEHOLDER_ERROR)
    }

    pub fn get_manifest(&self) -> &pax_manifest::PaxManifest {
        unreachable!("{}", PLACEHOLDER_ERROR)
    }
}
