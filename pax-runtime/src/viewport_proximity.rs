//! Opt-in global viewport observation over the runtime's shared prepared geometry.

use crate::api::{
    Event, Layer, Property, ViewportProximityChange, ViewportProximityEnter, ViewportProximityExit,
    ViewportProximitySnapshot,
};
use crate::constants::*;
use crate::scene_geometry::SpatialIndex;
use crate::{ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};
use kurbo::{Affine, Rect, Shape};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

/// Cumulative observation work, separate from render preparation and application handlers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewportProximityStats {
    /// Indexed geometry entries refreshed after dependency changes.
    pub index_updates: u64,
    /// Scroll domains queried (cold nested domains are pruned).
    pub domain_queries: u64,
    /// Spatial candidates considered, including scroll-owner proxies.
    pub candidates: u64,
    /// Detailed target samples, including terminal samples for previously active targets.
    pub samples: u64,
    /// Local event deliveries to registered handlers.
    pub events: u64,
}

struct Registration {
    generation: u64,
    chain: Vec<u32>,
}
struct Dependency {
    users: usize,
    _effect: Property<()>,
    parent: Option<u32>,
    owner: Option<u32>,
    scrolls: bool,
    canvas: bool,
    escapes: bool,
    unclipped_scroll: bool,
}

#[derive(Default)]
pub(crate) struct ViewportProximityState {
    registrations: BTreeMap<u32, Registration>,
    dependencies: HashMap<u32, Dependency>,
    indices: HashMap<Option<u32>, SpatialIndex>,
    indexed: HashMap<u32, (Option<u32>, Rect)>,
    dirty: BTreeSet<u32>,
    pending_bindings: BTreeSet<u32>,
    active: BTreeMap<u32, ViewportProximitySnapshot>,
    escaped: BTreeSet<u32>,
    pending: bool,
    rebind: bool,
    escape_dirty: bool,
    generation: u64,
    stats: ViewportProximityStats,
}

fn has_handlers(node: &ExpandedNode, key: &str) -> bool {
    node.instance_node
        .borrow()
        .base()
        .get_handler_registry()
        .is_some_and(|registry| {
            registry
                .borrow()
                .handlers
                .get(key)
                .is_some_and(|handlers| !handlers.is_empty())
        })
}

impl RuntimeContext {
    /// Observation counters. With no proximity bindings, no observation records are installed.
    pub fn viewport_proximity_stats(&self) -> ViewportProximityStats {
        self.viewport_proximity.borrow().stats
    }

    pub(crate) fn viewport_dependency(&self, id: u32) -> bool {
        self.viewport_proximity
            .borrow()
            .dependencies
            .contains_key(&id)
    }

    pub(crate) fn invalidate_viewport_presentation(&self) {
        let mut state = self.viewport_proximity.borrow_mut();
        if !state.registrations.is_empty() {
            state.pending = true;
        }
    }

    pub(crate) fn viewport_structure_changed(&self) {
        let mut state = self.viewport_proximity.borrow_mut();
        if !state.registrations.is_empty() {
            state.rebind = true;
            state.pending = true;
        }
    }

    fn invalidate_viewport_dependency(&self, id: u32) {
        let mut state = self.viewport_proximity.borrow_mut();
        if state.dependencies.contains_key(&id) {
            state.dirty.insert(id);
            state.pending = true;
            drop(state);
            self.invalidate_scene_geometry(id);
        }
    }

    pub(crate) fn register_viewport_proximity(self: &Rc<Self>, node: &Rc<ExpandedNode>) {
        if ![
            VIEWPORT_PROXIMITY_ENTER_HANDLERS,
            VIEWPORT_PROXIMITY_CHANGE_HANDLERS,
            VIEWPORT_PROXIMITY_EXIT_HANDLERS,
        ]
        .iter()
        .any(|key| has_handlers(node, key))
        {
            return;
        }
        let id = node.id.to_u32();
        if self
            .viewport_proximity
            .borrow()
            .registrations
            .contains_key(&id)
        {
            return;
        }
        let chain = Vec::new();
        let mut state = self.viewport_proximity.borrow_mut();
        state.generation += 1;
        let generation = state.generation;
        state.pending_bindings.insert(id);
        state.escape_dirty = true;
        state
            .registrations
            .insert(id, Registration { generation, chain });
        state.dirty.insert(id);
        state.pending = true;
    }

