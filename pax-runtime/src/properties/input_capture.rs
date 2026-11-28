//! Immutable input geometry for the staged-presentation prototype. Querying a
//! capture reads only values; resolving a recipient separately checks its mounted
//! incarnation. Ordinary input continues using the existing live query path.

use super::*;
use kurbo::Point;
use pax_runtime_api::properties::{
    DependencyEventQueue, DependencyEventSubscription, PropertyReadScope,
};
use pax_runtime_api::HitShape;

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);

/// Limits on captured nodes and value/path payload. Shared mesh topology,
/// allocator overhead and query scratch storage are not included in this count.
#[derive(Clone, Copy)]
pub struct InputCaptureLimits {
    pub nodes: usize,
    pub payload_bytes: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputCaptureError {
    Unsettled(&'static str),
    Capacity,
    InvalidScroll,
    IdentityExhausted,
}

/// Identifies one mounted incarnation, including for native scroll observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputIdentity {
    pub id: u32,
    pub mount_epoch: u64,
    view: u64,
}

impl InputIdentity {
    fn in_view(self, view: u64) -> Self {
        Self { view, ..self }
    }
}

/// Opaque recipient from one captured view. Resolve this exact target before
/// dispatch; do not query the newer live scene if resolution fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedInputTarget {
    pub identity: InputIdentity,
    view: u64,
    index: usize,
}

#[derive(Clone)]
struct Scroll {
    offset: (f64, f64),
    // Projected world translations for one local x/y scroll unit.
    affine: [[f64; 2]; 2],
    #[cfg(feature = "3d")]
    world: [[f64; 3]; 2],
    #[cfg(feature = "3d")]
    depth: [f64; 2],
}

#[derive(Clone)]
struct Node {
    identity: InputIdentity,
    // Weak ownership is used only during dispatch validation, never geometry
    // queries. It also distinguishes a new ExpandedNode reusing an ID/epoch.
    mounted: Weak<ExpandedNode>,
    parent: Option<usize>,
    template_parent: Option<usize>,
    flattened_slot_controller: bool,
    layout: crate::TransformAndBounds<crate::node_interface::NodeLocal, pax_runtime_api::Window>,
    bounds_parent: (f64, f64),
    measured_size: Option<(f64, f64)>,
    subtree_layout_hull: crate::LayoutHull,
    exit: bool,
    retained_exit: bool,
    unclippable: bool,
    invisible: bool,
    raycastable: bool,
    wheel_cancellable: bool,
    clips: bool,
    scroll: Option<Scroll>,
    shape: HitShape,
    transform: Affine,
    bounds: (f64, f64),
    #[cfg(feature = "3d")]
    geometry: crate::SpatialGeometry,
    #[cfg(feature = "3d")]
    spatial: crate::SpatialGeometry,
    #[cfg(feature = "3d")]
    camera: crate::camera::PresentationView,
    #[cfg(feature = "3d")]
    layer: u32,
}

impl Node {
    fn payload_bytes(&self) -> usize {
        std::mem::size_of::<Node>()
            + std::mem::size_of::<Rc<Node>>()
            + 6 * std::mem::size_of::<usize>()
            + self.path_bytes()
    }

    fn path_bytes(&self) -> usize {
        match &self.shape {
            HitShape::Path { path, .. } => std::mem::size_of_val(path.elements()),
            _ => 0,
        }
    }
}

// Captured bodies are immutable. Identity equality lets persistent-map diffs
// skip shared pages without comparing hit paths and unrelated input payloads.
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[derive(Clone, Copy)]
struct Inherited {
    affine: Affine,
    clipped: bool,
    skip: bool,
    #[cfg(feature = "3d")]
    world: [f64; 3],
    #[cfg(feature = "3d")]
    depth: f64,
}
impl Default for Inherited {
    fn default() -> Self {
        Self {
            affine: Affine::IDENTITY,
            clipped: false,
            skip: false,
            #[cfg(feature = "3d")]
            world: [0.; 3],
            #[cfg(feature = "3d")]
            depth: 0.,
        }
    }
}

/// Complete application hit geometry. Native scrolling is supplied as observed
/// offsets against captured host axes; authored host motion remains frozen.
/// This view is not automatically installed by the ordinary render loop.
#[derive(Clone)]
pub struct CapturedInputView {
    application: u64,
    view: u64,
    nodes: InputNodes,
    root: Option<usize>,
    // Stable incarnation slots and persistent child order let an edit preserve
    // unrelated geometry and routing indices. Only render children are traversed by hit tests.
    children: imbl::OrdMap<usize, Rc<Vec<usize>>>,
    subtree_escapes: imbl::OrdSet<usize>,

    scroll_indices: imbl::OrdSet<usize>,
    #[cfg(test)]
    pub(crate) query_work: Cell<usize>,
    payload_bytes: usize,
    viewport: pax_runtime_api::Viewport,
    #[cfg(feature = "3d")]
    spatial_possible: bool,
}

#[derive(Clone, Default)]
struct InputNodes(imbl::OrdMap<usize, Rc<Node>>);
impl InputNodes {
    fn get(&self, index: usize) -> Option<&Rc<Node>> {
        self.0.get(&index)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl std::ops::Index<usize> for InputNodes {
    type Output = Rc<Node>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[&index]
    }
}

impl CapturedInputView {
    fn target_at(&self, index: usize) -> CapturedInputTarget {
        CapturedInputTarget {
            identity: self.nodes[index].identity.in_view(self.view),
            view: self.view,
            index,
        }
    }

    fn target_for_node(
        &self,
        context: &RuntimeContext,
        node: &Rc<ExpandedNode>,
    ) -> Option<CapturedInputTarget> {
        let index = node.id.to_u32() as usize;
        self.nodes.get(index)?;
        let target = self.target_at(index);
        self.resolve_target(context, target)
            .filter(|current| Rc::ptr_eq(current, node))
            .map(|_| target)
    }

    fn template_route(&self, target: CapturedInputTarget) -> Vec<CapturedInputTarget> {
        let mut route = Vec::new();
        let mut next = Some(target.index);
        while let Some(index) = next {
            if route.len() >= self.nodes.len() {
                return Vec::new();
            }
            route.push(self.target_at(index));
            next = self.nodes[index].template_parent;
        }
        route
    }

    fn same_recipient(&self, a: CapturedInputTarget, other: &Self, b: CapturedInputTarget) -> bool {
        a.identity.id == b.identity.id
            && a.identity.mount_epoch == b.identity.mount_epoch
            && self.nodes[a.index].mounted.as_ptr() == other.nodes[b.index].mounted.as_ptr()
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }

    /// View-scoped identities prevent observations from a newer/reused native
    /// host being mistaken for scrolling of this displayed incarnation.
    pub fn scroll_hosts(&self) -> impl Iterator<Item = InputIdentity> + '_ {
        self.scroll_indices
            .iter()
            .map(|&index| self.nodes[index].identity.in_view(self.view))
    }

