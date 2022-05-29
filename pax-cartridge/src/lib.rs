
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
use pax_core::{ComponentInstance, RenderNodePtr, PropertyExpression, RenderNodePtrList, RenderTreeContext, ExpressionContext, PaxEngine, RenderNode, InstanceRegistry, HandlerRegistry, InstantiationArgs, ConditionalInstance, SlotInstance, StackFrame};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use piet_common::RenderContext;

use pax_runtime_api::{ArgsCoproduct, SizePixels, PropertyInstance, PropertyLiteral, Size2D, Transform2D};

//generate dependencies, pointing to userland cartridge (same logic as in PropertiesCoproduct)
// use pax_example::pax_types::{Root};
// use pax_example::pax_types::pax_std::primitives::{Rectangle, Group, Text};
// use pax_example::pax_types::pax_std::types::{Color, Font, Stroke, Size, StackerCellProperties, StackerDirection};
// use pax_example::pax_types::pax_std::components::Stacker;
//
//dependency paths below come from pax_primitive macro, where these crate+module paths are passed as parameters:
// use pax_std_primitives::{RectangleInstance, GroupInstance, ScrollerInstance, FrameInstance, TextInstance};



const PLACEHOLDER_ERROR : &str = "Fatal: the placeholder cartridge is still attached -- a custom cartridge should be attached during compilation.";

pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<u64, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_component_stacker<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    unreachable!("{}", PLACEHOLDER_ERROR)
}

pub fn instantiate_root_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {
    unreachable!("{}", PLACEHOLDER_ERROR)
}