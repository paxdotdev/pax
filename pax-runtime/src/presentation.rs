//! Transient placement, independent of authored ownership and reconciliation.

use crate::node_interface::NodeLocal;
use crate::{ExpandedNode, ExpandedNodeIdentifier, NodeError, RuntimeContext};
use pax_runtime_api::math::{Transform2, Vector2};
use pax_runtime_api::{Interpolatable, LocalProperty, NativeLiquidGlassScope, Window};
use std::{
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy)]
pub(crate) struct MoveOrigin {
    pub transform: Transform2<NodeLocal, Window>,
    pub bounds: (f64, f64),
}
impl Interpolatable for MoveOrigin {}

pub(crate) fn bind_move_geometry(
    node: &Rc<ExpandedNode>,
    context: &Rc<RuntimeContext>,
    target: LocalProperty<crate::SpatialGeometry>,
) -> LocalProperty<crate::SpatialGeometry> {
    let origin = node.move_origin.clone();
    let progress = node.move_progress.clone();
    let bounds = node.layout_bounds.clone();
    let deps = [
        target.untyped(),
        origin.untyped(),
        progress.untyped(),
        bounds.untyped(),
    ];
    let weak_node = Rc::downgrade(node);
    let weak_context = Rc::downgrade(context);
    LocalProperty::computed(
        move || {
            let target = target.get();
            let Some(origin) = origin.get() else {
                return target;
            };
            let progress = progress.get();
            if progress == 1.0 && !node_is_flying(&weak_node) {
                return target;
            }
            let (Some(node), Some(context), Some(projected)) = (
                weak_node.upgrade(),
                weak_context.upgrade(),
                target.orthographic_projection(),
            ) else {
                return target;
            };
            let bounds = bounds.get();
            if bounds.0 <= 1e-9 || bounds.1 <= 1e-9 {
                return target;
            }
            let scroll: Transform2<Window> = Transform2::new(
                context
                    .destination_scroll_transform_for_node(&node)
                    .as_coeffs(),
            );
            let destination = scroll
                * projected.transform
                * Transform2::scale_sep(Vector2::new(projected.bounds.0, projected.bounds.1));
            let interpolated = origin.transform.interpolate(&destination, progress);
            crate::SpatialGeometry::from(crate::TransformAndBounds {
                transform: (if node.move_in_flight.get() {
                    Transform2::identity()
                } else {
                    scroll.inverse()
                }) * interpolated
                    * Transform2::scale_sep(Vector2::new(1.0 / bounds.0, 1.0 / bounds.1)),
                bounds,
            })
        },
        &deps,
    )
}

fn node_is_flying(node: &Weak<ExpandedNode>) -> bool {
    node.upgrade().is_some_and(|node| node.move_in_flight.get())
}

#[derive(Clone)]
pub(crate) struct FlightAnchor {
    // Keep the first surviving lower contribution as the insertion anchor if
    // the departing Overlay disappears. Higher contributions stay above us.
    above: Vec<ExpandedNodeIdentifier>,
    continuing: bool,
}

struct Flight {
    node: Weak<ExpandedNode>,
    anchor: FlightAnchor,
    serial: u64,
    duration: pax_runtime_api::Duration,
    start_frame: u64,
    start_millis: u64,
}

#[derive(Clone)]
pub(crate) struct AuthoredAttachment {
    pub parent: Weak<ExpandedNode>,
    pub frame: LocalProperty<Option<ExpandedNodeIdentifier>>,
    pub glass: LocalProperty<Option<NativeLiquidGlassScope>>,
}

#[derive(Clone)]
struct Placement {
    node: Weak<ExpandedNode>,
    parent: Weak<ExpandedNode>,
    // Anchor only to authored children, so moving two neighbors cannot create
    // circular insertion dependencies. Index is the fallback if the anchor ends.
    before: Option<ExpandedNodeIdentifier>,
    index: usize,
    detached: bool,
}

#[derive(Default)]
pub(crate) struct PresentationState {
    placements: HashMap<ExpandedNodeIdentifier, Placement>,
    overlays: HashSet<ExpandedNodeIdentifier>,
    flights: HashMap<ExpandedNodeIdentifier, Flight>,
    next_flight: u64,
}