    /// Convert an event point using the displayed target's geometry, including
    /// observed native scrolling. This must accompany captured hit routing: using
    /// the live NodeContext transform would aim a drag at the old node but measure
    /// it in its newer layout. No property/node reads occur here. Foreign targets,
    /// singular mappings and clipped perspective points return None.
    pub fn local_point(
        &self,
        target: CapturedInputTarget,
        point: Point,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
    ) -> Result<Option<Point>, InputCaptureError> {
        if target.view != self.view || !point.x.is_finite() || !point.y.is_finite() {
            return Ok(None);
        }
        let Some(node) = self
            .nodes
            .get(target.index)
            .filter(|node| node.identity.in_view(self.view) == target.identity)
        else {
            return Ok(None);
        };
        let state = self.inherited_scroll(node, scroll_offsets)?;
        let normalized = node
            .local_point(point, state)
            .map(|point| Point::new(point.x / node.bounds.0, point.y / node.bounds.1));
        Ok(normalized.filter(|point| point.x.is_finite() && point.y.is_finite()))
    }

    fn inherited_scroll(
        &self,
        node: &Node,
        offsets: &HashMap<InputIdentity, (f64, f64)>,
    ) -> Result<Inherited, InputCaptureError> {
        let mut ancestors = Vec::new();
        let mut parent = node.parent;
        while let Some(index) = parent {
            if ancestors.len() >= self.nodes.len() {
                return Err(InputCaptureError::Unsettled("render_parent_cycle"));
            }
            ancestors.push(index);
            parent = self.nodes[index].parent;
        }
        let mut state = Inherited::default();
        for index in ancestors.into_iter().rev() {
            self.nodes[index].apply_scroll(&mut state, offsets, self.view)?;
        }
        Ok(state)
    }

    fn node_context(
        &self,
        record: &Node,
        node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
        scroll: Inherited,
    ) -> crate::api::NodeContext {
        let mut result = node.get_node_context(context);
        let projection = Some(pax_runtime_api::math::ProjectedPlane::affine(
            scroll.affine * record.transform,
        ));
        #[cfg(feature = "3d")]
        let projection = if record.camera.is_perspective() {
            record.projection(scroll)
        } else {
            projection
        };
        result.captured_event_geometry = Some(Rc::new(crate::api::CapturedEventGeometry {
            projection,
            bounds: record.bounds,
        }));
        result.bounds_self = LocalProperty::new(record.bounds);
        result.bounds_parent = LocalProperty::new(record.bounds_parent);
        result.measured_size = LocalProperty::new(record.measured_size);
        result.subtree_layout_hull = LocalProperty::new(record.subtree_layout_hull);
        result.node_transform_and_bounds = record.layout;
        result.viewport = LocalProperty::new(self.viewport);
        result
    }

    /// Builds an event context for this exact displayed recipient. Geometric
    /// fields and local-point mapping remain captured even if a handler saves
    /// the context. Application state, time and lifecycle remain live.
    #[doc(hidden)]
    pub fn event_context(
        &self,
        context: &Rc<RuntimeContext>,
        target: CapturedInputTarget,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
    ) -> Result<Option<crate::api::NodeContext>, InputCaptureError> {
        let Some(node) = self.resolve_target(context, target) else {
            return Ok(None);
        };
        let _graph = context.property_graph.enter();
        let record = &self.nodes[target.index];
        let scroll = self.inherited_scroll(record, scroll_offsets)?;
        Ok(Some(self.node_context(record, &node, context, scroll)))
    }

