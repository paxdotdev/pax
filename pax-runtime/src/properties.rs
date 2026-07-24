use crate::api::math::Point2;
use crate::api::Window;
use crate::constants::{ACCEL_HANDLERS, GYRO_HANDLERS, PRE_RENDER_HANDLERS, TICK_HANDLERS};
use pax_language::interpreter::property_resolution::IdentifierResolver;
use pax_manifest::cartridge_generation::{TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT};
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::{NativeMessage, ScreenshotData};
use pax_runtime_api::properties::{
    drain_effects, drain_effects_with_report, property_has_direct_outbound,
    property_outbound_debug_names, register_effect_property, register_effect_property_with_name,
    UntypedProperty,
};
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Event, Interpolatable, LightShape, MouseOut, MouseOver,
    Property, RenderContext, SceneLight, SceneLighting, Store, Variable,
};
use_RefCell!();
use kurbo::{Affine, Point};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::sync::OnceLock;
use std::time::Instant;

use crate::{ExpandedNode, Globals};

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
    source_to_root: Affine,
    root_to_target: Affine,
) -> SceneLight {
    if matches!(light.shape, LightShape::Point) {
        let point = Point::new(light.position.x, light.position.y);
        let point = root_to_target * (source_to_root * point);
        light.position.x = point.x;
        light.position.y = point.y;
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
    active_touch_targets: RefCell<HashMap<i64, ExpandedNodeIdentifier>>,
    queued_custom_events: RefCell<Vec<(Rc<ExpandedNode>, &'static str)>>,
    queued_renders: RefCell<Vec<Rc<ExpandedNode>>>,
    pub layer_count: Cell<usize>,
    pub dirty_canvases: Rc<RefCell<Vec<bool>>>,
    dirty_canvas_nodes: RefCell<HashSet<ExpandedNodeIdentifier>>,
    canvas_node_light_masks: RefCell<HashMap<ExpandedNodeIdentifier, u32>>,
    lighting_overflow_counts: RefCell<HashMap<usize, usize>>,
    targeted_canvas_replay_node_ids: RefCell<HashMap<usize, HashSet<u32>>>,
    removed_canvas_nodes: RefCell<Vec<(usize, u32)>>,
    occlusion_dirty: Cell<bool>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Coarse classification of changes to a native or browser-owned scroller surface.
pub enum ScrollerSurfaceStateChange {
    Unchanged,
    ScrollOnly,
    Structural,
}

impl ScrollerSurfaceState {
    fn close_enough(lhs: f64, rhs: f64) -> bool {
        (lhs - rhs).abs() <= 1e-4
    }

    fn structural_eq(&self, other: &Self) -> bool {
        Self::close_enough(self.viewport_width, other.viewport_width)
            && Self::close_enough(self.viewport_height, other.viewport_height)
            && Self::close_enough(self.content_width, other.content_width)
            && Self::close_enough(self.content_height, other.content_height)
            && self.clip_content == other.clip_content
    }

    fn scroll_eq(&self, other: &Self) -> bool {
        Self::close_enough(self.scroll_x, other.scroll_x)
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
        Self {
            next_uid: Cell::new(ExpandedNodeIdentifier(0)),
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_expanded_node: RefCell::new(Weak::new()),
            node_cache: RefCell::new(NodeCache::new()),
            active_touch_targets: Default::default(),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            canvas_node_light_masks: Default::default(),
            lighting_overflow_counts: Default::default(),
            targeted_canvas_replay_node_ids: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
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
            active_touch_targets: Default::default(),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            canvas_node_light_masks: Default::default(),
            lighting_overflow_counts: Default::default(),
            targeted_canvas_replay_node_ids: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
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
            active_touch_targets: Default::default(),
            queued_custom_events: Default::default(),
            queued_renders: Default::default(),
            layer_count: Cell::default(),
            last_topmost_element: Default::default(),
            dirty_canvases: Default::default(),
            dirty_canvas_nodes: Default::default(),
            canvas_node_light_masks: Default::default(),
            lighting_overflow_counts: Default::default(),
            targeted_canvas_replay_node_ids: Default::default(),
            removed_canvas_nodes: Default::default(),
            occlusion_dirty: Cell::new(true),
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
        if node.is_import_settings_node() {
            self.import_settings_node_count
                .set(self.import_settings_node_count.get() + 1);
        }
        self.mark_occlusion_dirty();
    }

    /// Remove a node from runtime lookup caches.
    pub fn remove_from_cache(&self, node: &Rc<ExpandedNode>) {
        borrow_mut!(self.node_cache).remove_from_cache(node);
        borrow_mut!(self.canvas_node_light_masks).remove(&node.id);
        borrow_mut!(self.active_touch_targets).retain(|_, target| *target != node.id);
        self.unregister_node_lifecycle_handlers(node.id);
        if node.is_import_settings_node() {
            self.import_settings_node_count
                .set(self.import_settings_node_count.get().saturating_sub(1));
        }
        self.mark_occlusion_dirty();
    }

    pub fn has_import_settings_nodes(&self) -> bool {
        self.import_settings_node_count.get() > 0
    }

    pub fn register_node_effect_property(&self, node: ExpandedNodeIdentifier, prop: &Property<()>) {
        self.register_node_effect_property_named(node, prop, "node effect");
    }

    pub fn register_node_effect_property_named(
        &self,
        node: ExpandedNodeIdentifier,
        prop: &Property<()>,
        effect_name: &str,
    ) {
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
        prop: &Property<()>,
        effect_name: &str,
    ) {
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
    ) -> Property<()> {
        let prop = Property::computed(effect, dependencies);
        self.register_node_effect_property_named(node, &prop, "registered node effect");
        prop
    }

    pub fn drain_node_effects(&self) {
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
                Some(previous) if previous.structural_eq(&state) && previous.scroll_eq(&state) => {
                    ScrollerSurfaceStateChange::Unchanged
                }
                Some(previous) if previous.structural_eq(&state) => {
                    ScrollerSurfaceStateChange::ScrollOnly
                }
                _ => ScrollerSurfaceStateChange::Structural,
            };
            if change != ScrollerSurfaceStateChange::Unchanged {
                states.insert(id, state);
            }
            change
        };

        match change {
            ScrollerSurfaceStateChange::Unchanged => {}
            ScrollerSurfaceStateChange::ScrollOnly => {
                self.mark_layer_canvas_plans_dirty();
                self.mark_scroller_content_layers_dirty(id);
            }
            ScrollerSurfaceStateChange::Structural => {
                self.mark_layer_canvas_plans_dirty();
                self.mark_occlusion_dirty();
            }
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
            self.mark_layer_canvas_plans_dirty();
        }

        change
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

    fn scroller_content_presentation_transform(&self, node: &ExpandedNode) -> Affine {
        fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
            if content <= viewport || !value.is_finite() {
                return 0.0;
            }
            value.max(0.0).min((content - viewport).max(0.0))
        }

        let root_delegates_to_page_scroll = self.get_root_scroller_id() == Some(node.id.to_u32())
            && self.get_visual_viewport_state().is_some();
        if root_delegates_to_page_scroll {
            return Affine::IDENTITY;
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

        if scroll_x.abs() <= f64::EPSILON && scroll_y.abs() <= f64::EPSILON {
            return Affine::IDENTITY;
        }

        let world_transform = Affine::from(node.transform_and_bounds.get().transform);
        let inverse_world = Affine::from(node.transform_and_bounds.get().transform.inverse());
        world_transform * Affine::translate((-scroll_x, -scroll_y)) * inverse_world
    }

    /// Resolve the browser/native presentation transform inherited by a node from ancestor
    /// scrollers. Layout transforms remain in content coordinates, while events arrive in the
    /// scrolled window coordinates that the user sees.
    pub(crate) fn presentation_scroll_transform_for_node(&self, node: &ExpandedNode) -> Affine {
        let mut ancestors = Vec::new();
        let mut current = node.render_parent_node();
        while let Some(parent) = current {
            current = parent.render_parent_node();
            ancestors.push(parent);
        }
        ancestors.reverse();

        ancestors
            .into_iter()
            .filter(|ancestor| borrow!(ancestor.instance_node).scrolls_content(ancestor))
            .fold(Affine::IDENTITY, |transform, scroller| {
                transform * self.scroller_content_presentation_transform(&scroller)
            })
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

    pub fn dirty_canvas_node_ids(&self) -> Vec<ExpandedNodeIdentifier> {
        borrow!(self.dirty_canvas_nodes).iter().copied().collect()
    }

    pub fn mark_canvas_nodes_on_layer_dirty(&self, layer: usize) {
        let node_cache = borrow!(self.node_cache);
        let dirty_nodes = &mut *borrow_mut!(self.dirty_canvas_nodes);
        for node in node_cache.eid_to_node.values() {
            if node.occlusion.get().render_layer_id == layer
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

    pub fn collect_scene_lighting_for_layer(&self, layer: usize) -> SceneLighting {
        let layer_to_root_transforms = self.layer_to_root_transforms(layer);
        let root_to_target = layer_to_root_transforms[&layer].inverse();
        let node_cache = borrow!(self.node_cache);
        let mut scoped_lights = Vec::new();
        let mut topmost_ambient = None;

        for node in node_cache.eid_to_node.values() {
            let occlusion = node.occlusion.get();
            let source_layer = occlusion.render_layer_id;
            let Some(source_to_root) = layer_to_root_transforms.get(&source_layer) else {
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
                        transform_scene_light_for_layer(light, *source_to_root, root_to_target),
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

        let mut resolved_masks = Vec::new();
        for node in node_cache.eid_to_node.values() {
            if node.occlusion.get().render_layer_id != layer
                || borrow!(node.instance_node).base().flags().layer != crate::api::Layer::Canvas
            {
                continue;
            }

            let frame_ancestry = Self::light_frame_ancestry(node);
            let mask = scoped_lights.iter().enumerate().fold(
                0u32,
                |mask, (slot, (_, _, owner_frame, _))| {
                    if light_reaches_frame_ancestry(owner_frame.as_ref(), &frame_ancestry) {
                        mask | (1u32 << slot)
                    } else {
                        mask
                    }
                },
            );
            resolved_masks.push((node.id, mask));
        }
        drop(node_cache);

        let mut changed_nodes = Vec::new();
        {
            let mut masks = borrow_mut!(self.canvas_node_light_masks);
            for (node_id, mask) in resolved_masks {
                if masks.insert(node_id, mask) != Some(mask) {
                    changed_nodes.push(node_id);
                }
            }
        }
        borrow_mut!(self.dirty_canvas_nodes).extend(changed_nodes);

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

    fn nearest_light_frame(node: &ExpandedNode) -> Option<ExpandedNodeIdentifier> {
        let mut current = node.render_parent_node();
        while let Some(parent) = current {
            if borrow!(parent.instance_node).establishes_light_frame() {
                return Some(parent.id);
            }
            current = parent.render_parent_node();
        }
        None
    }

    fn light_frame_ancestry(node: &ExpandedNode) -> HashSet<ExpandedNodeIdentifier> {
        let mut frames = HashSet::new();
        let mut current = node.render_parent_node();
        while let Some(parent) = current {
            if borrow!(parent.instance_node).establishes_light_frame() {
                frames.insert(parent.id);
            }
            current = parent.render_parent_node();
        }
        frames
    }

    fn layer_to_root_transforms(&self, layer: usize) -> HashMap<usize, Affine> {
        let mut transforms = HashMap::new();
        transforms.insert(layer, self.layer_to_root_transform(layer));

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
            transforms
                .entry(owner_layer)
                .or_insert_with(|| self.layer_to_root_transform(owner_layer));
            if owner_layer == current_layer {
                break;
            }
            current_layer = owner_layer;
        }

        transforms
    }

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

    fn canvas_surface_transform_for_node(&self, node: &ExpandedNode) -> Affine {
        let transform = Affine::from(node.transform_and_bounds.get().transform);
        let own_layer = node.occlusion.get().render_layer_id;
        let mut parent_frame_id = node.parent_frame.get();
        while let Some(current_parent_frame_id) = parent_frame_id {
            let Some(parent_frame) = self.get_expanded_node_by_eid(current_parent_frame_id) else {
                break;
            };
            if parent_frame.occlusion.get().render_layer_id != own_layer {
                return Affine::from(parent_frame.transform_and_bounds.get().transform.inverse())
                    * transform;
            }
            parent_frame_id = parent_frame.parent_frame.get();
        }
        transform
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
        layer_nodes.extend(node_ids.iter().copied());
    }

    pub fn take_targeted_canvas_replay_node_ids(&self) -> HashMap<usize, HashSet<u32>> {
        let targeted = &mut *borrow_mut!(self.targeted_canvas_replay_node_ids);
        std::mem::take(targeted)
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
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        let Some(root_node) = root.or_else(|| borrow!(self.root_expanded_node).upgrade()) else {
            return accum;
        };
        let mut to_process = vec![(root_node, false, Affine::IDENTITY, false)];
        while let Some((node, clipped, active_scroll_transform, retained_for_exit)) =
            to_process.pop()
        {
            // make sure slot sources are updated for this node
            node.compute_flattened_projected_children();
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
            let retained_children = borrow!(node.exiting_children).clone();
            to_process.extend(
                node.children
                    .get()
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
                            retained_for_exit,
                        )
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
            match self.role {
                TestLightingRole::Light(intensity) if self.enabled.get() => Some(SceneLight {
                    intensity,
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
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        crate::engine::occlusion::update_node_occlusion(&root, &context);
        (context, root)
    }

    fn clear_dirty_canvas_nodes(context: &RuntimeContext) {
        for id in context.dirty_canvas_node_ids() {
            context.clear_canvas_node_dirty(&id);
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
