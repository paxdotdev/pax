use std::collections::BTreeSet;
use std::rc::Rc;
use std::{cell::RefCell, iter};

use crate::{
    BaseInstance, ExpandedNode, ExpressionTable, Globals, InstanceFlags, InstanceNode,
    InstanceNodePtrList, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::{Layer, Timeline};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `slot_children` and
/// a `dyn Any` properties object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance {
    pub template: InstanceNodePtrList,
    pub slot_children: BTreeSet<Rc<ExpandedNode>>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn Fn(&ExpandedNode, &ExpressionTable, &Globals)>,
    base: BaseInstance,
}

impl InstanceNode for ComponentInstance {
    fn instantiate(mut args: InstantiationArgs) -> Rc<Self> {
        let component_template = args.component_template.take();
        let template = component_template.unwrap_or_default();

        let compute_properties_fn = args.compute_properties_fn.take();
        let base = BaseInstance::new(
            args,
            InstanceFlags {
                invisible_to_slot: false,
                invisible_to_raycasting: true,
                layer: Layer::DontCare,
                is_component: true,
            },
        );
        Rc::new(ComponentInstance {
            base,
            template,
            compute_properties_fn: compute_properties_fn
                .expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
            slot_children: BTreeSet::new(),
        })
    }

    fn recompute_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &mut RuntimeContext,
    ) {
        // Expand slot children tree from the perspective of of this components
        // container component, using the environment of this current components
        // location. TODO make sure this is correct, and hook into this tree
        // in slot. Also make sure the update method below correctly updates
        // the tree.
        // self.slot_children =

        //change to expand children instead of self.template?
        let new_env = expanded_node.stack.push(&expanded_node.properties);
        let children_with_envs = self.template.iter().cloned().zip(iter::repeat(new_env));
        expanded_node.set_children(children_with_envs, context);
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        //Compute properties
        (*self.compute_properties_fn)(
            &expanded_node,
            context.expression_table(),
            context.globals(),
        );

        // TODO slot_children needs to be updated WITHOUT letting most nodes
        // send events such as mount/dismount and native patch updates for
        // changes within the tree fire. However, some nodes (the ones that
        // are) attached to slots SHOULD fire events. We still need to update
        // the entire tree however, so only firing updates for the slots
        // doesn't work either. Current idea for a solution: Introduce an
        // "attached" flag on ExpandedNodes that are set to true recursively
        // when attach_children is called on ExpandedNode (this also fires
        // mount events). For nodes whom attached = false, doing set_children
        // on them does't fire mount events and doing updates doesn't send
        // native_patches. This allows for tree updates on detatched trees,
        // without firing mount/dissmount or other updates.

        // for child in self.slot_children.iter() {
        //     child.update(context);
        // }
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Component").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