    /// Internal adapter entry point for handlers whose event type matches the
    /// supplied runtime handler key. Uses captured template ancestry, not render
    /// ancestry or a newer live tree. A retired recipient cancels this route;
    /// it never chooses a different hit behind it. Each ancestor is revalidated
    /// after the preceding handler, which may have unmounted it.
    ///
    /// This does not acquire/release pointer capture or execute native default
    /// actions; the chassis owns those event-specific semantics. Ordinary event
    /// routing does not call this entry point until front publication is enabled.
    #[doc(hidden)]
    pub fn dispatch_handlers<T: Clone + 'static>(
        &self,
        context: &Rc<RuntimeContext>,
        target: CapturedInputTarget,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        handler_key: &str,
        event: &Event<T>,
        bubbles: bool,
    ) -> Result<bool, InputCaptureError> {
        self.dispatch_selected_handlers(
            context,
            target,
            scroll_offsets,
            event,
            bubbles,
            true,
            |_| Some(handler_key),
        )
    }

    /// Dispatch mouse activation with the ordinary click/tap alias selection at
    /// each captured ancestor. A click-only or tap-only binding accepts either
    /// source; when both exist, only the source's corresponding binding runs.
    #[doc(hidden)]
    pub fn dispatch_click(
        &self,
        context: &Rc<RuntimeContext>,
        target: CapturedInputTarget,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        event: &Event<pax_runtime_api::Click>,
    ) -> Result<bool, InputCaptureError> {
        self.dispatch_activation(context, target, scroll_offsets, event, false)
    }

    /// Touch counterpart of `dispatch_click`, preserving source-specific aliases.
    #[doc(hidden)]
    pub fn dispatch_tap(
        &self,
        context: &Rc<RuntimeContext>,
        target: CapturedInputTarget,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        event: &Event<pax_runtime_api::Click>,
    ) -> Result<bool, InputCaptureError> {
        self.dispatch_activation(context, target, scroll_offsets, event, true)
    }

    fn dispatch_activation(
        &self,
        context: &Rc<RuntimeContext>,
        target: CapturedInputTarget,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        event: &Event<pax_runtime_api::Click>,
        touch: bool,
    ) -> Result<bool, InputCaptureError> {
        if self.resolve_target(context, target).is_none() {
            return Ok(event.cancelled());
        }
        let observers = context.outside_clicks.borrow().snapshot_excluding(|| {
            self.template_route(target)
                .into_iter()
                .map(|t| ExpandedNodeIdentifier(t.identity.id))
        });
        // Validate displayed observer geometry before any handler can change the scene.
        let mut outside = Vec::new();
        if let Some(observers) = observers {
            for (id, generation) in observers {
                let index = id.to_u32() as usize;
                if self.nodes.get(index).is_some() {
                    let recipient = self.target_at(index);
                    if let Some(geometry) =
                        self.event_context(context, recipient, scroll_offsets)?
                    {
                        outside.push((recipient, generation, geometry));
                    }
                }
            }
        }
        let cancelled = self.dispatch_selected_handlers(
            context,
            target,
            scroll_offsets,
            event,
            true,
            false,
            |node| node.pointer_activation_handler(touch),
        )?;
        for (recipient, generation, observer_context) in outside {
            if let Some(node) = self.resolve_target(context, recipient) {
                node.run_event_handlers_with_context(
                    crate::constants::CLICK_OUTSIDE_HANDLERS,
                    &Event::new(event.args.clone()),
                    context,
                    || observer_context,
                    || {
                        context
                            .outside_clicks
                            .borrow()
                            .is_current(node.id, generation)
                            && self.resolve_target(context, recipient).is_some()
                    },
                );
            }
        }
        Ok(cancelled)
    }

    fn dispatch_selected_handlers<'a, T: Clone + 'static>(
        &self,
        context: &Rc<RuntimeContext>,
        target: CapturedInputTarget,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        event: &Event<T>,
        bubbles: bool,
        stop_at_render_source: bool,
        select: impl Fn(&ExpandedNode) -> Option<&'a str>,
    ) -> Result<bool, InputCaptureError> {
        if self.resolve_target(context, target).is_none() {
            return Ok(event.cancelled());
        }
        // Validate every coordinate input before application callbacks run. A
        // malformed ancestor scroll observation must not partially dispatch.
        let mut route = Vec::new();
        let mut next = Some(target.index);
        while let Some(index) = next {
            if route.len() >= self.nodes.len() {
                return Err(InputCaptureError::Unsettled("template_cycle"));
            }
            let record = &self.nodes[index];
            route.push((index, self.inherited_scroll(record, scroll_offsets)?));
            next = if bubbles {
                record.template_parent
            } else {
                None
            };
        }
        let _graph = context.property_graph.enter();
        for (index, scroll) in route {
            let record = &self.nodes[index];
            let recipient = CapturedInputTarget {
                identity: record.identity.in_view(self.view),
                view: self.view,
                index,
            };
            // Projected if/for controllers are lexical links, not mounted
            // recipients. Like live dispatch, bubble through them. A recipient
            // that *was* mounted in the displayed frame must still validate.
            if record.flattened_slot_controller {
                continue;
            }
            let Some(node) = self.resolve_target(context, recipient) else {
                break;
            };
            // Match ordinary dispatch: generic events stop at a render source;
            // activation skips its bindings but continues through lexical scope.
            if stop_at_render_source && node.is_render_source() {
                break;
            }
            if let Some(handler_key) = select(&node) {
                node.run_event_handlers_with_context(
                    handler_key,
                    event,
                    context,
                    || self.node_context(record, &node, context, scroll),
                    || self.resolve_target(context, recipient).is_some(),
                );
            }
            if event.propagation_stopped() {
                break;
            }
        }
        Ok(event.cancelled())
    }

    /// Returns ordered recipients without reading any node/property or invoking
    /// callbacks. Missing offsets use the captured position. Nonfinite observed
    /// offsets fail the query rather than retarget input through invalid geometry.
    pub fn hit_targets(
        &self,
        point: Point,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        hit_invisible: bool,
    ) -> Result<Vec<CapturedInputTarget>, InputCaptureError> {
        self.hit_targets_under(point, scroll_offsets, hit_invisible, None)
    }

    fn hit_targets_under(
        &self,
        point: Point,
        scroll_offsets: &HashMap<InputIdentity, (f64, f64)>,
        hit_invisible: bool,
        root: Option<usize>,
    ) -> Result<Vec<CapturedInputTarget>, InputCaptureError> {
        #[cfg(test)]
        self.query_work.set(0);
        if !point.x.is_finite() || !point.y.is_finite() {
            return Ok(vec![]);
        }
        for (identity, offset) in scroll_offsets {
            if identity.view == self.view
                && (!offset.0.is_finite() || !offset.1.is_finite())
                && self.nodes.get(identity.id as usize).is_some_and(|n| {
                    n.identity.in_view(self.view) == *identity && n.scroll.is_some()
                })
            {
                return Err(InputCaptureError::InvalidScroll);
            }
        }
        // A clipped scroller partition can be skipped without visiting its
        // descendants. Escaping content keeps the conservative traversal.
        let mut pending = root
            .or(self.root)
            .map(|i| vec![(i, Inherited::default())])
            .unwrap_or_default();
        let mut hits = Vec::new();
        #[cfg(feature = "3d")]
        let mut depths = Vec::new();
        #[cfg(feature = "3d")]
        let mut spatial_hit = false;
        while let Some((index, mut state)) = pending.pop() {
            #[cfg(test)]
            self.query_work.set(self.query_work.get() + 1);
            let node = &self.nodes[index];
            if root == Some(index) {
                // Explicit subtree queries disregard outer clips/exit state,
                // just like live queries, but keep inherited native scrolling.
                state = self.inherited_scroll(node, scroll_offsets)?;
            }
            state.clipped &= !node.unclippable;
            state.skip |= !hit_invisible && node.exit;
            if state.skip || (state.clipped && !self.subtree_escapes.contains(&index)) {
                continue;
            }
            let can_hit = hit_invisible || !node.invisible;
            let (geometry_hit, solid_depth) = if !state.clipped && (can_hit || node.clips) {
                node.test(point, state)
            } else {
                (false, None)
            };
            #[cfg(not(feature = "3d"))]
            let _ = solid_depth;
            let hit = geometry_hit && node.raycastable;
            if hit && can_hit && !state.clipped {
                #[cfg(feature = "3d")]
                {
                    let solid_depth = self.spatial_possible.then_some(solid_depth).flatten();
                    let plane = if node.camera.is_perspective() {
                        node.projection(state).and_then(|p| p.depth_plane())
                    } else {
                        self.spatial_possible
                            .then(|| {
                                node.geometry
                                    .depth_plane(pax_runtime_api::math::Transform2::new(
                                        (state.affine * node.transform).as_coeffs(),
                                    ))
                            })
                            .flatten()
                    };
                    spatial_hit |= solid_depth.is_some() || plane.is_some() || state.depth != 0.;
                    let plane = plane.unwrap_or_default();
                    depths.push((
                        solid_depth.unwrap_or_else(|| plane.depth_at(point.x, point.y))
                            + if node.camera.is_perspective() {
                                0.
                            } else {
                                state.depth
                            },
                        plane.backface,
                        hits.len(),
                        node.layer,
                    ));
                }
                hits.push(CapturedInputTarget {
                    identity: node.identity.in_view(self.view),
                    view: self.view,
                    index,
                });
            }
            state.clipped |= !hit && node.clips;
            node.apply_scroll(&mut state, scroll_offsets, self.view)?;
            if !state.clipped || self.subtree_escapes.contains(&index) {
                if let Some(children) = self.children.get(&index) {
                    pending.extend(children.iter().rev().map(|&child| (child, state)));
                }
            }
        }
        #[cfg(feature = "3d")]
        if spatial_hit {
            let mut ordered: Vec<_> = hits.into_iter().zip(depths).collect();
            ordered.sort_by(|(_, a), (_, b)| {
                b.3.cmp(&a.3)
                    .then_with(|| pax_runtime_api::DepthPlane::compare_point_depths(b.0, a.0))
                    .then_with(|| {
                        let order = |v: &(f64, bool, usize, u32)| {
                            pax_runtime_api::DepthPlane {
                                backface: v.1,
                                ..Default::default()
                            }
                            .coplanar_order(-(v.2 as i64))
                        };
                        order(b).cmp(&order(a))
                    })
            });
            hits = ordered.into_iter().map(|(hit, _)| hit).collect();
        }
        Ok(hits)
    }

    /// Resolves the captured recipient only if the application and mounted node
    /// are still the same. Failure is cancellation, never permission to choose
    /// the next hit behind an obsolete front node.
    pub fn resolve_target(
        &self,
        context: &RuntimeContext,
        target: CapturedInputTarget,
    ) -> Option<Rc<ExpandedNode>> {
        if self.application != context.property_graph.id()
            || target.view != self.view
            || matches!(
                context.application.context().phase(),
                pax_runtime_api::AppPhase::Closing | pax_runtime_api::AppPhase::Closed
            )
        {
            return None;
        }
        let record = self.nodes.get(target.index)?;
        if record.identity.in_view(self.view) != target.identity {
            return None;
        }
        let node = record.mounted.upgrade()?;
        let current =
            context.get_expanded_node_by_eid(ExpandedNodeIdentifier(target.identity.id))?;
        (node.attached.get() > 0
            && node.mount_epoch.get() == target.identity.mount_epoch
            && Rc::ptr_eq(&node, &current))
        .then_some(node)
    }
}

