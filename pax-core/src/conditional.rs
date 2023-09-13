use std::cell::RefCell;
use std::rc::Rc;

use piet_common::RenderContext;
use crate::{RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, InstantiationArgs};
use pax_runtime_api::{Layer, PropertyInstance, Size2D, Transform2D};
use pax_properties_coproduct::{TypesCoproduct};

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance<R: 'static + RenderContext> {
    pub instance_id: u32,

    pub boolean_expression: Box<dyn PropertyInstance<bool>>,
    pub true_branch_children: RenderNodePtrList<R>,
    pub false_branch_children: RenderNodePtrList<R>,
    pub cleanup_children: RenderNodePtrList<R>,

    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
}

impl<R: 'static + RenderContext> RenderNode<R> for ConditionalInstance<R> {

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            true_branch_children: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            boolean_expression: args.conditional_boolean_expression.expect("Conditional requires boolean_expression"),
            false_branch_children: Rc::new(RefCell::new(vec![])),
            cleanup_children:  Rc::new(RefCell::new(vec![])),
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {

        if let Some(boolean_expression) = rtc.compute_vtable_value(self.boolean_expression._get_vtable_id()) {
            let old_value = *self.boolean_expression.get();
            let new_value = if let TypesCoproduct::bool(v) = boolean_expression { v } else { unreachable!() };

            let mut instance_registry = (*rtc.engine.instance_registry).borrow_mut();
            if old_value && !new_value {
                (*self.true_branch_children).borrow_mut().iter().for_each(|child| {
                    let instance_id = (*(*child)).borrow_mut().get_instance_id();
                    instance_registry.deregister(instance_id);
                    instance_registry.mark_for_unmount(instance_id);
                });
                self.cleanup_children = self.true_branch_children.clone();
            }
            self.boolean_expression.set(new_value);
        }
    }

    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        if *self.boolean_expression.get() {
            Rc::clone(&self.true_branch_children)
        } else {
            Rc::clone(&self.false_branch_children)
        }
    }
    fn pop_cleanup_children(&mut self) -> RenderNodePtrList<R> {
        let ret = self.cleanup_children.clone();
        self.cleanup_children = Rc::new(RefCell::new(vec![]));
        ret
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }

}