impl RuntimeContext {
    pub(crate) fn departing_flight(&self, node: &Rc<ExpandedNode>) -> Option<FlightAnchor> {
        let mut current = Some(node.clone());
        while let Some(node) = current {
            if let Some(flight) = self.presentation.borrow().flights.get(&node.id) {
                let mut anchor = flight.anchor.clone();
                anchor.continuing = true;
                return Some(anchor);
            }
            if node.instance_node.borrow().is_overlay() {
                let mut overlays = Vec::new();
                if let Some(root) = self.get_root_expanded_node() {
                    collect_overlays(&root, &mut overlays);
                }
                let above = overlays
                    .into_iter()
                    .skip_while(|n| n.id != node.id)
                    .map(|n| n.id)
                    .collect();
                return Some(FlightAnchor {
                    above,
                    continuing: false,
                });
            }
            current = node.render_parent_node();
        }
        None
    }

    pub(crate) fn begin_move_flight(
        self: &Rc<Self>,
        node: &Rc<ExpandedNode>,
        departing: Option<FlightAnchor>,
        duration: pax_runtime_api::Duration,
    ) {
        let destination = self.departing_flight(node);
        let anchor = match (departing, destination) {
            (None, None) => return,
            (Some(from), Some(to)) => {
                if from.above == to.above && !from.continuing {
                    return;
                }
                let contributions = self.viewport_contributions();
                let rank = |anchor: &FlightAnchor| {
                    contributions
                        .iter()
                        .position(|n| anchor.above.contains(&n.id))
                        .unwrap_or(contributions.len())
                };
                if rank(&from) <= rank(&to) {
                    from
                } else {
                    to
                }
            }
            (Some(anchor), None) | (None, Some(anchor)) => anchor,
        };
        node.mark_canvas_subtree_dirty_for_reparent(self, true);
        {
            let mut state = self.presentation.borrow_mut();
            let serial = state.next_flight;
            state.next_flight += 1;
            let globals = self.globals();
            state.flights.insert(
                node.id,
                Flight {
                    node: Rc::downgrade(node),
                    anchor,
                    serial,
                    duration,
                    start_frame: globals.elapsed_frames.get(),
                    start_millis: globals.elapsed_millis.get(),
                },
            );
        }
        node.move_in_flight.set(true);
        node.rebind_move_presentation(self);
        node.mark_canvas_subtree_dirty_for_reparent(self, false);
        self.presentation_changed();
    }

    pub(crate) fn finish_move_flights(self: &Rc<Self>) {
        if self.presentation.borrow().flights.is_empty() {
            return;
        }
        // Read animation properties outside the registry borrow: completion may
        // drain effects that need the effective tree.
        let globals = self.globals();
        let frames = globals.elapsed_frames.get();
        let millis = globals.elapsed_millis.get();
        let nodes: Vec<_> = self
            .presentation
            .borrow()
            .flights
            .values()
            .filter(|f| {
                let (elapsed, duration) = if f.duration.is_frame_based() {
                    (
                        frames.saturating_sub(f.start_frame) as f64,
                        f.duration.raw_value(),
                    )
                } else {
                    (
                        millis.saturating_sub(f.start_millis) as f64,
                        f.duration.as_milliseconds_f64(),
                    )
                };
                elapsed >= duration.max(0.0)
            })
            .filter_map(|f| f.node.upgrade())
            .collect();
        for node in nodes {
            // Easing curves can cross 1 before their duration ends. Completion
            // follows the animation clock, never an intermediate eased value.
            node.move_progress.get();
            node.mark_canvas_subtree_dirty_for_reparent(self, true);
            self.cancel_move_flight(&node);
            node.move_progress.cancel_transitions();
            node.move_origin.set(None);
            node.rebind_move_presentation(self);
            node.mark_canvas_subtree_dirty_for_reparent(self, false);
        }
    }

    fn cancel_move_flight(&self, node: &ExpandedNode) {
        node.move_in_flight.set(false);
        let removed = self
            .presentation
            .borrow_mut()
            .flights
            .remove(&node.id)
            .is_some();
        if removed {
            self.presentation_changed();
        }
    }

    fn viewport_contributions(&self) -> Vec<Rc<ExpandedNode>> {
        let mut result = Vec::new();
        if let Some(root) = self.get_root_expanded_node() {
            collect_overlays(&root, &mut result);
        }
        let state = self.presentation.borrow();
        let mut flights: Vec<_> = state.flights.values().collect();
        flights.sort_by_key(|f| f.serial);
        for flight in flights {
            let Some(node) = flight.node.upgrade().filter(|n| n.attached.get() > 0) else {
                continue;
            };
            let index = result
                .iter()
                .position(|n| flight.anchor.above.contains(&n.id))
                .unwrap_or(result.len());
            result.insert(index, node);
        }
        result
    }

    pub(crate) fn presentation_domain(&self, id: ExpandedNodeIdentifier) -> u32 {
        let contributions = self.viewport_contributions();
        contributions
            .iter()
            .position(|n| n.id == id)
            .map(|index| (contributions.len() - index) as u32)
            .unwrap_or(0)
    }

