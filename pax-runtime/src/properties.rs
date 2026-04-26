use crate::api::math::Point2;
use crate::api::Window;
use crate::constants::{PRE_RENDER_HANDLERS, TICK_HANDLERS};
use pax_language::interpreter::property_resolution::IdentifierResolver;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::{NativeMessage, ScreenshotData};
use pax_runtime_api::properties::{drain_effects, register_effect_property, UntypedProperty};
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Event, Interpolatable, MouseOut, MouseOver, Property,
    RenderContext, Store, Variable,
};
use_RefCell!();
use kurbo::Affine;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::{ExpandedNode, Globals};

#[cfg(feature = "designtime")]
use crate::{ComponentInstance, InstanceNode};

impl Interpolatable for ExpandedNodeIdentifier {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Stable runtime identifier assigned to an expanded node.
pub struct ExpandedNodeIdentifier(pub u32);

impl ExpandedNodeIdentifier {
    /// Convert to the integer id passed across chassis message boundaries.
    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

/// Shared context for properties pass recursion
pub struct RuntimeContext {
    next_uid: Cell<ExpandedNodeIdentifier>,
    messages: RefCell<Vec<NativeMessage>>,
    globals: RefCell<Globals>,
    root_expanded_node: RefCell<Weak<ExpandedNode>>,
    #[cfg(feature = "designtime")]
    pub userland_frame_instance_node: RefCell<Option<Rc<dyn InstanceNode>>>,
    #[cfg(feature = "designtime")]
    pub userland_root_expanded_node: RefCell<Option<Rc<ExpandedNode>>>,
    node_cache: RefCell<NodeCache>,
    last_topmost_element: RefCell<Weak<ExpandedNode>>,
    queued_custom_events: RefCell<Vec<(Rc<ExpandedNode>, &'static str)>>,
    queued_renders: RefCell<Vec<Rc<ExpandedNode>>>,
    pub layer_count: Cell<usize>,
    pub dirty_canvases: Rc<RefCell<Vec<bool>>>,
    dirty_canvas_nodes: RefCell<HashSet<ExpandedNodeIdentifier>>,
    removed_canvas_nodes: RefCell<Vec<(usize, u32)>>,
    occlusion_dirty: Cell<bool>,
    layer_canvas_plan_generation: Cell<u32>,
    tick_handler_nodes: RefCell<Vec<ExpandedNodeIdentifier>>,
    pre_render_handler_nodes: RefCell<Vec<ExpandedNodeIdentifier>>,
    screenshot_map: Rc<RefCell<HashMap<u32, ScreenshotData>>>,
    scroller_surface_states: RefCell<HashMap<u32, ScrollerSurfaceState>>,
    layer_scroller_owners: RefCell<HashMap<usize, ExpandedNodeIdentifier>>,
    root_scroller_id: Cell<Option<u32>>,
    visual_viewport_state: Cell<Option<VisualViewportState>>,
}

#[derive(Clone, Copy, Debug, Default)]
/// Last-known scroll state for a native or browser-owned scroller surface.
pub struct ScrollerSurfaceState {
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub presentation_scroll_x: f64,
    pub presentation_scroll_y: f64,
    pub clip_content: bool,
}

#[derive(Clone, Copy, Debug, Default)]
/// Browser visual viewport state used when page scrolling participates in root scroller behavior.
pub struct VisualViewportState {
    pub width: f64,
    pub height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub page_scroll_x: f64,
    pub page_scroll_y: f64,
}

struct NodeCache {
    eid_to_node: HashMap<ExpandedNodeIdentifier, Rc<ExpandedNode>>,
    uni_to_eid: HashMap<UniqueTemplateNodeIdentifier, Vec<ExpandedNodeIdentifier>>,
}

impl NodeCache {
    fn new() -> Self {
        Self {
            eid_to_node: Default::default(),
            uni_to_eid: Default::default(),
        }
    }

    // Add this node to all relevant constant lookup cache structures
    fn add_to_cache(&mut self, node: &Rc<ExpandedNode>) {
        self.eid_to_node.insert(node.id, Rc::clone(&node));
        let uni = borrow!(node.instance_node)
            .base()
            .template_node_identifier
            .clone();
        if let Some(uni) = uni {
            self.uni_to_eid.entry(uni).or_default().push(node.id);
        }
    }