/// A scoped platform input dispatch against one published view. Dropping the
/// guard restores the enclosing route; layout/lifecycle work outside the scope
/// continues to observe the working scene.
#[doc(hidden)]
pub struct CapturedInputScope {
    context: Weak<RuntimeContext>,
    routing: Rc<CapturedInputRouting>,
}

impl Drop for CapturedInputScope {
    fn drop(&mut self) {
        if let Some(context) = self.context.upgrade() {
            context
                .captured_input_scopes
                .borrow_mut()
                .retain(|routing| !Rc::ptr_eq(routing, &self.routing));
        }
    }
}

pub(crate) struct CapturedInputRouting {
    view: Rc<CapturedInputView>,
    scroll_offsets: HashMap<InputIdentity, (f64, f64)>,
}

pub(crate) struct CapturedInputHover {
    routing: Rc<CapturedInputRouting>,
    target: CapturedInputTarget,
}

impl RuntimeContext {
    /// Route one interrupt through the displayed hit geometry and lexical
    /// ancestry. Invalid observations reject the whole dispatch before callbacks.
    #[doc(hidden)]
    pub fn enter_captured_input(
        self: &Rc<Self>,
        view: Rc<CapturedInputView>,
        scroll_offsets: HashMap<InputIdentity, (f64, f64)>,
    ) -> Result<CapturedInputScope, InputCaptureError> {
        if view.application != self.property_graph.id() {
            return Err(InputCaptureError::Unsettled("application"));
        }
        if scroll_offsets
            .values()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
        {
            return Err(InputCaptureError::InvalidScroll);
        }
        let routing = Rc::new(CapturedInputRouting {
            view,
            scroll_offsets,
        });
        self.captured_input_scopes
            .borrow_mut()
            .push(routing.clone());
        Ok(CapturedInputScope {
            context: Rc::downgrade(self),
            routing,
        })
    }

    pub(crate) fn active_captured_input(&self) -> Option<Rc<CapturedInputRouting>> {
        self.captured_input_scopes.borrow().last().cloned()
    }

    /// Validate addressed native input before it can mutate application state.
    /// A published control cannot deliver to a replacement with the same ID.
    #[doc(hidden)]
    pub fn accepts_displayed_input(&self, node: &Rc<ExpandedNode>) -> bool {
        self.active_captured_input()
            .is_none_or(|routing| routing.view.target_for_node(self, node).is_some())
    }

    #[doc(hidden)]
    pub fn captured_wheel_cancellation(&self, node: &Rc<ExpandedNode>) -> Option<bool> {
        self.active_captured_input()
            .map(|routing| routing.route_contains(self, node, |record| record.wheel_cancellable))
    }

    #[doc(hidden)]
    pub fn captured_scroller_ancestor(&self, node: &Rc<ExpandedNode>) -> Option<bool> {
        self.active_captured_input()
            .map(|routing| routing.route_contains(self, node, |record| record.scroll.is_some()))
    }

    #[doc(hidden)]
    pub fn captured_input_descendant(
        &self,
        node: &Rc<ExpandedNode>,
        ancestor: ExpandedNodeIdentifier,
    ) -> Option<bool> {
        self.active_captured_input().map(|routing| {
            let Some(target) = routing.view.target_for_node(self, node) else {
                return false;
            };
            let mut index = Some(target.index);
            while let Some(current) = index {
                let record = &routing.view.nodes[current];
                if record.identity.id == ancestor.to_u32() {
                    return true;
                }
                index = record.parent;
            }
            false
        })
    }
}

impl CapturedInputRouting {
    fn route_contains(
        &self,
        context: &RuntimeContext,
        node: &Rc<ExpandedNode>,
        predicate: impl Fn(&Node) -> bool,
    ) -> bool {
        let Some(target) = self.view.target_for_node(context, node) else {
            return false;
        };
        let mut index = Some(target.index);
        while let Some(current) = index {
            let record = &self.view.nodes[current];
            if predicate(record) {
                return true;
            }
            index = record.template_parent;
        }
        false
    }

    #[allow(dead_code)]
    pub(crate) fn visit_postorder(
        &self,
        context: &RuntimeContext,
        callback: &mut impl FnMut(&Rc<ExpandedNode>),
    ) {
        // Capture order is forward preorder. Its reversal matches the ordinary
        // reverse-child postorder without consulting today's projected tree.
        let mut pending = self
            .view
            .root
            .map(|root| vec![(root, false)])
            .unwrap_or_default();
        while let Some((index, visited)) = pending.pop() {
            if visited {
                if let Some(node) = self
                    .view
                    .resolve_target(context, self.view.target_at(index))
                {
                    callback(&node);
                }
            } else {
                pending.push((index, true));
                if let Some(children) = self.view.children.get(&index) {
                    pending.extend(children.iter().map(|&child| (child, false)));
                }
            }
        }
    }

    pub(crate) fn query(
        &self,
        context: &RuntimeContext,
        root: Option<&Rc<ExpandedNode>>,
        ray: Point2<Window>,
        limit_one: bool,
        hit_invisible: bool,
        accum: &mut Vec<Rc<ExpandedNode>>,
    ) {
        let root = match root {
            Some(node) => match self.view.target_for_node(context, node) {
                Some(target) => Some(target.index),
                None => return,
            },
            None => None,
        };
        let Ok(hits) = self.view.hit_targets_under(
            Point::new(ray.x, ray.y),
            &self.scroll_offsets,
            hit_invisible,
            root,
        ) else {
            return;
        };
        // Truncate before resolving. A dead front recipient must not expose a
        // live node behind it to a single-target query.
        for target in hits
            .into_iter()
            .take(if limit_one { 1 } else { usize::MAX })
        {
            if let Some(node) = self.view.resolve_target(context, target) {
                accum.push(node);
            }
        }
    }