    pub(crate) fn flight_children(
        &self,
        parent: &ExpandedNode,
        mut children: Vec<Rc<ExpandedNode>>,
    ) -> Vec<Rc<ExpandedNode>> {
        if self.presentation.borrow().flights.is_empty() {
            return children;
        }
        let is_root = self
            .get_root_expanded_node()
            .is_some_and(|n| n.id == parent.id);
        {
            let state = self.presentation.borrow();
            children.retain(|n| {
                !state.flights.contains_key(&n.id) && !(is_root && state.overlays.contains(&n.id))
            });
        }
        if is_root {
            let mut contributions = self.viewport_contributions();
            contributions.extend(children);
            contributions
        } else {
            children
        }
    }

    pub(crate) fn register_overlay(&self, node: &ExpandedNode) {
        self.presentation.borrow_mut().overlays.insert(node.id);
        self.presentation_changed();
    }

    pub(crate) fn authored_order_changed(&self) {
        if !self.presentation.borrow().overlays.is_empty() {
            self.presentation_changed();
        }
    }

    pub(crate) fn is_transferred(&self, node: ExpandedNodeIdentifier) -> bool {
        self.presentation.borrow().placements.contains_key(&node)
    }

    pub(crate) fn is_detached(&self, node: ExpandedNodeIdentifier) -> bool {
        self.presentation
            .borrow()
            .placements
            .get(&node)
            .is_some_and(|p| p.detached)
    }

    pub(crate) fn detached_nodes(self: &Rc<Self>, owner: &Rc<ExpandedNode>) -> crate::NodeQuery {
        let mut nodes: Vec<_> = self
            .presentation
            .borrow()
            .placements
            .values()
            .filter(|p| p.detached)
            .filter_map(|p| p.node.upgrade())
            .filter(|n| {
                n.containing_component
                    .upgrade()
                    .is_some_and(|c| c.id == owner.id)
            })
            .collect();
        nodes.sort_by_key(|n| n.id.to_u32());
        crate::NodeQuery::from_nodes(nodes.iter().map(|n| crate::NodeRef::new(n, self)).collect())
    }

    pub(crate) fn visiting_nodes(self: &Rc<Self>, host: &Rc<ExpandedNode>) -> crate::NodeQuery {
        let placements: Vec<_> = self
            .presentation
            .borrow()
            .placements
            .values()
            .filter(|p| !p.detached)
            .cloned()
            .collect();
        let mut nodes: Vec<_> = placements
            .into_iter()
            .filter_map(|p| {
                let parent = p.parent.upgrade()?;
                let node = p.node.upgrade()?;
                (authored_within(&parent, host.id) && !authored_within(&node, host.id))
                    .then_some(node)
            })
            .collect();
        nodes.sort_by_key(|n| n.id.to_u32());
        crate::NodeQuery::from_nodes(nodes.iter().map(|n| crate::NodeRef::new(n, self)).collect())
    }

    pub(crate) fn placed_children(
        &self,
        parent: &ExpandedNode,
        children: Vec<Rc<ExpandedNode>>,
    ) -> Vec<Rc<ExpandedNode>> {
        let state = self.presentation.borrow();
        if state.placements.is_empty() && state.overlays.is_empty() {
            return children;
        }
        let is_root = self
            .get_root_expanded_node()
            .is_some_and(|n| n.id == parent.id);
        let has_overlays = !state.overlays.is_empty();
        let mut incoming: Vec<_> = state
            .placements
            .values()
            .filter(|p| !p.detached && p.parent.upgrade().is_some_and(|n| n.id == parent.id))
            .cloned()
            .collect();
        let children: Vec<_> = children
            .into_iter()
            .filter(|n| !state.placements.contains_key(&n.id) && !state.overlays.contains(&n.id))
            .collect();
        drop(state);
        if incoming.is_empty() {
            if is_root && has_overlays {
                let mut overlays = Vec::new();
                collect_overlays(parent, &mut overlays);
                overlays.extend(children);
                return overlays;
            }
            return children;
        }
        // Control-flow and Slot nodes transport content but contribute no paint
        // or clipping hooks. Their children retain their bound geometry, while
        // this semantic sequence permits insertion between repeated roots.
        let mut result = Vec::new();
        for child in children {
            if crate::node_query::transparent(&child) {
                flatten_presentation_children(&child, self, &mut result);
            } else {
                result.push(child);
            }
        }
        incoming.sort_by_key(|p| p.index);
        for placement in incoming {
            let Some(node) = placement.node.upgrade().filter(|n| n.attached.get() > 0) else {
                continue;
            };
            let index = placement
                .before
                .and_then(|anchor| result.iter().position(|n| n.id == anchor))
                .unwrap_or(placement.index.min(result.len()));
            result.insert(index, node);
        }
        if is_root && has_overlays {
            let mut overlays = Vec::new();
            collect_overlays(parent, &mut overlays);
            overlays.extend(result);
            overlays
        } else {
            result
        }
    }

