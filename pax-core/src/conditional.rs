use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    handle_vtable_update, recurse_expand_nodes, with_properties_unwrapped, ExpandedNode,
    InstanceNode, InstanceNodePtr, InstanceNodePtrList, InstantiationArgs, PropertiesTreeContext,
    RenderTreeContext,
};
use pax_runtime_api::{CommonProperties, Layer, PropertyInstance, Size};
use piet_common::RenderContext;

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance<R: 'static + RenderContext> {
    pub instance_id: u32,
    instance_children: InstanceNodePtrList<R>,

    instance_prototypical_properties_factory: Box<dyn FnMut() -> Rc<RefCell<dyn Any>>>,
    instance_prototypical_common_properties_factory:
        Box<dyn FnMut() -> Rc<RefCell<CommonProperties>>>,
}

///Contains the expression of a conditional, evaluated as an expression.
#[derive(Default)]
pub struct ConditionalProperties {
    pub boolean_expression: Box<dyn pax_runtime_api::PropertyInstance<bool>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ConditionalInstance<R> {
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
            instance_children: match args.children {
                None => Rc::new(RefCell::new(vec![])),
                Some(children) => children,
            },
            instance_prototypical_common_properties_factory: args
                .prototypical_common_properties_factory,
            instance_prototypical_properties_factory: args.prototypical_properties_factory,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }
    fn manages_own_subtree_for_expansion(&self) -> bool {
        true
    }
    fn expand_node_and_compute_properties(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = ExpandedNode::get_or_create_with_prototypical_properties(
            ptc,
            &(self.instance_prototypical_properties_factory)(),
            &(self.instance_prototypical_common_properties_factory)(),
        );
        let properties_wrapped = this_expanded_node.borrow().get_properties();

        // evaluate boolean expression
        let evaluated_condition = with_properties_unwrapped!(
            &properties_wrapped,
            ConditionalProperties,
            |properties: &mut ConditionalProperties| {
                handle_vtable_update!(ptc, properties.boolean_expression, bool);
                //return evaluated value
                *properties.boolean_expression.get()
            }
        );

        // recurse into instance children, stitch ExpandedNode subtree and return subtree root (this_expanded_node)
        for child in self.instance_children.borrow().iter() {
            let mut new_ptc = ptc.clone();
            new_ptc.current_expanded_node = None;
            new_ptc.current_instance_node = Rc::clone(child);
            new_ptc.current_instance_id = child.borrow().get_instance_id();

            // handle false conditional by marking for unmount; continue to recurse into subtree and compute / expand
            if !evaluated_condition {
                new_ptc.marked_for_unmount = true
            }

            let child_expanded_node = recurse_expand_nodes(&mut new_ptc);
            child_expanded_node.borrow_mut().parent_expanded_node =
                Some(Rc::downgrade(&this_expanded_node));
            this_expanded_node
                .borrow_mut()
                .append_child_expanded_node(child_expanded_node);
        }

        this_expanded_node
    }

    fn is_invisible_to_slot(&self) -> bool {
        true
    }

    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::clone(&self.instance_children)
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode<R>>,
    ) -> std::fmt::Result {
        f.debug_struct("Conditional").finish()
    }
}
