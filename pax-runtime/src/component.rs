use std::cell::Ref;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Interpolatable, PaxValue, Property, ToPaxValue, Variable,
};

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
                is_slot: false,
            },
        );
        Rc::new(ComponentInstance {
            base,
            template,
            timeline: None,
        })
    }

    fn handle_setup_slot_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        if let Some(containing_component) = expanded_node.containing_component.upgrade() {
            let env = if let Some(stack_frame) =
                ScrollPosition::create_builtin_if_exists(borrow!(expanded_node.properties_scope))
            {
                expanded_node.stack.push(stack_frame)
            } else {
                Rc::clone(&expanded_node.stack)
            };
            let children = borrow!(self.base().get_instance_children());
            let children_with_env = children.iter().cloned().zip(iter::repeat(env));
            let new_slot_children = containing_component.create_children_detached(
                children_with_env,
                context,
                &Rc::downgrade(expanded_node),
            );
            *borrow_mut!(expanded_node.expanded_slot_children) = Some(new_slot_children);
        }
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let mut properties_scope = borrow_mut!(expanded_node.properties_scope);
        properties_scope.insert(
            "$suspended".to_string(),
            Variable::new_from_typed_property(expanded_node.suspended.clone()),
        );
        let new_env = expanded_node.stack.push(properties_scope.clone());
        let children = borrow!(self.template);
        let children_with_envs = children.iter().cloned().zip(iter::repeat(new_env));
        expanded_node.children.replace_with(Property::new_with_name(
            expanded_node.generate_children(
                children_with_envs,
                context,
                &expanded_node.parent_frame,
                true,
            ),
            &format!("component (node id: {})", expanded_node.id.0),
        ));
    }

    /// Updates the expanded node, recomputing its properties and possibly updating its children
    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        expanded_node.compute_flattened_slot_children();
        expanded_node
            .expanded_and_flattened_slot_children
            .read(|slot_children| {
                for child in slot_children {
                    child.recurse_update(context);
                }
            });
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        if let Some(slot_children) = borrow_mut!(expanded_node.expanded_slot_children).take() {
            for slot_child in slot_children {
                slot_child.recurse_unmount(context);
            }
        }
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

#[derive(Debug, Clone, Default)]
pub struct ScrollPosition {
    pub x: f64,
    pub y: f64,
}

impl Interpolatable for ScrollPosition {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        ScrollPosition {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

impl ToPaxValue for ScrollPosition {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("x".to_string(), self.x.to_pax_value()),
                ("y".to_string(), self.y.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ScrollPosition {
    pub fn create_builtin_if_exists(
        property_scope: Ref<HashMap<String, Variable>>,
    ) -> Option<HashMap<String, Variable>> {
        let scroll_pos_x: Property<f64> = Property::new_from_untyped(
            property_scope
                .get("scroll_pos_x")?
                .get_untyped_property()
                .clone(),
        );
        let scroll_pos_y: Property<f64> = Property::new_from_untyped(
            property_scope
                .get("scroll_pos_y")?
                .get_untyped_property()
                .clone(),
        );
        let deps = [scroll_pos_x.untyped(), scroll_pos_y.untyped()];
        let scroll_position = Property::computed(
            move || ScrollPosition {
                x: scroll_pos_x.get(),
                y: scroll_pos_y.get(),
            },
            &deps,
        );

        let scroll_position_var = Variable::new_from_typed_property(scroll_position);
        let stack_frame = vec![("$scroll_position".to_string(), scroll_position_var)]
            .into_iter()
            .collect();
        Some(stack_frame)
    }
}