    pub(crate) fn dispatch<T: Clone + 'static>(
        &self,
        context: &Rc<RuntimeContext>,
        node: &Rc<ExpandedNode>,
        key: &str,
        event: &Event<T>,
        bubbles: bool,
    ) -> bool {
        let Some(target) = self.view.target_for_node(context, node) else {
            return event.cancelled();
        };
        self.view
            .dispatch_handlers(context, target, &self.scroll_offsets, key, event, bubbles)
            .unwrap_or_else(|_| event.cancelled())
    }

    pub(crate) fn activation(
        &self,
        context: &Rc<RuntimeContext>,
        node: &Rc<ExpandedNode>,
        event: &Event<pax_runtime_api::Click>,
        touch: bool,
    ) -> bool {
        let Some(target) = self.view.target_for_node(context, node) else {
            return event.cancelled();
        };
        let result = if touch {
            self.view
                .dispatch_tap(context, target, &self.scroll_offsets, event)
        } else {
            self.view
                .dispatch_click(context, target, &self.scroll_offsets, event)
        };
        result.unwrap_or_else(|_| event.cancelled())
    }

    pub(crate) fn topmost(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        ray: Point2<Window>,
    ) -> Option<Rc<ExpandedNode>> {
        let hits = self
            .view
            .hit_targets(Point::new(ray.x, ray.y), &self.scroll_offsets, false)
            .ok()?;
        let target = hits
            .first()
            .copied()
            .or_else(|| self.view.root.map(|root| self.view.target_at(root)))
            .filter(|target| self.view.resolve_target(context, *target).is_some());
        let previous = context.captured_input_hover.borrow_mut().take();
        let mut leaving = previous
            .as_ref()
            .map(|h| h.routing.view.template_route(h.target))
            .unwrap_or_default();
        let mut entering = target
            .map(|t| self.view.template_route(t))
            .unwrap_or_default();
        leaving.reverse();
        entering.reverse();
        let common = previous.as_ref().map_or(0, |old| {
            leaving
                .iter()
                .zip(&entering)
                .take_while(|(a, b)| old.routing.view.same_recipient(**a, &self.view, **b))
                .count()
        });
        if let Some(old) = &previous {
            for leave in leaving.into_iter().skip(common) {
                let _ = old.routing.view.dispatch_handlers(
                    context,
                    leave,
                    &old.routing.scroll_offsets,
                    "mouse_out",
                    &Event::new(MouseOut {}),
                    false,
                );
            }
        }
        for enter in entering.into_iter().skip(common) {
            let _ = self.view.dispatch_handlers(
                context,
                enter,
                &self.scroll_offsets,
                "mouse_over",
                &Event::new(MouseOver {}),
                false,
            );
        }
        // Hover callbacks can unmount the hit; never return that stale recipient.
        let node = target.and_then(|target| self.view.resolve_target(context, target));
        *context.last_topmost_element.borrow_mut() =
            node.as_ref().map(Rc::downgrade).unwrap_or_default();
        *context.captured_input_hover.borrow_mut() =
            target
                .filter(|_| node.is_some())
                .map(|target| CapturedInputHover {
                    routing: self.clone(),
                    target,
                });
        node
    }
}

impl Node {
    fn apply_scroll(
        &self,
        state: &mut Inherited,
        offsets: &HashMap<InputIdentity, (f64, f64)>,
        view: u64,
    ) -> Result<(), InputCaptureError> {
        let Some(scroll) = &self.scroll else {
            return Ok(());
        };
        let (x, y) = offsets
            .get(&self.identity.in_view(view))
            .copied()
            .unwrap_or(scroll.offset);
        if !x.is_finite() || !y.is_finite() {
            return Err(InputCaptureError::InvalidScroll);
        }
        state.affine *= Affine::translate((
            scroll.affine[0][0] * x + scroll.affine[1][0] * y,
            scroll.affine[0][1] * x + scroll.affine[1][1] * y,
        ));
        #[cfg(feature = "3d")]
        {
            for axis in 0..3 {
                state.world[axis] += scroll.world[0][axis] * x + scroll.world[1][axis] * y;
            }
            state.depth += scroll.depth[0] * x + scroll.depth[1] * y;
        }
        Ok(())
    }

    fn local_point(&self, point: Point, state: Inherited) -> Option<Point> {
        #[cfg(feature = "3d")]
        if self.camera.is_perspective() {
            return self.projection(state)?.local_point(point);
        }
        let transform = state.affine * self.transform;
        if !transform.determinant().is_finite() || transform.determinant() == 0. {
            return None;
        }
        let local = transform.inverse() * point;
        (local.x.is_finite() && local.y.is_finite()).then_some(local)
    }

    #[cfg(feature = "3d")]
    fn projection(&self, state: Inherited) -> Option<pax_runtime_api::math::ProjectedPlane> {
        crate::visibility::PresentationInputs {
            geometry: self.spatial,
            view: self.camera,
            scroll: state.world,
        }
        .project()
    }

    fn test(&self, point: Point, state: Inherited) -> (bool, Option<f64>) {
        #[cfg(feature = "3d")]
        {
            if let HitShape::Mesh(mesh) = &self.shape {
                use pax_runtime_api::math::{Generic, Transform3, Vector3};
                let mut world = Transform3::<Generic>::new(mesh.transform);
                let point = if self.camera.is_perspective() {
                    let [x, y, z] = state.world;
                    world = Transform3::translate(Vector3::new(x, y, z)) * world;
                    point
                } else {
                    state.affine.inverse() * point
                };
                let depth = self
                    .camera
                    .mesh(world.coeffs())
                    .and_then(|p| mesh.projected(p).depth_at(point.x, point.y));
                return (depth.is_some(), depth);
            }
            if !self.camera.is_perspective()
                && self.geometry.is_spatial()
                && self
                    .geometry
                    .plane()
                    .is_none_or(|plane| plane.normal().z.abs() <= 1e-12)
            {
                return (false, None);
            }
            if self.camera.is_perspective() {
                let hit = self.projection(state).is_some_and(|projection| {
                    projection
                        .mapping()
                        .inverse()
                        .and_then(|inverse| inverse.project(point))
                        .is_some_and(|local| {
                            projection.contains_local(local) && self.shape.contains_local(local)
                        })
                });
                return (hit, None);
            }
        }
        (
            self.shape
                .contains_local((state.affine * self.transform).inverse() * point),
            None,
        )
    }
}

