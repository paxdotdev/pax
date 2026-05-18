//! Container-facing child ontology and geometry seams.
//!
//! The runtime carries several child concepts, but container consumers should
//! reason about them on two axes:
//!
//! 1. Semantic role
//!    - `received_children`: the payload a node received from its caller
//!    - encapsulated implementation children: the node's own private template or
//!      primitive-assembled structure
//! 2. Lifecycle slice of the received payload
//!    - active received children: present in `NodeContext::received_children`
//!    - retained exiting received children: present in
//!      `NodeContext::retained_received_children`
//!
//! Engine details such as projection still exist, but they are transport
//! mechanisms rather than the primary semantic abstraction.

use crate::api::math::Transform2;
use crate::api::{borrow, NodeContext, Property, Size};
use crate::node_interface::NodeLocal;
use crate::{
    apply_padding_frame, project_child_layout_hull_to_parent_space, resolve_padded_autosize_axis,
    ExpandedNode, LayoutHull,
};
use pax_runtime_api::{properties::UntypedProperty, Interpolatable};
use std::rc::Rc;

const MEASURED_SIZE_EPSILON: f64 = 1e-9;

/// Trait for nodes that semantically interpret child content.
///
/// Containers should treat `NodeContext::received_children` as their canonical
/// payload set and `NodeContext::retained_received_children` as the set of
/// exit-retained payload nodes that may still need placement or transition
/// handling.
///
/// Encapsulated implementation children remain a private detail of the node's
/// own template or primitive assembly and should generally not drive container
/// layout logic.
///
/// Containers can call this from their existing mount logic to install reactive
/// behavior on top of those normalized views.
pub trait Container {
    fn bind_container(&self, _ctx: &NodeContext) {}
}

/// Measure the aggregate layout hull contributed by received content in the
/// container's local coordinate space.
///
/// Empty content is treated as a zero-sized valid hull so autosized containers
/// can collapse to `0x0` when they have no children.
pub fn measure_content_children_layout_hull(ctx: &NodeContext) -> LayoutHull {
    let content_children = ctx.received_children.get();
    if content_children.is_empty() {
        return LayoutHull::from_axis_ranges(Some((0.0, 0.0)), Some((0.0, 0.0)));
    }

    let (padding_x, padding_y) = node_padding(ctx);
    let content_transform_and_bounds =
        apply_padding_frame(ctx.node_transform_and_bounds, padding_x, padding_y);

    let mut hull: Option<LayoutHull> = None;
    for child in content_children.iter() {
        if child.is_layout_breakout() {
            // Breakout nodes stay parent-local but are intentionally excluded
            // from the parent's measured hull.
            continue;
        }
        let projected_hull = project_child_layout_hull_to_parent_space(
            content_transform_and_bounds,
            child.transform_and_bounds.get(),
            child.subtree_layout_hull.get(),
        );
        hull = Some(match hull {
            Some(existing) => existing.union(projected_hull),
            None => projected_hull,
        });
    }

    hull.unwrap_or_else(|| LayoutHull::from_axis_ranges(Some((0.0, 0.0)), Some((0.0, 0.0))))
}

/// Measure forward autosize extents from received content.
pub fn measure_content_children_forward_extents(ctx: &NodeContext) -> (Option<f64>, Option<f64>) {
    let hull = measure_content_children_layout_hull(ctx);
    (hull.forward_extent_x(), hull.forward_extent_y())
}

/// Resolve one axis of autosize given the public `autosize` toggle plus an optional override.
pub fn resolve_axis_autosize(
    autosize: bool,
    axis_override: Option<bool>,
    default_when_enabled: bool,
) -> bool {
    axis_override.unwrap_or(autosize && default_when_enabled)
}

