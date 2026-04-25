use pax_engine::api::Property;
use pax_engine::pax;
use pax_runtime::api::{borrow, Layer};
use pax_runtime::{
    bind_content_measurement_effect, resolve_axis_autosize, sync_content_autosize_with_axes,
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::iter;
use std::rc::Rc;

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[primitive("pax_std::core::group::GroupInstance")]
pub struct Group {
    /// Automatically sizes the group to its direct content children when possible.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,
}

impl Default for Group {
    fn default() -> Self {
        Self {
            autosize: Property::new(false),
            autosize_x: Property::new(None),
            autosize_y: Property::new(None),
        }
    }
}

// Runtime instance backing `<Group>`.
pub struct GroupInstance {
    base: BaseInstance,
}

impl InstanceNode for GroupInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&pax_runtime::ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_g: &mut Group| f.debug_struct("Group").finish()),
            None => f.debug_struct("Group").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let ctx = expanded_node.get_node_context(context);
        let (autosize, autosize_x, autosize_y) =
            expanded_node.with_properties_unwrapped(|group: &mut Group| {
                (
                    group.autosize.clone(),
                    group.autosize_x.clone(),
                    group.autosize_y.clone(),
                )
            });
        let deps = [
            autosize.untyped(),
            autosize_x.untyped(),
            autosize_y.untyped(),
        ];
        bind_content_measurement_effect(
            expanded_node,
            &ctx,
            "group autosize",
            &deps,
            move |node, node_ctx| {
                sync_content_autosize_with_axes(
                    node,
                    node_ctx,
                    resolve_axis_autosize(autosize.get(), autosize_x.get(), true),
                    resolve_axis_autosize(autosize.get(), autosize_y.get(), true),
                );
            },
        );
    }

    fn handle_control_flow_node_expansion(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // Detached sidecar trees are not mounted, but mask sources still need a
        // fully expanded subtree so their descendants can contribute coverage.
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let children = expanded_node.generate_children(
            children_with_envs,
            context,
            &expanded_node.parent_frame,
            true,
        );
        for child in children.iter() {
            child.recurse_control_flow_expansion(context);
        }
        expanded_node.children.set(children);
    }
}
