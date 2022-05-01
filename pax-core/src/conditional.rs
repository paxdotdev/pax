use std::cell::RefCell;
use std::rc::Rc;

use piet_common::RenderContext;
use crate::{HandlerRegistry, ComponentInstance, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, InstantiationArgs};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance<R: 'static + RenderContext> {
    pub instance_id: u64,
    pub primitive_children: RenderNodePtrList<R>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub boolean_expression: Box<dyn PropertyInstance<bool>>,
    pub empty_children: RenderNodePtrList<R>,
}

impl<R: 'static + RenderContext> RenderNode<R> for ConditionalInstance<R> {
    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            primitive_children: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            boolean_expression: args.conditional_boolean_expression.expect("Conditional requires boolean_expression"),
            empty_children: Rc::new(RefCell::new(vec![]))
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        if let Some(boolean_expression) = rtc.compute_vtable_value(self.boolean_expression._get_vtable_id()) {
            let old_value = *self.boolean_expression.get();
            let new_value = if let TypesCoproduct::bool(v) = boolean_expression { v } else { unreachable!() };

            if old_value != new_value {
                // if new_value {
                //     //mount event
                //     // - mark subtree mounted
                //
                //     (*self.primitive_children).borrow_mut().iter().for_each(|child| {
                //         (*(*child)).borrow_mut().recurse_set_mounted(rtc, mounted);
                //     })
                //
                //
                //     //   lab journal: can we recycle the id?  that is: remove elements (recursively) from mounted_set on dismount, but keep the instance in the instance_map.
                //     //   When turning back on subtree, visit all nodes and remount (+ register existing ID back into mounted set)
                // } else {
                //     //dismount event
                //     // - mark subtree dismounted
                //     // - don't fire lifecycle event here -- remove from set, let engine::`recurse_...` handle checking instance_registry and firing event
                //     (*rtc.engine.instance_registry).borrow_mut().mark_unmounted(
                //         (*self.primitive_children).borrow().get_instance_id()
                //     )
                // }

                //mark subtree with the appropriate `mount` status

                (*self.primitive_children).borrow_mut().iter().for_each(|child| {
                    (*(*child)).borrow_mut().recurse_set_mounted(rtc, new_value);
                })
            }
            self.boolean_expression.set(new_value);
        }
    }

    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
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