    // Remove this node from all relevant constant lookup cache structures
    fn remove_from_cache(&mut self, node: &Rc<ExpandedNode>) {
        self.eid_to_node.remove(&node.id);
        if let Some(uni) = &borrow!(node.instance_node).base().template_node_identifier {
            self.uni_to_eid
                .entry(uni.clone())
                .or_default()
                .retain(|&n| n != node.id);
        }
    }
}

impl RuntimeContext {
    #[cfg(not(feature = "designtime"))]
    /// Create a runtime context for normal app execution.
    pub fn new(globals: Globals) -> Self {
        Self {
            next_uid: Cell::new(ExpandedNodeIdentifier(0)),
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            node_cache: RefCell::new(NodeCache::new()),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
            layer_canvas_plan_generation: Cell::new(1),
            tick_handler_nodes: Default::default(),
            pre_render_handler_nodes: Default::default(),
            screenshot_map: Default::default(),
            scroller_surface_states: Default::default(),
            layer_scroller_owners: Default::default(),
            root_scroller_id: Cell::new(None),
            visual_viewport_state: Cell::new(None),
        }
    }

    #[cfg(feature = "designtime")]
    /// Create a runtime context with the userland component tracked for designer tools.
    pub fn new(globals: Globals, userland: Rc<ComponentInstance>) -> Self {
        Self {
            next_uid: Cell::new(ExpandedNodeIdentifier(0)),
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            userland_frame_instance_node: RefCell::new(Some(userland)),
            userland_root_expanded_node: Default::default(),
            node_cache: RefCell::new(NodeCache::new()),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
            layer_canvas_plan_generation: Cell::new(1),
            tick_handler_nodes: Default::default(),
            pre_render_handler_nodes: Default::default(),
            screenshot_map: Default::default(),
            scroller_surface_states: Default::default(),
            layer_scroller_owners: Default::default(),
            root_scroller_id: Cell::new(None),
            visual_viewport_state: Cell::new(None),
        }
    }

    #[cfg(feature = "designtime")]
    /// Create a runtime context before any userland tree has been mounted.
    pub fn new_empty(globals: Globals) -> Self {
        Self {
            next_uid: Cell::new(ExpandedNodeIdentifier(0)),
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            userland_frame_instance_node: RefCell::new(None),
            userland_root_expanded_node: Default::default(),
            node_cache: RefCell::new(NodeCache::new()),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
            layer_canvas_plan_generation: Cell::new(1),
            tick_handler_nodes: Default::default(),
            pre_render_handler_nodes: Default::default(),
            screenshot_map: Default::default(),
            scroller_surface_states: Default::default(),
            layer_scroller_owners: Default::default(),
            root_scroller_id: Cell::new(None),
            visual_viewport_state: Cell::new(None),
        }
    }

    /// Store the root expanded node after it has been initialized.
    pub fn register_root_expanded_node(&self, root: &Rc<ExpandedNode>) {
        *borrow_mut!(self.root_expanded_node) = Rc::downgrade(root);
    }

    /// Clear the registered root expanded node.
    pub fn clear_root_expanded_node(&self) {
        *borrow_mut!(self.root_expanded_node) = Weak::new();
    }

    /// Add a node to runtime lookup caches.
    pub fn add_to_cache(&self, node: &Rc<ExpandedNode>) {
        borrow_mut!(self.node_cache).add_to_cache(node);
        self.register_node_lifecycle_handlers(node);
        self.mark_occlusion_dirty();
    }

    /// Remove a node from runtime lookup caches.
    pub fn remove_from_cache(&self, node: &Rc<ExpandedNode>) {
        borrow_mut!(self.node_cache).remove_from_cache(node);
        self.unregister_node_lifecycle_handlers(node.id);
        self.mark_occlusion_dirty();
    }

    pub fn register_node_effect_property(
        &self,
        _node: ExpandedNodeIdentifier,
        prop: &Property<()>,
    ) {
        register_effect_property(prop);
    }

