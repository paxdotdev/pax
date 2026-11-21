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
use crate::api::{borrow, LocalProperty, NodeContext, Size};
use crate::node_interface::NodeLocal;
use crate::{
    apply_padding_frame, layout_hulls_equivalent, project_child_layout_hull_with_nodes,
    resolve_padded_autosize_axis, ExpandedNode, LayoutHull,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum ContentMeasurementGeometry {
    /// Measure child content without container-assigned placement.
    Intrinsic,
    /// Measure child content after placement in the container's local space.
    Placed,
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
        let projected_hull = project_child_layout_hull_with_nodes(
            content_transform_and_bounds,
            child.transform_and_bounds.get(),
            child.subtree_layout_hull.get(),
            &ctx.expanded_node,
            &Rc::downgrade(child),
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

    if (!width_explicit && autosize_width) || (!height_explicit && autosize_height) {
        if let Some(node) = ctx.expanded_node.upgrade() {
            ctx.runtime_context.register_startup_measurement(
                &node,
                crate::engine::startup::StartupAxes {
                    x: !width_explicit && autosize_width,
                    y: !height_explicit && autosize_height,
                },
            );
        }
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
    geometry: ContentMeasurementGeometry,
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
        node_ctx.bounds_self.untyped(),
        width_prop.untyped(),
        height_prop.untyped(),
        padding_x_prop.untyped(),
        padding_y_prop.untyped(),
    ];
    deps.extend(extra_deps.iter().cloned());

    let placed_measurement = if geometry == ContentMeasurementGeometry::Placed {
        let node_transform_and_bounds = expanded_node.transform_and_bounds.clone();
        let mut projected_child_hulls = Vec::new();
        for child in node_ctx.received_children.get().iter() {
            let child_cp = child.get_common_properties();
            let child_layout_role = borrow!(child_cp).layout_role.clone();
            let projection_name = format!(
                "{listener_name} projected child layout hull (parent id: {}, child id: {})",
                expanded_node.id.0, child.id.0
            );
            projected_child_hulls.push(crate::projected_child_layout_hull_property(
                node_transform_and_bounds.clone(),
                padding_x_prop.clone(),
                padding_y_prop.clone(),
                child.transform_and_bounds.clone(),
                child.subtree_layout_hull.clone(),
                child_layout_role,
                (Rc::downgrade(expanded_node), Rc::downgrade(child)),
                &projection_name,
            ));
        }

        let measurement_deps = projected_child_hulls
            .iter()
            .map(LocalProperty::untyped)
            .collect::<Vec<_>>();

        let measurement_name = format!("{listener_name} placed measurement");
        let measurement = LocalProperty::computed_with_cutoff_and_name(
            move || {
                if projected_child_hulls.is_empty() {
                    return LayoutHull::from_axis_ranges(Some((0.0, 0.0)), Some((0.0, 0.0)));
                }
                let hull = projected_child_hulls
                    .iter()
                    .fold(LayoutHull::default(), |hull, child| hull.union(child.get()));
                if hull.x_range().is_none() && hull.y_range().is_none() {
                    LayoutHull::from_axis_ranges(Some((0.0, 0.0)), Some((0.0, 0.0)))
                } else {
                    hull
                }
            },
            &measurement_deps,
            layout_hulls_equivalent,
            &measurement_name,
        );
        deps.push(measurement.untyped());
        Some(measurement)
    } else {
        for child in node_ctx.received_children.get().iter() {
            let child_cp = child.get_common_properties();
            deps.push(borrow!(child_cp).layout_role.untyped());
            deps.push(child.subtree_layout_hull.untyped());
        }
        None
    };

    let weak_node = Rc::downgrade(expanded_node);
    let runtime_context = Rc::clone(runtime_context);
    let bounds_self = node_ctx.bounds_self.clone();
    expanded_node
        .content_measurement_listener
        .replace_with(LocalProperty::computed_with_name(
            move || {
                let _ = bounds_self.get();
                if let Some(measurement) = &placed_measurement {
                    let _ = measurement.get();
                }
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

/// Bind Group or Frame autosizing only while at least one implicit axis uses its children.
#[doc(hidden)]
pub fn bind_content_autosize(
    node: &Rc<ExpandedNode>,
    ctx: &NodeContext,
    name: &'static str,
    autosize: LocalProperty<bool>,
    autosize_x: LocalProperty<Option<bool>>,
    autosize_y: LocalProperty<Option<bool>>,
) {
    if node.content_measurement_bound.replace(true) {
        return;
    }

    let props = node.get_common_properties();
    let width = borrow!(props).width.clone();
    let height = borrow!(props).height.clone();
    let deps = [
        width.untyped(),
        height.untyped(),
        autosize.untyped(),
        autosize_x.untyped(),
        autosize_y.untyped(),
    ];
    let weak_node = Rc::downgrade(node);
    let context = Rc::clone(&ctx.runtime_context);
    let previous = std::cell::Cell::new(None);
    let mode = LocalProperty::computed_with_name(
        move || {
            let enabled = autosize.get();
            let x = resolve_axis_autosize(enabled, autosize_x.get(), true);
            let y = resolve_axis_autosize(enabled, autosize_y.get(), true);
            let axes = (width.get().is_none() && x, height.get().is_none() && y);
            let old = previous.replace(Some(axes));
            if old == Some(axes) {
                return;
            }
            let Some(node) = weak_node.upgrade() else {
                return;
            };
            if axes.0 || axes.1 {
                install_content_measurement_effect(
                    &node,
                    &node.get_node_context(&context),
                    name,
                    ContentMeasurementGeometry::Placed,
                    &[],
                    move |node, ctx| sync_content_autosize_with_axes(node, ctx, axes.0, axes.1),
                );
            } else {
                if old.is_some() {
                    // Retire both the child-list subscription and the projected child hulls.
                    node.content_measurement_rebind_listener
                        .replace_with(LocalProperty::default());
                    node.content_measurement_listener
                        .replace_with(LocalProperty::default());
                }
                if node.measured_size.get().is_some() {
                    node.measured_size.set(None);
                }
            }
        },
        &deps,
        "content autosize axes",
    );
    ctx.runtime_context
        .register_expanded_node_effect_property_named(node, &mode, "content autosize axes");
    node.subscriptions.borrow_mut().push(mode);
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
    geometry: ContentMeasurementGeometry,
    extra_deps: &[UntypedProperty],
    effect: F,
) where
    F: Fn(&Rc<ExpandedNode>, &NodeContext) + Clone + 'static,
{
    if expanded_node.content_measurement_bound.replace(true) {
        return;
    }
    install_content_measurement_effect(
        expanded_node,
        ctx,
        listener_name,
        geometry,
        extra_deps,
        effect,
    );
}

fn install_content_measurement_effect<F>(
    expanded_node: &Rc<ExpandedNode>,
    ctx: &NodeContext,
    listener_name: &'static str,
    geometry: ContentMeasurementGeometry,
    extra_deps: &[UntypedProperty],
    effect: F,
) where
    F: Fn(&Rc<ExpandedNode>, &NodeContext) + Clone + 'static,
{
    rebind_content_measurement_effect(
        expanded_node,
        &ctx.runtime_context,
        listener_name,
        geometry,
        extra_deps,
        effect.clone(),
    );
    ctx.runtime_context
        .register_expanded_node_effect_property_named(
            expanded_node,
            &expanded_node.content_measurement_listener,
            listener_name,
        );

    let weak_node = Rc::downgrade(expanded_node);
    let runtime_context = Rc::clone(&ctx.runtime_context);
    let extra_deps = extra_deps.to_vec();
    let rebind_name = format!("{listener_name} rebind");
    expanded_node
        .content_measurement_rebind_listener
        .replace_with(LocalProperty::computed_with_name(
            move || {
                let Some(node) = weak_node.upgrade() else {
                    return;
                };
                rebind_content_measurement_effect(
                    &node,
                    &runtime_context,
                    listener_name,
                    geometry,
                    &extra_deps,
                    effect.clone(),
                );
            },
            // Rebinding reads the received-child list itself. Depending on its
            // unit-valued notifier without reading that notifier leaves it
            // dirty, which can suppress invalidation on subsequent changes.
            &[ctx.received_children.untyped()],
            &rebind_name,
        ));
    ctx.runtime_context
        .register_expanded_node_effect_property_named(
            expanded_node,
            &expanded_node.content_measurement_rebind_listener,
            &rebind_name,
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

/// Reactive placement assigned by a container. A snapped cell supplies its own
/// available size. Its translation still participates in paint and hit testing.
/// Use `get`, `set`, or `set_if_neq` for snapped frames. Easing, rebinding,
/// and borrowing the underlying `LocalProperty` through `as_property` or
/// dereferencing use the full frame for subsequent layout reads.
#[derive(Default)]
pub struct ContainerPlacement {
    frame: LocalProperty<Option<ContainerFrame>>,
    dimensions: std::cell::RefCell<Option<ContainerDimensions>>,
    // A retained property handle can change binding or begin easing at any time.
    follows_full_frame: std::cell::Cell<bool>,
}

struct ContainerDimensions {
    node: std::rc::Weak<ExpandedNode>,
    inherited: LocalProperty<crate::TransformAndBounds<NodeLocal, pax_runtime_api::Window>>,
    target: LocalProperty<(f64, f64)>,
    cell: Option<LocalProperty<(f64, f64)>>,
}

impl ContainerDimensions {
    fn target_bounds(
        node: &ExpandedNode,
        dependency: UntypedProperty,
        extent: impl Fn() -> (f64, f64) + 'static,
    ) -> LocalProperty<(f64, f64)> {
        let common = node.get_common_properties();
        let common = common.borrow();
        let (width, height) = (common.width.clone(), common.height.clone());
        let measured = node.measured_size.clone();
        let deps = [
            width.untyped(),
            height.untyped(),
            measured.untyped(),
            dependency,
        ];
        LocalProperty::computed(
            move || {
                let size = measured.get();
                crate::layout::resolve_layout_bounds(
                    width.get().or(size.map(|s| Size::Pixels(s.0.into()))),
                    height.get().or(size.map(|s| Size::Pixels(s.1.into()))),
                    extent(),
                )
            },
            &deps,
        )
    }

    fn inherited_target(
        node: &ExpandedNode,
        parent: &LocalProperty<crate::TransformAndBounds<NodeLocal, pax_runtime_api::Window>>,
    ) -> LocalProperty<(f64, f64)> {
        let parent = parent.clone();
        Self::target_bounds(node, parent.untyped(), move || parent.get().bounds)
    }

    fn assign(&mut self, bounds: Option<(f64, f64)>) {
        match (&self.cell, bounds) {
            (Some(cell), Some(bounds)) => {
                cell.set_if_neq(bounds);
            }
            (None, Some(bounds)) => {
                let Some(node) = self.node.upgrade() else {
                    return;
                };
                let cell = LocalProperty::new(bounds);
                self.cell = Some(cell.clone());
                self.target
                    .replace_with(Self::target_bounds(&node, cell.untyped(), move || {
                        cell.get()
                    }));
            }
            (Some(_), None) => {
                let Some(node) = self.node.upgrade() else {
                    return;
                };
                self.target
                    .replace_with(Self::inherited_target(&node, &self.inherited));
                self.cell = None;
            }
            (None, None) => {}
        }
    }
}

impl ContainerPlacement {
    pub(crate) fn frame(&self) -> &LocalProperty<Option<ContainerFrame>> {
        &self.frame
    }

    pub(crate) fn bind_target_bounds(
        &self,
        node: &Rc<ExpandedNode>,
        inherited: &LocalProperty<crate::TransformAndBounds<NodeLocal, pax_runtime_api::Window>>,
    ) -> LocalProperty<(f64, f64)> {
        let target = ContainerDimensions::inherited_target(node, inherited);
        let mut dimensions = ContainerDimensions {
            node: Rc::downgrade(node),
            target: target.clone(),
            inherited: inherited.clone(),
            cell: None,
        };
        if !self.follows_full_frame.get() {
            dimensions.assign(self.get().map(|frame| frame.bounds));
        }
        self.dimensions.replace(Some(dimensions));
        target
    }

    /// Reads the current parent-local frame, including active reflow interpolation.
    pub fn get(&self) -> Option<ContainerFrame> {
        self.frame.get()
    }

    /// Assigns a cell frame, or restores the padded parent extent when cleared.
    pub fn set(&self, frame: Option<ContainerFrame>) {
        self.frame.set(frame);
        if !self.follows_full_frame.get() {
            if let Some(dimensions) = self.dimensions.borrow_mut().as_mut() {
                dimensions.assign(frame.map(|frame| frame.bounds));
            }
        }
    }

    /// Assigns the frame when its size or placement changed.
    pub fn set_if_neq(&self, frame: Option<ContainerFrame>) -> bool {
        if self.get() == frame {
            return false;
        }
        self.set(frame);
        true
    }

    /// Returns the underlying reactive property for easing or custom bindings.
    /// Future size reads follow this handle, including any subsequent rebinding.
    pub fn as_property(&self) -> &LocalProperty<Option<ContainerFrame>> {
        if !self.follows_full_frame.replace(true) {
            if let Some(dimensions) = self.dimensions.borrow_mut().as_mut() {
                dimensions.assign(None);
            }
        }
        &self.frame
    }
}

impl std::ops::Deref for ContainerPlacement {
    type Target = LocalProperty<Option<ContainerFrame>>;

    fn deref(&self) -> &Self::Target {
        self.as_property()
    }
}

#[cfg(test)]
mod tests {
    use crate::api::math::{Transform2, Vector2};
    use crate::api::{CommonProperties, Layer, LayoutRole, Size};
    use crate::{
        bind_content_measurement_effect, measured_size_needs_update, sync_content_autosize,
        sync_content_autosize_with_axes, BaseInstance, ComponentInstance, ContainerFrame,
        ContentMeasurementGeometry, ExpandedNode, Globals, InstanceFlags, InstanceNode,
        InstantiationArgs, RouteLocation, RuntimePropertiesStackFrame, TransformAndBounds,
    };
    use pax_runtime_api::pax_value::PaxAny;
    use pax_runtime_api::{LocalProperty, Platform, TargetInfo, OS};
    use std::cell::{Cell, RefCell};
    use std::fmt;
    use std::rc::Rc;

    fn test_globals() -> Globals {
        Globals {
            elapsed_frames: LocalProperty::new(0),
            elapsed_millis: LocalProperty::new(0),
            viewport: LocalProperty::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (100.0, 100.0),
            }),
            gyro: LocalProperty::new(Default::default()),
            accel: LocalProperty::new(Default::default()),
            route_location: LocalProperty::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: LocalProperty::new(true),
            browser_allows_nested_scroller_vector_layers: LocalProperty::new(true),
            platform: Platform::Unknown,
            os: OS::Unknown,
            renderer_is_gpu: true,
            target: TargetInfo::new(Platform::Unknown, OS::Unknown),
            #[cfg(feature = "designtime")]
            designtime: crate::test_support::designtime(),
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
            component_camera: None,
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
            component_camera: None,
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
    fn leaf_hull_survives_attachment_and_follows_replaced_layout_sources() {
        let leaf = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let parent = TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root = ComponentInstance::instantiate(component_args(Some(vec![parent]), None));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(root, &context);
        root.recurse_update(&context);
        let parent = root.children.get()[0].clone();
        let leaf = parent.children.get()[0].clone();
        let common = leaf.get_common_properties();
        let hull = leaf.subtree_layout_hull.clone();
        let px = |v: i32| Some(Size::Pixels(v.into()));
        leaf.set_measured_size(21.0, 34.0);
        assert_eq!(hull.get().x_range(), Some((0.0, 21.0)));
        common.borrow().x.set(Some(Size::Percent(50.into())));
        assert_eq!(hull.get().x_range(), None);
        assert_eq!(hull.get().y_range(), Some((0.0, 34.0)));
        context.drain_node_effects();
        assert_eq!(leaf.layout_hull_rebuilds.get(), 1);

        let old_x = common.borrow().x.clone();
        let new_x = LocalProperty::new(old_x.get());
        common.borrow_mut().x = new_x.clone();
        root.attach_children(vec![leaf.clone()], &context, &root.parent_frame);
        context.drain_node_effects();
        assert_eq!(leaf.layout_hull_rebuilds.get(), 2);
        old_x.set(px(0));
        assert_eq!(hull.get().x_range(), None);
        new_x.set(px(0));
        assert_eq!(hull.get().x_range(), Some((0.0, 21.0)));
    }

    #[test]
    fn toggling_content_autosize_measures_current_children_on_implicit_axes() {
        let leaf = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let parent = TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![parent]), None));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        let parent = root.children.get()[0].clone();
        let child = parent.children.get()[0].clone();
        let common = parent.get_common_properties();
        let child_common = child.get_common_properties();
        let px = |n: i32| Some(Size::Pixels(n.into()));
        child_common.borrow().width.set(px(23));
        child_common.borrow().height.set(px(31));
        let enabled = LocalProperty::new(false);
        let y = LocalProperty::<Option<bool>>::default();
        super::bind_content_autosize(
            &parent,
            &parent.get_node_context(&context),
            "test autosize",
            enabled.clone(),
            LocalProperty::default(),
            y.clone(),
        );
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), None);

        enabled.set(true);
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), Some((23.0, 31.0)));
        common.borrow().width.set(px(80));
        y.set(Some(false));
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), None);

        child_common.borrow().height.set(px(56));
        context.drain_node_effects();
        y.set(Some(true));
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), Some((80.0, 56.0)));
        common.borrow().height.set(px(48));
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), None);

        let replacement: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let new_children = parent.generate_children(
            [(replacement, Rc::clone(&parent.stack))],
            &context,
            &parent.parent_frame,
            true,
        );
        let new_common = new_children[0].get_common_properties();
        parent.children.set(new_children);
        new_common.borrow().height.set(px(72));
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), None);
        common.borrow().height.set(None);
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), Some((80.0, 72.0)));
        new_common.borrow().height.set(px(84));
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), Some((80.0, 84.0)));
        enabled.set(false);
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), Some((80.0, 84.0)));
        y.set(None);
        context.drain_node_effects();
        assert_eq!(parent.measured_size.get(), None);
        assert_eq!(parent.transform_and_bounds.get().bounds, (80.0, 100.0));
    }

    #[test]
    fn placed_measurement_ignores_ancestor_translation_but_observes_child_position() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        let child_common = child.get_common_properties();
        child_common
            .borrow()
            .width
            .set(Some(Size::Pixels(20.into())));
        child_common
            .borrow()
            .height
            .set(Some(Size::Pixels(20.into())));
        child_common.borrow().x.set(Some(Size::Pixels(0.into())));
        child_common.borrow().y.set(Some(Size::Pixels(0.into())));
        context.drain_node_effects();

        let effect_runs = Rc::new(Cell::new(0));
        let effect_runs_for_effect = Rc::clone(&effect_runs);
        let node_ctx = direct_node.get_node_context(&context);
        bind_content_measurement_effect(
            &direct_node,
            &node_ctx,
            "placed measurement test",
            ContentMeasurementGeometry::Placed,
            &[],
            move |_, _| effect_runs_for_effect.set(effect_runs_for_effect.get() + 1),
        );
        context.drain_node_effects();
        let baseline = effect_runs.get();

        let viewport = context.globals().viewport;
        viewport.set(TransformAndBounds {
            transform: Transform2::translate(Vector2::new(25.0, 15.0)),
            bounds: (100.0, 100.0),
        });
        context.drain_node_effects();
        assert_eq!(effect_runs.get(), baseline);

        child_common.borrow().x.set(Some(Size::Pixels(12.into())));
        context.drain_node_effects();
        assert_eq!(effect_runs.get(), baseline + 1);

        // A sliding drawer is still rendered at its authored position, but its
        // motion must not wake the ancestor's content measurement.
        child_common
            .borrow()
            .layout_role
            .set(Some(LayoutRole::Breakout));
        context.drain_node_effects();
        let breakout_baseline = effect_runs.get();
        for offset in [100, 75, 50, 25, 0, 25, 50, 75, 100] {
            child_common
                .borrow()
                .x
                .set(Some(Size::Pixels(offset.into())));
            context.drain_node_effects();
            assert_eq!(effect_runs.get(), breakout_baseline);
        }
        child_common.borrow().layout_role.set(None);
        context.drain_node_effects();
        assert_eq!(effect_runs.get(), breakout_baseline + 1);
    }

    #[test]
    fn assigned_cell_dimensions_propagate_synchronously_and_ignore_translation() {
        let leaf = TestDirectNode::instantiate(direct_node_args(vec![]));
        let parent = TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(
            ComponentInstance::instantiate(component_args(Some(vec![parent]), None)),
            &context,
        );
        let cell = root.children.get()[0].clone();
        let child = cell.children.get()[0].clone();
        child
            .get_common_properties()
            .borrow()
            .width
            .set(Some(Size::Percent(50.into())));
        let width = root.get_common_properties().borrow().width.clone();
        let padding = root.get_common_properties().borrow().padding_x.clone();
        width.set(Some(Size::Pixels(100.into())));
        assert_eq!(child.layout_bounds.get().0, 50.);
        width.set(Some(Size::Pixels(200.into())));
        assert_eq!(child.layout_bounds.get().0, 100.);

        let frame = |width, y| {
            Some(ContainerFrame {
                transform: Transform2::translate(Vector2::new(0., y)),
                bounds: (width, 20.),
            })
        };
        cell.container_frame.set(frame(80., 10.));
        let reads = Rc::new(Cell::new(0));
        let observed = {
            let reads = reads.clone();
            let bounds = child.layout_bounds.clone();
            let deps = [bounds.untyped()];
            LocalProperty::computed(
                move || {
                    reads.set(reads.get() + 1);
                    bounds.get()
                },
                &deps,
            )
        };
        assert_eq!(observed.get().0, 40.);
        width.set(Some(Size::Pixels(300.into())));
        padding.set(Some(Size::Percent(10.into())));
        cell.container_frame.set(frame(80., 11.));
        assert_eq!(observed.get().0, 40.);
        assert_eq!(reads.get(), 1);
        assert_eq!(child.transform_and_bounds.get().transform.m[4], 30.);
        assert_eq!(child.transform_and_bounds.get().transform.m[5], 11.);
        cell.container_frame.set(frame(100., 11.));
        assert_eq!(observed.get().0, 50.);
        cell.container_frame.set(None);
        assert_eq!(observed.get().0, 120.);
        width.set(Some(Size::Pixels(400.into())));
        assert_eq!(observed.get().0, 160.);

        let _graph = context.property_graph.enter();
        let clock = LocalProperty::new(0_u64);
        pax_runtime_api::properties::register_time(&clock);
        cell.container_frame.set(frame(80., 10.));
        cell.container_frame
            .ease_to(frame(120., 20.), 10, pax_runtime_api::EasingCurve::Linear);
        clock.set(5);
        // Follow the actual eased frame at every read, including its size policy.
        let current = cell.container_frame.get().unwrap();
        assert_eq!(observed.get().0, current.bounds.0 * 0.5);
        assert_eq!(child.transform_and_bounds.get().transform.m[5], 15.);
        clock.set(10);
        assert_eq!(observed.get().0, 60.);
        cell.container_frame.cancel_transitions();

        // A custom container may retain and rebind the underlying property.
        let raw = cell.container_frame.as_property().clone();
        cell.container_frame.set(frame(100., 10.));
        let source = LocalProperty::new(frame(60., 2.));
        let binding = source.clone();
        raw.replace_with(LocalProperty::computed(
            move || binding.get(),
            &[source.untyped()],
        ));
        assert_eq!(observed.get().0, 30.);
        source.set(None);
        assert_eq!(observed.get().0, 160.);
    }

    #[test]
    fn collapsed_ancestor_preserves_measurement_and_accepts_new_child_sizes() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct = TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![direct]), None));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        let parent = root.children.get()[0].clone();
        let child = parent.children.get()[0].clone();
        let parent_common = parent.get_common_properties();
        parent_common
            .borrow()
            .padding_x
            .set(Some(Size::Pixels(5.into())));
        parent_common
            .borrow()
            .padding_y
            .set(Some(Size::Pixels(10.into())));
        let child_common = child.get_common_properties();
        child_common
            .borrow()
            .width
            .set(Some(Size::Pixels(20.into())));
        child_common
            .borrow()
            .height
            .set(Some(Size::Pixels(30.into())));
        child_common.borrow().x.set(Some(Size::Pixels(7.into())));
        child_common.borrow().y.set(Some(Size::Pixels(11.into())));
        child.container_frame.set(Some(ContainerFrame {
            transform: Transform2::translate(Vector2::new(3.0, 4.0)),
            bounds: (100.0, 100.0),
        }));
        context.drain_node_effects();
        let baseline = parent.subtree_layout_hull.get();
        let measured =
            super::measure_content_children_layout_hull(&parent.get_node_context(&context));
        assert_eq!(measured.x_range(), Some((10.0, 30.0)));
        assert_eq!(measured.y_range(), Some((15.0, 45.0)));

        for scale in [0.0, 0.5, 1.0] {
            root.get_common_properties()
                .borrow()
                .scale_x
                .set(Some(Size::Percent((scale * 100.0).into())));
            root.get_common_properties()
                .borrow()
                .scale_y
                .set(Some(Size::Percent((scale * 100.0).into())));
            context.drain_node_effects();
            assert_eq!(parent.subtree_layout_hull.get(), baseline);
            assert_eq!(
                super::measure_content_children_layout_hull(&parent.get_node_context(&context)),
                measured
            );
        }

        root.get_common_properties()
            .borrow()
            .scale_x
            .set(Some(Size::Percent(0.into())));
        root.get_common_properties()
            .borrow()
            .scale_y
            .set(Some(Size::Percent(0.into())));
        child_common
            .borrow()
            .height
            .set(Some(Size::Pixels(60.into())));
        context.drain_node_effects();
        let changed =
            super::measure_content_children_layout_hull(&parent.get_node_context(&context));
        assert_eq!(changed.y_range(), Some((15.0, 75.0)));
        assert_eq!(
            parent.subtree_layout_hull.get().forward_extent_y(),
            Some(95.0)
        );
    }

    #[test]
    fn intrinsic_measurement_ignores_assigned_frame_but_observes_child_size() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let child = direct_node.children.get().first().cloned().unwrap();
        let child_common = child.get_common_properties();
        child_common
            .borrow()
            .width
            .set(Some(Size::Pixels(20.into())));
        child_common
            .borrow()
            .height
            .set(Some(Size::Pixels(20.into())));
        child_common.borrow().x.set(Some(Size::Pixels(0.into())));
        child_common.borrow().y.set(Some(Size::Pixels(0.into())));
        context.drain_node_effects();

        let effect_runs = Rc::new(Cell::new(0));
        let effect_runs_for_effect = Rc::clone(&effect_runs);
        let node_ctx = direct_node.get_node_context(&context);
        bind_content_measurement_effect(
            &direct_node,
            &node_ctx,
            "intrinsic measurement test",
            ContentMeasurementGeometry::Intrinsic,
            &[],
            move |_, _| effect_runs_for_effect.set(effect_runs_for_effect.get() + 1),
        );
        context.drain_node_effects();
        let baseline = effect_runs.get();
        let baseline_hull = child.subtree_layout_hull.get();

        child.container_frame.set(Some(ContainerFrame {
            transform: Transform2::translate(Vector2::new(20.0, 10.0)),
            bounds: (100.0, 100.0),
        }));
        context.drain_node_effects();
        assert_eq!(effect_runs.get(), baseline);

        child_common
            .borrow()
            .width
            .set(Some(Size::Pixels(40.into())));
        context.drain_node_effects();
        assert_ne!(child.subtree_layout_hull.get(), baseline_hull);
        assert_eq!(effect_runs.get(), baseline + 1);
    }

    #[test]
    fn sync_content_autosize_includes_padding() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
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
