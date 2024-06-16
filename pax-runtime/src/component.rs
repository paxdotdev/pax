use std::iter;
use std::rc::Rc;

use pax_runtime_api::{borrow, borrow_mut, use_RefCell, Property};
use rustc_hash::FxHashMap;

use_RefCell!();
use crate::api::{Layer, Timeline};
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstanceNodePtrList,
    InstantiationArgs, RuntimeContext,
};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `slot_children` and
/// a `PaxType` properties object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance {
    pub template: InstanceNodePtrList,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
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
            timeline: None,
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        if let Some(containing_component) = expanded_node.containing_component.upgrade() {
            let env = Rc::clone(&expanded_node.stack);
            let children = borrow!(self.base().get_instance_children());
            let children_with_env = children.iter().cloned().zip(iter::repeat(env));
            *borrow_mut!(expanded_node.expanded_slot_children) =
                Some(containing_component.create_children_detached(
                    children_with_env,
                    context,
                    &Rc::downgrade(expanded_node),
                ));
        }
        let properties_scope = borrow!(expanded_node.properties_scope);
        let new_env = expanded_node.stack.push(
            properties_scope.clone(),
            &*borrow!(expanded_node.properties),
        );
        let children = borrow!(self.template);
        let children_with_envs = children.iter().cloned().zip(iter::repeat(new_env));
        let new_children = expanded_node.generate_children(children_with_envs, context);
        expanded_node.attach_children(new_children, context);
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        if let Some(slot_children) = borrow_mut!(expanded_node.expanded_slot_children).take() {
            for slot_child in slot_children {
                slot_child.recurse_unmount(context);
            }
        }
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        if let Some(slot_children) = borrow_mut!(expanded_node.expanded_slot_children).as_ref() {
            for slot_child in slot_children {
                slot_child.recurse_update(context);
            }
        }
        expanded_node.compute_flattened_slot_children();
    }

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
