use piet_common::RenderContext;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    recurse_expand_nodes, BaseInstance, ExpandedNode, InstanceFlags, InstanceNode,
    InstanceNodePtrList, InstantiationArgs, PropertiesTreeContext,
};

use pax_runtime_api::{Layer, Timeline};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `slot_children` and
/// a `dyn Any` properties object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance<R: 'static + RenderContext> {
    pub template: InstanceNodePtrList<R>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn Fn(Rc<RefCell<dyn Any>>, &mut PropertiesTreeContext<R>)>,
    base: BaseInstance<R>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ComponentInstance<R> {
    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<Self> {
        let component_template = args.component_template.take();
        let template = component_template.unwrap_or_default();

        let compute_properties_fn = args.compute_properties_fn.take();
        let base = BaseInstance::new(
            args,
            InstanceFlags {
                invisible_to_slot: false,
                invisible_to_raycasting: true,
                layer: Layer::DontCare,
            },
        );
        let ret = Rc::new(ComponentInstance {
            base,
            template,
            compute_properties_fn: compute_properties_fn
                .expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
        });
        ret
    }

    fn expand_node_and_compute_properties(
        &self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = self.base().expand(ptc);

        let expanded_and_flattened_slot_children = {
            let slot_children = self.base().get_children();
            //Expand children in the context of the current containing component
            let mut expanded_slot_children = vec![];
            for child in slot_children {
                let mut new_ptc = ptc.clone();
                new_ptc.current_instance_node = Rc::clone(child);
                new_ptc.current_expanded_node = None;
                let child_expanded_node = recurse_expand_nodes(&mut new_ptc);
                expanded_slot_children.push(child_expanded_node);
            }

            //Now flatten those expanded children, ignoring (replacing with children) and node that`is_invisible_to_slot`, namely
            //[`ConditionalInstance`] and [`RepeatInstance`]
            let mut expanded_and_flattened_slot_children = vec![];
            for expanded_slot_child in expanded_slot_children {
                expanded_and_flattened_slot_children.extend(flatten_expanded_node_for_slot(
                    &Rc::clone(&expanded_slot_child),
                ));
            }

            expanded_and_flattened_slot_children
        };

        {
            this_expanded_node
                .borrow_mut()
                .set_expanded_and_flattened_slot_children(Some(
                    expanded_and_flattened_slot_children,
                ));
        }

        let last_containing_component = std::mem::replace(
            &mut ptc.current_containing_component,
            Rc::downgrade(&this_expanded_node),
        );

        //Compute properties
        (*self.compute_properties_fn)(
            Rc::clone(&this_expanded_node.borrow().get_properties()),
            ptc,
        );

        ptc.push_stack_frame(Rc::clone(&this_expanded_node.borrow().get_properties()));

        for child in &self.template {
            let mut new_ptc = ptc.clone();
            new_ptc.current_instance_node = Rc::clone(child);
            new_ptc.current_expanded_node = None;
            let child_expanded_node = recurse_expand_nodes(&mut new_ptc);
            child_expanded_node.borrow_mut().parent_expanded_node =
                Rc::downgrade(&this_expanded_node);
            this_expanded_node
                .borrow_mut()
                .append_child_expanded_node(child_expanded_node);
        }

        ptc.pop_stack_frame();
        ptc.current_containing_component = last_containing_component;
        this_expanded_node
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode<R>>,
    ) -> std::fmt::Result {
        f.debug_struct("Component").finish()
    }

    fn base(&self) -> &BaseInstance<R> {
        &self.base
    }
}

/// Given some InstanceNodePtrList, distill away all "slot-invisible" nodes (namely, `if` and `for`)
/// and return another InstanceNodePtrList with a flattened top-level list of nodes.
/// Helper function that accepts a
fn flatten_expanded_node_for_slot<R: 'static + RenderContext>(
    node: &Rc<RefCell<ExpandedNode<R>>>,
) -> Vec<Rc<RefCell<ExpandedNode<R>>>> {
    let mut result = vec![];

    let is_invisible_to_slot = {
        let node_borrowed = node.borrow();
        let instance_node_borrowed = Rc::clone(&node_borrowed.instance_node);
        instance_node_borrowed.base().flags().invisible_to_slot
    };
    if is_invisible_to_slot {
        // If the node is invisible, recurse on its children
        for child in node.borrow().get_children_expanded_nodes().iter() {
            result.extend(flatten_expanded_node_for_slot(child));
        }
    } else {
        // If the node is visible, add it to the result
        result.push(Rc::clone(node));
    }

    result
}
