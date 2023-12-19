use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    handle_vtable_update, with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PropertiesTreeContext,
};
use pax_runtime_api::Layer;
use piet_common::RenderContext;

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance<R: 'static + RenderContext> {
    base: BaseInstance<R>,
}

///Contains the expression of a conditional, evaluated as an expression.
#[derive(Default)]
pub struct ConditionalProperties {
    pub boolean_expression: Box<dyn pax_runtime_api::PropertyInstance<bool>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ConditionalInstance<R> {
    fn instantiate(args: InstantiationArgs<R>) -> Rc<Self>
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

    fn expand_node_and_compute_properties(
        &self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = self.base().expand(ptc);

        ptc.current_expanded_node = Some(Rc::clone(&this_expanded_node));

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

        for conditional_child in self.base().get_children() {
            let mut new_ptc = ptc.clone();
            new_ptc.current_expanded_node = None;
            new_ptc.current_instance_node = Rc::clone(conditional_child);

            let expanded_child = crate::recurse_expand_nodes(&mut new_ptc);

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
        _expanded_node: Option<&ExpandedNode<R>>,
    ) -> std::fmt::Result {
        f.debug_struct("Conditional").finish()
    }

    fn base(&self) -> &BaseInstance<R> {
        &self.base
    }
}
