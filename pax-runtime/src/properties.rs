use crate::api::math::Point2;
use crate::api::Window;
use crate::constants::{ACCEL_HANDLERS, GYRO_HANDLERS, PRE_RENDER_HANDLERS, TICK_HANDLERS};
use pax_language::interpreter::property_resolution::IdentifierResolver;
use pax_manifest::cartridge_generation::{TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT};
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::{NativeMessage, SafeAreaInsets, ScreenshotData};
use pax_runtime_api::properties::{
    drain_effects, drain_effects_with_report, property_has_direct_outbound,
    property_outbound_debug_names, register_effect_property, register_effect_property_with_name,
    UntypedProperty,
};
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Event, Interpolatable, LightShape, LocalProperty, MouseOut,
    MouseOver, RenderContext, SceneLight, SceneLighting, Variable,
};
use_RefCell!();
use kurbo::Affine;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::sync::OnceLock;
use std::time::Instant;

mod input_query;

use crate::{ExpandedNode, Globals};

enum QueuedEvent {
    Custom(Rc<ExpandedNode>, &'static str),
    Submit(Rc<ExpandedNode>),
}

#[cfg(feature = "designtime")]
use crate::{ComponentInstance, InstanceNode};

fn effect_drain_instrumentation_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("PAX_EFFECT_DRAIN_INSTRUMENTATION")
            .map(|value| {
                let value = value.to_ascii_lowercase();
                matches!(value.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    })
}

fn format_effect_top(top_effects: &[(String, usize)]) -> String {
    top_effects
        .iter()
        .map(|(name, count)| format!("{count}x {name}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn summarize_debug_names(names: Vec<String>) -> Vec<(String, usize)> {
    let mut counts = HashMap::new();
    for name in names {
        let category = name
            .split_once(" (")
            .map(|(category, _)| category)
            .unwrap_or(&name)
            .to_owned();
        *counts.entry(category).or_insert(0) += 1;
    }
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|(left_name, left_count), (right_name, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_name.cmp(right_name))
    });
    counts
}

impl Interpolatable for ExpandedNodeIdentifier {}

fn transform_scene_light_for_layer(
    mut light: SceneLight,
    source_scroll: [f64; 3],
    target_scroll: [f64; 3],
) -> SceneLight {
    if matches!(light.shape, LightShape::Point | LightShape::Strip) {
        light.position.x += source_scroll[0] - target_scroll[0];
        light.position.y += source_scroll[1] - target_scroll[1];
        light.position.z += source_scroll[2] - target_scroll[2];
    }
    light
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Stable runtime identifier assigned to an expanded node.
pub struct ExpandedNodeIdentifier(pub u32);

impl ExpandedNodeIdentifier {
    /// Convert to the integer id passed across chassis message boundaries.
    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Default)]
struct LayerLightRecipients {
    // Shared by siblings; these are render-tree scopes, not template ancestry.
    ancestry: HashMap<ExpandedNodeIdentifier, Rc<Vec<ExpandedNodeIdentifier>>>,
    pending: HashSet<ExpandedNodeIdentifier>,
}

/// Shared context for properties pass recursion
pub struct RuntimeContext {
    /// A host may prepare this application while another application is visible.
    /// Lifetime and measurement work continue; entry animations wait for publication.
    pub presentation_held: Cell<bool>,
    pub(crate) presentation_entry_prepared: Cell<bool>,
    pub(crate) presentation: RefCell<crate::presentation::PresentationState>,
    pub(crate) presentation_order_generation: LocalProperty<u64>,
    pub(crate) startup: RefCell<crate::engine::startup::Startup>,
    pub(crate) replaying_startup_input: RefCell<Option<crate::engine::startup::StartupInput>>,
    pub application: pax_runtime_api::application::ApplicationInstance,
    /// Owner-thread bridge for application properties.
    pub property_graph: pax_runtime_api::PropertyGraph,
    messages: RefCell<Vec<NativeMessage>>,
    globals: RefCell<Globals>,
    root_expanded_node: RefCell<Weak<ExpandedNode>>,
    #[cfg(feature = "designtime")]
    pub userland_frame_instance_node: RefCell<Option<Rc<dyn InstanceNode>>>,
    #[cfg(feature = "designtime")]
    pub userland_root_expanded_node: RefCell<Option<Rc<ExpandedNode>>>,
    node_cache: RefCell<NodeCache>,
    last_topmost_element: RefCell<Weak<ExpandedNode>>,
    active_touch_targets: RefCell<HashMap<i64, ExpandedNodeIdentifier>>,
    pub(crate) mouse_input: RefCell<crate::mouse_input::MouseInput>,
    pub(crate) wheel_regions: RefCell<crate::wheel_input::WheelRegions>,
    queued_custom_events: RefCell<Vec<QueuedEvent>>,
    queued_renders: RefCell<Vec<Rc<ExpandedNode>>>,
    pub layer_count: Cell<usize>,
    pub dirty_canvases: Rc<RefCell<Vec<bool>>>,
    dirty_canvas_nodes: RefCell<HashSet<ExpandedNodeIdentifier>>,
    canvas_node_orders: RefCell<HashMap<usize, HashMap<u32, i32>>>,
    scene_geometry: RefCell<crate::scene_geometry::SceneGeometry>,
    #[cfg(test)]
    input_query_unindexed: Cell<bool>,
    pub(crate) viewport_proximity: RefCell<crate::viewport_proximity::ViewportProximityState>,
    canvas_node_light_masks: RefCell<HashMap<ExpandedNodeIdentifier, u32>>,
    scene_lighting_nodes: RefCell<HashSet<ExpandedNodeIdentifier>>,
    scene_lighting_dirty: Cell<bool>,
    // Slot order and lexical owners determine masks; light color/position do not.
    layer_light_scopes:
        RefCell<HashMap<usize, Vec<(ExpandedNodeIdentifier, Option<ExpandedNodeIdentifier>)>>>,
    lighting_topology_dirty: Cell<bool>,
    layer_light_recipients: RefCell<HashMap<usize, LayerLightRecipients>>,
    #[cfg(test)]
    lighting_mask_node_visits: Cell<usize>,
    lighting_overflow_counts: RefCell<HashMap<usize, usize>>,
    // Nodes dirty solely because a physical tile needs replay. An independent
    // invalidation removes eligibility even if the node also intersects that tile.
    targeted_canvas_replay_node_ids: RefCell<HashMap<usize, HashSet<u32>>>,
    removed_canvas_nodes: RefCell<Vec<(usize, u32)>>,
    occlusion_dirty: Cell<bool>,
    pub(crate) occlusion_state: RefCell<crate::engine::occlusion::OcclusionState>,
    layer_canvas_plan_generation: Cell<u32>,
    canvas_drawable_layers: RefCell<HashSet<usize>>,
    import_settings_node_count: Cell<usize>,
    tick_handler_nodes: RefCell<Vec<ExpandedNodeIdentifier>>,
    pre_render_handler_nodes: RefCell<Vec<ExpandedNodeIdentifier>>,
    gyro_handler_nodes: RefCell<Vec<ExpandedNodeIdentifier>>,
    accel_handler_nodes: RefCell<Vec<ExpandedNodeIdentifier>>,
    screenshot_map: Rc<RefCell<HashMap<u32, ScreenshotData>>>,
    scroller_surface_states: RefCell<HashMap<u32, ScrollerSurfaceState>>,
    layer_scroller_owners: RefCell<HashMap<usize, ExpandedNodeIdentifier>>,
    root_scroller_id: Cell<Option<u32>>,
    visual_viewport_state: Cell<Option<VisualViewportState>>,
    safe_area_insets: LocalProperty<SafeAreaInsets>,
}

#[derive(Clone, Copy, Debug, Default)]
/// Last-known scroll state for a native or browser-owned scroller surface.
pub struct ScrollerSurfaceState {
    /// Origin of the finite native pane in stable content coordinates.
    pub content_origin: [f64; 2],
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Coarse classification of changes to a native or browser-owned scroller surface.
pub enum ScrollerSurfaceStateChange {
    /// No surface or presentation state changed.
    Unchanged,
    /// Only the content origin or sampled scroll offsets changed.
    ScrollOnly,
    /// Viewport/content extents changed without changing clipping ownership.
    Geometry,
    /// A surface was introduced or its clipping policy changed.
    Structural,
}

impl ScrollerSurfaceState {
    fn close_enough(lhs: f64, rhs: f64) -> bool {
        (lhs - rhs).abs() <= 1e-4
    }

    fn geometry_eq(&self, other: &Self) -> bool {
        Self::close_enough(self.viewport_width, other.viewport_width)
            && Self::close_enough(self.viewport_height, other.viewport_height)
            && Self::close_enough(self.content_width, other.content_width)
            && Self::close_enough(self.content_height, other.content_height)
    }

    fn scroll_eq(&self, other: &Self) -> bool {
        self.content_origin == other.content_origin
            && Self::close_enough(self.scroll_x, other.scroll_x)
            && Self::close_enough(self.scroll_y, other.scroll_y)
            && Self::close_enough(self.presentation_scroll_x, other.presentation_scroll_x)
            && Self::close_enough(self.presentation_scroll_y, other.presentation_scroll_y)
    }
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
        let application =
            pax_runtime_api::application::ApplicationInstance::for_runtime(globals.target);
        {
            let graph = application.property_graph();
            let _entered = graph.enter();
            pax_runtime_api::properties::register_time(&globals.elapsed_frames);
            pax_runtime_api::properties::register_millis(&globals.elapsed_millis);
        }
        Self {
            property_graph: application.property_graph(),
            presentation_held: Cell::new(false),
            presentation_entry_prepared: Cell::new(false),
            presentation: Default::default(),
            presentation_order_generation: LocalProperty::new(0),
            startup: Default::default(),
            replaying_startup_input: Default::default(),
            application,
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            node_cache: RefCell::new(NodeCache::new()),
            active_touch_targets: Default::default(),
            mouse_input: Default::default(),
            wheel_regions: Default::default(),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            canvas_node_orders: Default::default(),
            scene_geometry: Default::default(),
            #[cfg(test)]
            input_query_unindexed: Default::default(),
            viewport_proximity: Default::default(),
            canvas_node_light_masks: Default::default(),
            scene_lighting_nodes: Default::default(),
            scene_lighting_dirty: Default::default(),
            layer_light_scopes: Default::default(),
            lighting_topology_dirty: Cell::new(true),
            layer_light_recipients: Default::default(),
            #[cfg(test)]
            lighting_mask_node_visits: Cell::new(0),
            lighting_overflow_counts: Default::default(),
            targeted_canvas_replay_node_ids: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
            occlusion_state: Default::default(),
            layer_canvas_plan_generation: Cell::new(1),
            canvas_drawable_layers: Default::default(),
            import_settings_node_count: Cell::new(0),
            tick_handler_nodes: Default::default(),
            pre_render_handler_nodes: Default::default(),
            gyro_handler_nodes: Default::default(),
            accel_handler_nodes: Default::default(),
            screenshot_map: Default::default(),
            scroller_surface_states: Default::default(),
            layer_scroller_owners: Default::default(),
            root_scroller_id: Cell::new(None),
            visual_viewport_state: Cell::new(None),
            safe_area_insets: LocalProperty::default(),
        }
    }

    #[cfg(feature = "designtime")]
    /// Create a runtime context with the userland component tracked for designtime tools.
    pub fn new(globals: Globals, userland: Rc<ComponentInstance>) -> Self {
        let application =
            pax_runtime_api::application::ApplicationInstance::for_runtime(globals.target);
        {
            let graph = application.property_graph();
            let _entered = graph.enter();
            pax_runtime_api::properties::register_time(&globals.elapsed_frames);
            pax_runtime_api::properties::register_millis(&globals.elapsed_millis);
        }
        Self {
            property_graph: application.property_graph(),
            presentation_held: Cell::new(false),
            presentation_entry_prepared: Cell::new(false),
            presentation: Default::default(),
            presentation_order_generation: LocalProperty::new(0),
            startup: Default::default(),
            replaying_startup_input: Default::default(),
            application,
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            userland_frame_instance_node: RefCell::new(Some(userland)),
            userland_root_expanded_node: Default::default(),
            node_cache: RefCell::new(NodeCache::new()),
            active_touch_targets: Default::default(),
            mouse_input: Default::default(),
            wheel_regions: Default::default(),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            canvas_node_orders: Default::default(),
            scene_geometry: Default::default(),
            #[cfg(test)]
            input_query_unindexed: Default::default(),
            viewport_proximity: Default::default(),
            canvas_node_light_masks: Default::default(),
            scene_lighting_nodes: Default::default(),
            scene_lighting_dirty: Default::default(),
            layer_light_scopes: Default::default(),
            lighting_topology_dirty: Cell::new(true),
            layer_light_recipients: Default::default(),
            #[cfg(test)]
            lighting_mask_node_visits: Cell::new(0),
            lighting_overflow_counts: Default::default(),
            targeted_canvas_replay_node_ids: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
            occlusion_state: Default::default(),
            layer_canvas_plan_generation: Cell::new(1),
            canvas_drawable_layers: Default::default(),
            import_settings_node_count: Cell::new(0),
            tick_handler_nodes: Default::default(),
            pre_render_handler_nodes: Default::default(),
            gyro_handler_nodes: Default::default(),
            accel_handler_nodes: Default::default(),
            screenshot_map: Default::default(),
            scroller_surface_states: Default::default(),
            layer_scroller_owners: Default::default(),
            root_scroller_id: Cell::new(None),
            visual_viewport_state: Cell::new(None),
            safe_area_insets: LocalProperty::default(),
        }
    }

    #[cfg(feature = "designtime")]
    /// Create a runtime context before any userland tree has been mounted.
    pub fn new_empty(globals: Globals) -> Self {
        let application =
            pax_runtime_api::application::ApplicationInstance::for_runtime(globals.target);
        {
            let graph = application.property_graph();
            let _entered = graph.enter();
            pax_runtime_api::properties::register_time(&globals.elapsed_frames);
            pax_runtime_api::properties::register_millis(&globals.elapsed_millis);
        }
        Self {
            property_graph: application.property_graph(),
            presentation_held: Cell::new(false),
            presentation_entry_prepared: Cell::new(false),
            presentation: Default::default(),
            presentation_order_generation: LocalProperty::new(0),
            startup: Default::default(),
            replaying_startup_input: Default::default(),
            wheel_regions: Default::default(),
            application,
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            userland_frame_instance_node: RefCell::new(None),
            userland_root_expanded_node: Default::default(),
            node_cache: RefCell::new(NodeCache::new()),
            active_touch_targets: Default::default(),
            mouse_input: Default::default(),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            canvas_node_orders: Default::default(),
            scene_geometry: Default::default(),
            #[cfg(test)]
            input_query_unindexed: Default::default(),
            viewport_proximity: Default::default(),
            canvas_node_light_masks: Default::default(),
            scene_lighting_nodes: Default::default(),
            scene_lighting_dirty: Default::default(),
            layer_light_scopes: Default::default(),
            lighting_topology_dirty: Cell::new(true),
            layer_light_recipients: Default::default(),
            #[cfg(test)]
            lighting_mask_node_visits: Cell::new(0),
            lighting_overflow_counts: Default::default(),
            targeted_canvas_replay_node_ids: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
            occlusion_state: Default::default(),
            layer_canvas_plan_generation: Cell::new(1),
            canvas_drawable_layers: Default::default(),
            import_settings_node_count: Cell::new(0),
            tick_handler_nodes: Default::default(),
            pre_render_handler_nodes: Default::default(),
            gyro_handler_nodes: Default::default(),
            accel_handler_nodes: Default::default(),
            screenshot_map: Default::default(),
            scroller_surface_states: Default::default(),
            layer_scroller_owners: Default::default(),
            root_scroller_id: Cell::new(None),
            visual_viewport_state: Cell::new(None),
            safe_area_insets: LocalProperty::default(),
        }
    }

    /// Store the root expanded node after it has been initialized.
    pub fn register_root_expanded_node(&self, root: &Rc<ExpandedNode>) {
        *borrow_mut!(self.root_expanded_node) = Rc::downgrade(root);
        // Synchronous mounting may discover overlays before the root is stored.
        self.authored_order_changed(root);
        #[cfg(feature = "3d")]
        crate::camera::bind_root(root, self.globals().renderer_is_gpu);
    }

    #[cfg(not(feature = "3d"))]
    pub(crate) fn rebind_root_camera(&self, _node: &Rc<ExpandedNode>) {}

    #[cfg(feature = "3d")]
    pub(crate) fn rebind_root_camera(&self, node: &Rc<ExpandedNode>) {
        #[cfg(feature = "designtime")]
        let root = self
            .get_userland_root_expanded_node()
            .or_else(|| self.get_root_expanded_node());
        #[cfg(not(feature = "designtime"))]
        let root = self.get_root_expanded_node();
        if root.as_ref().is_some_and(|root| root.id == node.id) {
            crate::camera::bind_root(node, self.globals().renderer_is_gpu);
        }
    }

    /// Clear the registered root expanded node.
    pub fn clear_root_expanded_node(&self) {
        *borrow_mut!(self.root_expanded_node) = Weak::new();
        let mut state = self.occlusion_state.borrow_mut();
        let stats = state.stats;
        *state = Default::default();
        state.stats = stats;
    }