impl RuntimeContext {
    /// Captures a settled application's input shapes, clipping policy, order,
    /// scroll axes and mounted identities. No event callbacks run. Native
    /// measurement/font readiness must be established by the chassis beforehand.
    #[doc(hidden)]
    fn capture_input_view(
        &self,
        limits: InputCaptureLimits,
        producer: &mut InputPublisher,
        reuse: Option<(&CapturedInputView, &HashSet<u32>)>,
    ) -> Result<CapturedInputView, InputCaptureError> {
        let _graph = self.property_graph.enter();
        if matches!(
            self.application.context().phase(),
            pax_runtime_api::AppPhase::Closing | pax_runtime_api::AppPhase::Closed
        ) {
            return Err(InputCaptureError::Unsettled("app_closing"));
        }
        if let Some(reason) = self.settled_surface_blocker() {
            if reason != "geometry" {
                return Err(InputCaptureError::Unsettled(reason));
            }
        }
        self.prepare_scene_geometry();
        if let Some(reason) = self.settled_surface_blocker() {
            return Err(InputCaptureError::Unsettled(reason));
        }
        let view = NEXT_VIEW
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| InputCaptureError::IdentityExhausted)?;
        let front = reuse.map(|(front, _)| front);
        let mut nodes = front.map(|v| v.nodes.clone()).unwrap_or_default();
        let mut children = front.map(|v| v.children.clone()).unwrap_or_default();
        let mut scroll_indices = imbl::OrdSet::new();
        // The presentation walk supplies hit order. Route-only ancestors supply
        // event context, but must never acquire hittable child edges.
        struct Visit {
            key: usize,
            parent: Option<usize>,
            escapes_clip: bool,
        }
        let mut order = Vec::<Visit>::new();
        let mut routes: Vec<Rc<ExpandedNode>> = Vec::new();
        let mut seen = HashSet::new();
        let mut presentation_node_count = None;
        let mut payload_bytes = 0usize;
        let viewport = self.globals().viewport.get();
        let mut stack = borrow!(self.root_expanded_node)
            .upgrade()
            .filter(|root| !root.is_render_source())
            .map(|root| vec![(root, None, false)])
            .unwrap_or_default();
        loop {
            let (node, parent, retained_exit, route_only) =
                if let Some((node, parent, exit)) = stack.pop() {
                    (node, parent, exit, false)
                } else {
                    presentation_node_count.get_or_insert(order.len());
                    let next = std::iter::from_fn(|| routes.pop())
                        .find(|node| !seen.contains(&Rc::as_ptr(node)));
                    let Some(node) = next else {
                        break;
                    };
                    (node, None, false, true)
                };
            if order.len() >= limits.nodes {
                return Err(InputCaptureError::Capacity);
            }
            node.compute_flattened_projected_children();
            let index = order.len();
            seen.insert(Rc::as_ptr(&node));
            let template_parent = node.template_parent.upgrade();
            let route_parent = route_only
                .then(|| node.presentation_parent_node())
                .flatten();
            let parent_key = parent
                .map(|i: usize| order[i].key)
                .or_else(|| route_parent.as_ref().map(|p| p.id.to_u32() as usize));
            let template_key = template_parent.as_ref().map(|p| p.id.to_u32() as usize);
            // Keep unresolved lexical and presentation parents alive until their
            // records exist. Published views retain only their IDs and weak mounts.
            routes.extend(
                template_parent
                    .into_iter()
                    .chain(route_parent)
                    .filter(|p| !seen.contains(&Rc::as_ptr(p))),
            );
            let id = node.id.to_u32();
            // Subscriptions cover reactive inputs. Incarnation and both parent
            // relationships also govern the meaning of a retained record.
            let retained = reuse.and_then(|(front, dirty)| {
                let old = front.nodes.get(id as usize)?;
                (!dirty.contains(&id)
                    && old.mounted.as_ptr() == Rc::as_ptr(&node)
                    && old.identity.mount_epoch == node.mount_epoch.get()
                    && old.parent == parent_key
                    && old.template_parent == template_key
                    && old.retained_exit == retained_exit
                    && producer.watches.get(&id)?.geometry.same_parent(&node))
                .then(|| old.clone())
            });
            let record = match retained {
                Some(record) => record,
                None => {
                    let mut record = producer.record(self, &node)?.node;
                    record.parent = parent_key;
                    record.template_parent = template_key;
                    record.exit |= retained_exit;
                    record.retained_exit = retained_exit;
                    let record = Rc::new(record);
                    nodes.0.insert(id as usize, record.clone());
                    record
                }
            };
            payload_bytes = payload_bytes
                .checked_add(record.payload_bytes())
                .ok_or(InputCaptureError::Capacity)?;
            if payload_bytes > limits.payload_bytes {
                return Err(InputCaptureError::Capacity);
            }
            let key = id as usize;
            order.push(Visit {
                key,
                parent,
                escapes_clip: record.unclippable,
            });
            if record.scroll.is_some() {
                scroll_indices.insert(key);
            }
            let mut current = if route_only {
                Vec::new()
            } else {
                node.presentation_children(self)
            };
            current.retain(|child| !child.is_render_source());
            let ids = || current.iter().map(|child| child.id.to_u32() as usize);
            if current.is_empty() {
                if children.contains_key(&key) {
                    children.remove(&key);
                }
            } else if !children
                .get(&key)
                .is_some_and(|old| old.iter().copied().eq(ids()))
            {
                children.insert(key, Rc::new(ids().collect()));
            }
            let exiting = borrow!(node.exiting_children);
            stack.extend(current.into_iter().rev().map(|child| {
                let retained = exiting.iter().any(|old| Rc::ptr_eq(old, &child));
                (child, Some(index), retained)
            }));
        }
        // Routes above were followed to closure, including lexical ancestors
        // outside the presentation tree. Their node IDs are stable snapshot slots.
        if let Some(reason) = self.settled_surface_blocker() {
            return Err(InputCaptureError::Unsettled(reason));
        }
        if reuse.is_some() {
            producer.watches.retain(|&id, _| {
                let key = id as usize;
                if nodes
                    .get(key)
                    .is_some_and(|n| seen.contains(&n.mounted.as_ptr()))
                {
                    return true;
                }
                nodes.0.remove(&key);
                children.remove(&key);
                false
            });
        }
        let count = presentation_node_count.unwrap_or(0);
        payload_bytes = payload_bytes
            .checked_add(order.len() * std::mem::size_of::<(u32, usize)>())
            .ok_or(InputCaptureError::Capacity)?;
        if payload_bytes > limits.payload_bytes {
            return Err(InputCaptureError::Capacity);
        }
        for i in (0..count).rev() {
            if let (Some(parent), true) = (order[i].parent, order[i].escapes_clip) {
                order[parent].escapes_clip = true;
            }
        }
        Ok(CapturedInputView {
            application: self.property_graph.id(),
            view,
            nodes,
            root: (count > 0).then(|| order[0].key),
            children,
            subtree_escapes: order
                .into_iter()
                .filter_map(|visit| visit.escapes_clip.then_some(visit.key))
                .collect(),
            scroll_indices,
            #[cfg(test)]
            query_work: Cell::new(0),
            payload_bytes,
            viewport: pax_runtime_api::Viewport::new(viewport.bounds.0, viewport.bounds.1),
            #[cfg(feature = "3d")]
            spatial_possible: self.globals().renderer_is_gpu
                && self.scene_geometry.borrow().may_have_spatial_nodes(),
        })
    }
}

type WindowLayout =
    crate::TransformAndBounds<crate::node_interface::NodeLocal, pax_runtime_api::Window>;

// These are source values, including the parent's full transform and projected
// bounds. A primitive may have read any part of them while preparing a shape.
#[derive(Clone)]
struct GeometryInputs {
    parent_layout: Option<(Weak<ExpandedNode>, WindowLayout)>,
    projected_bounds: (f64, f64),
}

impl GeometryInputs {
    fn same_parent(&self, node: &ExpandedNode) -> bool {
        self.parent_layout
            .as_ref()
            .map(|(parent, _)| parent.as_ptr())
            == node.render_parent_node().as_ref().map(Rc::as_ptr)
    }

    // Only geometry sources fired. A local primitive's shape and clip can be
    // reused across a pose change when both coordinate charts keep their
    // dimensions. Arbitrary shapes, posed meshes and scroll bases recapture.
    fn local_pose(&mut self, node: &Rc<ExpandedNode>, front: &Node) -> Option<Node> {
        let instance = borrow!(node.instance_node);
        if front.scroll.is_some()
            || !instance.geometry_is_scroller_local()
            || instance.scrolls_content(node)
        {
            return None;
        }
        #[cfg(feature = "3d")]
        if matches!(front.shape, HitShape::Mesh(_)) {
            return None;
        }
        let layout = node.transform_and_bounds.get();
        let projected = node.presentation_transform_and_bounds.get();
        if layout.bounds != front.bounds || projected.bounds != self.projected_bounds {
            return None;
        }
        match (&mut self.parent_layout, node.render_parent_node()) {
            (Some((source, old)), Some(parent)) if source.as_ptr() == Rc::as_ptr(&parent) => {
                let current = parent.transform_and_bounds.get();
                if old.bounds != current.bounds {
                    return None;
                }
                *old = current;
            }
            (None, None) => {}
            _ => return None,
        }
        let mut next = front.clone();
        next.layout = layout;
        next.transform = Affine::from(projected.transform);
        #[cfg(feature = "3d")]
        {
            next.spatial = node.spatial_geometry.get();
            next.geometry = node.presentation_geometry.get();
        }
        Some(next)
    }