/// Resolve a node's measured size from its received content.
///
/// Explicit axes keep their current container bounds; implicit axes use the
/// measured forward extents when they are valid. If an implicit axis cannot be
/// measured safely, this returns `None` so the caller can fall back.
pub fn resolve_content_autosize_measurement(
    ctx: &NodeContext,
    width_explicit: bool,
    height_explicit: bool,
) -> Option<(f64, f64)> {
    resolve_content_autosize_measurement_with_axes(ctx, width_explicit, height_explicit, true, true)
}

/// Resolve a node's measured size from received content with explicit per-axis autosize control.
pub fn resolve_content_autosize_measurement_with_axes(
    ctx: &NodeContext,
    width_explicit: bool,
    height_explicit: bool,
    autosize_width: bool,
    autosize_height: bool,
) -> Option<(f64, f64)> {
    if width_explicit && height_explicit {
        return None;
    }
    if !autosize_width && !autosize_height {
        return None;
    }

    let bounds = ctx.bounds_self.get();
    let (content_width, content_height) = measure_content_children_forward_extents(ctx);
    let (padding_x, padding_y) = node_padding(ctx);

    Some((
        if width_explicit || !autosize_width {
            bounds.0
        } else {
            resolve_padded_autosize_axis(content_width?, padding_x)?
        },
        if height_explicit || !autosize_height {
            bounds.1
        } else {
            resolve_padded_autosize_axis(content_height?, padding_y)?
        },
    ))
}

fn node_padding(ctx: &NodeContext) -> (Option<Size>, Option<Size>) {
    let Some(node) = ctx.expanded_node.upgrade() else {
        return (None, None);
    };
    let common_props = node.get_common_properties();
    let common_props = borrow!(common_props);
    (common_props.padding_x.get(), common_props.padding_y.get())
}

/// Update `measured_size` from received content when autosize is enabled.
pub fn sync_content_autosize(expanded_node: &Rc<ExpandedNode>, ctx: &NodeContext, enabled: bool) {
    sync_content_autosize_with_axes(expanded_node, ctx, enabled, enabled);
}

/// Update `measured_size` from received content with explicit per-axis autosize control.
pub fn sync_content_autosize_with_axes(
    expanded_node: &Rc<ExpandedNode>,
    ctx: &NodeContext,
    autosize_width: bool,
    autosize_height: bool,
) {
    let measured_size = if autosize_width || autosize_height {
        let common_props = expanded_node.get_common_properties();
        let common_props = borrow!(common_props);
        let width_explicit = common_props.width.get().is_some();
        let height_explicit = common_props.height.get().is_some();
        drop(common_props);

        resolve_content_autosize_measurement_with_axes(
            ctx,
            width_explicit,
            height_explicit,
            autosize_width,
            autosize_height,
        )
    } else {
        None
    };

    match measured_size {
        Some(measured_size) => {
            let previous = expanded_node.measured_size.get();
            if measured_size_needs_update(previous, measured_size) {
                expanded_node.set_measured_size(measured_size.0, measured_size.1);
            }
        }
        None => {
            if expanded_node.measured_size.get().is_some() {
                expanded_node.measured_size.set(None);
            }
        }
    }
}

pub fn measured_size_needs_update(previous: Option<(f64, f64)>, next: (f64, f64)) -> bool {
    match previous {
        Some((previous_width, previous_height)) => {
            axis_needs_update(previous_width, next.0) || axis_needs_update(previous_height, next.1)
        }
        None => true,
    }
}

fn axis_needs_update(previous: f64, next: f64) -> bool {
    if previous == next {
        return false;
    }
    if !previous.is_finite() || !next.is_finite() {
        return previous.to_bits() != next.to_bits();
    }
    (previous - next).abs() > MEASURED_SIZE_EPSILON
}