    /// Add a node to runtime lookup caches.
    pub fn add_to_cache(&self, node: &Rc<ExpandedNode>) {
        borrow_mut!(self.node_cache).add_to_cache(node);
        self.refresh_scene_lighting_node(node);
        self.register_node_lifecycle_handlers(node);
        if borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas {
            // Initial drawing belongs to mount, independent of whether lighting
            // happens to assign a retained mask to this primitive.
            self.mark_canvas_node_dirty(node.id);
            if !node.is_render_source() {
                self.set_canvas_dirty(node.occlusion.get().render_layer_id);
            }
        }
        if node.is_import_settings_node() {
            self.import_settings_node_count
                .set(self.import_settings_node_count.get() + 1);
        }
        self.mark_occlusion_dirty_for_node(node.id.to_u32());
    }

    /// Remove a node from runtime lookup caches.
    pub fn remove_from_cache(&self, node: &Rc<ExpandedNode>) {
        self.remove_wheel_region(node.id.to_u32());
        borrow_mut!(self.node_cache).remove_from_cache(node);
        // Render work belongs to a mounted instance. A stale ID otherwise makes
        // every filtered traversal fall back to the whole tree indefinitely.
        borrow_mut!(self.dirty_canvas_nodes).remove(&node.id);
        for orders in self.canvas_node_orders.borrow_mut().values_mut() {
            orders.remove(&node.id.to_u32());
        }
        for ids in borrow_mut!(self.targeted_canvas_replay_node_ids).values_mut() {
            ids.remove(&node.id.to_u32());
        }
        if borrow_mut!(self.scene_lighting_nodes).remove(&node.id) {
            // Ancestor lights can affect canvases other than their own layer.
            self.mark_scene_lighting_dirty();
        }
        self.unregister_viewport_proximity(node.id.to_u32());
        borrow_mut!(self.scene_geometry).remove(node.id.to_u32());
        self.occlusion_state.borrow_mut().remove(node.id.to_u32());
        borrow_mut!(self.canvas_node_light_masks).remove(&node.id);
        borrow_mut!(self.active_touch_targets).retain(|_, target| *target != node.id);
        borrow_mut!(self.mouse_input).forget(node.id);
        self.unregister_node_lifecycle_handlers(node.id);
        if node.is_import_settings_node() {
            self.import_settings_node_count
                .set(self.import_settings_node_count.get().saturating_sub(1));
        }
        self.mark_occlusion_dirty_for_node(node.id.to_u32());
    }

    pub fn has_import_settings_nodes(&self) -> bool {
        self.import_settings_node_count.get() > 0
    }

    pub(crate) fn refresh_scene_lighting_node(&self, node: &ExpandedNode) {
        let mut contributors = borrow_mut!(self.scene_lighting_nodes);
        let changed = if borrow!(node.instance_node).has_scene_lighting() {
            contributors.insert(node.id)
        } else {
            contributors.remove(&node.id)
        };
        if changed {
            self.mark_scene_lighting_dirty();
        }
    }

    pub fn register_node_effect_property(
        &self,
        node: ExpandedNodeIdentifier,
        prop: &LocalProperty<()>,
    ) {
        self.register_node_effect_property_named(node, prop, "node effect");
    }

    pub fn register_node_effect_property_named(
        &self,
        node: ExpandedNodeIdentifier,
        prop: &LocalProperty<()>,
        effect_name: &str,
    ) {
        if self.property_graph.diagnostic_names_enabled() {
            let category = self
                .get_expanded_node_by_eid(node)
                .map(|node| Self::startup_node_category(&node))
                .unwrap_or_else(|| "<unknown>".to_owned());
            pax_runtime_api::properties::set_property_diagnostic_name(
                &prop.untyped(),
                &format!("{effect_name} (node id: {}, {category})", node.0),
            );
        }
        if effect_drain_instrumentation_enabled() {
            let debug_name = self.node_effect_debug_name(node, effect_name);
            register_effect_property_with_name(prop, &debug_name);
        } else {
            register_effect_property(prop);
        }
    }

    pub fn register_expanded_node_effect_property_named(
        &self,
        node: &Rc<ExpandedNode>,
        prop: &LocalProperty<()>,
        effect_name: &str,
    ) {
        if self.property_graph.diagnostic_names_enabled() {
            pax_runtime_api::properties::set_property_diagnostic_name(
                &prop.untyped(),
                &format!(
                    "{effect_name} (node id: {}, {})",
                    node.id.0,
                    Self::startup_node_category(node)
                ),
            );
        }
        if effect_drain_instrumentation_enabled() {
            let debug_name = Self::expanded_node_effect_debug_name(node, effect_name);
            register_effect_property_with_name(prop, &debug_name);
        } else {
            register_effect_property(prop);
        }
    }

    pub fn register_node_effect(
        &self,
        node: ExpandedNodeIdentifier,
        dependencies: &[UntypedProperty],
        effect: impl Fn() + 'static,
    ) -> LocalProperty<()> {
        let prop = LocalProperty::computed(effect, dependencies);
        self.register_node_effect_property_named(node, &prop, "registered node effect");
        prop
    }

    pub fn drain_node_effects(&self) {
        self.drain_node_effects_with_observer(|_| {});
    }

    pub(crate) fn drain_node_effects_with_observer(&self, mut completed: impl FnMut(&'static str)) {
        let _graph = self.property_graph.enter();
        self.property_graph
            .import_batch_with_observer(1024, &mut completed);
        self.drain_local_node_effects();
        completed("property_effects");
    }

    fn drain_local_node_effects(&self) {
        const MAX_NODE_EFFECTS_PER_TICK: usize = 100_000;
        if !effect_drain_instrumentation_enabled() {
            let drained = drain_effects(MAX_NODE_EFFECTS_PER_TICK);
            if drained == MAX_NODE_EFFECTS_PER_TICK {
                log::warn!(
                    "reactive drain hit {} evaluations in one tick; deferring remaining work",
                    MAX_NODE_EFFECTS_PER_TICK
                );
            }
            return;
        }

        let start = Instant::now();
        let report = drain_effects_with_report(MAX_NODE_EFFECTS_PER_TICK);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        if report.budget_exhausted {
            log::warn!(
                "reactive drain hit {} evaluations in one tick; deferring remaining work",
                MAX_NODE_EFFECTS_PER_TICK
            );
        }
        if report.popped > 0 || report.cutoffs_evaluated > 0 {
            let globals = self.globals();
            let frame_dependents = summarize_debug_names(property_outbound_debug_names(
                &globals.elapsed_frames.untyped(),
            ));
            let millis_dependents = summarize_debug_names(property_outbound_debug_names(
                &globals.elapsed_millis.untyped(),
            ));
            let frame_clock = globals.elapsed_frames.untyped();
            let millis_clock = globals.elapsed_millis.untyped();
            let (entering, exiting, enter_cleanup, exit_cleanup, stale_clock_nodes) = {
                let nodes = borrow!(self.node_cache);
                nodes.eid_to_node.values().fold(
                    (0, 0, 0, 0, Vec::new()),
                    |(entering, exiting, enter_cleanup, exit_cleanup, mut stale_clock_nodes),
                     node| {
                        let phase = node.transition_phase.get();
                        if phase != TRANSITION_PHASE_ENTER
                            && phase != TRANSITION_PHASE_EXIT
                            && (property_has_direct_outbound(
                                &frame_clock,
                                &node.transition_playhead.untyped(),
                            ) || property_has_direct_outbound(
                                &millis_clock,
                                &node.transition_playhead_millis.untyped(),
                            ))
                        {
                            stale_clock_nodes
                                .push(Self::expanded_node_effect_debug_name(node, "idle clock"));
                        }
                        (
                            entering + usize::from(phase == TRANSITION_PHASE_ENTER),
                            exiting + usize::from(phase == TRANSITION_PHASE_EXIT),
                            enter_cleanup + usize::from(node.enter_cleanup_active.get()),
                            exit_cleanup + usize::from(node.exit_cleanup_active.get()),
                            stale_clock_nodes,
                        )
                    },
                )
            };
            println!(
                "[PaxEffectDrain] ran={} popped={} clean={} unregistered={} missing={} remaining={} cutoffs={} suppressed={} propagated={} remaining_cutoffs={} elapsed_ms={:.3} transitions=enter:{}/exit:{}/enter_cleanup:{}/exit_cleanup:{} stale_clocks=[{}] frame_deps=[{}] millis_deps=[{}] effects=[{}] cutoff_nodes=[{}]",
                report.ran,
                report.popped,
                report.skipped_clean,
                report.skipped_unregistered,
                report.skipped_missing,
                report.remaining,
                report.cutoffs_evaluated,
                report.cutoffs_suppressed,
                report.cutoffs_propagated,
                report.remaining_cutoffs,
                elapsed_ms,
                entering,
                exiting,
                enter_cleanup,
                exit_cleanup,
                stale_clock_nodes.join("; "),
                format_effect_top(&frame_dependents),
                format_effect_top(&millis_dependents),
                format_effect_top(&report.top_effects),
                format_effect_top(&report.top_cutoffs),
            );
        }
    }

    fn node_effect_debug_name(&self, node: ExpandedNodeIdentifier, effect_name: &str) -> String {
        let node = borrow!(self.node_cache).eid_to_node.get(&node).cloned();
        if let Some(node) = node {
            Self::expanded_node_effect_debug_name(&node, effect_name)
        } else {
            format!("{effect_name} node={node:?}")
        }
    }

    fn expanded_node_effect_debug_name(node: &Rc<ExpandedNode>, effect_name: &str) -> String {
        let instance_node = borrow!(node.instance_node);
        let base = instance_node.base();
        let type_name = base
            .template_node_type_id
            .as_ref()
            .map(|type_id| {
                type_id
                    .get_pascal_identifier()
                    .unwrap_or_else(|| type_id.to_string())
            })
            .unwrap_or_else(|| "<unknown>".to_owned());

        format!("{effect_name} type={type_name}")
    }

    pub(crate) fn startup_node_category(node: &ExpandedNode) -> String {
        let instance = node.instance_node.borrow();
        let base = instance.base();
        let component = base
            .template_node_identifier
            .as_ref()
            .map(|id| id.get_containing_component_type_id().to_string())
            .unwrap_or_else(|| "<unknown>".to_owned());
        let kind = base
            .template_node_type_id
            .as_ref()
            .map(|id| id.get_pascal_identifier().unwrap_or_else(|| id.to_string()))
            .unwrap_or_else(|| "<unknown>".to_owned());
        format!("component={component} type={kind}")
    }

    pub fn tick_handler_nodes(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.tick_handler_nodes).clone()
    }

    pub fn pre_render_handler_nodes(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.pre_render_handler_nodes).clone()
    }