    pub(crate) fn move_node(
        self: &Rc<Self>,
        node: &Rc<ExpandedNode>,
        parent: &Rc<ExpandedNode>,
        index: usize,
    ) -> Result<(), NodeError> {
        if parent.attached.get() == 0
            || !parent.instance_node.borrow().accepts_transferred_children()
            || node.is_render_source()
            || node.instance_node.borrow().is_overlay()
            || parent.is_render_source()
            || crate::node_query::transparent(node)
            || node.authored_attachment.borrow().is_none()
        {
            return Err(NodeError::InvalidDestination);
        }
        // Validate both current placement and authored ancestry. The latter
        // guarantees that a later explicit return-home cannot introduce a cycle.
        if ancestry_contains(parent, node.id) {
            return Err(NodeError::Cycle);
        }
        let mut children = crate::node_query::content_children(parent, self);
        children.retain(|child| child.id != node.id);
        if index > children.len() {
            return Err(NodeError::IndexOutOfBounds {
                index,
                len: children.len(),
            });
        }
        children.insert(index, node.clone());

        node.mark_canvas_subtree_dirty_for_reparent(self, true);
        self.cancel_move_flight(node);
        if node.move_origin.get().is_some() {
            node.move_progress.cancel_transitions();
            node.move_origin.set(None);
        }
        {
            let mut state = self.presentation.borrow_mut();
            state.placements.insert(
                node.id,
                Placement {
                    node: Rc::downgrade(node),
                    parent: Rc::downgrade(parent),
                    before: None,
                    index,
                    detached: false,
                },
            );
            let mut anchor = None;
            for (index, child) in children.iter().enumerate().rev() {
                if let Some(placement) = state.placements.get_mut(&child.id) {
                    placement.before = anchor;
                    placement.index = index;
                } else {
                    anchor = Some(child.id);
                }
            }
        }
        node.container_frame.set(None);
        node.ensure_parent_bindings(
            parent,
            self,
            &parent.parent_frame,
            &parent.liquid_glass_scope,
        );
        if node.attached.get() == 0 {
            node.recurse_mount(self);
            // Remount replaces common-property handles, including suspension.
            node.ensure_parent_bindings(
                parent,
                self,
                &parent.parent_frame,
                &parent.liquid_glass_scope,
            );
        }
        node.rebind_parent_bounds_for_mounted_subtree(self);
        node.mark_canvas_subtree_dirty_for_reparent(self, false);
        self.presentation_changed();
        Ok(())
    }

    pub(crate) fn move_home(self: &Rc<Self>, node: &Rc<ExpandedNode>) -> Result<(), NodeError> {
        if !self.is_transferred(node.id) {
            if node.move_in_flight.get() {
                node.mark_canvas_subtree_dirty_for_reparent(self, true);
                self.cancel_move_flight(node);
                node.move_progress.cancel_transitions();
                node.move_origin.set(None);
                node.rebind_move_presentation(self);
                node.mark_canvas_subtree_dirty_for_reparent(self, false);
            }
            return Ok(());
        }
        let attachment = node
            .authored_attachment
            .borrow()
            .clone()
            .ok_or(NodeError::HomeUnavailable)?;
        let parent = attachment
            .parent
            .upgrade()
            .filter(|n| authored_ancestry_mounted(n))
            .ok_or(NodeError::HomeUnavailable)?;
        if parent.id == node.id || parent.is_descendant_of(&node.id) {
            return Err(NodeError::Cycle);
        }
        node.mark_canvas_subtree_dirty_for_reparent(self, true);
        self.cancel_move_flight(node);
        if node.move_origin.get().is_some() {
            node.move_progress.cancel_transitions();
            node.move_origin.set(None);
        }
        self.presentation.borrow_mut().placements.remove(&node.id);
        node.container_frame.set(None);
        node.ensure_parent_bindings(&parent, self, &attachment.frame, &attachment.glass);
        if node.attached.get() == 0 {
            node.recurse_mount(self);
            node.ensure_parent_bindings(&parent, self, &attachment.frame, &attachment.glass);
        }
        node.rebind_parent_bounds_for_mounted_subtree(self);
        node.mark_canvas_subtree_dirty_for_reparent(self, false);
        self.presentation_changed();
        Ok(())
    }