fn rebind_content_measurement_effect<F>(
    expanded_node: &Rc<ExpandedNode>,
    runtime_context: &Rc<crate::RuntimeContext>,
    listener_name: &'static str,
    extra_deps: &[UntypedProperty],
    effect: F,
) where
    F: Fn(&Rc<ExpandedNode>, &NodeContext) + Clone + 'static,
{
    let common_props = expanded_node.get_common_properties();
    let (width_prop, height_prop, padding_x_prop, padding_y_prop) = {
        let common_props = borrow!(common_props);
        (
            common_props.width.clone(),
            common_props.height.clone(),
            common_props.padding_x.clone(),
            common_props.padding_y.clone(),
        )
    };
    let node_ctx = expanded_node.get_node_context(runtime_context);
    let mut deps = vec![
        expanded_node.transform_and_bounds.untyped(),
        width_prop.untyped(),
        height_prop.untyped(),
        padding_x_prop.untyped(),
        padding_y_prop.untyped(),
    ];
    deps.extend(extra_deps.iter().cloned());
    for child in node_ctx.received_children.get().iter() {
        let child_cp = child.get_common_properties();
        deps.push(borrow!(child_cp).layout_role.untyped());
        deps.push(child.transform_and_bounds.untyped());
        deps.push(child.subtree_layout_hull.untyped());
    }

    let weak_node = Rc::downgrade(expanded_node);
    let runtime_context = Rc::clone(runtime_context);
    expanded_node
        .content_measurement_listener
        .replace_with(Property::computed_with_name(
            move || {
                let Some(node) = weak_node.upgrade() else {
                    return;
                };
                let node_ctx = node.get_node_context(&runtime_context);
                effect(&node, &node_ctx);
            },
            &deps,
            listener_name,
        ));
}

/// Bind a reactive content-measurement effect to this node.
///
/// The effect is re-evaluated after the tree update pass, before occlusion and
/// layer-plan generation, and its dependency list is rebound whenever the
/// normalized received-child list changes.
pub fn bind_content_measurement_effect<F>(
    expanded_node: &Rc<ExpandedNode>,
    ctx: &NodeContext,
    listener_name: &'static str,
    extra_deps: &[UntypedProperty],
    effect: F,
) where
    F: Fn(&Rc<ExpandedNode>, &NodeContext) + Clone + 'static,
{
    if expanded_node.content_measurement_bound.replace(true) {
        return;
    }

    rebind_content_measurement_effect(
        expanded_node,
        &ctx.runtime_context,
        listener_name,
        extra_deps,
        effect.clone(),
    );
    ctx.runtime_context.register_node_effect_property(
        expanded_node.id,
        &expanded_node.content_measurement_listener,
    );

    let weak_node = Rc::downgrade(expanded_node);
    let runtime_context = Rc::clone(&ctx.runtime_context);
    let extra_deps = extra_deps.to_vec();
    let rebind_name = format!("{listener_name} rebind");
    expanded_node
        .content_measurement_rebind_listener
        .replace_with(Property::computed_with_name(
            move || {
                let Some(node) = weak_node.upgrade() else {
                    return;
                };
                rebind_content_measurement_effect(
                    &node,
                    &runtime_context,
                    listener_name,
                    &extra_deps,
                    effect.clone(),
                );
            },
            &[ctx.received_children_changed.untyped()],
            &rebind_name,
        ));
    ctx.runtime_context.register_node_effect_property(
        expanded_node.id,
        &expanded_node.content_measurement_rebind_listener,
    );
}

/// Engine-internal selector for which child family should be normalized into
/// `NodeContext::received_children`.
///
/// This is a provenance selector, not the semantic API surface.
///
/// `Owned` means the node's received payload already lives in its active child
/// tree.
///
/// `Projected` means the node receives payload from its caller and the runtime
/// threads that payload through projection so it can be consumed by `slot(...)`
/// within the node's encapsulated implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceivedChildrenSource {
    Owned,
    Projected,
}

/// Parent-local frame assigned by a container to one of its received children.
///
/// This behaves like a virtual wrapper node inside the parent: the frame's
/// transform is composed onto the parent transform and its bounds become the
/// child container bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContainerFrame {
    pub transform: Transform2<NodeLocal, NodeLocal>,
    pub bounds: (f64, f64),
}