    pub fn gyro_handler_nodes(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.gyro_handler_nodes).clone()
    }

    pub fn accel_handler_nodes(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.accel_handler_nodes).clone()
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
        if handlers.get(GYRO_HANDLERS).is_some_and(|h| !h.is_empty()) {
            self.register_lifecycle_handler(&self.gyro_handler_nodes, node.id);
        }
        if handlers.get(ACCEL_HANDLERS).is_some_and(|h| !h.is_empty()) {
            self.register_lifecycle_handler(&self.accel_handler_nodes, node.id);
        }
    }

    fn unregister_node_lifecycle_handlers(&self, id: ExpandedNodeIdentifier) {
        borrow_mut!(self.tick_handler_nodes).retain(|node_id| *node_id != id);
        borrow_mut!(self.pre_render_handler_nodes).retain(|node_id| *node_id != id);
        borrow_mut!(self.gyro_handler_nodes).retain(|node_id| *node_id != id);
        borrow_mut!(self.accel_handler_nodes).retain(|node_id| *node_id != id);
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

    /// Route a touch sequence to the node hit at touch-down, even after the finger moves away.
    pub fn capture_touch_target(&self, identifier: i64, target: ExpandedNodeIdentifier) {
        borrow_mut!(self.active_touch_targets).insert(identifier, target);
    }

    /// Resolve the node captured for an active touch sequence.
    pub fn captured_touch_target(&self, identifier: i64) -> Option<Rc<ExpandedNode>> {
        let target = borrow!(self.active_touch_targets)
            .get(&identifier)
            .copied()?;
        self.get_expanded_node_by_eid(target)
    }

    /// Release and resolve the node captured for a completed touch sequence.
    pub fn release_touch_target(&self, identifier: i64) -> Option<Rc<ExpandedNode>> {
        let target = borrow_mut!(self.active_touch_targets).remove(&identifier)?;
        self.get_expanded_node_by_eid(target)
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
    pub fn set_scroller_surface_state(
        &self,
        id: u32,
        state: ScrollerSurfaceState,
    ) -> ScrollerSurfaceStateChange {
        let change = {
            let mut states = borrow_mut!(self.scroller_surface_states);
            let change = match states.get(&id) {
                Some(previous) if previous.clip_content != state.clip_content => {
                    ScrollerSurfaceStateChange::Structural
                }
                Some(previous) if !previous.geometry_eq(&state) => {
                    ScrollerSurfaceStateChange::Geometry
                }
                Some(previous) if previous.scroll_eq(&state) => {
                    ScrollerSurfaceStateChange::Unchanged
                }
                Some(_) => ScrollerSurfaceStateChange::ScrollOnly,
                _ => ScrollerSurfaceStateChange::Structural,
            };
            if change != ScrollerSurfaceStateChange::Unchanged {
                states.insert(id, state);
            }
            change
        };

        match change {
            ScrollerSurfaceStateChange::Unchanged => {}
            ScrollerSurfaceStateChange::ScrollOnly | ScrollerSurfaceStateChange::Geometry => {
                // Extents affect viewport presentation and canvas residency, not
                // layer ownership. Actual node/clip geometry has its own dirty listener.
                self.mark_occlusion_presentation_dirty(id);
                self.invalidate_viewport_presentation();
                self.mark_layer_canvas_plans_dirty();
                self.mark_scroller_content_layers_dirty(id);
            }
            ScrollerSurfaceStateChange::Structural => {
                self.invalidate_viewport_presentation();
                self.mark_layer_canvas_plans_dirty();
                self.mark_occlusion_dirty_for_node(id);
            }
        }

        if change != ScrollerSurfaceStateChange::Unchanged {
            self.invalidate_scroll_input(id);
            self.invalidate_projected_scroll_descendants(id);
        }

        change
    }

    /// Update hot scroll offsets for an existing native scroller surface without touching
    /// structural state.
    pub fn update_scroller_surface_scroll(
        &self,
        id: u32,
        scroll_x: f64,
        scroll_y: f64,
        presentation_scroll_x: f64,
        presentation_scroll_y: f64,
    ) -> ScrollerSurfaceStateChange {
        let change = {
            let mut states = borrow_mut!(self.scroller_surface_states);
            let Some(state) = states.get_mut(&id) else {
                return ScrollerSurfaceStateChange::Unchanged;
            };
            let next = ScrollerSurfaceState {
                scroll_x,
                scroll_y,
                presentation_scroll_x,
                presentation_scroll_y,
                ..*state
            };
            let change = if state.scroll_eq(&next) {
                ScrollerSurfaceStateChange::Unchanged
            } else {
                ScrollerSurfaceStateChange::ScrollOnly
            };
            if change != ScrollerSurfaceStateChange::Unchanged {
                *state = next;
            }
            change
        };

        if change == ScrollerSurfaceStateChange::ScrollOnly {
            self.invalidate_scroll_input(id);
            self.mark_occlusion_presentation_dirty(id);
            self.invalidate_viewport_presentation();
            self.mark_layer_canvas_plans_dirty();
            self.invalidate_projected_scroll_descendants(id);
        }

        change
    }

    /// Remove cached scroller surface state.
    pub fn remove_scroller_surface_state(&self, id: u32) {
        borrow_mut!(self.scroller_surface_states).remove(&id);
        self.invalidate_scroll_input(id);
        self.invalidate_viewport_presentation();
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Fetch cached scroller surface state by node id.
    pub fn get_scroller_surface_state(&self, id: u32) -> Option<ScrollerSurfaceState> {
        borrow!(self.scroller_surface_states).get(&id).cloned()
    }

    /// Fetch the presentation scroll offset for a native scroller surface, falling back to the
    /// authoritative scroll position when presentation scroll is unavailable.
    pub fn get_scroller_surface_scroll(&self, id: u32) -> Option<(f64, f64)> {
        borrow!(self.scroller_surface_states).get(&id).map(|state| {
            let scroll_x = if state.presentation_scroll_x.is_finite() {
                state.presentation_scroll_x
            } else {
                state.scroll_x
            };
            let scroll_y = if state.presentation_scroll_y.is_finite() {
                state.presentation_scroll_y
            } else {
                state.scroll_y
            };
            (scroll_x, scroll_y)
        })
    }

    pub(crate) fn scroller_presentation_offset(&self, node: &ExpandedNode) -> (f64, f64) {
        fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
            if content <= viewport || !value.is_finite() {
                return 0.0;
            }
            value.max(0.0).min((content - viewport).max(0.0))
        }

        let root_delegates_to_page_scroll = self.get_root_scroller_id() == Some(node.id.to_u32())
            && self.get_visual_viewport_state().is_some();
        if root_delegates_to_page_scroll {
            return (0.0, 0.0);
        }

        let instance_node = borrow!(node.instance_node);
        let (scroll_x, scroll_y) = if self.get_root_scroller_id() == Some(node.id.to_u32()) {
            if let Some(visual) = self.get_visual_viewport_state() {
                let visual_x = visual.page_scroll_x + visual.offset_x;
                let visual_y = visual.page_scroll_y + visual.offset_y;
                if visual_x.is_finite() && visual_y.is_finite() {
                    if let Some(state) = self.get_scroller_surface_state(node.id.to_u32()) {
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
                            clamp_offset(visual_x, state.content_width, viewport_width),
                            clamp_offset(visual_y, state.content_height, viewport_height),
                        )
                    } else {
                        (visual_x, visual_y)
                    }
                } else {
                    self.get_scroller_surface_scroll(node.id.to_u32())
                        .or_else(|| instance_node.resolve_scroll_offset(node))
                        .unwrap_or((0.0, 0.0))
                }
            } else {
                self.get_scroller_surface_scroll(node.id.to_u32())
                    .or_else(|| instance_node.resolve_scroll_offset(node))
                    .unwrap_or((0.0, 0.0))
            }
        } else {
            self.get_scroller_surface_scroll(node.id.to_u32())
                .or_else(|| instance_node.resolve_scroll_offset(node))
                .unwrap_or((0.0, 0.0))
        };

        (scroll_x, scroll_y)
    }

    pub(crate) fn compute_scroller_presentation_transform(
        &self,
        node: &ExpandedNode,
        (scroll_x, scroll_y): (f64, f64),
    ) -> Affine {
        if scroll_x.abs() <= f64::EPSILON && scroll_y.abs() <= f64::EPSILON {
            return Affine::IDENTITY;
        }
        #[cfg(feature = "3d")]
        {
            let geometry = node.presentation_geometry.get();
            if geometry.is_spatial() {
                let m = geometry.world_transform().coeffs();
                // Project the local scroll vector directly. The projected plane can be
                // singular while its local coordinate frame remains perfectly valid.
                return Affine::translate((
                    -m[0] * scroll_x - m[4] * scroll_y,
                    -m[1] * scroll_x - m[5] * scroll_y,
                ));
            }
        }
        let world_transform = Affine::from(node.presentation_transform_and_bounds.get().transform);
        let inverse_world = Affine::from(
            node.presentation_transform_and_bounds
                .get()
                .transform
                .inverse(),
        );
        world_transform * Affine::translate((-scroll_x, -scroll_y)) * inverse_world
    }

    #[cfg(feature = "3d")]
    fn scroller_content_depth_offset(&self, node: &ExpandedNode) -> f64 {
        node.scroll_state.get().depth_step
    }

    #[cfg(feature = "3d")]
    pub(crate) fn presented_depth_plane(
        &self,
        node: &ExpandedNode,
        presented: Affine,
    ) -> Option<pax_runtime_api::DepthPlane> {
        if node.has_perspective_camera() {
            return self.depth_plane_in_presentation(
                node,
                self.prepared_presentation(node).depth_plane(),
            );
        }
        let offset = self.presentation_depth_offset(node);
        let plane =
            self.depth_plane_in_presentation(
                node,
                node.presentation_geometry.get().depth_plane(
                    pax_runtime_api::math::Transform2::new(presented.as_coeffs()),
                ),
            );
        if plane.is_none() && offset == 0.0 {
            return None;
        }
        let mut plane = plane.unwrap_or_default();
        plane.coefficients[2] += offset;
        Some(plane)
    }

    #[cfg(feature = "3d")]
    /// Attach the node's independent presentation domain to canvas/native depth.
    pub fn depth_plane_in_presentation(
        &self,
        node: &ExpandedNode,
        plane: Option<pax_runtime_api::DepthPlane>,
    ) -> Option<pax_runtime_api::DepthPlane> {
        let layer = node.presentation_layer.get();
        if plane.is_none() && layer == 0 {
            return None;
        }
        let mut plane = plane.unwrap_or_default();
        plane.presentation_layer = layer;
        Some(plane)
    }

    #[cfg(feature = "3d")]
    pub(crate) fn presentation_depth_offset(&self, node: &ExpandedNode) -> f64 {
        node.scroll_state.get().inherited.depth
    }

    /// Resolve the browser/native presentation transform inherited by a node from ancestor
    /// scrollers. Layout transforms remain in content coordinates, while events arrive in the
    /// scrolled window coordinates that the user sees.
    pub(crate) fn presentation_scroll_transform_for_node(&self, node: &ExpandedNode) -> Affine {
        node.scroll_state.get().inherited.affine
    }

    pub(crate) fn destination_scroll_transform_for_node(&self, node: &ExpandedNode) -> Affine {
        node.render_parent_node()
            .map_or(Affine::IDENTITY, |parent| {
                parent.scroll_state.get().children.affine
            })
    }

    pub(crate) fn scroller_content_presentation_transform(&self, node: &ExpandedNode) -> Affine {
        node.scroll_state.get().step
    }

    /// Clear render-layer-to-scroller ownership before recomputing occlusion.
    pub fn clear_layer_scroller_owners(&self) {
        borrow_mut!(self.layer_scroller_owners).clear();
        self.mark_layer_canvas_plans_dirty();
    }

    /// Record that a render layer is owned by a particular scroller.
    pub fn register_layer_scroller_owner(
        &self,
        layer_id: usize,
        scroller_id: ExpandedNodeIdentifier,
    ) {
        borrow_mut!(self.layer_scroller_owners).insert(layer_id, scroller_id);
        self.mark_layer_canvas_plans_dirty();
    }

    /// Find the scroller that owns a render layer, when one exists.
    pub fn get_layer_scroller_owner(&self, layer_id: usize) -> Option<ExpandedNodeIdentifier> {
        borrow!(self.layer_scroller_owners).get(&layer_id).copied()
    }

    fn mark_scroller_content_layers_dirty(&self, scroller_id: u32) {
        let layers = borrow!(self.layer_scroller_owners)
            .iter()
            .filter_map(|(layer, owner)| (owner.to_u32() == scroller_id).then_some(*layer))
            .collect::<Vec<_>>();
        for layer in layers {
            self.set_canvas_dirty(layer);
        }
    }

    fn invalidate_projected_scroll_descendants(&self, scroller_id: u32) {
        let Some(owner) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(scroller_id)) else {
            return;
        };
        if !owner.has_perspective_camera() {
            return;
        }
        // A scroll translation occurs before projection. Surfaces at different
        // depths cannot share the browser's single post-projection translation.
        let mut pending = owner.children.get();
        while let Some(node) = pending.pop() {
            node.changed_listener.invalidate();
            self.mark_canvas_node_dirty(node.id);
            pending.extend(node.children.get());
        }
    }

    /// Mark which node currently delegates root scrolling behavior to the page.
    pub fn set_root_scroller_id(&self, id: Option<u32>) {
        let previous = self.root_scroller_id.replace(id);
        for id in previous.into_iter().chain(id) {
            self.invalidate_scroll_input(id);
        }
        self.invalidate_viewport_presentation();
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Current page-scroll-backed root scroller id.
    pub fn get_root_scroller_id(&self) -> Option<u32> {
        self.root_scroller_id.get()
    }

    /// Live native safe-area data used by opt-in layout primitives. Defaults to zero.
    pub fn safe_area_insets(&self) -> LocalProperty<SafeAreaInsets> {
        self.safe_area_insets.clone()
    }

    /// Update native safe-area data without changing the viewport or root layout.
    pub fn set_safe_area_insets(&self, insets: SafeAreaInsets) {
        let valid = |value: f64| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        self.safe_area_insets.set_if_neq(SafeAreaInsets {
            top: valid(insets.top),
            right: valid(insets.right),
            bottom: valid(insets.bottom),
            left: valid(insets.left),
        });
    }

    /// Cache the browser visual viewport state for root scroller math.
    pub fn set_visual_viewport_state(&self, state: VisualViewportState) {
        self.visual_viewport_state.set(Some(state));
        if let Some(id) = self.root_scroller_id.get() {
            self.invalidate_scroll_input(id);
        }
        self.invalidate_viewport_presentation();
        self.mark_layer_canvas_plans_dirty();
        self.mark_occlusion_dirty();
    }

    /// Clear cached visual viewport state.
    pub fn clear_visual_viewport_state(&self) {
        self.visual_viewport_state.set(None);
        if let Some(id) = self.root_scroller_id.get() {
            self.invalidate_scroll_input(id);
        }
        self.invalidate_viewport_presentation();
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
        // Render-layer reassignment does not change observed layout/scroll
        // geometry. Presentation setters invalidate observation explicitly.
        let next = self.layer_canvas_plan_generation.get().wrapping_add(1);
        self.layer_canvas_plan_generation.set(next.max(1));
    }

    pub fn layer_canvas_plan_generation(&self) -> u32 {
        self.layer_canvas_plan_generation.get()
    }

    /// Replace the set of render layers that currently contain canvas drawables.
    pub fn set_canvas_drawable_layers(&self, layers: HashSet<usize>) {
        let mut current = borrow_mut!(self.canvas_drawable_layers);
        if *current == layers {
            return;
        }
        *current = layers;
        self.mark_layer_canvas_plans_dirty();
    }

    /// Return whether a render layer currently has canvas work to paint.
    pub fn layer_has_canvas_drawables(&self, layer: usize) -> bool {
        borrow!(self.canvas_drawable_layers).contains(&layer)
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

    pub(crate) fn has_canvas_node_removals_on_layer(&self, layer: usize) -> bool {
        borrow!(self.removed_canvas_nodes)
            .iter()
            .any(|(owner, _)| *owner == layer)
    }

    pub fn has_canvas_render_work(&self) -> bool {
        self.has_dirty_canvas_layers()
            || self.has_canvas_node_removals()
            || self.scene_lighting_dirty.get()
            || !self.canvas_node_orders.borrow().is_empty()
    }

    pub(crate) fn queue_canvas_node_order(&self, layer: usize, id: u32, z: i32) {
        self.canvas_node_orders
            .borrow_mut()
            .entry(layer)
            .or_default()
            .insert(id, z);
    }

    pub(crate) fn take_canvas_node_orders(&self) -> HashMap<usize, HashMap<u32, i32>> {
        std::mem::take(&mut *self.canvas_node_orders.borrow_mut())
    }

    /// Schedule lighting resolution without invalidating canvas geometry or contents.
    /// The retained renderer decides which surfaces depend on the changed lights.
    pub fn mark_scene_lighting_dirty(&self) {
        self.scene_lighting_dirty.set(true);
    }

    pub(crate) fn take_scene_lighting_dirty(&self) -> bool {
        self.scene_lighting_dirty.replace(false)
    }

    /// Whether a presentation-only pass may inspect cached geometry without evaluating
    /// scene state. Pending geometry, native compositing, and effects block the pass.
    /// Lighting/removals are checked per layer, and content per selected node:
    /// never-rendered cold layers must not block unrelated cached surfaces forever.
    pub fn can_present_settled_surfaces(&self) -> bool {
        self.settled_surface_blocker().is_none()
    }

    pub(crate) fn settled_surface_blocker(&self) -> Option<&'static str> {
        if self.startup.borrow().blocks_presentation() {
            Some("startup")
        } else if pax_runtime_api::properties::has_pending_effects() {
            Some("effects")
        } else if self.occlusion_dirty.get() {
            Some("occlusion")
        } else if !borrow!(self.scene_geometry).is_settled() {
            Some("geometry")
        } else {
            None
        }
    }

    pub(crate) fn can_replay_settled_layer(&self, layer: usize, nodes: &[u32]) -> bool {
        self.can_prepare_settled_layer(layer)
            && !nodes
                .iter()
                .any(|id| self.is_canvas_node_dirty(&ExpandedNodeIdentifier(*id)))
    }

    pub(crate) fn can_prepare_settled_layer(&self, layer: usize) -> bool {
        // A settled layer can contain render-dirty nodes which never had a GPU
        // target, or are awaiting decoded pixels. They need bounded capture, not
        // another application tick. Application invalidation dirties the layer
        // and/or the scene/effect guards checked by the caller.
        !self.is_canvas_dirty(&layer)
            && !self.canvas_node_orders.borrow().contains_key(&layer)
            && !borrow!(self.removed_canvas_nodes)
                .iter()
                .any(|(owner, _)| *owner == layer)
    }

    pub(crate) fn settled_canvas_nodes_intersecting(
        &self,
        layer: usize,
        regions: &[kurbo::Rect],
    ) -> Vec<u32> {
        borrow_mut!(self.scene_geometry).query_exact(layer, regions, false)
    }

    /// Enables bounded structural-invalidation origin attribution for profiling.
    pub fn set_occlusion_diagnostics(&self, enabled: bool) {
        self.occlusion_state.borrow_mut().diagnostics = enabled;
    }

    #[track_caller]
    pub fn mark_occlusion_dirty(&self) {
        self.mark_occlusion_dirty_with_origin(None);
    }

    #[track_caller]
    pub(crate) fn mark_occlusion_dirty_for_node(&self, id: u32) {
        self.mark_occlusion_dirty_with_origin(Some(id));
    }

    #[track_caller]
    fn mark_occlusion_dirty_with_origin(&self, node: Option<u32>) {
        self.scene_geometry.borrow_mut().input.topology_dirty = true;
        self.occlusion_dirty.set(true);
        let mut state = self.occlusion_state.borrow_mut();
        state.structural = true;
        if let Some(id) = node {
            state.structural_nodes.insert(id);
        } else {
            state.structural_unknown = true;
        }
        if state.diagnostics {
            let caller = std::panic::Location::caller();
            // Aggregate repeated call sites; keep one representative node. Fixed
            // storage avoids allocating or retaining scene nodes during profiling.
            if let Some(origin) = state
                .invalidations
                .iter_mut()
                .flatten()
                .find(|origin| origin.file == caller.file() && origin.line == caller.line())
            {
                origin.count = origin.count.saturating_add(1);
                origin.node = node;
            } else if let Some(slot) = state.invalidations.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(crate::engine::occlusion::OcclusionInvalidation {
                    file: caller.file(),
                    line: caller.line(),
                    node,
                    count: 1,
                });
            }
        }
        drop(state);
        self.mark_lighting_topology_dirty();
    }

    pub(crate) fn mark_lighting_topology_dirty(&self) {
        // Reconcile ownership after occlusion settles. A structural edit is not
        // evidence that light slots, recipient masks, or GPU contents changed.
        self.lighting_topology_dirty.set(true);
        if !borrow!(self.scene_lighting_nodes).is_empty()
            || !borrow!(self.canvas_node_light_masks).is_empty()
        {
            self.mark_scene_lighting_dirty();
        }
    }

    /// Invalidate native compositing after a node's paint or coverage changes.
    /// Unmounted nodes (including settings sidecars) do not participate. Mounted
    /// render sources invalidate their capture geometry and owning visible mask.
    /// Mount/unmount notifications separately reconcile scene membership.
    pub fn mark_node_occlusion_dirty(&self, mut id: u32) {
        loop {
            let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) else {
                return;
            };
            // Source records are cached for capture but absent from visible
            // compositing. Reconcile their owning mask, including nested sources,
            // so source animation does not look like a missing scene node.
            borrow_mut!(self.scene_geometry).invalidate_existing(id);
            let Some(owner) = node.render_source_owner() else {
                break;
            };
            id = owner.to_u32();
            self.invalidate_clip_input(id);
        }
        self.occlusion_dirty.set(true);
        self.occlusion_state.borrow_mut().dirty.insert(id);
    }

    fn mark_occlusion_presentation_dirty(&self, id: u32) {
        self.scene_geometry
            .borrow_mut()
            .input
            .invalidate_subtree(id);
        self.occlusion_dirty.set(true);
        self.occlusion_state.borrow_mut().scrolled.insert(id);
    }

    /// Cumulative work performed by native compositing, including conservative rebuilds.
    pub fn occlusion_stats(&self) -> crate::engine::occlusion::OcclusionStats {
        let mut stats = self.occlusion_state.borrow().stats;
        let scene = self.scene_geometry.borrow();
        stats.scroll_lineage_preparations = scene.presentation_work.scroll.get();
        stats.clip_geometry_preparations = scene.presentation_work.clip.get();
        #[cfg(feature = "3d")]
        {
            stats.projection_preparations = scene.presentations.projections;
        }
        stats.mask_plan_reuses = scene.mask_plans.stats.reuses;
        stats.mask_path_mappings = scene.mask_plans.stats.mappings;
        stats.mask_path_serializations = scene.mask_plans.stats.serializations;
        (stats.visibility_cache_entries, stats.visibility_cache_bytes) =
            scene.mask_plans.residency();
        #[cfg(feature = "3d")]
        {
            stats.solid_relation_evaluations = scene.relations.borrow().stats.evaluations;
            stats.solid_relation_reuses = scene.relations.borrow().stats.reuses;
            let (entries, bytes) = scene.relations.borrow().residency();
            stats.visibility_cache_entries += entries;
            stats.visibility_cache_bytes += bytes;
        }
        stats
    }

    pub(crate) fn prepare_mask_plan(
        &self,
        node: &ExpandedNode,
        plan: crate::engine::occlusion::MaskPlan,
    ) -> crate::engine::occlusion::MaskOutput {
        self.scene_geometry
            .borrow_mut()
            .mask_plans
            .prepare((node.id.to_u32(), node.mount_epoch.get()), plan)
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

    /// Invalidate a registered node's drawing. Unmounted/unknown nodes are ignored;
    /// registering a new or remounted canvas node schedules its initial drawing.
    pub fn mark_canvas_node_dirty(&self, id: ExpandedNodeIdentifier) {
        let Some(node) = self.get_expanded_node_by_eid(id) else {
            return;
        };
        borrow_mut!(self.scene_geometry).invalidate(id.to_u32());
        let owner = node
            .render_source_owner()
            .and_then(|id| self.get_expanded_node_by_eid(id));
        if let Some(owner) = owner {
            owner.changed_listener.invalidate();
            return;
        }
        borrow_mut!(self.dirty_canvas_nodes).insert(id);
        #[cfg(feature = "shaders")]
        {
            // Filter support can reach neighboring tiles beyond a leaf's original coverage. Include
            // the owning boundaries in damage/replay selection; native coverage remains unchanged.
            let mut parent = self
                .get_expanded_node_by_eid(id)
                .and_then(|n| n.presentation_parent_node());
            while let Some(node) = parent {
                if node.shader_effect_bound.get() {
                    borrow_mut!(self.dirty_canvas_nodes).insert(node.id);
                    for ids in borrow_mut!(self.targeted_canvas_replay_node_ids).values_mut() {
                        ids.remove(&node.id.to_u32());
                    }
                }
                parent = node.presentation_parent_node();
            }
        }

        // Replay is surface-local; an independent content/animation change must
        // also reach surviving tiles, even when this node intersects the replay.
        for ids in borrow_mut!(self.targeted_canvas_replay_node_ids).values_mut() {
            ids.remove(&id.to_u32());
        }
    }

    pub fn clear_canvas_node_dirty(&self, id: &ExpandedNodeIdentifier) {
        borrow_mut!(self.dirty_canvas_nodes).remove(id);
    }

    pub fn is_canvas_node_dirty(&self, id: &ExpandedNodeIdentifier) -> bool {
        borrow!(self.dirty_canvas_nodes).contains(id)
    }

    pub fn dirty_canvas_node_ids(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.dirty_canvas_nodes).iter().copied().collect()
    }

    pub fn mark_canvas_nodes_on_layer_dirty(&self, layer: usize) {
        if let Some(ids) = borrow_mut!(self.targeted_canvas_replay_node_ids).get_mut(&layer) {
            ids.clear();
        }
        let node_cache = borrow!(self.node_cache);
        let dirty_nodes = &mut *borrow_mut!(self.dirty_canvas_nodes);
        for node in node_cache.eid_to_node.values() {
            if !node.is_render_source()
                && node.occlusion.get().render_layer_id == layer
                && borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas
            {
                dirty_nodes.insert(node.id);
            }
        }
    }

    /// Return the direct-light membership mask resolved for a retained canvas node.
    pub fn canvas_node_light_mask(&self, id: ExpandedNodeIdentifier) -> u32 {
        borrow!(self.canvas_node_light_masks)
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    fn reconcile_light_recipients(&self) {
        if !self.lighting_topology_dirty.replace(false) {
            return;
        }
        let cache = borrow!(self.node_cache);
        let mut previous = borrow_mut!(self.layer_light_recipients);
        let mut next = HashMap::<usize, LayerLightRecipients>::new();
        let mut inherited =
            HashMap::<ExpandedNodeIdentifier, Rc<Vec<ExpandedNodeIdentifier>>>::new();
        for node in cache.eid_to_node.values() {
            if borrow!(node.instance_node).base().flags().layer != crate::api::Layer::Canvas {
                continue;
            }
            // Memoize each parent once across the whole reconciliation, rather
            // than walking every recipient's ancestry again for every layer.
            let mut current = node.presentation_parent_node();
            let mut missing = Vec::new();
            let mut ancestry = Rc::new(Vec::new());
            while let Some(parent) = current {
                if let Some(known) = inherited.get(&parent.id) {
                    ancestry = Rc::clone(known);
                    break;
                }
                current = parent.presentation_parent_node();
                missing.push(parent);
            }
            for parent in missing.into_iter().rev() {
                if borrow!(parent.instance_node).establishes_light_frame() {
                    let mut frames = ancestry.as_ref().clone();
                    frames.push(parent.id);
                    ancestry = Rc::new(frames);
                }
                inherited.insert(parent.id, Rc::clone(&ancestry));
            }
            let layer = node.occlusion.get().render_layer_id;
            let recipients = next.entry(layer).or_default();
            let old = previous.get(&layer);
            if old.is_none_or(|old| {
                old.ancestry.get(&node.id) != Some(&ancestry) || old.pending.contains(&node.id)
            }) {
                recipients.pending.insert(node.id);
            }
            recipients.ancestry.insert(node.id, ancestry);
        }
        // Includes replacement of a Canvas primitive by a non-Canvas primitive.
        // Layer moves stay pending in the destination even if the source layer
        // resolves first, and uncollected changes survive another reconciliation.
        borrow_mut!(self.canvas_node_light_masks).retain(|id, _| {
            cache.eid_to_node.get(id).is_some_and(|node| {
                next.get(&node.occlusion.get().render_layer_id)
                    .is_some_and(|recipients| recipients.ancestry.contains_key(id))
            })
        });
        borrow_mut!(self.layer_light_scopes).retain(|layer, _| next.contains_key(layer));
        borrow_mut!(self.lighting_overflow_counts).retain(|layer, _| next.contains_key(layer));
        *previous = next;
    }

    /// Resolve registered direct/ambient contributors in the target layer's coordinates.
    ///
    /// Light values remain live on every render. Structural changes reconcile recipient
    /// layer and LightFrame ancestry once; stable light slots only revisit changed
    /// recipients. Slot/scope changes rebuild masks within the affected layer.
    /// Contributors are independent of visibility; unlit scenes need no recipient scan.
    pub fn collect_scene_lighting_for_layer(&self, layer: usize) -> SceneLighting {
        if borrow!(self.scene_lighting_nodes).is_empty()
            && borrow!(self.canvas_node_light_masks).is_empty()
        {
            if self.lighting_topology_dirty.replace(false) {
                borrow_mut!(self.layer_light_recipients).clear();
                borrow_mut!(self.layer_light_scopes).clear();
                borrow_mut!(self.lighting_overflow_counts).clear();
            }
            borrow_mut!(self.lighting_overflow_counts).remove(&layer);
            return SceneLighting::with_default_ambient(Vec::new());
        }
        self.reconcile_light_recipients();
        let layer_scrolls = self.lighting_layer_scrolls(layer);
        let target_scroll = layer_scrolls[&layer];
        let node_cache = borrow!(self.node_cache);
        let mut scoped_lights = Vec::new();
        let mut topmost_ambient = None;

        for id in borrow!(self.scene_lighting_nodes).iter() {
            let Some(node) = node_cache.eid_to_node.get(id) else {
                continue;
            };
            if node.is_render_source() {
                continue;
            }
            let occlusion = node.occlusion.get();
            let source_layer = occlusion.render_layer_id;
            let Some(source_scroll) = layer_scrolls.get(&source_layer) else {
                continue;
            };

            let (light, ambient) = {
                let instance_node = borrow!(node.instance_node);
                (
                    instance_node.resolve_scene_light(node, self),
                    instance_node.resolve_scene_ambient_light(node, self),
                )
            };
            if let Some(light) = light {
                if light.enabled {
                    scoped_lights.push((
                        occlusion.z_index,
                        node.id,
                        Self::nearest_light_frame(node),
                        transform_scene_light_for_layer(light, *source_scroll, target_scroll),
                    ));
                }
            }
            if let Some(ambient) = ambient {
                let should_replace = topmost_ambient
                    .as_ref()
                    .map(|(z_index, id, _)| (occlusion.z_index, node.id) >= (*z_index, *id))
                    .unwrap_or(true);
                if should_replace {
                    topmost_ambient = Some((occlusion.z_index, node.id, ambient));
                }
            }
        }

        scoped_lights
            .sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        let overflow = scoped_lights
            .len()
            .saturating_sub(SceneLighting::MAX_LIGHTS);
        scoped_lights.truncate(SceneLighting::MAX_LIGHTS);

        {
            let mut overflow_counts = borrow_mut!(self.lighting_overflow_counts);
            if overflow == 0 {
                overflow_counts.remove(&layer);
            } else if overflow_counts.insert(layer, overflow) != Some(overflow) {
                log::warn!(
                    "canvas layer {layer} has more than {} enabled lights; omitting {overflow}",
                    SceneLighting::MAX_LIGHTS
                );
            }
        }

        let scopes = scoped_lights
            .iter()
            .map(|(_, id, frame, _)| (*id, *frame))
            .collect::<Vec<_>>();
        let scopes_changed = borrow!(self.layer_light_scopes).get(&layer) != Some(&scopes);
        if let Some(recipients) = borrow_mut!(self.layer_light_recipients).get_mut(&layer) {
            let candidates = if scopes_changed {
                recipients.pending.clear();
                recipients.ancestry.keys().copied().collect::<Vec<_>>()
            } else {
                recipients.pending.drain().collect::<Vec<_>>()
            };
            let mut masks = borrow_mut!(self.canvas_node_light_masks);
            // Empty masks are implicit. An unlit/ambient-only scene never needs a
            // recipient scan, unless retained masks from earlier lights need clearing.
            if !scopes.is_empty() || !masks.is_empty() {
                let mut dirty_nodes = borrow_mut!(self.dirty_canvas_nodes);
                for id in candidates {
                    if scopes.is_empty() && !masks.contains_key(&id) {
                        continue;
                    }
                    #[cfg(test)]
                    self.lighting_mask_node_visits
                        .set(self.lighting_mask_node_visits.get() + 1);
                    let mask = if scopes.is_empty() {
                        0
                    } else {
                        let ancestry = &recipients.ancestry[&id];
                        scopes
                            .iter()
                            .enumerate()
                            .fold(0u32, |mask, (slot, (_, owner))| {
                                if owner.is_none_or(|owner| ancestry.contains(&owner)) {
                                    mask | (1u32 << slot)
                                } else {
                                    mask
                                }
                            })
                    };
                    let previous = if mask == 0 {
                        masks.remove(&id)
                    } else {
                        masks.insert(id, mask)
                    }
                    .unwrap_or(0);
                    if previous != mask {
                        self.set_canvas_dirty(layer);
                        dirty_nodes.insert(id);
                        if let Some(ids) =
                            borrow_mut!(self.targeted_canvas_replay_node_ids).get_mut(&layer)
                        {
                            ids.remove(&id.to_u32());
                        }
                    }
                }
            }
        }
        borrow_mut!(self.layer_light_scopes).insert(layer, scopes);
        drop(node_cache);

        let lights = scoped_lights
            .into_iter()
            .map(|(_, _, _, light)| light)
            .collect::<Vec<_>>();

        if let Some((_, _, ambient)) = topmost_ambient {
            SceneLighting {
                active: true,
                ambient_is_authored: true,
                ambient,
                lights,
            }
        } else {
            SceneLighting::with_default_ambient(lights)
        }
    }

    pub(crate) fn lighting_view_for_layer(&self, layer: usize) -> [f32; 4] {
        #[cfg(all(feature = "3d", feature = "designtime"))]
        let root = self
            .get_userland_root_expanded_node()
            .or_else(|| self.get_root_expanded_node());
        #[cfg(all(feature = "3d", not(feature = "designtime")))]
        let root = self.get_root_expanded_node();
        #[cfg(feature = "3d")]
        if let Some(root) = root {
            let mut view = root.presentation_view.get().lighting_view;
            if view[3] != 0. {
                let scroll = self.lighting_layer_scrolls(layer)[&layer];
                for axis in 0..3 {
                    view[axis] -= scroll[axis];
                }
            }
            return view.map(|v| v as f32);
        }
        let _ = layer;
        [0., 0., 1., 0.]
    }

    fn nearest_light_frame(node: &ExpandedNode) -> Option<ExpandedNodeIdentifier> {
        let mut current = node.presentation_parent_node();
        while let Some(parent) = current {
            if borrow!(parent.instance_node).establishes_light_frame() {
                return Some(parent.id);
            }
            current = parent.presentation_parent_node();
        }
        None
    }

    #[cfg(test)]
    fn light_frame_ancestry(node: &ExpandedNode) -> HashSet<ExpandedNodeIdentifier> {
        let mut frames = HashSet::new();
        let mut current = node.presentation_parent_node();
        while let Some(parent) = current {
            if borrow!(parent.instance_node).establishes_light_frame() {
                frames.insert(parent.id);
            }
            current = parent.presentation_parent_node();
        }
        frames
    }

    // Lighting stays in world axes. Subtract the target layer's scroll displacement
    // from lights so retained local geometry needs no replay merely because it scrolls.
    // A tilted scroller contributes all three components of its local scroll vector.
    fn lighting_layer_scrolls(&self, layer: usize) -> HashMap<usize, [f64; 3]> {
        let mut chain = Vec::new();
        let mut current = layer;
        let mut visited = HashSet::new();
        while visited.insert(current) {
            let Some(owner) = self
                .get_layer_scroller_owner(current)
                .and_then(|id| self.get_expanded_node_by_eid(id))
            else {
                break;
            };
            let (x, y) = self.scroller_presentation_offset(&owner);
            #[cfg(feature = "3d")]
            let delta = {
                let m = owner.spatial_geometry.get().world_transform().coeffs();
                [
                    -m[0] * x - m[4] * y,
                    -m[1] * x - m[5] * y,
                    -m[2] * x - m[6] * y,
                ]
            };
            #[cfg(not(feature = "3d"))]
            let delta = {
                let m = owner.transform_and_bounds.get().transform.m;
                [-m[0] * x - m[2] * y, -m[1] * x - m[3] * y, 0.0]
            };
            chain.push((current, delta));
            current = owner.occlusion.get().render_layer_id;
        }
        let mut result = HashMap::from([(current, [0.; 3])]);
        let mut sum = [0.; 3];
        for (layer, delta) in chain.into_iter().rev() {
            for i in 0..3 {
                sum[i] += delta[i];
            }
            result.insert(layer, sum);
        }
        result.entry(layer).or_insert([0.; 3]);
        result
    }

    #[cfg(test)]
    fn layer_to_root_transform(&self, layer: usize) -> Affine {
        let mut transform = Affine::IDENTITY;
        let mut current_layer = layer;
        let mut visited = HashSet::new();

        while let Some(owner_id) = self.get_layer_scroller_owner(current_layer) {
            if !visited.insert(current_layer) {
                break;
            }
            let Some(owner_node) = self.get_expanded_node_by_eid(owner_id) else {
                break;
            };
            let owner_layer = owner_node.occlusion.get().render_layer_id;
            let owner_transform = self.canvas_surface_transform_for_node(&owner_node);
            let (scroll_x, scroll_y) = self
                .get_scroller_surface_scroll(owner_node.id.to_u32())
                .or_else(|| borrow!(owner_node.instance_node).resolve_scroll_offset(&owner_node))
                .unwrap_or((0.0, 0.0));
            transform = owner_transform * Affine::translate((-scroll_x, -scroll_y)) * transform;

            if owner_layer == current_layer {
                break;
            }
            current_layer = owner_layer;
        }

        transform
    }

    pub fn canvas_surface_to_world(&self, layer: usize) -> Affine {
        self.get_layer_scroller_owner(layer)
            .and_then(|owner| self.get_expanded_node_by_eid(owner))
            .map_or(Affine::IDENTITY, |owner| {
                Affine::from(owner.transform_and_bounds.get().transform)
            })
    }

    /// Projects retained clip geometry into window presentation coordinates,
    /// including ancestor scrolling and the camera's clipping equations.
    pub(crate) fn presented_clip_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        if let Some(local) = node.local_clip_geometry.get().0 {
            return Some(
                self.presented_projection_for_node(node)?
                    .project_path(&local, None),
            );
        }
        let path = node.world_clip_geometry.get().0?;
        let mapping = self.presented_projection_for_node(node)?.mapping()
            * node.presentation_projection()?.mapping().inverse()?;
        Some(mapping.project_path(&path, 0.1))
    }

    /// Projects a node after applying ancestor scrolling in world space.
    pub fn presented_projection_for_node(
        &self,
        node: &ExpandedNode,
    ) -> Option<pax_runtime_api::math::ProjectedPlane> {
        #[cfg(feature = "3d")]
        if node.has_perspective_camera() {
            return self.prepared_presentation(node).projection();
        }
        self.compute_presented_projection(node)
    }

    #[cfg(feature = "3d")]
    pub(crate) fn prepared_presentation(
        &self,
        node: &ExpandedNode,
    ) -> Rc<crate::visibility::PreparedPresentation> {
        // Read inputs before borrowing the cache: reactive computation may access
        // other scene records, and a handler can query before queued effects run.
        let inputs = crate::visibility::PresentationInputs {
            geometry: node.spatial_geometry.get(),
            view: node.presentation_view.get(),
            scroll: self.world_scroll_offset(node),
        };
        self.scene_geometry
            .borrow_mut()
            .presentations
            .prepare_inputs(node.id.to_u32(), node.mount_epoch.get(), inputs)
    }

    #[cfg(all(test, feature = "3d"))]
    pub(crate) fn presentation_stats(&self) -> crate::visibility::PresentationStats {
        self.scene_geometry.borrow().presentations.stats
    }

    #[cfg(feature = "3d")]
    pub(crate) fn support_depth(
        &self,
        node: &ExpandedNode,
    ) -> ((u32, u64), pax_runtime_api::DepthPlane) {
        let owner = node
            .support_node
            .get()
            .and_then(|id| self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)));
        let owner = owner.as_deref().unwrap_or(node);
        let transform = self.presentation_scroll_transform_for_node(owner)
            * Affine::from(owner.presentation_transform_and_bounds.get().transform);
        let depth = self
            .presented_depth_plane(owner, transform)
            .unwrap_or_default();
        ((owner.id.to_u32(), owner.mount_epoch.get()), depth)
    }

    /// Retained mounted identities and the runtime-wide coverage budget for GPU replay.
    #[cfg(feature = "3d")]
    pub fn canvas_coverage_context(
        &self,
        node: &ExpandedNode,
        domain: usize,
    ) -> pax_runtime_api::visibility::CoverageContext {
        let support = node
            .support_node
            .get()
            .and_then(|id| self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)));
        let support = support.as_deref().unwrap_or(node);
        pax_runtime_api::visibility::CoverageContext {
            node: (node.id.to_u32(), node.mount_epoch.get()),
            support: (support.id.to_u32(), support.mount_epoch.get()),
            domain,
            relations: self.scene_geometry.borrow().relations.clone(),
        }
    }

    #[cfg(feature = "3d")]
    pub(crate) fn solid_front_coverage(
        &self,
        occluder: &ExpandedNode,
        support: (u32, u64),
        domain: usize,
        solid: &Rc<pax_runtime_api::mesh3::ProjectedConvexMesh>,
        plane: pax_runtime_api::DepthPlane,
    ) -> Rc<kurbo::BezPath> {
        self.scene_geometry
            .borrow()
            .relations
            .borrow_mut()
            .front_coverage(
                crate::visibility::RelationKey {
                    occluder: (occluder.id.to_u32(), occluder.mount_epoch.get()),
                    support,
                    domain,
                    chart: 0,
                },
                solid,
                plane,
            )
    }

    pub(crate) fn presented_local_point(
        &self,
        node: &ExpandedNode,
        point: kurbo::Point,
    ) -> Option<kurbo::Point> {
        #[cfg(feature = "3d")]
        if node.has_perspective_camera() {
            return self.prepared_presentation(node).local_point(point);
        }
        self.compute_presented_projection(node)?.local_point(point)
    }

    fn compute_presented_projection(
        &self,
        node: &ExpandedNode,
    ) -> Option<pax_runtime_api::math::ProjectedPlane> {
        #[cfg(not(feature = "3d"))]
        {
            Some(
                node.presentation_projection()?
                    .post_affine(self.presentation_scroll_transform_for_node(node)),
            )
        }
        #[cfg(feature = "3d")]
        {
            crate::visibility::PresentationInputs {
                geometry: node.spatial_geometry.get(),
                view: node.presentation_view.get(),
                scroll: self.world_scroll_offset(node),
            }
            .project()
        }
    }

    #[cfg(feature = "3d")]
    fn world_scroll_offset(&self, node: &ExpandedNode) -> [f64; 3] {
        node.scroll_state.get().inherited.world
    }

    /// Matrix delivered to a native host whose ancestor scrollers already apply
    /// their own projected scroll motion. Compensates that motion so each plane
    /// receives its world-space scroll displacement before perspective division.
    pub fn native_presentation_matrix_for_node(&self, node: &ExpandedNode) -> Vec<f64> {
        let mapping = self
            .native_presentation_projection_for_node(node)
            .map(|p| p.mapping());
        mapping.map_or_else(
            || vec![0.; 6],
            |mapping| {
                mapping.as_affine().map_or_else(
                    || mapping.matrix3d().to_vec(),
                    |affine| affine.as_coeffs().to_vec(),
                )
            },
        )
    }

    /// Native presentation with camera clipping retained for projected clip paths.
    pub fn native_presentation_projection_for_node(
        &self,
        node: &ExpandedNode,
    ) -> Option<pax_runtime_api::math::ProjectedPlane> {
        if !node.has_perspective_camera() {
            return node.presentation_projection();
        }
        Some(
            self.presented_projection_for_node(node)?
                .post_mapping(self.native_scroll_mapping(node)?.inverse()?),
        )
    }

    fn native_scroll_mapping(
        &self,
        node: &ExpandedNode,
    ) -> Option<pax_runtime_api::math::PlanarProjection> {
        #[cfg(feature = "3d")]
        {
            node.scroll_state.get().inherited.native
        }
        #[cfg(not(feature = "3d"))]
        {
            let _ = node;
            Some(Default::default())
        }
    }

    /// Projects into the owning canvas's scroll-content chart. The browser
    /// applies the owner's projection and local scroll after this rendering.
    /// Camera clipping remains in the source plane's original coordinates.
    pub fn canvas_surface_projection_for_node(
        &self,
        node: &ExpandedNode,
    ) -> Option<pax_runtime_api::math::ProjectedPlane> {
        let projection = self.presented_projection_for_node(node)?;
        if node.is_render_source() {
            return Some(projection);
        }
        Some(
            projection
                .post_mapping(self.window_to_canvas_mapping(node.occlusion.get().render_layer_id)?),
        )
    }

    /// Maps presented window coordinates into a canvas's scroll-content chart.
    /// This also applies to detached sources whose logical layer differs from the consumer.
    pub fn window_to_canvas_mapping(
        &self,
        layer: usize,
    ) -> Option<pax_runtime_api::math::PlanarProjection> {
        use pax_runtime_api::math::PlanarProjection;
        let owner = self
            .get_layer_scroller_owner(layer)
            .and_then(|id| self.get_expanded_node_by_eid(id));
        match owner {
            Some(owner) => {
                let (x, y) = self.scroller_presentation_offset(&owner);
                Some(
                    PlanarProjection::from(Affine::translate((x, y)))
                        * self
                            .presented_projection_for_node(&owner)?
                            .mapping()
                            .inverse()?,
                )
            }
            None => Some(PlanarProjection::default()),
        }
    }

    /// Projects a detached source directly into its consuming canvas, preserving
    /// camera clipping before the canvas owner's projective mapping is inverted.
    pub fn canvas_projection_on_layer(
        &self,
        node: &ExpandedNode,
        layer: usize,
    ) -> Option<pax_runtime_api::math::ProjectedPlane> {
        Some(
            self.presented_projection_for_node(node)?
                .post_mapping(self.window_to_canvas_mapping(layer)?),
        )
    }

    pub fn canvas_surface_transform_for_node(&self, node: &ExpandedNode) -> Affine {
        let transform = Affine::from(node.presentation_transform_and_bounds.get().transform);
        if node.is_render_source() {
            return transform;
        }
        let own_layer = node.occlusion.get().render_layer_id;
        // Structural Frame/Mask ancestors can retain a different/default layer
        // without owning a canvas. Using them as the origin displaces content
        // relative to clips and alpha masks in the actual Scroller surface.
        self.get_layer_scroller_owner(own_layer)
            .and_then(|owner| self.get_expanded_node_by_eid(owner))
            .map_or(transform, |owner| {
                Affine::from(
                    owner
                        .presentation_transform_and_bounds
                        .get()
                        .transform
                        .inverse(),
                ) * transform
            })
    }

    pub(crate) fn invalidate_scene_geometry(&self, id: u32) {
        borrow_mut!(self.scene_geometry).invalidate(id);
    }

    pub(crate) fn remove_observation_geometry(&self, id: u32) {
        if let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) {
            let instance = borrow!(node.instance_node);
            if instance.base().flags().layer != crate::api::Layer::DontCare
                || instance.materializes_native_surface(&node)
            {
                return;
            }
        }
        borrow_mut!(self.scene_geometry).remove_geometry(id);
    }

    /// Prepare geometry dirtied by scene changes. Surface replay alone does not invalidate it.
    pub fn prepare_scene_geometry(&self) {
        let dirty = borrow_mut!(self.scene_geometry).take_dirty();
        for id in dirty {
            // A primitive may have requested another pending record while preparing.
            if borrow!(self.scene_geometry).get(id).is_some() {
                continue;
            }
            if let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) {
                if borrow!(node.instance_node).base().flags().layer != crate::api::Layer::DontCare
                    || borrow!(node.instance_node).materializes_native_surface(&node)
                    || self.viewport_dependency(id)
                {
                    self.prepare_canvas_geometry(&node);
                } else {
                    borrow_mut!(self.scene_geometry).remove_geometry(id);
                }
            }
        }
    }

    #[cfg(feature = "3d")]
    fn presented_mesh_projection_for_node(
        &self,
        node: &ExpandedNode,
        transform: [f64; 16],
    ) -> Option<pax_runtime_api::mesh3::MeshProjection> {
        use pax_runtime_api::math::{Generic, Transform3, Vector3};
        let mut world = Transform3::<Generic>::new(transform);
        if node.has_perspective_camera() {
            let offset = self.world_scroll_offset(node);
            world = Transform3::translate(Vector3::new(offset[0], offset[1], offset[2])) * world;
        }
        node.presentation_view.get().mesh(world.coeffs())
    }

    /// Projects a solid into a specified canvas chart. Detached alpha sources use
    /// their consumer's layer, which can differ from the source's logical layer.
    #[cfg(feature = "3d")]
    pub fn canvas_mesh_projection_on_layer(
        &self,
        node: &ExpandedNode,
        layer: usize,
        transform: [f64; 16],
    ) -> Option<pax_runtime_api::mesh3::MeshProjection> {
        Some(self.presented_mesh_projection_for_node(node, transform)?
            .post_mapping(self.window_to_canvas_mapping(layer)?))
    }

    fn prepare_canvas_geometry(
        &self,
        node: &ExpandedNode,
    ) -> Rc<crate::scene_geometry::PreparedCanvasGeometry> {
        let tab = node.presentation_transform_and_bounds.get();
        let local = borrow!(node.instance_node).prepare_canvas_geometry(node);
        let mut prepared = crate::scene_geometry::PreparedCanvasGeometry::new(
            node.occlusion.get().render_layer_id,
            tab.bounds,
            self.canvas_surface_transform_for_node(node),
            local,
        );
        if node.has_perspective_camera() {
            prepared.projection = self.canvas_surface_projection_for_node(node);
            prepared.world_projection = self.presented_projection_for_node(node);
            prepared.coverage_bounds = prepared.local.local_bounds.map(|bounds| {
                prepared
                    .projection
                    .and_then(|p| p.coverage(bounds, None))
                    .map(|r| r.inflate(64., 64.))
                    .unwrap_or(kurbo::Rect::ZERO)
            });
        }
        prepared.world_transform = Affine::from(tab.transform);
        #[cfg(feature = "3d")]
        {
            prepared.spatial_geometry = node.presentation_geometry.get();
            prepared.solid = borrow!(node.instance_node).resolve_convex_mesh(node);
            if let Some(solid) = &prepared.solid {
                let projection = self.presented_mesh_projection_for_node(node, solid.transform);
                prepared.projected_solid = projection.map(|projection| solid.projected(projection));
                let mapping = if node.is_render_source() {
                    Some(pax_runtime_api::math::PlanarProjection::default())
                } else if node.has_perspective_camera() {
                    self.window_to_canvas_mapping(prepared.layer)
                } else {
                    Some(
                        self.get_layer_scroller_owner(prepared.layer)
                            .and_then(|id| self.get_expanded_node_by_eid(id))
                            .map_or(Affine::IDENTITY, |owner| {
                                Affine::from(
                                    owner
                                        .presentation_transform_and_bounds
                                        .get()
                                        .transform
                                        .inverse(),
                                )
                            })
                            .into(),
                    )
                };
                prepared.solid_projection = projection
                    .zip(mapping)
                    .map(|(projection, mapping)| projection.post_mapping(mapping));
                prepared.coverage_bounds = prepared
                    .solid_projection
                    .and_then(|projection| solid.projected(projection).coverage_bounds())
                    .map(|bounds| {
                        if bounds.area() == 0. {
                            bounds
                        } else {
                            bounds.inflate(64., 64.)
                        }
                    });
            }
        }
        // Detached mask sources reuse prepared geometry without entering visible
        // replay or native-compositing indexes.
        prepared.is_canvas = !node.is_render_source()
            && borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas;
        prepared.is_native = !node.is_render_source()
            && (matches!(
                borrow!(node.instance_node).base().flags().layer,
                crate::api::Layer::Native | crate::api::Layer::NativeNonOccluding
            ) || borrow!(node.instance_node).materializes_native_surface(node));
        borrow_mut!(self.scene_geometry).insert(node.id.to_u32(), prepared)
    }

    /// Obtain the same prepared record used by spatial replay selection.
    pub fn canvas_geometry_for_node(
        &self,
        node: &ExpandedNode,
    ) -> Rc<crate::scene_geometry::PreparedCanvasGeometry> {
        let cached = borrow!(self.scene_geometry).get(node.id.to_u32());
        cached.unwrap_or_else(|| self.prepare_canvas_geometry(node))
    }

    /// Work counters for geometry preparation and spatial queries.
    pub fn scene_geometry_stats(&self) -> crate::scene_geometry::SceneGeometryStats {
        borrow!(self.scene_geometry).stats
    }

    /// Select canvas nodes using settled runtime geometry, even before their first draw.
    /// Call after draining scene effects; querying does not advance layout or dispatch events.
    pub fn canvas_nodes_intersecting(
        &self,
        layer: usize,
        regions: &[kurbo::Rect],
    ) -> Option<Vec<u32>> {
        self.prepare_scene_geometry();
        borrow_mut!(self.scene_geometry).query(layer, regions)
    }

    pub(crate) fn compositing_nodes_intersecting(
        &self,
        layer: usize,
        regions: &[kurbo::Rect],
        native: bool,
    ) -> Vec<u32> {
        let mut geometry = borrow_mut!(self.scene_geometry);
        if native {
            geometry.query_exact(layer, regions, true)
        } else {
            geometry.query_occlusion(layer, regions)
        }
    }

    pub(crate) fn set_occlusion_bounds(
        &self,
        id: u32,
        layer: usize,
        bounds: crate::scene_geometry::CoverageBounds,
    ) {
        self.scene_geometry
            .borrow_mut()
            .set_occlusion_bounds(id, layer, bounds);
    }

    pub(crate) fn clear_occlusion_bounds(&self) {
        self.scene_geometry.borrow_mut().clear_occlusion_bounds();
    }

    /// Select surface replay candidates without invalidating unchanged content. The engine
    /// asks its backend which drawings are reusable before marking cache misses dirty.
    pub fn request_canvas_replay(&self, update: pax_runtime_api::ReplayCanvasLayerUpdate) {
        if let Some(ids) = update
            .regions
            .as_deref()
            .and_then(|regions| self.canvas_nodes_intersecting(update.layer, regions))
        {
            self.mark_targeted_canvas_replay_nodes(update.layer, &ids);
        } else {
            self.mark_canvas_nodes_on_layer_dirty(update.layer);
        }
        self.set_canvas_dirty(update.layer);
    }

    pub fn mark_canvas_nodes_on_layer_dirty_by_id(&self, layer: usize, node_ids: &[u32]) {
        let node_cache = borrow!(self.node_cache);
        let dirty_nodes = &mut *borrow_mut!(self.dirty_canvas_nodes);
        for node_id in node_ids {
            let id = ExpandedNodeIdentifier(*node_id);
            let Some(node) = node_cache.eid_to_node.get(&id) else {
                continue;
            };
            if node.occlusion.get().render_layer_id == layer
                && borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas
            {
                dirty_nodes.insert(id);
            }
        }
    }

    pub fn mark_targeted_canvas_replay_nodes(&self, layer: usize, node_ids: &[u32]) {
        let mut targeted = borrow_mut!(self.targeted_canvas_replay_node_ids);
        let layer_nodes = targeted.entry(layer).or_default();
        let dirty = borrow!(self.dirty_canvas_nodes);
        // Never relabel an independently dirty node as replay-only: its
        // changed pixels need updating outside the requested region as well.
        for id in node_ids {
            if !dirty.contains(&ExpandedNodeIdentifier(*id)) {
                layer_nodes.insert(*id);
            }
        }
    }

    pub fn take_targeted_canvas_replay_node_ids(&self) -> HashMap<usize, HashSet<u32>> {
        let targeted = &mut *borrow_mut!(self.targeted_canvas_replay_node_ids);
        std::mem::take(targeted)
    }

    pub fn mark_all_canvas_nodes_dirty(&self) {
        for ids in borrow_mut!(self.targeted_canvas_replay_node_ids).values_mut() {
            ids.clear();
        }
        let node_cache = borrow!(self.node_cache);
        let dirty_nodes = &mut *borrow_mut!(self.dirty_canvas_nodes);
        for node in node_cache.eid_to_node.values() {
            if !node.is_render_source()
                && borrow!(node.instance_node).base().flags().layer == crate::api::Layer::Canvas
            {
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
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        let Some(root_node) = root.or_else(|| borrow!(self.root_expanded_node).upgrade()) else {
            return accum;
        };
        let _graph = self.property_graph.enter();
        let input = self.prepare_input_query(&root_node, kurbo::Point::new(ray.x, ray.y));
        #[cfg(not(feature = "3d"))]
        let spatial_possible = false;
        #[cfg(feature = "3d")]
        let spatial_possible =
            self.globals().renderer_is_gpu && self.scene_geometry.borrow().may_have_spatial_nodes();
        #[cfg(feature = "3d")]
        let mut depths = std::collections::HashMap::new();
        #[cfg(feature = "3d")]
        let mut has_spatial_hit = false;
        let mut to_process = vec![(root_node, false, Affine::IDENTITY, 0.0, false)];
        while let Some((
            node,
            clipped,
            active_scroll_transform,
            active_depth_offset,
            retained_for_exit,
        )) = to_process.pop()
        {
            if node.is_render_source() {
                continue;
            }
            // make sure slot sources are updated for this node
            if !self.startup.borrow().is_active() && input.is_none() {
                node.compute_flattened_projected_children();
            }
            if !hit_invisible
                && (retained_for_exit || node.transition_phase.get() == TRANSITION_PHASE_EXIT)
            {
                continue;
            }
            // Browser-composited scrollers move descendants outside the engine transform tree.
            // Fold active scroll offsets into hit-testing so event rays line up with presented content.
            let (scroll_transform, clips_content) = {
                let instance_node = borrow!(node.instance_node);
                let scrolls_content = instance_node.scrolls_content(&node);
                let clips_content = instance_node.clips_content(&node);
                drop(instance_node);
                (
                    if scrolls_content {
                        self.scroller_content_presentation_transform(&node)
                    } else {
                        Affine::IDENTITY
                    },
                    clips_content,
                )
            };
            let descendant_scroll_transform = active_scroll_transform * scroll_transform;
            let candidate = input
                .as_ref()
                .is_none_or(|q| q.candidates.contains(&node.id.to_u32()));
            if candidate {
                let mut scene = self.scene_geometry.borrow_mut();
                scene.input.stats.exact_tests += 1;
                if input
                    .as_ref()
                    .is_some_and(|q| q.unknown.contains(&node.id.to_u32()))
                {
                    scene.input.stats.unknown_tests += 1;
                }
            }
            #[cfg(not(feature = "3d"))]
            let descendant_depth_offset = active_depth_offset;
            #[cfg(feature = "3d")]
            let descendant_depth_offset = active_depth_offset
                + if spatial_possible && borrow!(node.instance_node).scrolls_content(&node) {
                    self.scroller_content_depth_offset(&node)
                } else {
                    0.0
                };
            #[cfg(feature = "3d")]
            let presentation = node
                .has_perspective_camera()
                .then(|| self.prepared_presentation(&node));
            let (geometry_hit, solid_depth) = if candidate {
                let point = kurbo::Point::new(ray.x, ray.y);
                if input.is_some() {
                    self.scene_geometry.borrow().input.records[&node.id.to_u32()]
                        .hit
                        .test(point)
                } else {
                    self.prepare_node_hit(&node, active_scroll_transform)
                        .1
                        .test(point)
                }
            } else {
                (false, None)
            };
            #[cfg(not(feature = "3d"))]
            let _ = solid_depth;
            let hit = geometry_hit
                && borrow!(node.get_common_properties())
                    ._raycastable
                    .get()
                    .unwrap_or(true);
            if hit && !clipped {
                if hit_invisible
                    || !borrow!(node.instance_node)
                        .base()
                        .flags()
                        .invisible_to_raycasting
                {
                    if limit_one && !spatial_possible {
                        return vec![node];
                    }
                    #[cfg(feature = "3d")]
                    {
                        // Prepared mesh geometry is also usable by custom CPU
                        // primitives; that does not opt their query into GPU depth order.
                        let solid_depth = spatial_possible.then_some(solid_depth).flatten();
                        let presented = active_scroll_transform
                            * Affine::from(node.presentation_transform_and_bounds.get().transform);
                        let plane = if node.has_perspective_camera() {
                            presentation.as_ref().and_then(|p| p.depth_plane())
                        } else {
                            spatial_possible
                                .then(|| {
                                    node.presentation_geometry.get().depth_plane(
                                        pax_runtime_api::math::Transform2::new(
                                            presented.as_coeffs(),
                                        ),
                                    )
                                })
                                .flatten()
                        };
                        has_spatial_hit |=
                            solid_depth.is_some() || plane.is_some() || active_depth_offset != 0.0;
                        let plane = plane.unwrap_or_default();
                        depths.insert(
                            node.id,
                            (
                                solid_depth.unwrap_or_else(|| plane.depth_at(ray.x, ray.y))
                                    + if node.has_perspective_camera() {
                                        0.
                                    } else {
                                        active_depth_offset
                                    },
                                plane.backface,
                                accum.len(),
                                node.presentation_layer.get(),
                            ),
                        );
                    }
                    accum.push(Rc::clone(&node));
                }
            }
            let clipped = clipped || (!hit && clips_content);
            let retained_children = borrow!(node.exiting_children).clone();
            to_process.extend(
                input
                    .as_ref()
                    .map(|q| &q.children)
                    .map(|children| children.get(&node.id.to_u32()).cloned().unwrap_or_default())
                    .unwrap_or_else(|| node.presentation_children(self))
                    .iter()
                    .cloned()
                    .map(|v| {
                        let retained_for_exit = retained_children
                            .iter()
                            .any(|retained| Rc::ptr_eq(retained, &v));
                        let cp = v.get_common_properties();
                        let unclippable = borrow!(cp).unclippable.get().unwrap_or(false);
                        (
                            v,
                            clipped && !unclippable,
                            descendant_scroll_transform,
                            descendant_depth_offset,
                            retained_for_exit,
                        )
                    })
                    .rev(),
            )
        }
        #[cfg(feature = "3d")]
        if has_spatial_hit {
            accum.sort_by(|a, b| {
                let a = depths.get(&a.id).copied().unwrap_or((0.0, false, 0, 0));
                let b = depths.get(&b.id).copied().unwrap_or((0.0, false, 0, 0));
                // Quantize only numerical ties; geometric depth precedes authored order.
                b.3.cmp(&a.3)
                    .then_with(|| pax_runtime_api::DepthPlane::compare_point_depths(b.0, a.0))
                    .then_with(|| {
                        let order = |value: (f64, bool, usize, u32)| {
                            pax_runtime_api::DepthPlane {
                                backface: value.1,
                                ..Default::default()
                            }
                            .coplanar_order(-(value.2 as i64))
                        };
                        order(b).cmp(&order(a))
                    })
            });
        }
        if limit_one {
            accum.truncate(1);
        }
        accum
    }
    /// Capture recipients without dispatching hover events or evaluating pending
    /// structural children. A missing hit stays missing when the event is replayed.
    pub fn capture_startup_input(
        &self,
        ray: Option<Point2<Window>>,
    ) -> crate::engine::startup::StartupInput {
        // No scene has been published during this first conservative bootstrap.
        // Clicking its background must not later activate an unseen control.
        let ray = ray.filter(|_| !self.startup.borrow().is_active());
        let target = ray.and_then(|ray| {
            self.get_elements_beneath_ray(None, ray, true, vec![], false)
                .first()
                .map(|node| node.id)
        });
        let recipients = self
            .node_cache
            .borrow()
            .eid_to_node
            .values()
            .map(|node| (node.id, node.mount_epoch.get()))
            .collect();
        crate::engine::startup::StartupInput { recipients, target }
    }

    /// Alias for `get_elements_beneath_ray` with `limit_one = true`
    pub fn get_topmost_element_beneath_ray(
        self: &Rc<Self>,
        ray: Point2<Window>,
    ) -> Option<Rc<ExpandedNode>> {
        if let Some(input) = self.replaying_startup_input.borrow().as_ref() {
            return input
                .target
                .and_then(|id| self.get_expanded_node_by_eid(id))
                .filter(|node| self.accepts_startup_input(node));
        }
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
        // Native DOM/mask identities must not collide between applications in
        // the same runtime. Allocation is local to the runtime's owner thread.
        thread_local! { static NEXT_NODE: Cell<u32> = const { Cell::new(0) }; }
        NEXT_NODE.with(|next| {
            let id = next.get();
            next.set(id.checked_add(1).expect("expanded node identity exhausted"));
            ExpandedNodeIdentifier(id)
        })
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
        let previous_gpu = globals.renderer_is_gpu;
        f(&mut globals);
        let changed = previous_gpu != globals.renderer_is_gpu;
        drop(globals);
        if changed {
            #[cfg(feature = "designtime")]
            let root = self
                .get_userland_root_expanded_node()
                .or_else(|| self.get_root_expanded_node());
            #[cfg(not(feature = "designtime"))]
            let root = self.get_root_expanded_node();
            if let Some(root) = root {
                self.rebind_root_camera(&root);
            }
        }
    }

    pub fn queue_custom_event(&self, source_expanded_node: Rc<ExpandedNode>, name: &'static str) {
        let mut queued_custom_events = borrow_mut!(self.queued_custom_events);
        queued_custom_events.push(QueuedEvent::Custom(source_expanded_node, name));
    }

    pub(crate) fn queue_submit(&self, node: Rc<ExpandedNode>) {
        borrow_mut!(self.queued_custom_events).push(QueuedEvent::Submit(node));
    }

    pub fn flush_custom_events(self: &Rc<Self>) -> Result<(), String> {
        // Release the queue borrow before calling application handlers, which
        // may enqueue another action for the next flush.
        let to_flush = std::mem::take(&mut *borrow_mut!(self.queued_custom_events));
        for event in to_flush {
            match event {
                QueuedEvent::Custom(target, ident) => target.dispatch_custom_event(ident, self)?,
                QueuedEvent::Submit(target) => {
                    target.emit_submit(pax_runtime_api::SubmitTrigger::Programmatic, self);
                }
            }
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
        #[cfg(feature = "3d")]
        {
            if let Some(previous) = self
                .get_userland_root_expanded_node()
                .or_else(|| self.get_root_expanded_node())
            {
                previous
                    .own_camera_view
                    .replace_with(LocalProperty::new(None));
            }
            if let Some(root) = &root {
                crate::camera::bind_root(root, self.globals().renderer_is_gpu);
            }
        }
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
    loading: Option<pax_runtime_api::LoadingContext>,
    parent: Option<Rc<RuntimePropertiesStackFrame>>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(symbols_within_frame: HashMap<String, Variable>) -> Rc<Self> {
        Rc::new(Self {
            symbols_within_frame,
            loading: None,
            parent: None,
        })
    }

    pub fn push(self: &Rc<Self>, symbols_within_frame: HashMap<String, Variable>) -> Rc<Self> {
        Rc::new(RuntimePropertiesStackFrame {
            symbols_within_frame,
            loading: None,
            parent: Some(Rc::clone(self)),
        })
    }

    /// Install loading state only in the loading component's expression ancestry.
    /// Ordinary application roots never inherit this context.
    #[doc(hidden)]
    pub fn push_loading(self: &Rc<Self>, loading: pax_runtime_api::LoadingContext) -> Rc<Self> {
        Rc::new(Self {
            symbols_within_frame: HashMap::from([("@loading".into(), loading.as_variable())]),
            loading: Some(loading),
            parent: Some(self.clone()),
        })
    }

    pub(crate) fn loading(&self) -> Option<pax_runtime_api::LoadingContext> {
        self.loading
            .clone()
            .or_else(|| self.parent.as_ref()?.loading())
    }

    pub fn pop(self: &Rc<Self>) -> Option<Rc<Self>> {
        self.parent.clone()
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

#[cfg(test)]
fn light_reaches_frame_ancestry(
    owner_frame: Option<&ExpandedNodeIdentifier>,
    frame_ancestry: &HashSet<ExpandedNodeIdentifier>,
) -> bool {
    owner_frame.is_none_or(|frame| frame_ancestry.contains(frame))
}

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext {
    pub stack_frame: Rc<RuntimePropertiesStackFrame>,
}

#[cfg(test)]
mod light_scope_tests {
    use super::*;
    use crate::api::math::Transform2;
    use crate::{
        BaseInstance, CommonPropertiesInit, ComponentInstance, InstanceFlags, InstanceNode,
        InstantiationArgs, PropertiesInit, PropertiesScopeInit, RouteLocation, TransformAndBounds,
    };
    use pax_runtime_api::pax_value::{PaxAny, PaxValue};
    use pax_runtime_api::{Layer, Platform, SceneAmbientLight, TargetInfo, OS};
    use std::fmt;

    #[derive(Clone, Copy)]
    enum TestLightingRole {
        Frame,
        Light(f64),
        Ambient(f64),
        Surface,
    }

    struct TestLightingNode {
        base: BaseInstance,
        role: TestLightingRole,
        enabled: Cell<bool>,
        intensity_multiplier: Cell<f64>,
        resolutions: Cell<usize>,
    }

    impl TestLightingNode {
        fn new(
            role: TestLightingRole,
            children: Vec<Rc<dyn InstanceNode>>,
        ) -> Rc<TestLightingNode> {
            Rc::new(Self {
                base: BaseInstance::new(
                    node_args(children),
                    InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: true,
                        layer: match role {
                            TestLightingRole::Frame => Layer::DontCare,
                            TestLightingRole::Light(_)
                            | TestLightingRole::Ambient(_)
                            | TestLightingRole::Surface => Layer::Canvas,
                        },
                        is_component: false,
                        is_slot: false,
                    },
                ),
                role,
                enabled: Cell::new(true),
                intensity_multiplier: Cell::new(1.0),
                resolutions: Cell::new(0),
            })
        }

        fn as_instance(self: &Rc<Self>) -> Rc<dyn InstanceNode> {
            self.clone()
        }

        fn set_enabled(&self, enabled: bool) {
            self.enabled.set(enabled);
        }
    }

    impl InstanceNode for TestLightingNode {
        fn has_scene_lighting(&self) -> bool {
            matches!(
                self.role,
                TestLightingRole::Light(_) | TestLightingRole::Ambient(_)
            )
        }

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
                        layer: Layer::Canvas,
                        is_component: false,
                        is_slot: false,
                    },
                ),
                role: TestLightingRole::Surface,
                enabled: Cell::new(true),
                intensity_multiplier: Cell::new(1.0),
                resolutions: Cell::new(0),
            })
        }

        fn resolve_debug(
            &self,
            f: &mut fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> fmt::Result {
            f.debug_struct("TestLightingNode").finish()
        }

        fn resolve_scene_light(
            &self,
            _expanded_node: &ExpandedNode,
            _context: &RuntimeContext,
        ) -> Option<SceneLight> {
            assert!(self.has_scene_lighting());
            self.resolutions.set(self.resolutions.get() + 1);
            match self.role {
                TestLightingRole::Light(intensity) if self.enabled.get() => Some(SceneLight {
                    intensity: intensity * self.intensity_multiplier.get(),
                    ..Default::default()
                }),
                _ => None,
            }
        }

        fn resolve_scene_ambient_light(
            &self,
            _expanded_node: &ExpandedNode,
            _context: &RuntimeContext,
        ) -> Option<SceneAmbientLight> {
            assert!(self.has_scene_lighting());
            self.resolutions.set(self.resolutions.get() + 1);
            match self.role {
                TestLightingRole::Ambient(intensity) if self.enabled.get() => {
                    Some(SceneAmbientLight {
                        intensity,
                        ..Default::default()
                    })
                }
                _ => None,
            }
        }

        fn establishes_light_frame(&self) -> bool {
            matches!(self.role, TestLightingRole::Frame)
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    struct ScopedLightingFixture {
        context: Rc<RuntimeContext>,
        _root: Rc<ExpandedNode>,
        root_light: Rc<TestLightingNode>,
        outer_light: Rc<TestLightingNode>,
        ambient: Rc<TestLightingNode>,
        root_surface: Rc<ExpandedNode>,
        outer_surface: Rc<ExpandedNode>,
        nested_surface: Rc<ExpandedNode>,
        sibling_surface: Rc<ExpandedNode>,
        sibling_frame: Rc<ExpandedNode>,
    }

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

    fn properties_factory() -> crate::PropertiesFactory {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
        })
    }

    fn node_args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Default,
            prototypical_properties: PropertiesInit::Factory(properties_factory()),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn component_args(template: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Default,
            prototypical_properties: PropertiesInit::Factory(properties_factory()),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(template)),
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn mount_test_tree(
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> (Rc<RuntimeContext>, Rc<ExpandedNode>) {
        let root_component = ComponentInstance::instantiate(component_args(children));
        let context = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        crate::engine::occlusion::update_node_occlusion(&root, &context);
        borrow_mut!(context.dirty_canvases).resize(context.layer_count.get(), true);
        (context, root)
    }

    fn clear_dirty_canvas_nodes(context: &RuntimeContext) {
        for id in context.dirty_canvas_node_ids() {
            context.clear_canvas_node_dirty(&id);
        }
    }

    fn assert_light_masks_match_full_recompute(context: &RuntimeContext) {
        let cache = borrow!(context.node_cache);
        for node in cache.eid_to_node.values() {
            if borrow!(node.instance_node).base().flags().layer != Layer::Canvas {
                continue;
            }
            let scopes = borrow!(context.layer_light_scopes);
            let ancestry = RuntimeContext::light_frame_ancestry(node);
            let expected = scopes
                .get(&node.occlusion.get().render_layer_id)
                .into_iter()
                .flatten()
                .enumerate()
                .fold(0, |mask, (slot, (_, owner))| {
                    if light_reaches_frame_ancestry(owner.as_ref(), &ancestry) {
                        mask | (1 << slot)
                    } else {
                        mask
                    }
                });
            assert_eq!(
                context.canvas_node_light_mask(node.id),
                expected,
                "{:?}",
                node.id
            );
        }
    }

    fn scoped_lighting_fixture() -> ScopedLightingFixture {
        let nested_light = TestLightingNode::new(TestLightingRole::Light(3.0), Vec::new());
        let nested_surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let nested_frame = TestLightingNode::new(
            TestLightingRole::Frame,
            vec![nested_light.as_instance(), nested_surface.as_instance()],
        );

        let sibling_surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let sibling_frame =
            TestLightingNode::new(TestLightingRole::Frame, vec![sibling_surface.as_instance()]);

        let outer_light = TestLightingNode::new(TestLightingRole::Light(2.0), Vec::new());
        let outer_surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let outer_frame = TestLightingNode::new(
            TestLightingRole::Frame,
            vec![
                outer_light.as_instance(),
                outer_surface.as_instance(),
                nested_frame.as_instance(),
                sibling_frame.as_instance(),
            ],
        );

        let root_light = TestLightingNode::new(TestLightingRole::Light(1.0), Vec::new());
        let root_surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let ambient = TestLightingNode::new(TestLightingRole::Ambient(0.72), Vec::new());
        ambient.set_enabled(false);

        let (context, root) = mount_test_tree(vec![
            root_light.as_instance(),
            root_surface.as_instance(),
            outer_frame.as_instance(),
            ambient.as_instance(),
        ]);

        let root_children = root.children.get();
        let root_surface_node = root_children[1].clone();
        let outer_frame_node = root_children[2].clone();
        let outer_children = outer_frame_node.children.get();
        let outer_surface_node = outer_children[1].clone();
        let nested_frame_node = outer_children[2].clone();
        let sibling_frame_node = outer_children[3].clone();
        let nested_children = nested_frame_node.children.get();
        let nested_surface_node = nested_children[1].clone();
        let sibling_surface_node = sibling_frame_node.children.get()[0].clone();

        ScopedLightingFixture {
            context,
            _root: root,
            root_light,
            outer_light,
            ambient,
            root_surface: root_surface_node,
            outer_surface: outer_surface_node,
            nested_surface: nested_surface_node,
            sibling_surface: sibling_surface_node,
            sibling_frame: sibling_frame_node,
        }
    }

    #[test]
    fn root_lights_reach_every_frame_ancestry() {
        let ancestry = HashSet::from([ExpandedNodeIdentifier(10), ExpandedNodeIdentifier(20)]);
        assert!(light_reaches_frame_ancestry(None, &ancestry));
    }

    #[test]
    fn strip_light_layer_transform_preserves_both_endpoints() {
        let light = SceneLight {
            shape: LightShape::Strip,
            position: pax_runtime_api::Vector3::new(20., 30., 80.),
            half_extent: pax_runtime_api::Vector3::new(45., -60., 0.),
            ..Default::default()
        };
        let source = [100., -50., 20.];
        let target = [-12., 300., 5.];
        let resolved = transform_scene_light_for_layer(light.clone(), source, target);
        for sign in [-1., 1.] {
            let expected = kurbo::Point::new(
                light.position.x + sign * light.half_extent.x + source[0] - target[0],
                light.position.y + sign * light.half_extent.y + source[1] - target[1],
            );
            assert!(
                (resolved.position.x + sign * resolved.half_extent.x - expected.x).abs() < 1e-9
            );
            assert!(
                (resolved.position.y + sign * resolved.half_extent.y - expected.y).abs() < 1e-9
            );
        }
        assert_eq!(resolved.position.z, 95.);
        assert_eq!(resolved.half_extent, light.half_extent);
    }

    #[test]
    fn framed_lights_reach_only_descendants_of_their_owner_frame() {
        let outer_frame = ExpandedNodeIdentifier(10);
        let nested_frame = ExpandedNodeIdentifier(20);
        let sibling_frame = ExpandedNodeIdentifier(30);
        let nested_ancestry = HashSet::from([outer_frame, nested_frame]);

        assert!(light_reaches_frame_ancestry(
            Some(&outer_frame),
            &nested_ancestry
        ));
        assert!(light_reaches_frame_ancestry(
            Some(&nested_frame),
            &nested_ancestry
        ));
        assert!(!light_reaches_frame_ancestry(
            Some(&sibling_frame),
            &nested_ancestry
        ));
    }

    #[test]
    fn structural_invalidation_attribution_is_opt_in_bounded_and_consumed_per_rebuild() {
        let fixture = scoped_lighting_fixture();
        let ctx = &fixture.context;
        ctx.mark_occlusion_dirty();
        assert!(ctx
            .occlusion_state
            .borrow()
            .invalidations
            .iter()
            .all(Option::is_none));
        ctx.set_occlusion_diagnostics(true);
        for id in 0..20 {
            ctx.mark_occlusion_dirty_for_node(id);
        }
        let causes = ctx.occlusion_state.borrow().invalidations;
        assert_eq!(causes.iter().flatten().count(), 1);
        assert_eq!(causes[0].unwrap().count, 20);
        assert_eq!(causes[0].unwrap().node, Some(19));
        crate::engine::occlusion::update_node_occlusion(&fixture._root, ctx);
        let stats = ctx.occlusion_stats();
        assert_eq!(stats.rebuild_origins[0].unwrap().count, 20);
        assert!(ctx
            .occlusion_state
            .borrow()
            .invalidations
            .iter()
            .all(Option::is_none));
        crate::engine::occlusion::update_node_occlusion(&fixture._root, ctx);
        assert_eq!(ctx.occlusion_stats().passes, stats.passes + 1);
        assert_eq!(ctx.occlusion_stats().rebuilds, stats.rebuilds);
    }

    #[test]
    fn mounted_tree_resolves_root_nested_and_sibling_light_masks() {
        let fixture = scoped_lighting_fixture();

        let lighting = fixture.context.collect_scene_lighting_for_layer(0);

        assert_eq!(
            lighting
                .lights
                .iter()
                .map(|light| light.intensity)
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0]
        );
        assert!(!lighting.ambient_is_authored);
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.root_surface.id),
            0b001
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.outer_surface.id),
            0b011
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.nested_surface.id),
            0b111
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.sibling_surface.id),
            0b011
        );
    }

    #[test]
    fn unlit_scene_does_not_resolve_nonproviders_or_store_zero_masks() {
        let surfaces = (0..1000)
            .map(|_| TestLightingNode::new(TestLightingRole::Surface, Vec::new()))
            .collect::<Vec<_>>();
        let (context, _root) =
            mount_test_tree(surfaces.iter().map(TestLightingNode::as_instance).collect());
        clear_dirty_canvas_nodes(&context);
        for _ in 0..100 {
            let lighting = context.collect_scene_lighting_for_layer(0);
            assert!(lighting.lights.is_empty());
            assert!(!lighting.ambient_is_authored);
            assert_eq!(
                lighting.ambient.intensity,
                SceneLighting::DEFAULT_AMBIENT_INTENSITY
            );
        }
        assert!(surfaces
            .iter()
            .all(|surface| surface.resolutions.get() == 0));
        assert!(borrow!(context.canvas_node_light_masks).is_empty());
        assert!(context.dirty_canvas_node_ids().is_empty());
    }

    #[test]
    fn disabled_provider_enable_remove_and_remount_refresh_memberships() {
        let light = TestLightingNode::new(TestLightingRole::Light(2.0), Vec::new());
        light.set_enabled(false);
        let surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let (context, root) = mount_test_tree(vec![light.as_instance(), surface.as_instance()]);
        let children = root.children.get();
        let light_node = children[0].clone();
        let surface_node = &children[1];
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        light.set_enabled(true);
        assert_eq!(context.collect_scene_lighting_for_layer(0).lights.len(), 1);
        assert_eq!(context.canvas_node_light_mask(surface_node.id), 1);
        clear_dirty_canvas_nodes(&context);
        light_node.clone().recurse_unmount(&context);
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        assert_eq!(context.canvas_node_light_mask(surface_node.id), 0);
        assert!(context.is_canvas_node_dirty(&surface_node.id));
        assert!(borrow!(context.scene_lighting_nodes).is_empty());
        assert!(borrow!(context.canvas_node_light_masks).is_empty());
        light_node.recurse_mount(&context);
        assert_eq!(context.collect_scene_lighting_for_layer(0).lights.len(), 1);
        assert_eq!(context.canvas_node_light_mask(surface_node.id), 1);
    }

    #[test]
    fn template_replacement_refreshes_provider_capability_without_remount() {
        let surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let (context, root) = mount_test_tree(vec![surface.as_instance()]);
        let node = root.children.get()[0].clone();
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        let light = TestLightingNode::new(TestLightingRole::Light(3.0), Vec::new());
        node.recreate_with_new_data(light.as_instance(), &context);
        assert_eq!(
            context.collect_scene_lighting_for_layer(0).lights[0].intensity,
            3.0
        );
        node.recreate_with_new_data(surface.as_instance(), &context);
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        assert_eq!(context.canvas_node_light_mask(node.id), 0);
        assert!(borrow!(context.scene_lighting_nodes).is_empty());
    }

    #[test]
    fn slot_reallocation_and_render_reparenting_refresh_retained_masks() {
        let fixture = scoped_lighting_fixture();
        fixture.context.collect_scene_lighting_for_layer(0);
        clear_dirty_canvas_nodes(&fixture.context);

        fixture.outer_light.set_enabled(false);
        let lighting = fixture.context.collect_scene_lighting_for_layer(0);

        assert_eq!(
            lighting
                .lights
                .iter()
                .map(|light| light.intensity)
                .collect::<Vec<_>>(),
            vec![1.0, 3.0]
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.root_surface.id),
            0b01
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.outer_surface.id),
            0b01
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.nested_surface.id),
            0b11
        );
        assert!(fixture
            .context
            .is_canvas_node_dirty(&fixture.outer_surface.id));
        assert!(fixture
            .context
            .is_canvas_node_dirty(&fixture.nested_surface.id));
        assert!(!fixture
            .context
            .is_canvas_node_dirty(&fixture.root_surface.id));

        clear_dirty_canvas_nodes(&fixture.context);
        *borrow_mut!(fixture.nested_surface.render_parent) = Rc::downgrade(&fixture.sibling_frame);
        fixture.context.mark_occlusion_dirty();
        fixture.context.collect_scene_lighting_for_layer(0);

        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.nested_surface.id),
            0b01
        );
        assert!(fixture
            .context
            .is_canvas_node_dirty(&fixture.nested_surface.id));
    }

    #[test]
    fn default_and_authored_ambient_keep_empty_direct_masks_distinct() {
        let fixture = scoped_lighting_fixture();
        fixture.root_light.set_enabled(false);

        let default_ambient = fixture.context.collect_scene_lighting_for_layer(0);
        assert!(default_ambient.active);
        assert!(!default_ambient.ambient_is_authored);
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.root_surface.id),
            0
        );
        assert_eq!(
            default_ambient.ambient.intensity,
            SceneLighting::DEFAULT_AMBIENT_INTENSITY
        );

        fixture.ambient.set_enabled(true);
        let authored_ambient = fixture.context.collect_scene_lighting_for_layer(0);
        assert!(authored_ambient.active);
        assert!(authored_ambient.ambient_is_authored);
        assert_eq!(authored_ambient.ambient.intensity, 0.72);
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.root_surface.id),
            0
        );
    }

    #[test]
    fn light_overflow_selection_is_deterministic_and_recovers() {
        let lights = (0..10)
            .map(|index| {
                TestLightingNode::new(TestLightingRole::Light((index + 1) as f64), Vec::new())
            })
            .collect::<Vec<_>>();
        let surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let mut templates = lights
            .iter()
            .rev()
            .map(TestLightingNode::as_instance)
            .collect::<Vec<_>>();
        templates.push(surface.as_instance());
        let (context, root) = mount_test_tree(templates);
        let expanded = root.children.get();
        let surface_node = expanded[10].clone();

        let first = context.collect_scene_lighting_for_layer(0);
        let second = context.collect_scene_lighting_for_layer(0);
        let expected = vec![10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0];
        assert_eq!(
            first
                .lights
                .iter()
                .map(|light| light.intensity)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(first.lights, second.lights);
        assert_eq!(context.canvas_node_light_mask(surface_node.id), 0xff);
        assert_eq!(borrow!(context.lighting_overflow_counts).get(&0), Some(&2));

        lights[0].set_enabled(false);
        lights[1].set_enabled(false);
        let recovered = context.collect_scene_lighting_for_layer(0);
        assert_eq!(
            recovered
                .lights
                .iter()
                .map(|light| light.intensity)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(!borrow!(context.lighting_overflow_counts).contains_key(&0));
    }

    #[test]
    fn unlit_scene_does_no_lighting_scan_regardless_of_scene_size() {
        let surfaces = (0..1024)
            .map(|_| TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance())
            .collect();
        let (context, _root) = mount_test_tree(surfaces);
        clear_dirty_canvas_nodes(&context);
        for _ in 0..20 {
            context.set_canvas_dirty(0);
            let lighting = context.collect_scene_lighting_for_layer(0);
            assert!(lighting.lights.is_empty());
            assert!(!lighting.ambient_is_authored);
        }
        assert_eq!(context.lighting_mask_node_visits.get(), 0);
        assert!(borrow!(context.canvas_node_light_masks).is_empty());
        assert!(context.dirty_canvas_node_ids().is_empty());
    }

    #[test]
    fn lighting_masks_survive_geometry_and_value_changes_but_not_scope_changes() {
        let fixture = scoped_lighting_fixture();
        fixture.context.collect_scene_lighting_for_layer(0);
        clear_dirty_canvas_nodes(&fixture.context);
        let visits = fixture.context.lighting_mask_node_visits.get();
        fixture.root_light.intensity_multiplier.set(4.0);
        fixture
            .context
            .mark_node_occlusion_dirty(fixture.outer_surface.id.to_u32());
        let lighting = fixture.context.collect_scene_lighting_for_layer(0);
        assert_eq!(lighting.lights[0].intensity, 4.0);
        assert_eq!(fixture.context.lighting_mask_node_visits.get(), visits);
        assert!(fixture.context.dirty_canvas_node_ids().is_empty());

        // Move the root light into a sibling frame: slot identity is stable but
        // its lexical reach changes. Production reparenting invalidates structure.
        let root_light = fixture._root.children.get()[0].clone();
        *borrow_mut!(root_light.render_parent) = Rc::downgrade(&fixture.sibling_frame);
        fixture.context.mark_occlusion_dirty();
        fixture.context.collect_scene_lighting_for_layer(0);
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.root_surface.id),
            0
        );
        assert_eq!(
            fixture
                .context
                .canvas_node_light_mask(fixture.sibling_surface.id),
            0b011
        );
        assert!(fixture
            .context
            .is_canvas_node_dirty(&fixture.root_surface.id));
        assert!(fixture.context.lighting_mask_node_visits.get() > visits);
    }

    #[test]
    fn unrelated_structure_retains_light_slots_masks_and_clean_canvases() {
        let fixture = scoped_lighting_fixture();
        let ctx = &fixture.context;
        ctx.collect_scene_lighting_for_layer(0);
        clear_dirty_canvas_nodes(ctx);
        ctx.clear_all_dirty_canvases();
        let visits = ctx.lighting_mask_node_visits.get();

        // Generic child-list invalidation is not a lighting dependency change.
        ctx.mark_occlusion_dirty();
        assert!(ctx.take_scene_lighting_dirty());
        assert!(!ctx.has_dirty_canvas_layers());
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits);
        assert!(ctx.dirty_canvas_node_ids().is_empty());
        assert!(!ctx.has_dirty_canvas_layers());
        assert_light_masks_match_full_recompute(ctx);
    }

    #[test]
    fn recipient_removal_and_remount_only_recompute_that_recipient() {
        let fixture = scoped_lighting_fixture();
        let ctx = &fixture.context;
        ctx.collect_scene_lighting_for_layer(0);
        let visits = ctx.lighting_mask_node_visits.get();

        fixture.nested_surface.clone().recurse_unmount(ctx);
        clear_dirty_canvas_nodes(ctx);
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits);
        assert_eq!(ctx.canvas_node_light_mask(fixture.nested_surface.id), 0);
        assert!(ctx.dirty_canvas_node_ids().is_empty());

        fixture.nested_surface.clone().recurse_mount(ctx);
        clear_dirty_canvas_nodes(ctx);
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits + 1);
        assert_eq!(ctx.canvas_node_light_mask(fixture.nested_surface.id), 0b111);
        assert_eq!(ctx.dirty_canvas_node_ids(), vec![fixture.nested_surface.id]);
        assert_light_masks_match_full_recompute(ctx);
    }

    #[test]
    fn recipient_reparenting_only_recomputes_changed_ancestry() {
        let fixture = scoped_lighting_fixture();
        let ctx = &fixture.context;
        ctx.collect_scene_lighting_for_layer(0);
        clear_dirty_canvas_nodes(ctx);
        let visits = ctx.lighting_mask_node_visits.get();
        let old_parent = fixture.nested_surface.render_parent_node().unwrap();
        let mut children = fixture.sibling_frame.children.get();
        children.push(fixture.nested_surface.clone());
        let children = fixture.sibling_frame.attach_children(
            children,
            ctx,
            &fixture.sibling_frame.parent_frame,
        );
        fixture.sibling_frame.children.set(children);
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits + 1);
        assert_eq!(ctx.canvas_node_light_mask(fixture.nested_surface.id), 0b011);
        assert_eq!(ctx.dirty_canvas_node_ids(), vec![fixture.nested_surface.id]);
        assert_light_masks_match_full_recompute(ctx);

        let mut children = old_parent.children.get();
        children.push(fixture.nested_surface.clone());
        let children = old_parent.attach_children(children, ctx, &old_parent.parent_frame);
        old_parent.children.set(children);
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits + 2);
        assert_eq!(ctx.canvas_node_light_mask(fixture.nested_surface.id), 0b111);
        assert_light_masks_match_full_recompute(ctx);
    }

    #[test]
    fn uncollected_recipient_change_survives_another_topology_reconciliation() {
        let fixture = scoped_lighting_fixture();
        let ctx = &fixture.context;
        ctx.collect_scene_lighting_for_layer(0);
        let visits = ctx.lighting_mask_node_visits.get();
        *borrow_mut!(fixture.nested_surface.render_parent) = Rc::downgrade(&fixture.sibling_frame);
        ctx.mark_occlusion_dirty();
        ctx.collect_scene_lighting_for_layer(1);
        ctx.mark_occlusion_dirty();
        ctx.collect_scene_lighting_for_layer(1);
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits + 1);
        assert_eq!(ctx.canvas_node_light_mask(fixture.nested_surface.id), 0b011);
        assert_light_masks_match_full_recompute(ctx);
    }

    #[test]
    fn frame_replacement_and_slot_reordering_match_full_recompute() {
        let fixture = scoped_lighting_fixture();
        let ctx = &fixture.context;
        ctx.collect_scene_lighting_for_layer(0);
        let parent = fixture.nested_surface.render_parent_node().unwrap();
        // Replace the non-rendering scope while retaining its mounted children.
        parent.recreate_with_new_data(
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
            ctx,
        );
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(
            ctx.canvas_node_light_mask(fixture.sibling_surface.id),
            0b111
        );
        assert_light_masks_match_full_recompute(ctx);

        let provider = fixture._root.children.get()[0].clone();
        let mut occlusion = provider.occlusion.get();
        occlusion.z_index = -100;
        provider.occlusion.set(occlusion);
        ctx.mark_occlusion_dirty();
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.canvas_node_light_mask(fixture.root_surface.id), 0b100);
        assert_light_masks_match_full_recompute(ctx);
    }

    #[test]
    fn changed_light_slots_only_recompute_recipients_in_their_layer() {
        let first = TestLightingNode::new(TestLightingRole::Light(1.0), Vec::new());
        let second = TestLightingNode::new(TestLightingRole::Light(2.0), Vec::new());
        let (ctx, root) = mount_test_tree(vec![
            first.as_instance(),
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
            second.as_instance(),
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
        ]);
        let nodes = root.children.get();
        for node in &nodes[2..] {
            let mut occlusion = node.occlusion.get();
            occlusion.render_layer_id = 1;
            node.occlusion.set(occlusion);
        }
        borrow_mut!(ctx.dirty_canvases).resize(2, true);
        ctx.collect_scene_lighting_for_layer(0);
        ctx.collect_scene_lighting_for_layer(1);
        let visits = ctx.lighting_mask_node_visits.get();
        clear_dirty_canvas_nodes(&ctx);
        ctx.clear_all_dirty_canvases();

        second.set_enabled(false);
        ctx.mark_scene_lighting_dirty();
        ctx.collect_scene_lighting_for_layer(0);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits);
        assert!(!ctx.is_canvas_dirty(&0));
        ctx.collect_scene_lighting_for_layer(1);
        assert_eq!(ctx.lighting_mask_node_visits.get(), visits + 2);
        assert!(ctx.is_canvas_dirty(&1));
        assert!(!ctx.is_canvas_node_dirty(&nodes[1].id));
        assert!(ctx.is_canvas_node_dirty(&nodes[3].id));
        assert_light_masks_match_full_recompute(&ctx);
    }

    #[test]
    fn disabled_light_stays_registered_and_last_light_removal_clears_masks() {
        let light = TestLightingNode::new(TestLightingRole::Light(2.0), Vec::new());
        light.set_enabled(false);
        let surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let (context, root) = mount_test_tree(vec![light.as_instance(), surface.as_instance()]);
        let nodes = root.children.get();
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        assert_eq!(context.lighting_mask_node_visits.get(), 0);
        light.set_enabled(true);
        assert_eq!(context.collect_scene_lighting_for_layer(0).lights.len(), 1);
        assert_eq!(context.canvas_node_light_mask(nodes[1].id), 1);
        clear_dirty_canvas_nodes(&context);
        context.clear_all_dirty_canvases();
        nodes[0].clone().recurse_unmount(&context);
        assert!(context.is_canvas_dirty(&0));
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        assert_eq!(context.canvas_node_light_mask(nodes[1].id), 0);
        assert!(context.is_canvas_node_dirty(&nodes[1].id));
        assert!(borrow!(context.scene_lighting_nodes).is_empty());
        let visits = context.lighting_mask_node_visits.get();
        context.collect_scene_lighting_for_layer(0);
        assert_eq!(context.lighting_mask_node_visits.get(), visits);
    }

    #[test]
    fn ambient_only_scene_never_builds_direct_masks() {
        let ambient = TestLightingNode::new(TestLightingRole::Ambient(0.72), Vec::new());
        let (context, _root) = mount_test_tree(vec![
            ambient.as_instance(),
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
        ]);
        assert!(
            context
                .collect_scene_lighting_for_layer(0)
                .ambient_is_authored
        );
        ambient.set_enabled(false);
        assert!(
            !context
                .collect_scene_lighting_for_layer(0)
                .ambient_is_authored
        );
        ambient.set_enabled(true);
        assert_eq!(
            context
                .collect_scene_lighting_for_layer(0)
                .ambient
                .intensity,
            0.72
        );
        assert_eq!(context.lighting_mask_node_visits.get(), 0);
    }

    #[test]
    fn replacement_refreshes_lighting_capability_in_both_directions() {
        let (context, root) = mount_test_tree(vec![TestLightingNode::new(
            TestLightingRole::Surface,
            Vec::new(),
        )
        .as_instance()]);
        let node = root.children.get()[0].clone();
        context.collect_scene_lighting_for_layer(0);
        node.recreate_with_new_data(
            TestLightingNode::new(TestLightingRole::Light(5.0), Vec::new()).as_instance(),
            &context,
        );
        assert_eq!(
            context.collect_scene_lighting_for_layer(0).lights[0].intensity,
            5.0
        );
        node.recreate_with_new_data(
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
            &context,
        );
        assert!(context
            .collect_scene_lighting_for_layer(0)
            .lights
            .is_empty());
        assert_eq!(context.canvas_node_light_mask(node.id), 0);
    }

    #[test]
    fn ancestor_lights_follow_scroll_without_rescanning_and_clear_after_layer_move() {
        let light = TestLightingNode::new(TestLightingRole::Light(1.0), Vec::new());
        let (context, root) = mount_test_tree(vec![
            light.as_instance(),
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
            TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
        ]);
        let nodes = root.children.get();
        let surface = &nodes[2];
        let mut occlusion = surface.occlusion.get();
        occlusion.render_layer_id = 1;
        surface.occlusion.set(occlusion);
        context.register_layer_scroller_owner(1, nodes[1].id);
        context.set_scroller_surface_state(
            nodes[1].id.to_u32(),
            ScrollerSurfaceState {
                viewport_width: 100.0,
                viewport_height: 100.0,
                content_width: 100.0,
                content_height: 1000.0,
                clip_content: true,
                ..Default::default()
            },
        );
        let before = context.collect_scene_lighting_for_layer(1);
        let visits = context.lighting_mask_node_visits.get();
        context.set_scroller_surface_state(
            nodes[1].id.to_u32(),
            ScrollerSurfaceState {
                scroll_y: 300.0,
                presentation_scroll_y: 300.0,
                ..context
                    .get_scroller_surface_state(nodes[1].id.to_u32())
                    .unwrap()
            },
        );
        let after = context.collect_scene_lighting_for_layer(1);
        assert_eq!(
            after.lights[0].position.y - before.lights[0].position.y,
            300.0
        );
        assert_eq!(context.canvas_node_light_mask(surface.id), 1);
        assert_eq!(context.lighting_mask_node_visits.get(), visits);

        #[cfg(feature = "3d")]
        {
            // A quarter-turned scroller moves through world Z, not world Y.
            nodes[1].spatial_geometry.replace_with(LocalProperty::new(
                crate::SpatialGeometry::new(
                    pax_runtime_api::math::Transform3::rotate_x(std::f64::consts::FRAC_PI_2),
                    (100., 100.),
                ),
            ));
            let tilted = context.collect_scene_lighting_for_layer(1);
            assert!((tilted.lights[0].position.y - before.lights[0].position.y).abs() < 1e-9);
            assert!(
                (tilted.lights[0].position.z - before.lights[0].position.z - 300.).abs() < 1e-9
            );
            assert_eq!(context.lighting_mask_node_visits.get(), visits);
        }
        // Move the recipient into an unrelated layer. It must lose its old mask
        // even if the previous layer is collected before the new one.
        occlusion.render_layer_id = 2;
        surface.occlusion.set(occlusion);
        context.mark_occlusion_dirty();
        context.collect_scene_lighting_for_layer(1);
        assert!(context
            .collect_scene_lighting_for_layer(2)
            .lights
            .is_empty());
        assert_eq!(context.canvas_node_light_mask(surface.id), 0);
        borrow_mut!(context.dirty_canvases).resize(3, false);
        context.clear_all_dirty_canvases();
        nodes[0].clone().recurse_unmount(&context);
        assert!(!context.is_canvas_dirty(&1));
        assert!(context.take_scene_lighting_dirty());
    }

    #[test]
    fn detached_source_changes_preserve_incremental_scene_reconciliation() {
        let (context, root) = mount_test_tree(vec![TestLightingNode::new(
            TestLightingRole::Surface,
            Vec::new(),
        )
        .as_instance()]);
        let owner = root.children.get()[0].clone();
        let sources = owner.create_children_detached(
            vec![(
                TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
                owner.stack.clone(),
            )],
            &context,
            &Rc::downgrade(&owner),
        );
        let source = owner
            .attach_sidecar_children(sources, &context, &owner.parent_frame)
            .remove(0);
        source.mount_as_render_source(&owner, &context);
        let nested = source.create_children_detached(
            vec![(
                TestLightingNode::new(TestLightingRole::Surface, Vec::new()).as_instance(),
                source.stack.clone(),
            )],
            &context,
            &Rc::downgrade(&source),
        );
        let nested = source
            .attach_sidecar_children(nested, &context, &source.parent_frame)
            .remove(0);
        nested.mount_as_render_source(&source, &context);
        root.recurse_update(&context);
        crate::engine::occlusion::update_node_occlusion(&root, &context);
        let before = context.occlusion_stats();
        nested
            .get_common_properties()
            .borrow()
            .x
            .set(Some(pax_runtime_api::Size::Pixels(15.0.into())));
        context.mark_node_occlusion_dirty(nested.id.to_u32());
        assert!(context
            .occlusion_state
            .borrow()
            .dirty
            .contains(&owner.id.to_u32()));
        assert!(!context
            .occlusion_state
            .borrow()
            .dirty
            .contains(&source.id.to_u32()));
        assert!(!context
            .occlusion_state
            .borrow()
            .dirty
            .contains(&nested.id.to_u32()));
        root.recurse_update(&context);
        crate::engine::occlusion::update_node_occlusion(&root, &context);
        let after = context.occlusion_stats();
        assert_eq!(after.rebuilds, before.rebuilds);
        assert!(after.records_updated > before.records_updated);
        nested.recurse_unmount(&context);
        source.recurse_unmount(&context);
        root.recurse_unmount(&context);
    }

    #[test]
    fn canvas_surface_transform_uses_the_layer_owner_not_intermediate_frames() {
        let leaf = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let mask = TestLightingNode::new(TestLightingRole::Frame, vec![leaf.as_instance()]);
        let inner = TestLightingNode::new(TestLightingRole::Frame, vec![mask.as_instance()]);
        let outer = TestLightingNode::new(TestLightingRole::Frame, vec![inner.as_instance()]);
        let (context, root) = mount_test_tree(vec![outer.as_instance()]);
        let outer = root.children.get()[0].clone();
        let inner = outer.children.get()[0].clone();
        let mask = inner.children.get()[0].clone();
        let leaf = mask.children.get()[0].clone();

        let outer_transform = Affine::translate((40.0, 70.0)) * Affine::scale(1.5);
        let inner_local = Affine::translate((25.0, 35.0)) * Affine::rotate(0.3);
        let inner_transform = outer_transform * inner_local;
        let mask_transform = inner_transform * Affine::translate((60.0, 90.0));
        let leaf_local = Affine::translate((8.0, 12.0));
        let leaf_transform = mask_transform * leaf_local;
        for (node, transform, layer, parent_frame) in [
            (&outer, outer_transform, 0, None),
            (&inner, inner_transform, 1, Some(outer.id)),
            // Like a Mask in a Scroller, this structural frame stays on layer zero.
            (&mask, mask_transform, 0, Some(inner.id)),
            (&leaf, leaf_transform, 2, Some(mask.id)),
        ] {
            node.presentation_transform_and_bounds
                .replace_with(LocalProperty::new(TransformAndBounds {
                    transform: Transform2::new(transform.as_coeffs()),
                    bounds: (100.0, 100.0),
                }));
            let mut occlusion = node.occlusion.get();
            occlusion.render_layer_id = layer;
            node.occlusion.set(occlusion);
            node.parent_frame
                .replace_with(LocalProperty::new(parent_frame));
        }
        context.register_layer_scroller_owner(1, outer.id);
        context.register_layer_scroller_owner(2, inner.id);

        let assert_transform = |actual: Affine, expected: Affine| {
            for (actual, expected) in actual.as_coeffs().into_iter().zip(expected.as_coeffs()) {
                assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
            }
        };
        assert_transform(
            context.canvas_surface_transform_for_node(&leaf),
            Affine::translate((60.0, 90.0)) * leaf_local,
        );
        assert_transform(context.layer_to_root_transform(2), inner_transform);

        // A root canvas may also contain structural frames from another layer.
        // Without a surface owner its coordinates must remain in window space.
        context.clear_layer_scroller_owners();
        assert_transform(
            context.canvas_surface_transform_for_node(&leaf),
            leaf_transform,
        );
    }

    #[test]
    fn mouse_capture_routes_across_siblings_and_outside_until_release() {
        let mut knob = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let mut display = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        for node in [&mut knob, &mut display] {
            Rc::get_mut(node).unwrap().base = BaseInstance::new(
                node_args(vec![]),
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: Layer::Canvas,
                    is_component: false,
                    is_slot: false,
                },
            );
        }
        let (context, root) = mount_test_tree(vec![knob.as_instance(), display.as_instance()]);
        context.register_root_expanded_node(&root);
        let knob = root.children.get()[0].clone();
        let display = root.children.get()[1].clone();
        for (node, x) in [(&knob, 0.), (&display, 120.)] {
            node.presentation_transform_and_bounds
                .replace_with(LocalProperty::new(TransformAndBounds {
                    transform: Transform2::new(Affine::translate((x, 0.)).as_coeffs()),
                    bounds: (100., 100.),
                }));
        }
        use pax_runtime_api::MouseButton::Left;
        assert_eq!(
            context
                .mouse_down_target(Left, Point2::new(20., 20.))
                .unwrap()
                .id,
            knob.id
        );
        assert_eq!(
            context
                .mouse_move_target(Left, Point2::new(140., 20.))
                .unwrap()
                .id,
            knob.id
        );
        assert_eq!(
            context
                .mouse_move_target(Left, Point2::new(-100., -100.))
                .unwrap()
                .id,
            knob.id
        );
        assert_eq!(
            context
                .mouse_up_target(Left, Point2::new(140., 20.), false)
                .unwrap()
                .id,
            knob.id
        );
        assert!(context
            .mouse_click_target(Left, Point2::new(140., 20.))
            .is_none());
        assert_eq!(
            context
                .mouse_move_target(Left, Point2::new(140., 20.))
                .unwrap()
                .id,
            display.id
        );
        context.mouse_down_target(Left, Point2::new(140., 20.));
        context.mouse_up_target(Left, Point2::new(140., 20.), false);
        assert_eq!(
            context
                .mouse_click_target(Left, Point2::new(140., 20.))
                .unwrap()
                .id,
            display.id
        );
        context.mouse_down_target(Left, Point2::new(20., 20.));
        context.remove_from_cache(&knob);
        assert!(context
            .mouse_move_target(Left, Point2::new(140., 20.))
            .is_none());
    }

    #[test]
    fn touch_capture_releases_and_clears_removed_targets() {
        let surface = TestLightingNode::new(TestLightingRole::Surface, Vec::new());
        let (context, root) = mount_test_tree(vec![surface.as_instance()]);
        let surface_node = root.children.get()[0].clone();

        context.capture_touch_target(41, surface_node.id);
        assert!(Rc::ptr_eq(
            &context.captured_touch_target(41).unwrap(),
            &surface_node
        ));
        assert!(Rc::ptr_eq(
            &context.release_touch_target(41).unwrap(),
            &surface_node
        ));
        assert!(context.captured_touch_target(41).is_none());

        context.capture_touch_target(42, surface_node.id);
        context.remove_from_cache(&surface_node);
        assert!(context.captured_touch_target(42).is_none());
        assert!(context.release_touch_target(42).is_none());
    }
}

impl RuntimeContext {
    fn invalidate_scroll_input(&self, id: u32) {
        if let Some(node) = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)) {
            node.scroll_state.invalidate();
        }
    }

    pub(crate) fn presentation_work(
        &self,
    ) -> Rc<crate::scene_geometry::presentation::PreparationWork> {
        self.scene_geometry.borrow().presentation_work.clone()
    }

    /// Invalidate clip geometry stored outside the primitive's property graph,
    /// for example when a detached Mask source changes. Ordinary reactive clip
    /// properties are tracked automatically.
    pub fn mark_node_clip_dirty(&self, id: u32) {
        self.invalidate_clip_input(id);
        self.mark_node_occlusion_dirty(id);
    }

    fn invalidate_clip_input(&self, id: u32) {
        if let Some(node) = self.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) {
            node.local_clip_geometry.invalidate();
            node.world_clip_geometry.invalidate();
        }
    }
}
