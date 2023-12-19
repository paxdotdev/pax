use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    handle_vtable_update, with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PropertiesTreeContext,
};
use pax_runtime_api::Layer;

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance {
    base: BaseInstance,
}

///Contains the expression of a conditional, evaluated as an expression.
#[derive(Default)]
pub struct ConditionalProperties {
    pub boolean_expression: Box<dyn pax_runtime_api::PropertyInstance<bool>>,
}

impl InstanceNode for ConditionalInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: true,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                },
            ),
        })
    }

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<RefCell<ExpandedNode>> {
        let this_expanded_node = self
            .base()
            .expand_from_instance(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);

        let properties_wrapped = this_expanded_node.borrow().get_properties();
        // evaluate boolean expression
        let evaluated_condition = with_properties_unwrapped!(
            &properties_wrapped,
            ConditionalProperties,
            |properties: &mut ConditionalProperties| {
                handle_vtable_update!(ptc, this_expanded_node, properties.boolean_expression, bool);
                //return evaluated value
                *properties.boolean_expression.get()
            }
        );

        let id_chain = ptc.get_id_chain(self.base().get_instance_id());

        //TODO use this to do not do re-computations each frame
        let _present_last_frame = ptc.engine.node_registry.borrow().is_mounted(&id_chain);

        if !evaluated_condition {
            for cen in this_expanded_node.borrow().get_children_expanded_nodes() {
                ptc.engine
                    .node_registry
                    .borrow_mut()
                    .mark_for_unmount(cen.borrow().id_chain.clone());
            }

            {
                this_expanded_node.borrow_mut().clear_child_expanded_nodes();
            }
        }

        for child in self.base().get_children() {
            let mut new_ptc = ptc.clone();
            let expanded_child = Rc::clone(child).expand(&mut new_ptc);
            expanded_child.borrow_mut().parent_expanded_node = Rc::downgrade(&this_expanded_node);

            if evaluated_condition {
                new_ptc
                    .engine
                    .node_registry
                    .borrow_mut()
                    .revert_mark_for_unmount(&expanded_child.borrow().id_chain);
                this_expanded_node
                    .borrow_mut()
                    .append_child_expanded_node(expanded_child);
            }
        }
        this_expanded_node
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Conditional").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