impl Interpolatable for ContainerFrame {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            transform: self.transform.interpolate(&other.transform, t),
            bounds: self.bounds.interpolate(&other.bounds, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::math::Transform2;
    use crate::api::{CommonProperties, Layer, LayoutRole, Size};
    use crate::{
        measured_size_needs_update, sync_content_autosize, sync_content_autosize_with_axes,
        BaseInstance, ComponentInstance, ExpandedNode, Globals, InstanceFlags, InstanceNode,
        InstantiationArgs, RouteLocation, RuntimeContext, RuntimePropertiesStackFrame,
        TransformAndBounds,
    };
    use pax_runtime_api::pax_value::PaxAny;
    use pax_runtime_api::{Platform, Property, TargetInfo, OS};
    use std::cell::RefCell;
    use std::fmt;
    use std::rc::Rc;

    fn test_globals() -> Globals {
        Globals {
            elapsed_frames: Property::new(0),
            elapsed_millis: Property::new(0),
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (100.0, 100.0),
            }),
            gyro: Property::new(Default::default()),
            accel: Property::new(Default::default()),
            route_location: Property::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform: Platform::Unknown,
            os: OS::Unknown,
            target: TargetInfo::new(Platform::Unknown, OS::Unknown),
            get_elapsed_millis: Rc::new(|| 0),
        }
    }

    fn default_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    > {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })
    }

    fn default_common_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(CommonProperties::default())))
        })
    }

    fn component_args(
        template: Option<Vec<Rc<dyn InstanceNode>>>,
        children: Option<Vec<Rc<dyn InstanceNode>>>,
    ) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: children.map(RefCell::new),
            component_template: template.map(RefCell::new),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn direct_node_args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    struct TestDirectNode {
        base: BaseInstance,
    }

    impl InstanceNode for TestDirectNode {
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
            f: &mut fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> fmt::Result {
            f.debug_struct("TestDirectNode").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    #[test]
    fn owned_received_children_reflect_active_children() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let node_ctx = direct_node.get_node_context(&context);
        let received_children = node_ctx.received_children.get();

        assert_eq!(received_children.len(), 1);
        assert_eq!(received_children[0].id, direct_node.children.get()[0].id);
        assert_eq!(node_ctx.received_children_count.get(), 1);
        assert!(node_ctx.retained_received_children.get().is_empty());
    }

    #[test]
    fn projected_received_children_reflect_flattened_projected_children() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let slotted_component: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(component_args(Some(Vec::new()), Some(vec![leaf])));
        let root_component = ComponentInstance::instantiate(component_args(
            Some(vec![Rc::clone(&slotted_component)]),
            None,
        ));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let component_node = root.children.get().first().cloned().unwrap();
        let node_ctx = component_node.get_node_context(&context);
        let received_children = node_ctx.received_children.get();

        assert_eq!(received_children.len(), 1);
        assert_eq!(
            received_children[0].id,
            component_node
                .expanded_and_flattened_projected_children
                .get()[0]
                .id
        );
        assert_eq!(node_ctx.received_children_count.get(), 1);
        assert!(node_ctx.retained_received_children.get().is_empty());
    }

    #[test]
    fn sync_content_autosize_collapses_empty_content() {
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let node_ctx = direct_node.get_node_context(&context);

        sync_content_autosize(&direct_node, &node_ctx, true);

        assert_eq!(direct_node.measured_size.get(), Some((0.0, 0.0)));
    }

    #[test]
    fn sync_content_autosize_with_axes_keeps_unmanaged_axis_at_layout_bounds() {
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let node_ctx = direct_node.get_node_context(&context);

        sync_content_autosize_with_axes(&direct_node, &node_ctx, false, true);

        assert_eq!(direct_node.measured_size.get(), Some((100.0, 0.0)));
    }

    #[test]
    fn measured_size_update_ignores_float_jitter() {
        assert!(!measured_size_needs_update(
            Some((836.8000000000001, 228.6585365853658)),
            (836.8000000000001, 228.65853658536582),
        ));
        assert!(measured_size_needs_update(
            Some((836.8000000000001, 228.6585365853658)),
            (836.8000000000001, 228.6585375853658),
        ));
        assert!(measured_size_needs_update(None, (0.0, 0.0)));
    }

    #[test]
    fn sync_content_autosize_includes_padding() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        child.set_measured_size(50.0, 60.0);
        let direct_common_props = direct_node.get_common_properties();
        direct_common_props
            .borrow()
            .padding_x
            .set(Some(Size::Pixels(5.into())));
        direct_common_props
            .borrow()
            .padding_y
            .set(Some(Size::Pixels(10.into())));

        let node_ctx = direct_node.get_node_context(&context);
        sync_content_autosize(&direct_node, &node_ctx, true);

        assert_eq!(direct_node.measured_size.get(), Some((60.0, 80.0)));
    }

    #[test]
    fn subtree_layout_hull_includes_trailing_padding_for_ancestor_measurement() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        child.set_measured_size(50.0, 60.0);
        let direct_common_props = direct_node.get_common_properties();
        direct_common_props
            .borrow()
            .padding_x
            .set(Some(Size::Pixels(5.into())));
        direct_common_props
            .borrow()
            .padding_y
            .set(Some(Size::Pixels(10.into())));

        root.recurse_update(&context);
        context.drain_node_effects();

        let hull = direct_node.subtree_layout_hull.get();

        assert_eq!(hull.x_range(), Some((5.0, 60.0)));
        assert_eq!(hull.y_range(), Some((10.0, 80.0)));
        assert_eq!(hull.forward_extent_x(), Some(60.0));
        assert_eq!(hull.forward_extent_y(), Some(80.0));
    }

    #[test]
    fn subtree_layout_hull_solves_percent_padding_from_content_hull() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        child.set_measured_size(60.0, 80.0);
        direct_node.set_measured_size(333.0, 333.0);
        let direct_common_props = direct_node.get_common_properties();
        direct_common_props
            .borrow()
            .padding_x
            .set(Some(Size::Percent(20.into())));
        direct_common_props
            .borrow()
            .padding_y
            .set(Some(Size::Percent(10.into())));

        root.recurse_update(&context);
        context.drain_node_effects();

        let hull = direct_node.subtree_layout_hull.get();

        assert_eq!(hull.forward_extent_x(), Some(100.0));
        assert_eq!(hull.forward_extent_y(), Some(100.0));
        assert!((hull.x_range().unwrap().0 - 20.0).abs() < 1e-9);
        assert!((hull.y_range().unwrap().0 - 10.0).abs() < 1e-9);
    }

    #[test]
    fn sync_content_autosize_solves_percent_padding() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        child.set_measured_size(60.0, 80.0);
        let direct_common_props = direct_node.get_common_properties();
        direct_common_props
            .borrow()
            .padding_x
            .set(Some(Size::Percent(20.into())));
        direct_common_props
            .borrow()
            .padding_y
            .set(Some(Size::Percent(10.into())));

        let node_ctx = direct_node.get_node_context(&context);
        sync_content_autosize(&direct_node, &node_ctx, true);

        assert_eq!(direct_node.measured_size.get(), Some((100.0, 100.0)));
    }

    #[test]
    fn sync_content_autosize_ignores_breakout_children() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        child.set_measured_size(50.0, 60.0);
        let child_common_props = child.get_common_properties();
        child_common_props
            .borrow()
            .layout_role
            .set(Some(LayoutRole::Breakout));

        let node_ctx = direct_node.get_node_context(&context);
        sync_content_autosize(&direct_node, &node_ctx, true);

        assert_eq!(direct_node.measured_size.get(), Some((0.0, 0.0)));
    }
}