    pub fn register_node_effect(
        &self,
        node: ExpandedNodeIdentifier,
        dependencies: &[UntypedProperty],
        effect: impl Fn() + 'static,
    ) -> Property<()> {
        let prop = Property::computed(effect, dependencies);
        self.register_node_effect_property(node, &prop);
        prop
    }

    pub fn drain_node_effects(&self) {
        const MAX_NODE_EFFECTS_PER_TICK: usize = 100_000;
        let drained = drain_effects(MAX_NODE_EFFECTS_PER_TICK);
        if drained == MAX_NODE_EFFECTS_PER_TICK {
            log::warn!(
                "node effect drain hit {} effects in one tick; deferring remaining effects",
                MAX_NODE_EFFECTS_PER_TICK
            );
        }
    }

    pub fn tick_handler_nodes(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.tick_handler_nodes).clone()
    }

    pub fn pre_render_handler_nodes(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.pre_render_handler_nodes).clone()
    }

    fn register_node_lifecycle_handlers(&self, node: &Rc<ExpandedNode>) {
        let registry = borrow!(node.instance_node)
            .base()
            .handler_registry
            .as_ref()
            .map(Rc::clone);
        let Some(registry) = registry else {
            return;
        };
        let handlers = &borrow!(registry).handlers;
        if handlers.get(TICK_HANDLERS).is_some_and(|h| !h.is_empty()) {
            self.register_lifecycle_handler(&self.tick_handler_nodes, node.id);
        }
        if handlers
            .get(PRE_RENDER_HANDLERS)
            .is_some_and(|h| !h.is_empty())
        {
            self.register_lifecycle_handler(&self.pre_render_handler_nodes, node.id);
        }
    }

    fn unregister_node_lifecycle_handlers(&self, id: ExpandedNodeIdentifier) {
        borrow_mut!(self.tick_handler_nodes).retain(|node_id| *node_id != id);
        borrow_mut!(self.pre_render_handler_nodes).retain(|node_id| *node_id != id);
    }

    fn register_lifecycle_handler(
        &self,
        handler_nodes: &RefCell<Vec<ExpandedNodeIdentifier>>,
        id: ExpandedNodeIdentifier,
    ) {
        let mut handler_nodes = borrow_mut!(handler_nodes);
        if !handler_nodes.contains(&id) {
            handler_nodes.push(id);
        }
    }

    /// Look up an expanded node by runtime id.
    pub fn get_expanded_node_by_eid(&self, id: ExpandedNodeIdentifier) -> Option<Rc<ExpandedNode>> {
        borrow!(self.node_cache).eid_to_node.get(&id).cloned()
    }

    /// Store a screenshot payload delivered by the chassis.
    pub fn load_screenshot(&self, id: u32, data: ScreenshotData) -> bool {
        borrow_mut!(self.screenshot_map).insert(id, data);
        true
    }

    /// Shared screenshot capture map keyed by request id.
    pub fn get_screenshot_map(&self) -> Rc<RefCell<HashMap<u32, ScreenshotData>>> {
        Rc::clone(&self.screenshot_map)
    }

