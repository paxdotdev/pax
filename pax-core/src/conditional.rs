use std::cell::RefCell;
use std::rc::Rc;

use crate::{HandlerRegistry, ComponentInstance, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, InstantiationArgs};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

/// A special "control-flow" primitive, Conditional (@if) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `@if` syntax in templates.
pub struct ConditionalInstance {
    pub primitive_children: RenderNodePtrList,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub boolean_expression: Box<dyn PropertyInstance<bool>>,
    pub empty_children: RenderNodePtrList,
}

impl RenderNode for ConditionalInstance {

    fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> where Self: Sized {

        let new_id = pax_runtime_api::mint_unique_id();
        let ret = Rc::new(RefCell::new(Self {
            primitive_children: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            boolean_expression: args.conditional_boolean_expression.expect("Conditional requires boolean_expression"),
            empty_children: Rc::new(RefCell::new(vec![]))
        }));

        (*args.instance_map).borrow_mut().insert(new_id, Rc::clone(&ret) as Rc<RefCell<dyn RenderNode>>);
        ret
    }


    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        if let Some(boolean_expression) = rtc.get_vtable_computed_value(self.boolean_expression._get_vtable_id()) {
            let new_value = if let TypesCoproduct::bool(v) = boolean_expression { v } else { unreachable!() };
            self.boolean_expression.set(new_value);
        }
    }

    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList {
        if *self.boolean_expression.get() {
            Rc::clone(&self.primitive_children)
        } else {
            Rc::clone(&self.empty_children)
        }

    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }


}
