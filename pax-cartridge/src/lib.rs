use std::cell::RefCell;
use std::collections::HashMap;

use nohash_hasher::BuildNoHashHasher;
use pax_manifest::PaxManifest;
use pax_runtime::{ComponentInstance, ExpressionContext, InstantiationArgs};
use pax_runtime_api::pax_value::PaxAny;
use piet_common::RenderContext;
use std::rc::Rc;

const PLACEHOLDER_ERROR : &str = "Fatal: the development placeholder cartridge is still attached -- a defined cartridge must be attached during compilation.  This means that Pax compilation failed -- please try again with `pax build` or `pax run`.";
pub const INITIAL_MANIFEST: &str = "<manifest placeholder>";

pub fn instantiate_expression_table(
) -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>, BuildNoHashHasher<usize>> {
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
    pub fn new(_manifest: PaxManifest) -> Self {
        unreachable!("{}", PLACEHOLDER_ERROR)
    }

    pub fn get_main_component(&mut self) -> Rc<ComponentInstance> {
        unreachable!("{}", PLACEHOLDER_ERROR)
    }

    pub fn get_manifest(&self) -> &pax_manifest::PaxManifest {
        unreachable!("{}", PLACEHOLDER_ERROR)
    }
}