    /// Remember browser-owned scroller state for native compositing and scroll transforms.
    pub fn set_scroller_surface_state(&self, id: u32, state: ScrollerSurfaceState) {
        borrow_mut!(self.scroller_surface_states).insert(id, state);
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Remove cached scroller surface state.
    pub fn remove_scroller_surface_state(&self, id: u32) {
        borrow_mut!(self.scroller_surface_states).remove(&id);
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Fetch cached scroller surface state by node id.
    pub fn get_scroller_surface_state(&self, id: u32) -> Option<ScrollerSurfaceState> {
        borrow!(self.scroller_surface_states).get(&id).cloned()
    }

    /// Clear layer-to-scroller ownership before recomputing occlusion.
    pub fn clear_layer_scroller_owners(&self) {
        borrow_mut!(self.layer_scroller_owners).clear();
        self.mark_layer_canvas_plans_dirty();
    }

    /// Record that a canvas layer is owned by a particular scroller.
    pub fn register_layer_scroller_owner(
        &self,
        layer_id: usize,
        scroller_id: ExpandedNodeIdentifier,
    ) {
        borrow_mut!(self.layer_scroller_owners).insert(layer_id, scroller_id);
        self.mark_layer_canvas_plans_dirty();
    }

    /// Find the scroller that owns a canvas layer, when one exists.
    pub fn get_layer_scroller_owner(&self, layer_id: usize) -> Option<ExpandedNodeIdentifier> {
        borrow!(self.layer_scroller_owners).get(&layer_id).copied()
    }

    /// Mark which node currently delegates root scrolling behavior to the page.
    pub fn set_root_scroller_id(&self, id: Option<u32>) {
        self.root_scroller_id.set(id);
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Current page-scroll-backed root scroller id.
    pub fn get_root_scroller_id(&self) -> Option<u32> {
        self.root_scroller_id.get()
    }

    /// Cache the browser visual viewport state for root scroller math.
    pub fn set_visual_viewport_state(&self, state: VisualViewportState) {
        self.visual_viewport_state.set(Some(state));
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Clear cached visual viewport state.
    pub fn clear_visual_viewport_state(&self) {
        self.visual_viewport_state.set(None);
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Return cached browser visual viewport state, if available.
    pub fn get_visual_viewport_state(&self) -> Option<VisualViewportState> {
        self.visual_viewport_state.get()
    }

    /// Ensure the dirty-canvas table has entries up to the requested layer count.
    pub fn resize_canvas_layers_to(&self, id: usize) {
        let mut dirty_canvases = borrow_mut!(self.dirty_canvases);
        let old_len = dirty_canvases.len();
        dirty_canvases.resize(id, false);
        for dirty in dirty_canvases.iter_mut().skip(old_len) {
            *dirty = true;
        }
        if old_len != id {
            self.mark_layer_canvas_plans_dirty();
        }
    }

    pub fn mark_layer_canvas_plans_dirty(&self) {
        let next = self.layer_canvas_plan_generation.get().wrapping_add(1);
        self.layer_canvas_plan_generation.set(next.max(1));
    }

    pub fn layer_canvas_plan_generation(&self) -> u32 {
        self.layer_canvas_plan_generation.get()
    }

    /// Mark every canvas layer clean.
    pub fn clear_all_dirty_canvases(&self) {
        let mut dirty_canvases = borrow_mut!(self.dirty_canvases);
        for v in dirty_canvases.iter_mut() {
            *v = false;
        }
    }

    /// Mark a canvas layer dirty.
    pub fn set_canvas_dirty(&self, id: usize) {
        let mut dirty_canvases = borrow_mut!(self.dirty_canvases);
        if let Some(v) = dirty_canvases.get_mut(id) {
            *v = true;
        }
    }

    /// Check whether a canvas layer needs redraw.
    pub fn is_canvas_dirty(&self, id: &usize) -> bool {
        *borrow!(self.dirty_canvases).get(*id).unwrap_or(&true)
    }

    pub fn dirty_canvas_layers(&self) -> Vec<usize> {
        borrow!(self.dirty_canvases)
            .iter()
            .enumerate()
            .filter_map(|(index, dirty)| dirty.then_some(index))
            .collect()
    }

    pub fn has_dirty_canvas_layers(&self) -> bool {
        borrow!(self.dirty_canvases).iter().any(|dirty| *dirty)
    }

    pub fn has_dirty_canvas_nodes(&self) -> bool {
        !borrow!(self.dirty_canvas_nodes).is_empty()
    }

    pub fn has_canvas_node_removals(&self) -> bool {
        !borrow!(self.removed_canvas_nodes).is_empty()
    }

    pub fn has_canvas_render_work(&self) -> bool {
        self.has_dirty_canvas_layers() || self.has_canvas_node_removals()
    }

    pub fn mark_occlusion_dirty(&self) {
        self.occlusion_dirty.set(true);
    }

    pub fn take_occlusion_dirty(&self) -> bool {
        let dirty = self.occlusion_dirty.get();
        self.occlusion_dirty.set(false);
        dirty
    }

    pub fn set_all_canvases_dirty(&self) {
        let mut dirty_canvases = borrow_mut!(self.dirty_canvases);
        for v in dirty_canvases.iter_mut() {
            *v = true;
        }
    }

    pub fn mark_canvas_node_dirty(&self, id: ExpandedNodeIdentifier) {
        borrow_mut!(self.dirty_canvas_nodes).insert(id);
    }

    pub fn clear_canvas_node_dirty(&self, id: &ExpandedNodeIdentifier) {
        borrow_mut!(self.dirty_canvas_nodes).remove(id);
    }

    pub fn is_canvas_node_dirty(&self, id: &ExpandedNodeIdentifier) -> bool {
        borrow!(self.dirty_canvas_nodes).contains(id)
    }

    pub fn mark_canvas_nodes_on_layer_dirty(&self, layer: usize) {
        let node_cache = borrow!(self.node_cache);
        let dirty_nodes = &mut *borrow_mut!(self.dirty_canvas_nodes);
        for node in node_cache.eid_to_node.values() {
            if node.occlusion.get().occlusion_layer_id == layer
                && borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas
            {
                dirty_nodes.insert(node.id);
            }
        }
    }

    pub fn mark_all_canvas_nodes_dirty(&self) {
        let node_cache = borrow!(self.node_cache);
        let dirty_nodes = &mut *borrow_mut!(self.dirty_canvas_nodes);
        for node in node_cache.eid_to_node.values() {
            if borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas {
                dirty_nodes.insert(node.id);
            }
        }
    }

    pub fn enqueue_canvas_node_removal(&self, layer: usize, node_id: u32) {
        borrow_mut!(self.removed_canvas_nodes).push((layer, node_id));
    }

    pub fn take_canvas_node_removals(&self) -> Vec<(usize, u32)> {
        let mut removals = borrow_mut!(self.removed_canvas_nodes);
        std::mem::take(&mut *removals)
    }

    /// Finds all ExpandedNodes with the CommonProperty#id matching the provided string
    pub fn get_expanded_nodes_by_id(&self, id: &str) -> Vec<Rc<ExpandedNode>> {
        //v0 limitation: currently an O(n) lookup cost (could be made O(1) with an id->expandednode cache)
        borrow!(self.node_cache)
            .eid_to_node
            .values()
            .filter(|val| {
                let common_props = val.get_common_properties();
                let common_props = borrow!(common_props);
                common_props.id.get().is_some_and(|i| i == id)
            })
            .cloned()
            .collect()
    }

    /// Finds all ExpandedNodes with corresponding UniqueTemplateNodeIdentifier
    pub fn get_expanded_nodes_by_global_ids(
        &self,
        uni: &UniqueTemplateNodeIdentifier,
    ) -> Vec<Rc<ExpandedNode>> {
        let node_cache = borrow!(self.node_cache);
        node_cache
            .uni_to_eid
            .get(uni)
            .map(|eids| {
                let mut nodes = vec![];
                for e in eids {
                    if let Some(node) = node_cache.eid_to_node.get(e) {
                        nodes.push(Rc::clone(node));
                    } else {
                        log::warn!(
                            "failed to find node in engine for expanded node identifier: {:?}",
                            e
                        );
                    }
                }
                nodes
            })
            .unwrap_or_default()
    }

    /// Simple 2D raycasting: the coordinates of the ray represent a
    /// ray running orthogonally to the view plane, intersecting at
    /// the specified point `ray`.  Areas outside of clipping bounds will
    /// not register a `hit`, nor will elements that suppress input events.
    pub fn get_elements_beneath_ray(
        &self,
        root: Option<Rc<ExpandedNode>>,
        ray: Point2<Window>,
        limit_one: bool,
        mut accum: Vec<Rc<ExpandedNode>>,
        hit_invisible: bool,
    ) -> Vec<Rc<ExpandedNode>> {
        fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
            if content <= viewport {
                return 0.0;
            }
            if !value.is_finite() {
                return 0.0;
            }
            value.max(0.0).min((content - viewport).max(0.0))
        }

        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        let Some(root_node) = root.or_else(|| borrow!(self.root_expanded_node).upgrade()) else {
            return accum;
        };
        let mut to_process = vec![(root_node, false, Affine::IDENTITY)];
        while let Some((node, clipped, active_scroll_transform)) = to_process.pop() {
            // make sure slot sources are updated for this node
            node.compute_flattened_projected_children();
            // Browser-composited scrollers move descendants outside the engine transform tree.
            // Fold active scroll offsets into hit-testing so event rays line up with presented content.
            let (scroll_transform, clips_content) = {
                let instance_node = borrow!(node.instance_node);
                let scrolls_content = instance_node.scrolls_content(&node);
                let scroll_transform = if scrolls_content {
                    let root_delegates_to_page_scroll = self.get_root_scroller_id()
                        == Some(node.id.to_u32())
                        && self.get_visual_viewport_state().is_some();
                    if root_delegates_to_page_scroll {
                        Affine::IDENTITY
                    } else {
                        let (scroll_x, scroll_y) =
                            if self.get_root_scroller_id() == Some(node.id.to_u32()) {
                                if let Some(visual) = self.get_visual_viewport_state() {
                                    let visual_x = visual.page_scroll_x + visual.offset_x;
                                    let visual_y = visual.page_scroll_y + visual.offset_y;
                                    if visual_x.is_finite() && visual_y.is_finite() {
                                        if let Some(state) =
                                            self.get_scroller_surface_state(node.id.to_u32())
                                        {
                                            let viewport_width = if visual.width.is_finite() {
                                                visual.width
                                            } else {
                                                state.viewport_width
                                            };
                                            let viewport_height = if visual.height.is_finite() {
                                                visual.height
                                            } else {
                                                state.viewport_height
                                            };
                                            (
                                                clamp_offset(
                                                    visual_x,
                                                    state.content_width,
                                                    viewport_width,
                                                ),
                                                clamp_offset(
                                                    visual_y,
                                                    state.content_height,
                                                    viewport_height,
                                                ),
                                            )
                                        } else {
                                            (visual_x, visual_y)
                                        }
                                    } else {
                                        instance_node
                                            .resolve_scroll_offset(&node)
                                            .unwrap_or((0.0, 0.0))
                                    }
                                } else {
                                    instance_node
                                        .resolve_scroll_offset(&node)
                                        .unwrap_or((0.0, 0.0))
                                }
                            } else {
                                instance_node
                                    .resolve_scroll_offset(&node)
                                    .unwrap_or((0.0, 0.0))
                            };
                        if scroll_x.abs() > f64::EPSILON || scroll_y.abs() > f64::EPSILON {
                            let world_transform =
                                Affine::from(node.transform_and_bounds.get().transform);
                            let inverse_world =
                                Affine::from(node.transform_and_bounds.get().transform.inverse());
                            world_transform
                                * Affine::translate((-scroll_x, -scroll_y))
                                * inverse_world
                        } else {
                            Affine::IDENTITY
                        }
                    }
                } else {
                    Affine::IDENTITY
                };
                (scroll_transform, instance_node.clips_content(&node))
            };
            let descendant_scroll_transform = active_scroll_transform * scroll_transform;
            let hit = node.ray_cast_test(active_scroll_transform.inverse() * ray);
            if hit && !clipped {
                if hit_invisible
                    || !borrow!(node.instance_node)
                        .base()
                        .flags()
                        .invisible_to_raycasting
                {
                    //We only care about the topmost node getting hit, and the element
                    //pool is ordered by z-index so we can just resolve the whole
                    //calculation when we find the first matching node
                    if limit_one {
                        return vec![node];
                    }
                    accum.push(Rc::clone(&node));
                }
            }
            let clipped = clipped || (!hit && clips_content);
            to_process.extend(
                node.children
                    .get()
                    .iter()
                    .cloned()
                    .map(|v| {
                        let cp = v.get_common_properties();
                        let unclippable = borrow!(cp).unclippable.get().unwrap_or(false);
                        (v, clipped && !unclippable, descendant_scroll_transform)
                    })
                    .rev(),
            )
        }
        accum
    }
    /// Alias for `get_elements_beneath_ray` with `limit_one = true`
    pub fn get_topmost_element_beneath_ray(
        self: &Rc<Self>,
        ray: Point2<Window>,
    ) -> Option<Rc<ExpandedNode>> {
        let res = self.get_elements_beneath_ray(None, ray, true, vec![], false);
        let Some(new_topmost) = res
            .into_iter()
            .next()
            .or_else(|| borrow!(self.root_expanded_node).upgrade())
        else {
            *borrow_mut!(self.last_topmost_element) = Weak::new();
            return None;
        };

        // Send mouse over/out events if the hit element is different than last.
        // Use template ancestry rather than containing-component ancestry so
        // wrappers around slotted content can own hover affordances.
        let last_topmost = borrow!(self.last_topmost_element).upgrade();
        if Some(new_topmost.id) != last_topmost.as_ref().map(|n| n.id) {
            let (leaving, entering) = find_template_paths_to_common_ancestor(
                &last_topmost,
                &Some(Rc::clone(&new_topmost)),
            );
            for leave in leaving {
                leave.dispatch_mouse_out(Event::new(MouseOut {}), &self.globals(), self);
            }
            for enter in entering {
                enter.dispatch_mouse_over(Event::new(MouseOver {}), &self.globals(), self);
            }
            *borrow_mut!(self.last_topmost_element) = Rc::downgrade(&new_topmost);
        }
        Some(new_topmost)
    }

    pub fn gen_uid(&self) -> ExpandedNodeIdentifier {
        let val = self.next_uid.get();
        let next_val = ExpandedNodeIdentifier(val.0 + 1);
        self.next_uid.set(next_val);
        val
    }

    pub fn enqueue_native_message(&self, message: NativeMessage) {
        borrow_mut!(self.messages).push(message)
    }

    pub fn take_native_messages(&self) -> Vec<NativeMessage> {
        let mut messages = borrow_mut!(self.messages);
        std::mem::take(&mut *messages)
    }

    pub fn globals(&self) -> Globals {
        borrow!(self.globals).clone()
    }

    pub fn edit_globals(&self, f: impl Fn(&mut Globals)) {
        let mut globals = borrow_mut!(self.globals);
        f(&mut globals);
    }

    pub fn queue_custom_event(&self, source_expanded_node: Rc<ExpandedNode>, name: &'static str) {
        let mut queued_custom_events = borrow_mut!(self.queued_custom_events);
        queued_custom_events.push((source_expanded_node, name));
    }

    pub fn flush_custom_events(self: &Rc<Self>) -> Result<(), String> {
        let mut queued_custom_event = borrow_mut!(self.queued_custom_events);
        let to_flush: Vec<_> = std::mem::take(queued_custom_event.as_mut());
        for (target, ident) in to_flush {
            target.dispatch_custom_event(ident, self)?;
        }
        Ok(())
    }

    #[cfg(feature = "designtime")]
    pub fn get_userland_root_expanded_node(&self) -> Option<Rc<ExpandedNode>> {
        borrow!(self.userland_root_expanded_node).clone()
    }

    #[cfg(feature = "designtime")]
    pub fn get_userland_root_instance_node(&self) -> Option<Rc<dyn InstanceNode>> {
        borrow!(self.userland_frame_instance_node).clone()
    }

    #[cfg(feature = "designtime")]
    pub fn set_userland_root_expanded_node(&self, root: Option<Rc<ExpandedNode>>) {
        *borrow_mut!(self.userland_root_expanded_node) = root;
    }

    #[cfg(feature = "designtime")]
    pub fn set_userland_root_instance_node(&self, node: Option<Rc<dyn InstanceNode>>) {
        *borrow_mut!(self.userland_frame_instance_node) = node;
    }

    pub fn get_root_expanded_node(&self) -> Option<Rc<ExpandedNode>> {
        borrow!(self.root_expanded_node).upgrade()
    }

    pub fn queue_render(&self, expanded_node: Rc<ExpandedNode>) {
        borrow_mut!(self.queued_renders).push(expanded_node);
    }

    pub fn recurse_flush_queued_renders(self: &Rc<RuntimeContext>, rcs: &mut dyn RenderContext) {
        while !borrow!(self.queued_renders).is_empty() {
            for n in std::mem::take(&mut *borrow_mut!(self.queued_renders)) {
                n.recurse_render(self, rcs);
            }
        }
    }
}

fn find_template_paths_to_common_ancestor(
    last_topmost: &Option<Rc<ExpandedNode>>,
    new_topmost: &Option<Rc<ExpandedNode>>,
) -> (Vec<Rc<ExpandedNode>>, Vec<Rc<ExpandedNode>>) {
    let mut last_path = template_path_from_root(last_topmost);
    let mut new_path = template_path_from_root(new_topmost);
    let common_prefix_len = last_path
        .iter()
        .zip(new_path.iter())
        .take_while(|(last, new)| last.id == new.id)
        .count();

    // Remove common ancestors from both paths.
    last_path.drain(0..common_prefix_len);
    new_path.drain(0..common_prefix_len);

    (last_path, new_path)
}

fn template_path_from_root(node: &Option<Rc<ExpandedNode>>) -> Vec<Rc<ExpandedNode>> {
    let mut path = Vec::new();
    let mut current = node.clone();
    while let Some(node) = current {
        path.push(Rc::clone(&node));
        current = node.template_parent.upgrade();
    }
    path.reverse();
    path
}

/// Data structure for a single frame of our runtime stack, including
/// a reference to its parent frame and `properties` for
/// runtime evaluation, e.g. of Expressions.  `RuntimePropertiesStackFrame`s also track
/// timeline playhead position.
///
/// `Component`s push `RuntimePropertiesStackFrame`s before computing properties and pop them after computing, thus providing a
/// hierarchical store of node-relevant data that can be bound to symbols in expressions.

pub struct RuntimePropertiesStackFrame {
    symbols_within_frame: HashMap<String, Variable>,
    local_stores: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,
    parent: Option<Rc<RuntimePropertiesStackFrame>>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(symbols_within_frame: HashMap<String, Variable>) -> Rc<Self> {
        Rc::new(Self {
            symbols_within_frame,
            local_stores: Default::default(),
            parent: None,
        })
    }

    pub fn push(self: &Rc<Self>, symbols_within_frame: HashMap<String, Variable>) -> Rc<Self> {
        Rc::new(RuntimePropertiesStackFrame {
            symbols_within_frame,
            local_stores: Default::default(),
            parent: Some(Rc::clone(self)),
        })
    }

    pub fn pop(self: &Rc<Self>) -> Option<Rc<Self>> {
        self.parent.clone()
    }

    pub fn insert_stack_local_store<T: Store>(&self, store: T) {
        let type_id = TypeId::of::<T>();
        borrow_mut!(self.local_stores).insert(type_id, Box::new(store));
    }

    pub fn peek_stack_local_store<T: Store, V>(
        self: &Rc<Self>,
        f: impl FnOnce(&mut T) -> V,
    ) -> Result<V, String> {
        let mut current = Rc::clone(self);
        let type_id = TypeId::of::<T>();

        while !borrow!(current.local_stores).contains_key(&type_id) {
            current = current
                .parent
                .clone()
                .ok_or_else(|| format!("couldn't find store in local stack"))?;
        }
        let v = {
            let mut stores = borrow_mut!(current.local_stores);
            let store = stores.get_mut(&type_id).unwrap().downcast_mut().unwrap();
            f(store)
        };
        Ok(v)
    }

    pub fn resolve_symbol_as_variable(&self, symbol: &str) -> Option<Variable> {
        if let Some(e) = self.symbols_within_frame.get(&clean_symbol(symbol)) {
            Some(e.clone())
        } else {
            self.parent.as_ref()?.resolve_symbol_as_variable(symbol)
        }
    }

    pub fn resolve_symbol_as_erased_property(&self, symbol: &str) -> Option<UntypedProperty> {
        if let Some(e) = self.symbols_within_frame.get(&clean_symbol(symbol)) {
            Some(e.clone().get_untyped_property().clone())
        } else {
            self.parent
                .as_ref()?
                .resolve_symbol_as_erased_property(symbol)
        }
    }

    pub fn resolve_symbol(&self, symbol: &str) -> Option<Variable> {
        self.symbols_within_frame
            .get(&clean_symbol(symbol))
            .cloned()
            .or_else(|| self.parent.as_ref()?.resolve_symbol(symbol))
    }
}

fn clean_symbol(symbol: &str) -> String {
    symbol.replace("self.", "").replace("this.", "")
}

impl IdentifierResolver for RuntimePropertiesStackFrame {
    fn resolve(&self, name: String) -> Result<Variable, String> {
        self.resolve_symbol(&name)
            .ok_or_else(|| format!("Could not resolve symbol {}", name))
    }
}

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext {
    pub stack_frame: Rc<RuntimePropertiesStackFrame>,
}