    fn unchanged(&self, node: &Rc<ExpandedNode>, front: &Node) -> bool {
        if node.transform_and_bounds.get() != front.layout {
            return false;
        }
        let projected = node.presentation_transform_and_bounds.get();
        if Affine::from(projected.transform) != front.transform
            || projected.bounds != self.projected_bounds
        {
            return false;
        }
        #[cfg(feature = "3d")]
        if node.spatial_geometry.get() != front.spatial
            || node.presentation_geometry.get() != front.geometry
        {
            return false;
        }
        match (&self.parent_layout, node.render_parent_node()) {
            (Some((source, layout)), Some(parent)) => {
                source.as_ptr() == Rc::as_ptr(&parent)
                    && *layout == parent.transform_and_bounds.get()
            }
            (None, None) => true,
            _ => false,
        }
    }
}

/// Resolved geometry for one mounted incarnation. Relationships remain identities
/// until the immutable presentation's assembler resolves its routing ancestry.
#[derive(Clone)]
pub(crate) struct InputCaptureRecord {
    node: Node,
    geometry_inputs: GeometryInputs,
    template_parent: Option<InputIdentity>,
}

impl RuntimeContext {
    pub(crate) fn capture_input_record(&self, node: &Rc<ExpandedNode>) -> InputCaptureRecord {
        let viewport = self.globals().viewport.get();
        let instance = borrow!(node.instance_node);
        let shape = instance.prepare_hit_shape(&node);

        let scroll = instance.scrolls_content(&node).then(|| {
            let x = self
                .compute_scroller_presentation_transform(&node, (1., 0.))
                .as_coeffs();
            let y = self
                .compute_scroller_presentation_transform(&node, (0., 1.))
                .as_coeffs();
            #[cfg(feature = "3d")]
            let world = node.spatial_geometry.get().world_transform().coeffs();
            #[cfg(feature = "3d")]
            let geometry = node.presentation_geometry.get();
            #[cfg(feature = "3d")]
            let projected = geometry.world_transform().coeffs();
            Scroll {
                offset: self.scroller_presentation_offset(&node),
                affine: [[x[4], x[5]], [y[4], y[5]]],
                #[cfg(feature = "3d")]
                world: [
                    [-world[0], -world[1], -world[2]],
                    [-world[4], -world[5], -world[6]],
                ],
                #[cfg(feature = "3d")]
                depth: if geometry.is_spatial() {
                    [-projected[2], -projected[6]]
                } else {
                    [0.; 2]
                },
            }
        });
        let cp = node.get_common_properties();
        let cp = borrow!(cp);
        let parent = node.render_parent_node();
        let parent_layout = parent
            .as_ref()
            .map(|p| (Rc::downgrade(p), p.transform_and_bounds.get()));
        let bounds_parent = parent
            .as_ref()
            .zip(parent_layout.as_ref())
            .map(|(parent, (_, layout))| {
                let properties = parent.get_common_properties();
                let properties = borrow!(properties);
                crate::layout::apply_padding_frame(
                    *layout,
                    properties.padding_x.get(),
                    properties.padding_y.get(),
                )
                .bounds
            })
            .unwrap_or(viewport.bounds);
        let projected = node.presentation_transform_and_bounds.get();
        let record = Node {
            identity: InputIdentity {
                id: node.id.to_u32(),
                mount_epoch: node.mount_epoch.get(),
                view: 0,
            },
            mounted: Rc::downgrade(&node),
            parent: None,
            template_parent: None,
            flattened_slot_controller: node.attached.get() == 0
                && instance.base().flags().invisible_to_slot
                && !node.is_render_source(),
            layout: node.transform_and_bounds.get(),
            bounds_parent,
            measured_size: node.measured_size.get(),
            subtree_layout_hull: node.subtree_layout_hull.get(),
            exit: node.transition_phase.get() == TRANSITION_PHASE_EXIT,
            retained_exit: false,
            unclippable: cp.unclippable.get().unwrap_or(false),
            invisible: instance.base().flags().invisible_to_raycasting,
            raycastable: cp._raycastable.get().unwrap_or(true),
            wheel_cancellable: cp.wheel_events.get()
                == Some(pax_runtime_api::WheelEvents::Cancellable),
            clips: instance.clips_content(&node),
            scroll,
            shape,
            transform: Affine::from(projected.transform),
            bounds: node.transform_and_bounds.get().bounds,
            #[cfg(feature = "3d")]
            geometry: node.presentation_geometry.get(),
            #[cfg(feature = "3d")]
            spatial: node.spatial_geometry.get(),
            #[cfg(feature = "3d")]
            camera: node.presentation_view.get(),
            #[cfg(feature = "3d")]
            layer: node.presentation_layer.get(),
        };
        drop(cp);
        drop(instance);
        let identity = |node: Rc<ExpandedNode>| InputIdentity {
            id: node.id.to_u32(),
            mount_epoch: node.mount_epoch.get(),
            view: 0,
        };
        InputCaptureRecord {
            geometry_inputs: GeometryInputs {
                parent_layout,
                projected_bounds: projected.bounds,
            },
            node: record,
            template_parent: node.template_parent.upgrade().map(identity),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum TopologyChange {
    #[default]
    None,
    KnownOrigins,
    UnknownOrigin,
}

#[derive(Default)]
pub(super) struct CaptureChanges {
    pub(super) dirty: HashSet<u32>,
    topology: TopologyChange,
}

impl CaptureChanges {
    pub(super) fn structure_changed(&mut self, origin: Option<u32>) {
        if let Some(id) = origin {
            self.dirty.insert(id);
            if self.topology == TopologyChange::None {
                self.topology = TopologyChange::KnownOrigins;
            }
        } else {
            self.topology = TopologyChange::UnknownOrigin;
        }
    }
}

struct InputWatches {
    geometry: GeometryInputs,
    geometry_subscription: DependencyEventSubscription,
    other_subscription: DependencyEventSubscription,
}

/// Owns the incremental capture independently of the revision installed for input.
#[derive(Default)]
pub struct InputPublisher {
    latest: Option<Rc<CapturedInputView>>,
    changes: DependencyEventQueue,
    watches: HashMap<u32, InputWatches>,
}

impl InputPublisher {
    fn record(
        &mut self,
        ctx: &RuntimeContext,
        node: &Rc<ExpandedNode>,
    ) -> Result<InputCaptureRecord, InputCaptureError> {
        let reads = PropertyReadScope::enter(16384);
        let record = ctx.capture_input_record(node);
        let deps = reads.finish().map_err(|_| InputCaptureError::Capacity)?;
        let id = node.id.to_u32();
        let layout = node.transform_and_bounds.untyped().get_id();
        let projected = node.presentation_transform_and_bounds.untyped().get_id();
        let parent = node
            .render_parent_node()
            .map(|p| p.transform_and_bounds.untyped().get_id());
        #[cfg(feature = "3d")]
        let spatial = [
            node.spatial_geometry.untyped().get_id(),
            node.presentation_geometry.untyped().get_id(),
        ];
        let (geometry, other): (Vec<_>, Vec<_>) = deps.into_iter().partition(|dep| {
            let source = dep.get_id();
            #[cfg(feature = "3d")]
            if spatial.contains(&source) {
                return true;
            }
            source == layout || source == projected || Some(source) == parent
        });
        let key = u64::from(id) << 1;
        if let Some(watches) = self.watches.get_mut(&id) {
            watches.geometry = record.geometry_inputs.clone();
            watches.geometry_subscription.refresh(&geometry);
            watches.other_subscription.refresh(&other);
        } else {
            self.watches.insert(
                id,
                InputWatches {
                    geometry: record.geometry_inputs.clone(),
                    geometry_subscription: self.changes.watch(&geometry, key),
                    other_subscription: self.changes.watch(&other, key | 1),
                },
            );
        }
        Ok(record)
    }

    pub fn capture(
        &mut self,
        ctx: &RuntimeContext,
    ) -> Result<Rc<CapturedInputView>, InputCaptureError> {
        let _graph = ctx.property_graph.enter();
        if let Some(reason) = ctx.settled_surface_blocker() {
            if reason != "geometry" {
                return Err(InputCaptureError::Unsettled(reason));
            }
        }
        ctx.prepare_scene_geometry();
        if let Some(reason) = ctx.settled_surface_blocker() {
            return Err(InputCaptureError::Unsettled(reason));
        }
        let limits = InputCaptureLimits {
            nodes: 100_000,
            payload_bytes: 128 * 1024 * 1024,
        };
        let CaptureChanges {
            mut dirty,
            topology,
        } = ctx
            .input_capture_changes
            .borrow_mut()
            .replace(CaptureChanges::default())
            .unwrap_or_default();
        let mut geometry = Vec::new();
        while let Some(key) = self.changes.pop() {
            let id = (key >> 1) as u32;
            if key & 1 == 0 {
                geometry.push(id);
            } else {
                dirty.insert(id);
            }
        }
        // Membership and lexical routes can change far from the notifying node
        // (slots, overlays, retained exits). Rediscover edges from the live tree;
        // only invalidated or newly related bodies need new property reads.
        if topology == TopologyChange::KnownOrigins {
            dirty.extend(geometry);
            return self.capture_topology(ctx, limits, Some(&dirty));
        }
        if let Some(previous) = self
            .latest
            .clone()
            .filter(|_| topology == TopologyChange::None)
        {
            // With unchanged membership, only a record in the displayed view
            // can receive a replacement. Parked nodes can still signal a pose.
            dirty.retain(|id| previous.nodes.get(*id as usize).is_some());
            // A world-space property can be invalidated throughout a subtree
            // whose resolved pose did not move. Keep the immutable body and its
            // dynamic read subscriptions if every potentially changed source
            // is one of the checked geometry values and all still compare equal.
            let mut poses = Vec::new();
            for id in geometry {
                if dirty.contains(&id) {
                    continue;
                }
                let Some(old) = previous.nodes.get(id as usize) else {
                    continue;
                };
                let Some(node) = ctx.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
                    return self.capture_topology(ctx, limits, None);
                };
                if old.mounted.ptr_eq(&Rc::downgrade(&node))
                    && old.identity.mount_epoch == node.mount_epoch.get()
                {
                    if let Some(watches) = self.watches.get_mut(&id) {
                        if watches.geometry.unchanged(&node, old) {
                            continue;
                        }
                        if let Some(updated) = watches.geometry.local_pose(&node, old) {
                            poses.push((id, Rc::new(updated)));
                            continue;
                        }
                    }
                }
                dirty.insert(id);
            }
            if dirty.is_empty() && poses.is_empty() {
                return Ok(previous);
            }
            let mut next = (*previous).clone();
            next.view = NEXT_VIEW
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| InputCaptureError::IdentityExhausted)?;
            for (id, record) in poses {
                next.nodes.0.insert(id as usize, record);
            }
            for id in dirty {
                let Some(old) = previous.nodes.get(id as usize) else {
                    continue;
                };
                let Some(node) = ctx.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
                    return self.capture_topology(ctx, limits, None);
                };
                let updated = self.record(ctx, &node)?;
                let mut record = updated.node;
                if record.identity != old.identity
                    || record.unclippable != old.unclippable
                    || record.scroll.is_some() != old.scroll.is_some()
                    || old.template_parent != updated.template_parent.map(|id| id.id as usize)
                    || !record.mounted.ptr_eq(&old.mounted)
                {
                    return self.capture_topology(ctx, limits, None);
                }
                next.payload_bytes = next.payload_bytes - old.path_bytes() + record.path_bytes();
                if next.payload_bytes > limits.payload_bytes {
                    return Err(InputCaptureError::Capacity);
                }
                record.parent = old.parent;
                record.template_parent = old.template_parent;
                record.retained_exit = old.retained_exit;
                record.exit |= old.retained_exit;
                next.nodes.0.insert(id as usize, Rc::new(record));
            }
            let viewport = ctx.globals().viewport.get();
            next.viewport = pax_runtime_api::Viewport::new(viewport.bounds.0, viewport.bounds.1);
            #[cfg(feature = "3d")]
            {
                next.spatial_possible = ctx.globals().renderer_is_gpu
                    && ctx.scene_geometry.borrow().may_have_spatial_nodes();
            }
            let next = Rc::new(next);
            self.latest = Some(next.clone());
            return Ok(next);
        }
        self.capture_topology(ctx, limits, None)
    }