    fn acquire_viewport_chain(self: &Rc<Self>, target: &Rc<ExpandedNode>) -> Vec<u32> {
        let mut nodes = vec![target.clone()];
        let mut parent = target.render_parent_node();
        while let Some(node) = parent {
            parent = node.render_parent_node();
            nodes.push(node);
        }
        for node in nodes.iter().rev() {
            let id = node.id.to_u32();
            {
                let mut state = self.viewport_proximity.borrow_mut();
                if let Some(dependency) = state.dependencies.get_mut(&id) {
                    dependency.users += 1;
                    continue;
                }
            }
            let parent = node.render_parent_node().map(|p| p.id.to_u32());
            let owner = parent.and_then(|parent| {
                let state = self.viewport_proximity.borrow();
                let dependency = &state.dependencies[&parent];
                if dependency.scrolls {
                    Some(parent)
                } else {
                    dependency.owner
                }
            });
            let instance = node.instance_node.borrow();
            let scrolls = instance.scrolls_content(node);
            let canvas = instance.base().flags().layer == Layer::Canvas;
            let mut dependencies: Vec<_> = node
                .properties_scope
                .borrow()
                .iter()
                .filter(|(name, _)| instance.property_requires_occlusion_recompute(name))
                .map(|(_, value)| value.get_untyped_property().clone())
                .collect();
            drop(instance);
            dependencies.extend([
                node.transform_and_bounds.untyped(),
                node.parent_frame.untyped(),
                node.occlusion.untyped(),
                node.suspended.untyped(),
                node.get_common_properties().borrow().unclippable.untyped(),
            ]);
            let weak_context = Rc::downgrade(self);
            let effect = self.register_node_effect(node.id, &dependencies, move || {
                if let Some(context) = weak_context.upgrade() {
                    context.invalidate_viewport_dependency(id);
                }
            });
            let mut state = self.viewport_proximity.borrow_mut();
            state.dependencies.insert(
                id,
                Dependency {
                    users: 1,
                    _effect: effect,
                    parent,
                    owner,
                    scrolls,
                    canvas,
                    escapes: false,
                    unclipped_scroll: false,
                },
            );
            state.dirty.insert(id);
        }
        nodes.iter().map(|n| n.id.to_u32()).collect()
    }

    fn release_viewport_chain(&self, chain: Vec<u32>) {
        let mut remove_geometry = Vec::new();
        let mut state = self.viewport_proximity.borrow_mut();
        for id in chain {
            let Some(dependency) = state.dependencies.get_mut(&id) else {
                continue;
            };
            dependency.users -= 1;
            if dependency.users != 0 {
                continue;
            }
            let dependency = state.dependencies.remove(&id).unwrap();
            if let Some((owner, _)) = state.indexed.remove(&id) {
                if let Some(index) = state.indices.get_mut(&owner) {
                    index.remove(id);
                }
            }
            if dependency.scrolls {
                state.indices.remove(&Some(id));
            }
            state.dirty.remove(&id);
            if !dependency.canvas {
                remove_geometry.push(id);
            }
        }
        drop(state);
        for id in remove_geometry {
            self.remove_observation_geometry(id);
        }
    }

    pub(crate) fn unregister_viewport_proximity(&self, id: u32) {
        let registration = {
            let mut state = self.viewport_proximity.borrow_mut();
            state.active.remove(&id);
            state.pending_bindings.remove(&id);
            state.escaped.remove(&id);
            state.registrations.remove(&id)
        };
        if let Some(registration) = registration {
            self.release_viewport_chain(registration.chain);
            // A still-shared ordinary ancestor no longer needs a target index entry.
            let mut state = self.viewport_proximity.borrow_mut();
            if state.dependencies.get(&id).is_some_and(|d| !d.scrolls) {
                if let Some((owner, _)) = state.indexed.remove(&id) {
                    if let Some(index) = state.indices.get_mut(&owner) {
                        index.remove(id);
                    }
                }
            }
        }
    }

