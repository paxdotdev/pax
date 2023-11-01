use std::cell::RefCell;
use std::rc::Rc;

use crate::{InstantiationArgs, InstanceNode, InstanceNodePtr, InstanceNodePtrList, RenderTreeContext, ExpandedNode, PropertiesTreeContext};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{CommonProperties, Layer, PropertyInstance, Size};
use piet_common::RenderContext;

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance {
    pub instance_id: u32,

    // pub boolean_expression: Box<dyn PropertyInstance<bool>>,
    // pub true_branch_children: InstanceNodePtrList<R>,
    // pub false_branch_children: InstanceNodePtrList<R>,
    // pub cleanup_children: InstanceNodePtrList<R>,

    instance_prototypical_properties: Rc<RefCell<PropertiesCoproduct>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ConditionalInstance {
    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let mut node_registry = (*args.node_registry).borrow_mut();
        let instance_id = node_registry.mint_instance_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            // true_branch_children: match args.children {
            //     None => Rc::new(RefCell::new(vec![])),
            //     Some(children) => children,
            // },

            instance_prototypical_common_properties: Rc::new(RefCell::new(args.common_properties)),
            instance_prototypical_properties: Rc::new(RefCell::new(args.properties)),

            // boolean_expression: args
            //     .conditional_boolean_expression
            //     .expect("Conditional requires boolean_expression"),
            // false_branch_children: Rc::new(RefCell::new(vec![])),
            // cleanup_children: Rc::new(RefCell::new(vec![])),
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn handle_compute_properties(&mut self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {
        // if let Some(boolean_expression) =
        //     ptc.compute_vtable_value(self.boolean_expression._get_vtable_id())
        // {
        //     let old_value = *self.boolean_expression.get();
        //     let new_value = if let TypesCoproduct::bool(v) = boolean_expression {
        //         v
        //     } else {
        //         unreachable!()
        //     };
        //
        //     let mut node_registry = (*ptc.engine.node_registry).borrow_mut();
        //     if old_value && !new_value {
        //         (*self.true_branch_children)
        //             .borrow_mut()
        //             .iter()
        //             .for_each(|child| {
        //                 let instance_id = (*(*child)).borrow_mut().get_instance_id();
        //                 node_registry.deregister(instance_id);
        //                 node_registry.mark_for_unmount(instance_id);
        //             });
        //         self.cleanup_children = self.true_branch_children.clone();
        //     }
        //     self.boolean_expression.set(new_value);
        // }

        todo!()
    }

    fn is_invisible_to_slot(&self) -> bool {
        true
    }
    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        // if *self.boolean_expression.get() {
        //     Rc::clone(&self.true_branch_children)
        // } else {
        //     Rc::clone(&self.false_branch_children)
        // }
        todo!()
    }
    // fn get_size(&self) -> Option<(Size, Size)> {
    //     None
    // }
    // fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
    //     bounds
    // }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
