use crate::common::{native_surface_opacity, patch_if_needed};
use pax_engine::api::Property;
use pax_engine::pax;
use pax_message::{AnyCreatePatch, GlassSurfacePatch};
use pax_runtime::api::{borrow, Layer};
use pax_runtime::{
    bind_content_measurement_effect, resolve_axis_autosize, sync_content_autosize_with_axes,
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::cell::{Cell, RefCell};
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
    /// Corner radius used when the group materializes a native surface, in pixels.
    pub border_radius: Property<f64>,
}

impl Default for Group {
    fn default() -> Self {
        Self {
            autosize: Property::new(false),
            autosize_x: Property::new(None),
            autosize_y: Property::new(None),
            border_radius: Property::new(0.0),
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

    fn requires_non_reactive_update(&self, expanded_node: &ExpandedNode) -> bool {
        !expanded_node.content_measurement_bound.get()
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        if expanded_node.content_measurement_bound.get() {
            return;
        }
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

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();
        let initially_active = expanded_node.liquid_glass_scope.get().is_some();
        let created = Rc::new(Cell::new(initially_active));
        if initially_active {
            context.enqueue_native_message(pax_message::NativeMessage::GlassSurfaceCreate(
                AnyCreatePatch {
                    id,
                    parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                    render_layer_id: 0,
                },
            ));
        }

        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let new_children = expanded_node.generate_children(
            children_with_envs,
            context,
            &expanded_node.parent_frame,
            true,
        );
        expanded_node.children.set(new_children);

        let last_patch = Rc::new(RefCell::new(GlassSurfacePatch {
            id,
            ..Default::default()
        }));
        let weak_self_ref = Rc::downgrade(expanded_node);
        let context = Rc::clone(context);

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([
                expanded_node.transform_and_bounds.untyped(),
                expanded_node.computed_opacity.untyped(),
                expanded_node.occlusion.untyped(),
                expanded_node.liquid_glass_scope.untyped(),
            ])
            .collect();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };
                    let Some(liquid_glass) = expanded_node.liquid_glass_scope.get() else {
                        if created.replace(false) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::GlassSurfaceDelete(id),
                            );
                            *last_patch.borrow_mut() = GlassSurfacePatch {
                                id,
                                ..Default::default()
                            };
                        }
                        return;
                    };

                    if !created.get() {
                        context.enqueue_native_message(
                            pax_message::NativeMessage::GlassSurfaceCreate(AnyCreatePatch {
                                id,
                                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                                render_layer_id: 0,
                            }),
                        );
                        created.set(true);
                    }

                    let mut old_state = last_patch.borrow_mut();
                    let mut patch = GlassSurfacePatch {
                        id,
                        ..Default::default()
                    };

                    expanded_node.with_properties_unwrapped(|properties: &mut Group| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let max_radius = 0.5 * width.min(height);
                        let border_radius = properties.border_radius.get().clamp(0.0, max_radius);
                        let liquid_glass = liquid_glass.to_message();
                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &context),
                            ),
                            patch_if_needed(
                                &mut old_state.parent_frame,
                                &mut patch.parent_frame,
                                expanded_node.parent_frame.get().map(|v| v.to_u32()),
                            ),
                            patch_if_needed(
                                &mut old_state.z_index,
                                &mut patch.z_index,
                                expanded_node.occlusion.get().z_index,
                            ),
                            patch_if_needed(
                                &mut old_state.border_radius,
                                &mut patch.border_radius,
                                border_radius,
                            ),
                            patch_if_needed(
                                &mut old_state.liquid_glass,
                                &mut patch.liquid_glass,
                                liquid_glass,
                            ),
                        ];

                        if updates.into_iter().any(|v| v) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::GlassSurfaceUpdate(patch),
                            );
                        }
                    });
                },
                &deps,
            ));
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

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        if expanded_node.liquid_glass_scope.get().is_some() {
            context.enqueue_native_message(pax_message::NativeMessage::GlassSurfaceDelete(
                expanded_node.id.to_u32(),
            ));
        }
    }

    fn materializes_native_surface(&self, expanded_node: &ExpandedNode) -> bool {
        expanded_node.liquid_glass_scope.get().is_some()
    }

    fn materializes_native_surface_before_children(&self, expanded_node: &ExpandedNode) -> bool {
        self.materializes_native_surface(expanded_node)
    }
}