    fn refresh_viewport_index(self: &Rc<Self>) {
        let rebind = std::mem::take(&mut self.viewport_proximity.borrow_mut().rebind);
        if rebind {
            let ids: Vec<_> = self
                .viewport_proximity
                .borrow()
                .registrations
                .keys()
                .copied()
                .collect();
            for &id in &ids {
                let chain = std::mem::take(
                    &mut self
                        .viewport_proximity
                        .borrow_mut()
                        .registrations
                        .get_mut(&id)
                        .unwrap()
                        .chain,
                );
                self.release_viewport_chain(chain);
            }
            for id in ids {
                if let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) {
                    let chain = self.acquire_viewport_chain(&node);
                    self.viewport_proximity
                        .borrow_mut()
                        .registrations
                        .get_mut(&id)
                        .unwrap()
                        .chain = chain;
                }
            }
            self.viewport_proximity
                .borrow_mut()
                .pending_bindings
                .clear();
        } else {
            let pending =
                std::mem::take(&mut self.viewport_proximity.borrow_mut().pending_bindings);
            for id in pending {
                if let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) {
                    let chain = self.acquire_viewport_chain(&node);
                    if let Some(registration) = self
                        .viewport_proximity
                        .borrow_mut()
                        .registrations
                        .get_mut(&id)
                    {
                        registration.chain = chain;
                    }
                }
            }
        }
        self.drain_node_effects();
        self.viewport_proximity.borrow_mut().pending = false;
        let dirty = std::mem::take(&mut self.viewport_proximity.borrow_mut().dirty);
        let mut escapes_changed =
            rebind || std::mem::take(&mut self.viewport_proximity.borrow_mut().escape_dirty);
        for id in dirty {
            let (owner, indexed) = {
                let state = self.viewport_proximity.borrow();
                let Some(dependency) = state.dependencies.get(&id) else {
                    continue;
                };
                (
                    dependency.owner,
                    dependency.scrolls || state.registrations.contains_key(&id),
                )
            };
            let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
                continue;
            };
            let escapes = node
                .get_common_properties()
                .borrow()
                .unclippable
                .get()
                .unwrap_or(false);
            let unclipped_scroll = {
                let instance = node.instance_node.borrow();
                instance.scrolls_content(&node) && !instance.clips_content(&node)
            };
            let geometry = self.canvas_geometry_for_node(&node);
            let owner_transform = owner
                .and_then(|owner| self.get_expanded_node_by_eid(ExpandedNodeIdentifier(owner)))
                .map(|owner| self.canvas_geometry_for_node(&owner).world_transform)
                .unwrap_or(Affine::IDENTITY);
            let bounds = (owner_transform.inverse() * geometry.world_transform)
                .transform_rect_bbox(Rect::new(0.0, 0.0, geometry.bounds.0, geometry.bounds.1));
            let mut state = self.viewport_proximity.borrow_mut();
            let dependency = state.dependencies.get_mut(&id).unwrap();
            escapes_changed |=
                dependency.escapes != escapes || dependency.unclipped_scroll != unclipped_scroll;
            dependency.unclipped_scroll = unclipped_scroll;
            dependency.escapes = escapes;
            if indexed && state.indexed.get(&id) != Some(&(owner, bounds)) {
                if let Some((old_owner, _)) = state.indexed.remove(&id) {
                    if let Some(index) = state.indices.get_mut(&old_owner) {
                        index.remove(id);
                    }
                }
                state
                    .indices
                    .entry(owner)
                    .or_default()
                    .insert(id, Some(bounds));
                state.indexed.insert(id, (owner, bounds));
                state.stats.index_updates += 1;
            }
        }
        if escapes_changed {
            let mut state = self.viewport_proximity.borrow_mut();
            // Unclippable branches may extend outside their scroll-owner proxy. They
            // remain conservative candidates instead of being incorrectly pruned.
            state.escaped = state
                .registrations
                .iter()
                .filter_map(|(&id, registration)| {
                    registration
                        .chain
                        .iter()
                        .any(|id| {
                            state
                                .dependencies
                                .get(id)
                                .is_some_and(|d| d.escapes || d.unclipped_scroll)
                        })
                        .then_some(id)
                })
                .collect();
        }
    }

    /// Sample once after layout settles, then dispatch a frozen local event batch.
    pub(crate) fn dispatch_viewport_proximity(self: &Rc<Self>) {
        {
            let mut state = self.viewport_proximity.borrow_mut();
            if !state.pending || state.registrations.is_empty() {
                return;
            }
            state.pending = false;
        }
        self.refresh_viewport_index();
        let viewport = self.viewport_observation_rect();
        let mut frames = HashMap::new();
        let mut candidates = BTreeSet::new();
        self.query_viewport_domain(None, viewport, &mut frames, &mut candidates);
        {
            let state = self.viewport_proximity.borrow();
            candidates.extend(state.active.keys());
            candidates.extend(&state.escaped);
        }
        let mut batch = Vec::new();
        for id in candidates {
            if !self
                .viewport_proximity
                .borrow()
                .registrations
                .contains_key(&id)
            {
                continue;
            }
            let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
                continue;
            };
            if node.suspended.get() {
                let mut state = self.viewport_proximity.borrow_mut();
                state.active.remove(&id);
                continue;
            }
            let Some(frame) = self.viewport_frame(id, viewport, &mut frames) else {
                continue;
            };
            let geometry = self.canvas_geometry_for_node(&node);
            let transform = frame.inherited_scroll * geometry.world_transform;
            let bounds = transform.transform_rect_bbox(Rect::new(
                0.0,
                0.0,
                geometry.bounds.0,
                geometry.bounds.1,
            ));
            // A rotated line can have a positive AABB despite having no area.
            let has_area = geometry.bounds.0 > 0.0
                && geometry.bounds.1 > 0.0
                && transform.determinant().is_finite()
                && transform.determinant() != 0.0
                && [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
                    .iter()
                    .all(|v| v.is_finite());
            let origin = kurbo::Vec2::new(-viewport.x0, -viewport.y0);
            let current = ViewportProximitySnapshot {
                in_proximity: has_area && intersect(Some(bounds), frame.near).is_some(),
                bounds: bounds + origin,
                viewport_intersection: has_area
                    .then(|| intersect(Some(bounds), frame.visible))
                    .flatten()
                    .map(|r| r + origin),
            };
            let mut state = self.viewport_proximity.borrow_mut();
            state.stats.samples += 1;
            let previous = state.active.get(&id).copied();
            let generation = state.registrations[&id].generation;
            if current.in_proximity {
                state.active.insert(id, current);
            } else {
                state.active.remove(&id);
            }
            if previous == Some(current) || (previous.is_none() && !current.in_proximity) {
                continue;
            }
            if previous.is_none() {
                batch.push((
                    id,
                    generation,
                    Delivery::Enter(ViewportProximityEnter { current }),
                ));
            }
            batch.push((
                id,
                generation,
                Delivery::Change(ViewportProximityChange { previous, current }),
            ));
            if !current.in_proximity {
                batch.push((
                    id,
                    generation,
                    Delivery::Exit(ViewportProximityExit {
                        previous: previous.unwrap(),
                        current,
                    }),
                ));
            }
        }
        // No scene/registration borrow survives application code. Recheck generation
        // before each delivery so teardown/reload cannot receive the remainder of a batch.
        for (id, generation, delivery) in batch {
            if !self
                .viewport_proximity
                .borrow()
                .registrations
                .get(&id)
                .is_some_and(|registration| registration.generation == generation)
            {
                continue;
            }
            let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
                continue;
            };
            if node.suspended.get() {
                continue;
            }
            let key = delivery.key();
            if !has_handlers(&node, key) {
                continue;
            }
            self.viewport_proximity.borrow_mut().stats.events += 1;
            match delivery {
                Delivery::Enter(args) => {
                    node.dispatch_viewport_proximity_enter(Event::new(args), self)
                }
                Delivery::Change(args) => {
                    node.dispatch_viewport_proximity_change(Event::new(args), self)
                }
                Delivery::Exit(args) => {
                    node.dispatch_viewport_proximity_exit(Event::new(args), self)
                }
            }
        }
    }

    fn viewport_observation_rect(&self) -> Rect {
        if self.get_root_scroller_id().is_some() {
            if let Some(visual) = self.get_visual_viewport_state() {
                let x = visual.page_scroll_x + visual.offset_x;
                let y = visual.page_scroll_y + visual.offset_y;
                return Rect::new(x, y, x + visual.width, y + visual.height);
            }
        }
        let (width, height) = self.globals().viewport.get().bounds;
        Rect::new(0.0, 0.0, width, height)
    }

    fn query_viewport_domain(
        &self,
        owner: Option<u32>,
        viewport: Rect,
        frames: &mut HashMap<u32, ViewportFrame>,
        targets: &mut BTreeSet<u32>,
    ) {
        let (region, transform) = if let Some(owner) = owner {
            let Some(frame) = self.viewport_frame(owner, viewport, frames) else {
                return;
            };
            let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(owner)) else {
                return;
            };
            (
                frame.child_near,
                frame.child_scroll * self.canvas_geometry_for_node(&node).world_transform,
            )
        } else {
            (
                Some(viewport.inflate(viewport.width(), viewport.height())),
                Affine::IDENTITY,
            )
        };
        let Some(region) = region else {
            return;
        };
        let query = transform.inverse().transform_rect_bbox(region);
        let mut candidates = {
            let mut state = self.viewport_proximity.borrow_mut();
            let Some(index) = state.indices.get(&owner) else {
                return;
            };
            let mut candidates = HashSet::new();
            index.candidates(query, &mut candidates);
            state.stats.domain_queries += 1;
            state.stats.candidates += candidates.len() as u64;
            candidates
        };
        // A page-backed root viewport moves in document coordinates. Its content
        // remains queryable even when the original root layout box is far behind.
        if owner.is_none() && self.get_visual_viewport_state().is_some() {
            if let Some(root) = self.get_root_scroller_id() {
                if self.viewport_dependency(root) {
                    candidates.insert(root);
                }
            }
        }
        for id in candidates {
            let (target, scrolls) = {
                let state = self.viewport_proximity.borrow();
                (
                    state.registrations.contains_key(&id),
                    state.dependencies.get(&id).is_some_and(|d| d.scrolls),
                )
            };
            if target {
                targets.insert(id);
            }
            if scrolls {
                self.query_viewport_domain(Some(id), viewport, frames, targets);
            }
        }
    }

    fn viewport_frame(
        &self,
        id: u32,
        viewport: Rect,
        frames: &mut HashMap<u32, ViewportFrame>,
    ) -> Option<ViewportFrame> {
        if let Some(frame) = frames.get(&id) {
            return Some(*frame);
        }
        let (parent, scrolls, escapes) = {
            let state = self.viewport_proximity.borrow();
            let dependency = state.dependencies.get(&id)?;
            (dependency.parent, dependency.scrolls, dependency.escapes)
        };
        let parent = parent.and_then(|parent| self.viewport_frame(parent, viewport, frames));
        let inherited_scroll = parent.map_or(Affine::IDENTITY, |p| p.child_scroll);
        let root_near = Some(viewport.inflate(viewport.width(), viewport.height()));
        let visible = if escapes {
            Some(viewport)
        } else {
            parent.map_or(Some(viewport), |p| p.child_visible)
        };
        let near = if escapes {
            root_near
        } else {
            parent.map_or(root_near, |p| p.child_near)
        };
        let node = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id))?;
        let geometry = self.canvas_geometry_for_node(&node);
        let mut child_visible = visible;
        let mut child_near = near;
        let instance = node.instance_node.borrow();
        let page_root =
            self.get_root_scroller_id() == Some(id) && self.get_visual_viewport_state().is_some();
        let clip = instance
            .resolve_effect_clip_path(&node)
            .map(|path| (inherited_scroll * path).bounding_box());
        if scrolls && instance.clips_content(&node) {
            let bounds = if page_root {
                viewport
            } else {
                (inherited_scroll * geometry.world_transform).transform_rect_bbox(Rect::new(
                    0.0,
                    0.0,
                    geometry.bounds.0,
                    geometry.bounds.1,
                ))
            };
            child_visible = intersect(child_visible, Some(bounds));
            child_near = intersect(
                child_near,
                Some(bounds.inflate(viewport.width(), viewport.height())),
            );
        }
        if let Some(clip) = clip.filter(|_| !page_root) {
            child_visible = intersect(child_visible, Some(clip));
            child_near = intersect(
                child_near,
                Some(if scrolls {
                    clip.inflate(viewport.width(), viewport.height())
                } else {
                    clip
                }),
            );
        }
        drop(instance);
        let child_scroll = if scrolls {
            inherited_scroll * self.scroller_content_presentation_transform(&node)
        } else {
            inherited_scroll
        };
        let frame = ViewportFrame {
            inherited_scroll,
            child_scroll,
            visible,
            near,
            child_visible,
            child_near,
        };
        frames.insert(id, frame);
        Some(frame)
    }
}

#[derive(Clone, Copy)]
struct ViewportFrame {
    inherited_scroll: Affine,
    child_scroll: Affine,
    visible: Option<Rect>,
    near: Option<Rect>,
    child_visible: Option<Rect>,
    child_near: Option<Rect>,
}

fn intersect(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    let rect = a?.intersect(b?);
    (rect.width() > 0.0
        && rect.height() > 0.0
        && [rect.x0, rect.y0, rect.x1, rect.y1]
            .iter()
            .all(|x| x.is_finite()))
    .then_some(rect)
}

enum Delivery {
    Enter(ViewportProximityEnter),
    Change(ViewportProximityChange),
    Exit(ViewportProximityExit),
}
impl Delivery {
    fn key(&self) -> &'static str {
        match self {
            Self::Enter(_) => VIEWPORT_PROXIMITY_ENTER_HANDLERS,
            Self::Change(_) => VIEWPORT_PROXIMITY_CHANGE_HANDLERS,
            Self::Exit(_) => VIEWPORT_PROXIMITY_EXIT_HANDLERS,
        }
    }
}
