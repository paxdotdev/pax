use std::rc::Rc;
use std::{cell::RefCell, iter};

use crate::api::{Layer, Timeline};
use crate::{
    BaseInstance, ExpandedNode, ExpressionTable, Globals, InstanceFlags, InstanceNode,
    InstanceNodePtrList, InstantiationArgs, RuntimeContext,
};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `slot_children` and
/// a `dyn Any` properties object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance {
    pub template: InstanceNodePtrList,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn Fn(&ExpandedNode, &ExpressionTable, &Globals)>,
    base: BaseInstance,
}

// #[derive(Default)]
// pub struct ComponentProperties {
//     pub slot_children: BTreeSet<Rc<ExpandedNode>>,
// }

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
        })
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        // Compute properties
        (*self.compute_properties_fn)(
            &expanded_node,
            context.expression_table(),
            context.globals(),
        );

        // Update slot children. Needs to be done since a change in
        // a repeat can trigger changes in slot references.
        if let Some(slot_children) = expanded_node.expanded_slot_children.borrow().as_ref() {
            for slot_child in slot_children {
                slot_child.recurse_update(context);
            }
        }

        expanded_node.compute_flattened_slot_children();
    }

    fn handle_mount(&self, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        if let Some(containing_component) = expanded_node.containing_component.upgrade() {
            let env = Rc::clone(&expanded_node.stack);
            let children = self.base().get_instance_children().borrow();
            let children_with_env = children.iter().cloned().zip(iter::repeat(env));
            *expanded_node.expanded_slot_children.borrow_mut() =
                Some(containing_component.create_children_detached(children_with_env, context));
        }
        let property_names = expanded_node.instance_node.borrow().base().property_names.clone().unwrap_or_default();
        let new_env = expanded_node.stack.push(property_names, &expanded_node.properties.borrow());
        let children = self.template.borrow();
        let children_with_envs = children.iter().cloned().zip(iter::repeat(new_env));
        expanded_node.set_children(children_with_envs, context);
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

    fn get_template(&self) -> Option<&InstanceNodePtrList> {
        Some(&self.template)
    }
}