    fn presentation_changed(&self) {
        self.presentation_generation
            .set(self.presentation_generation.get().wrapping_add(1));
        self.mark_occlusion_dirty();
        self.viewport_structure_changed();
    }

    /// Called after the node becomes unmounted, before its owned descendants.
    /// Never evaluate structural expressions while tearing down a repeat item.
    pub(crate) fn remove_presentation(
        self: &Rc<Self>,
        node: &ExpandedNode,
        preserve_recovery: bool,
    ) {
        self.cancel_move_flight(node);
        let (removed, abandoned) = {
            let mut state = self.presentation.borrow_mut();
            let removed = (!preserve_recovery && state.placements.remove(&node.id).is_some())
                | state.overlays.remove(&node.id);
            let abandoned: Vec<_> = state
                .placements
                .iter()
                .filter(|(_, p)| {
                    !p.detached && p.parent.upgrade().is_none_or(|parent| parent.id == node.id)
                })
                .map(|(id, p)| (*id, p.node.upgrade()))
                .collect();
            for (id, _) in &abandoned {
                let placement = state.placements.get_mut(id).unwrap();
                placement.detached = true;
                placement.parent = Weak::new();
            }
            (removed, abandoned)
        };
        if !removed && abandoned.is_empty() {
            return;
        }
        for (_, child) in abandoned {
            let Some(child) = child.filter(|n| n.attached.get() > 0) else {
                continue;
            };
            let owner_alive = child
                .authored_attachment
                .borrow()
                .as_ref()
                .and_then(|a| a.parent.upgrade())
                .is_some_and(|parent| authored_ancestry_mounted(&parent));
            if owner_alive {
                child.clone().unmount_for_recovery(self);
                *child.render_parent.borrow_mut() = Weak::new();
            } else {
                child.recurse_unmount(self);
            }
        }
        self.presentation_changed();
    }
}

fn authored_within(node: &Rc<ExpandedNode>, id: ExpandedNodeIdentifier) -> bool {
    let mut pending = vec![node.clone()];
    let mut seen = HashSet::new();
    while let Some(node) = pending.pop() {
        if node.id == id {
            return true;
        }
        if !seen.insert(node.id) {
            continue;
        }
        pending.extend(node.template_parent.upgrade());
        pending.extend(
            node.authored_attachment
                .borrow()
                .as_ref()
                .and_then(|a| a.parent.upgrade()),
        );
    }
    false
}

fn collect_overlays(node: &ExpandedNode, result: &mut Vec<Rc<ExpandedNode>>) {
    // Ownership traversal supplies a stable authored order, even after moves.
    // Mounted lists are safe during teardown and incremental startup.
    for child in node.mounted_children.borrow().iter() {
        if child.attached.get() == 0 {
            continue;
        }
        if child.instance_node.borrow().is_overlay() {
            result.push(child.clone());
        }
        collect_overlays(child, result);
    }
}

fn flatten_presentation_children(
    node: &ExpandedNode,
    context: &RuntimeContext,
    result: &mut Vec<Rc<ExpandedNode>>,
) {
    for child in node.presentation_children(context) {
        if crate::node_query::transparent(&child) {
            flatten_presentation_children(&child, context, result);
        } else {
            result.push(child);
        }
    }
}

fn ancestry_contains(node: &Rc<ExpandedNode>, id: ExpandedNodeIdentifier) -> bool {
    let mut pending = vec![node.clone()];
    let mut seen = HashSet::new();
    while let Some(node) = pending.pop() {
        if node.id == id {
            return true;
        }
        if !seen.insert(node.id) {
            continue;
        }
        pending.extend(node.render_parent.borrow().upgrade());
        pending.extend(node.template_parent.upgrade());
        pending.extend(
            node.authored_attachment
                .borrow()
                .as_ref()
                .and_then(|a| a.parent.upgrade()),
        );
    }
    false
}

fn authored_ancestry_mounted(node: &Rc<ExpandedNode>) -> bool {
    let mut pending = vec![node.clone()];
    let mut seen = HashSet::new();
    while let Some(node) = pending.pop() {
        if !seen.insert(node.id) {
            continue;
        }
        if node.attached.get() == 0 {
            return false;
        }
        pending.extend(node.template_parent.upgrade());
        pending.extend(
            node.authored_attachment
                .borrow()
                .as_ref()
                .and_then(|a| a.parent.upgrade()),
        );
    }
    true
}