    fn capture_topology(
        &mut self,
        ctx: &RuntimeContext,
        limits: InputCaptureLimits,
        dirty: Option<&HashSet<u32>>,
    ) -> Result<Rc<CapturedInputView>, InputCaptureError> {
        let previous = self.latest.take().filter(|_| dirty.is_some());
        let reuse = previous.as_deref().zip(dirty);
        if reuse.is_none() {
            self.watches.clear();
            self.changes = Default::default();
        }
        let next = Rc::new(ctx.capture_input_view(limits, self, reuse)?);
        self.latest = Some(next.clone());
        Ok(next)
    }
}

impl CapturedInputView {
    pub fn enter(
        self: &Rc<Self>,
        ctx: &Rc<RuntimeContext>,
        observed: &HashMap<u32, (f64, f64)>,
    ) -> Result<CapturedInputScope, InputCaptureError> {
        let offsets = self
            .scroll_hosts()
            .filter_map(|key| {
                let record = self.nodes.get(key.id as usize)?;
                let current = ctx.get_expanded_node_by_eid(ExpandedNodeIdentifier(key.id))?;
                if current.mount_epoch.get() != key.mount_epoch
                    || !record.mounted.ptr_eq(&Rc::downgrade(&current))
                {
                    return None;
                }
                observed.get(&key.id).map(|&offset| (key, offset))
            })
            .collect();
        ctx.enter_captured_input(self.clone(), offsets)
    }
}
