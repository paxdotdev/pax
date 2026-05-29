use crate::constants::{PRE_RENDER_HANDLERS, TICK_HANDLERS};
use crate::{
    api::Property, ExpandedNodeIdentifier, RouteLocation, RuntimePropertiesStackFrame,
    TransformAndBounds, INTERNAL_ROUTE_LOCATION_SYMBOL,
};
use_RefCell!();
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use pax_message::NativeMessage;
use pax_runtime_api::{
    borrow, pax_value::PaxAny, use_RefCell, Accel, Event, Focus, Gyro, SelectStart, TargetInfo,
    Variable, Viewport, Window, OS,
};

use crate::api::{KeyDown, KeyPress, KeyUp, NodeContext, RenderContext};

use crate::{ComponentInstance, RuntimeContext};
use pax_runtime_api::Platform;

pub mod layer_surface;
pub mod layer_tiling;
pub mod node_interface;
pub mod occlusion;
// TODO move these to not be in engine - make separate crates?
pub mod pax_gpu_render_context;
pub mod piet_render_context;

/// The atomic unit of rendering; also the container for each unique tuple of computed properties.
/// Represents an expanded node, that is "expanded" in the context of computed properties and repeat expansion.
/// For example, a Rectangle inside `for i in 0..3` and a `for j in 0..4` would have 12 expanded nodes representing the 12 virtual Rectangles in the
/// rendered scene graph.
/// `ExpandedNode`s are architecturally "type-blind" — while they store typed data e.g. inside `computed_properties` and `computed_common_properties`,
/// they require coordinating with their "type-aware" [`InstanceNode`] to perform operations on those properties.
mod expanded_node;
pub use expanded_node::{
    ExpandedNode, RuntimeResolvedPropertyColumns, RuntimeResolvedPropertyEntry,
    RuntimeSettingsCondition, RuntimeSettingsLayer, RuntimeSettingsSignatureEntry,
    RuntimeSettingsSource,
};

use self::node_interface::NodeLocal;

fn saturating_u128_to_u64(value: u128) -> u64 {
    value.min(u64::MAX as u128) as u64
}

struct FilteredRenderPlan {
    dirty_nodes: HashSet<ExpandedNodeIdentifier>,
    render_path_nodes: HashSet<ExpandedNodeIdentifier>,
}

fn viewport_info_property(
    viewport_bounds: &Property<TransformAndBounds<NodeLocal, Window>>,
) -> Property<Viewport> {
    let cloned_viewport = viewport_bounds.clone();
    let deps = [cloned_viewport.untyped()];
    Property::computed(
        move || {
            let viewport = cloned_viewport.get();
            Viewport::new(viewport.bounds.0, viewport.bounds.1)
        },
        &deps,
    )
}

fn viewport_number_property(
    viewport: &Property<Viewport>,
    name: &str,
    accessor: fn(Viewport) -> f64,
) -> Property<f64> {
    let cloned_viewport = viewport.clone();
    let deps = [cloned_viewport.untyped()];
    Property::computed_with_name(move || accessor(cloned_viewport.get()), &deps, name)
}

fn viewport_bool_property(
    viewport: &Property<Viewport>,
    name: &str,
    accessor: fn(Viewport) -> bool,
) -> Property<bool> {
    let cloned_viewport = viewport.clone();
    let deps = [cloned_viewport.untyped()];
    Property::computed_with_name(move || accessor(cloned_viewport.get()), &deps, name)
}

#[cfg(feature = "designtime")]
use {crate::InstanceNode, pax_designtime::DesigntimeManager, pax_runtime_api::borrow_mut};

#[derive(Clone)]
/// Engine-wide reactive globals exposed to every component frame.
pub struct Globals {
    pub elapsed_frames: Property<u64>,
    pub elapsed_millis: Property<u64>,
    pub viewport: Property<TransformAndBounds<NodeLocal, Window>>,
    pub gyro: Property<Gyro>,
    pub accel: Property<Accel>,
    pub route_location: Property<RouteLocation>,
    pub browser_allows_scroller_vector_layers: Property<bool>,
    pub browser_allows_nested_scroller_vector_layers: Property<bool>,
    pub platform: Platform,
    pub os: OS,
    pub target: TargetInfo,
    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
    pub get_elapsed_millis: Rc<dyn Fn() -> u128>,
}

impl Globals {
    /// Build the root stack frame containing built-in globals plus internal engine state.
    pub fn stack_frame(&self) -> Rc<RuntimePropertiesStackFrame> {
        let target = self.target;
        let viewport = viewport_info_property(&self.viewport);

        let target_var = Variable::new_from_typed_property(Property::new(target));
        let viewport_var = Variable::new_from_typed_property(viewport.clone());
        let gyro_var = Variable::new_from_typed_property(self.gyro.clone());
        let accel_var = Variable::new_from_typed_property(self.accel.clone());
        let elapsed_frames_var = Variable::new_from_typed_property(self.elapsed_frames.clone());
        let elapsed_millis_var = Variable::new_from_typed_property(self.elapsed_millis.clone());
        let route_location_var = Variable::new_from_typed_property(self.route_location.clone());

        let global_scope = vec![
            ("$target".to_string(), target_var),
            ("$viewport".to_string(), viewport_var),
            (
                "$web".to_string(),
                Variable::new_from_typed_property(Property::new(target.web)),
            ),
            (
                "$native".to_string(),
                Variable::new_from_typed_property(Property::new(target.native)),
            ),
            (
                "$ios".to_string(),
                Variable::new_from_typed_property(Property::new(target.ios)),
            ),
            (
                "$iphone".to_string(),
                Variable::new_from_typed_property(Property::new(target.iphone)),
            ),
            (
                "$ipad".to_string(),
                Variable::new_from_typed_property(Property::new(target.ipad)),
            ),
            (
                "$macos".to_string(),
                Variable::new_from_typed_property(Property::new(target.macos)),
            ),
            (
                "$android".to_string(),
                Variable::new_from_typed_property(Property::new(target.android)),
            ),
            (
                "$windows".to_string(),
                Variable::new_from_typed_property(Property::new(target.windows)),
            ),
            (
                "$linux".to_string(),
                Variable::new_from_typed_property(Property::new(target.linux)),
            ),
            (
                "$mobile".to_string(),
                Variable::new_from_typed_property(Property::new(target.mobile)),
            ),
            (
                "$desktop".to_string(),
                Variable::new_from_typed_property(Property::new(target.desktop)),
            ),
            (
                "$major".to_string(),
                Variable::new_from_typed_property(viewport_number_property(
                    &viewport,
                    "$major",
                    |viewport| viewport.major,
                )),
            ),
            (
                "$minor".to_string(),
                Variable::new_from_typed_property(viewport_number_property(
                    &viewport,
                    "$minor",
                    |viewport| viewport.minor,
                )),
            ),
            (
                "$aspect".to_string(),
                Variable::new_from_typed_property(viewport_number_property(
                    &viewport,
                    "$aspect",
                    |viewport| viewport.aspect,
                )),
            ),
            (
                "$landscape".to_string(),
                Variable::new_from_typed_property(viewport_bool_property(
                    &viewport,
                    "$landscape",
                    |viewport| viewport.landscape,
                )),
            ),
            (
                "$portrait".to_string(),
                Variable::new_from_typed_property(viewport_bool_property(
                    &viewport,
                    "$portrait",
                    |viewport| viewport.portrait,
                )),
            ),
            (
                "$square".to_string(),
                Variable::new_from_typed_property(viewport_bool_property(
                    &viewport,
                    "$square",
                    |viewport| viewport.square,
                )),
            ),
            ("$gyro".to_string(), gyro_var),
            ("$accel".to_string(), accel_var),
            ("$frames".to_string(), elapsed_frames_var),
            ("$millis".to_string(), elapsed_millis_var),
            (
                INTERNAL_ROUTE_LOCATION_SYMBOL.to_string(),
                route_location_var,
            ),
        ]
        .into_iter()
        .collect();

        let root_env = RuntimePropertiesStackFrame::new(global_scope);
        root_env
    }
}

impl std::fmt::Debug for Globals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Globals")
            .field("elapsed_frames", &self.elapsed_frames)
            .field("elapsed_millis", &self.elapsed_millis)
            .field("viewport", &self.viewport)
            .field("gyro", &self.gyro)
            .field("accel", &self.accel)
            .field("route_location", &self.route_location)
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}

/// Singleton struct storing everything related to properties computation & rendering
pub struct PaxEngine {
    pub runtime_context: Rc<RuntimeContext>,
    pub root_expanded_node: Option<Rc<ExpandedNode>>,
    pub scroller_tiling_policy: layer_tiling::ScrollerTilingPolicy,
}

/// Indicates whether a handler should receive inline-node or containing-component properties.
pub enum HandlerLocation {
    Inline,
    Component,
}

/// Runtime event handler thunk generated from template bindings.
pub struct Handler {
    pub function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    pub location: HandlerLocation,
}

impl Handler {
    /// Build a handler whose `self` argument is the inline primitive/component.
    pub fn new_inline_handler(
        function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    ) -> Self {
        Handler {
            function,
            location: HandlerLocation::Inline,
        }
    }

    /// Build a handler whose `self` argument is the containing component.
    pub fn new_component_handler(
        function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    ) -> Self {
        Handler {
            function,
            location: HandlerLocation::Component,
        }
    }
}

/// Map from event key to one or more handlers registered on an instance node.
pub struct HandlerRegistry {
    pub handlers: HashMap<String, Vec<Handler>>,
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        HandlerRegistry {
            handlers: HashMap::new(),
        }
    }
}

/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl PaxEngine {
    #[cfg(not(feature = "designtime"))]
    fn build_globals(
        viewport_size: (f64, f64),
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
    ) -> Globals {
        use crate::api::math::Transform2;
        use pax_runtime_api::{properties, Functions};
        Functions::register_all_functions();

        let elapsed_frames = Property::new(0);
        let elapsed_millis = Property::new(saturating_u128_to_u64(get_elapsed_millis()));
        properties::register_time(&elapsed_frames);
        properties::register_millis(&elapsed_millis);
        Globals {
            elapsed_frames,
            elapsed_millis,
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: viewport_size,
            }),
            gyro: Property::new(Gyro::default()),
            accel: Property::new(Accel::default()),
            route_location: Property::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform,
            os,
            target: TargetInfo::new(platform, os),
            get_elapsed_millis: Rc::from(get_elapsed_millis),
        }
    }

    #[cfg(feature = "designtime")]
    fn build_globals(
        viewport_size: (f64, f64),
        designtime: Rc<RefCell<DesigntimeManager>>,
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
    ) -> Globals {
        use pax_runtime_api::{math::Transform2, properties, Functions};
        Functions::register_all_functions();

        let elapsed_frames = Property::new(0);
        let elapsed_millis = Property::new(saturating_u128_to_u64(get_elapsed_millis()));
        properties::register_time(&elapsed_frames);
        properties::register_millis(&elapsed_millis);
        Globals {
            elapsed_frames,
            elapsed_millis,
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: viewport_size,
            }),
            gyro: Property::new(Gyro::default()),
            accel: Property::new(Accel::default()),
            route_location: Property::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform,
            os,
            target: TargetInfo::new(platform, os),
            designtime: designtime.clone(),
            get_elapsed_millis: Rc::from(get_elapsed_millis),
        }
    }

    #[cfg(feature = "designtime")]
    pub fn new_empty_with_designtime(
        viewport_size: (f64, f64),
        designtime: Rc<RefCell<DesigntimeManager>>,
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
        scroller_tiling_policy: layer_tiling::ScrollerTilingPolicy,
    ) -> Self {
        let globals =
            Self::build_globals(viewport_size, designtime, platform, os, get_elapsed_millis);
        let runtime_context = Rc::new(RuntimeContext::new_empty(globals));
        PaxEngine {
            runtime_context,
            root_expanded_node: None,
            scroller_tiling_policy,
        }
    }

    #[cfg(not(feature = "designtime"))]
    pub fn new_empty(
        viewport_size: (f64, f64),
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
        scroller_tiling_policy: layer_tiling::ScrollerTilingPolicy,
    ) -> Self {
        let globals = Self::build_globals(viewport_size, platform, os, get_elapsed_millis);
        let runtime_context = Rc::new(RuntimeContext::new(globals));
        PaxEngine {
            runtime_context,
            root_expanded_node: None,
            scroller_tiling_policy,
        }
    }

    /// Mount a root component tree into an existing runtime kernel.
    pub fn mount_root_component(
        &mut self,
        main_component_instance: Rc<ComponentInstance>,
    ) -> Rc<ExpandedNode> {
        self.unmount();
        let root_node = ExpandedNode::initialize_root(
            Rc::clone(&main_component_instance),
            &self.runtime_context,
        );
        self.runtime_context.register_root_expanded_node(&root_node);
        self.root_expanded_node = Some(Rc::clone(&root_node));
        root_node
    }

    /// Detach the mounted root component tree, leaving the runtime kernel empty.
    pub fn unmount(&mut self) {
        let Some(root_expanded_node) = self.root_expanded_node.take() else {
            return;
        };

        #[cfg(feature = "designtime")]
        let root_id = root_expanded_node.id;
        root_expanded_node.recurse_unmount(&self.runtime_context);
        self.runtime_context.clear_root_expanded_node();
        self.runtime_context.layer_count.set(0);
        self.runtime_context.clear_layer_scroller_owners();
        self.runtime_context.set_root_scroller_id(None);
        self.runtime_context.clear_visual_viewport_state();

        #[cfg(feature = "designtime")]
        if self
            .runtime_context
            .get_userland_root_expanded_node()
            .as_ref()
            .is_some_and(|node| node.id == root_id)
        {
            self.runtime_context.set_userland_root_expanded_node(None);
        }
    }

    #[cfg(not(feature = "designtime"))]
    pub fn new(
        main_component_instance: Rc<ComponentInstance>,
        viewport_size: (f64, f64),
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
        scroller_tiling_policy: layer_tiling::ScrollerTilingPolicy,
    ) -> Self {
        let mut engine = Self::new_empty(
            viewport_size,
            platform,
            os,
            get_elapsed_millis,
            scroller_tiling_policy,
        );
        engine.mount_root_component(main_component_instance);
        engine
    }

    #[cfg(feature = "designtime")]
    pub fn new_with_designtime(
        userland_main_component_instance: Rc<ComponentInstance>,
        viewport_size: (f64, f64),
        designtime: Rc<RefCell<DesigntimeManager>>,
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
        scroller_tiling_policy: layer_tiling::ScrollerTilingPolicy,
    ) -> Self {
        let mut engine = Self::new_empty_with_designtime(
            viewport_size,
            designtime,
            platform,
            os,
            get_elapsed_millis,
            scroller_tiling_policy,
        );
        engine
            .runtime_context
            .set_userland_root_instance_node(Some(userland_main_component_instance.clone()));
        let root_expanded_node = engine.mount_root_component(userland_main_component_instance);
        engine
            .runtime_context
            .set_userland_root_expanded_node(Some(root_expanded_node));
        engine
    }

    #[cfg(feature = "designtime")]
    pub fn new_with_designer(
        designer_main_component_instance: Rc<ComponentInstance>,
        userland_main_component_instance: Rc<ComponentInstance>,
        viewport_size: (f64, f64),
        designtime: Rc<RefCell<DesigntimeManager>>,
        platform: Platform,
        os: OS,
        get_elapsed_millis: Box<dyn Fn() -> u128>,
        scroller_tiling_policy: layer_tiling::ScrollerTilingPolicy,
    ) -> Self {
        let mut engine = Self::new_empty_with_designtime(
            viewport_size,
            designtime,
            platform,
            os,
            get_elapsed_millis,
            scroller_tiling_policy,
        );
        engine
            .runtime_context
            .set_userland_root_instance_node(Some(userland_main_component_instance));
        engine.mount_root_component(designer_main_component_instance);
        engine
    }

    #[cfg(feature = "designtime")]
    pub fn partial_update_expanded_node(&mut self, new_instance: Rc<dyn InstanceNode>) {
        // update the expanded nodes that just got a new instance node
        let unique_id = new_instance
            .base()
            .template_node_identifier
            .clone()
            .expect("new instance node has unique identifier");

        let nodes = self
            .runtime_context
            .get_expanded_nodes_by_global_ids(&unique_id);
        for node in nodes {
            node.recreate_with_new_data(new_instance.clone(), &self.runtime_context);
        }
    }

    #[cfg(feature = "designtime")]
    pub fn full_reload_userland(&mut self, new_userland_instance: Rc<dyn InstanceNode>) {
        let node = borrow!(self.runtime_context.userland_root_expanded_node)
            .as_ref()
            .map(Rc::clone)
            .unwrap();
        *borrow_mut!(self.runtime_context.userland_frame_instance_node) =
            Some(Rc::clone(&new_userland_instance));
        node.fully_recreate_with_new_data(new_userland_instance.clone(), &self.runtime_context);
    }

    // NOTES: this is the order of different things being computed in recurse-expand-nodes
    // - expanded_node instantiated from instance_node.

    /// Workhorse methods of every tick.  Will be executed up to 240 Hz.
    /// Three phases:
    /// 1. Expand nodes & compute properties; recurse entire instance tree and evaluate ExpandedNodes, stitching
    ///    together parent/child relationships between ExpandedNodes along the way.
    /// 2. Compute layout (z-index & TransformAndBounds) by visiting ExpandedNode tree
    ///    in rendering order, writing computed rendering-specific values to ExpandedNodes
    /// 3. Render:
    ///     a. find lowest node (last child of last node)
    ///     b. start rendering, from lowest node on-up, throughout tree
    pub fn tick(&mut self) -> Vec<NativeMessage> {
        //
        // 1. UPDATE NODES (properties, etc.). This part we should be able to
        // completely remove once reactive properties dirty-dag is a thing.
        //
        if let Some(root_expanded_node) = &self.root_expanded_node {
            root_expanded_node.recurse_update(&self.runtime_context);
            if self.runtime_context.has_import_settings_nodes() {
                root_expanded_node.recurse_sync_import_settings(&self.runtime_context);
            }
        }

        let ctx = &self.runtime_context;
        ctx.drain_node_effects();
        self.run_lifecycle_handlers(TICK_HANDLERS, ctx.tick_handler_nodes());
        ctx.drain_node_effects();
        self.run_lifecycle_handlers(PRE_RENDER_HANDLERS, ctx.pre_render_handler_nodes());
        ctx.drain_node_effects();

        if ctx.take_occlusion_dirty() {
            if let Some(root_expanded_node) = &self.root_expanded_node {
                occlusion::update_node_occlusion(root_expanded_node, ctx);
            }
            ctx.drain_node_effects();
        }
        let globals = ctx.globals();
        let time = &globals.elapsed_frames;
        time.set(time.get() + 1);
        globals
            .elapsed_millis
            .set(saturating_u128_to_u64((globals.get_elapsed_millis)()));

        ctx.flush_custom_events().unwrap();
        let native_messages = ctx.take_native_messages();
        native_messages
    }

    fn run_lifecycle_handlers(&self, handler_key: &str, nodes: Vec<ExpandedNodeIdentifier>) {
        for node_id in nodes {
            if let Some(node) = self.runtime_context.get_expanded_node_by_eid(node_id) {
                node.run_lifecycle_handlers(handler_key, &self.runtime_context);
            }
        }
    }

    fn build_filtered_render_plan(
        &self,
        root: &Rc<ExpandedNode>,
        dirty_node_ids: &[ExpandedNodeIdentifier],
    ) -> Result<FilteredRenderPlan, &'static str> {
        let dirty_nodes: HashSet<_> = dirty_node_ids.iter().copied().collect();
        let mut render_path_nodes = HashSet::new();

        for dirty_node_id in &dirty_nodes {
            let mut node = self
                .runtime_context
                .get_expanded_node_by_eid(*dirty_node_id)
                .ok_or("missing_dirty_node")?;
            let mut local_path = HashSet::new();
            loop {
                if !local_path.insert(node.id) {
                    return Err("render_parent_cycle");
                }
                if node.is_unclippable() {
                    return Err("unclippable_path");
                }
                render_path_nodes.insert(node.id);
                if node.id == root.id {
                    break;
                }
                node = node.render_parent_node().ok_or("missing_render_parent")?;
            }
        }

        if !render_path_nodes.contains(&root.id) {
            return Err("missing_root_path");
        }

        Ok(FilteredRenderPlan {
            dirty_nodes,
            render_path_nodes,
        })
    }

    fn dirty_layer_can_use_targeted_replay_nodes(
        &self,
        layer: usize,
        dirty_node_ids: &[ExpandedNodeIdentifier],
        targeted_node_ids: &HashSet<u32>,
    ) -> bool {
        for dirty_node_id in dirty_node_ids {
            let Some(node) = self
                .runtime_context
                .get_expanded_node_by_eid(*dirty_node_id)
            else {
                return false;
            };
            if node.occlusion.get().render_layer_id != layer
                || borrow!(node.instance_node).base().flags().layer != crate::api::Layer::Canvas
            {
                continue;
            }
            if !targeted_node_ids.contains(&dirty_node_id.to_u32()) {
                return false;
            }
        }
        true
    }

    pub fn render(&mut self, rcs: &mut dyn RenderContext) {
        self.update_layer_count(rcs);

        if !self.runtime_context.has_canvas_render_work() {
            return;
        }

        let removals = self.runtime_context.take_canvas_node_removals();
        let mut dirty_layers = self.runtime_context.dirty_canvas_layers();
        dirty_layers.extend(removals.iter().map(|(layer, _)| *layer));
        dirty_layers.sort_unstable();
        dirty_layers.dedup();
        let removal_layers: HashSet<_> = removals.iter().map(|(layer, _)| *layer).collect();
        let targeted_replay_node_ids = self.runtime_context.take_targeted_canvas_replay_node_ids();
        let dirty_node_ids_before_expansion = self.runtime_context.dirty_canvas_node_ids();
        let retains_canvas_nodes = rcs.retains_canvas_nodes();
        if !retains_canvas_nodes && (!dirty_layers.is_empty() || !removals.is_empty()) {
            for layer in &dirty_layers {
                let can_use_targeted_replay_nodes = targeted_replay_node_ids
                    .get(layer)
                    .is_some_and(|targeted_node_ids| {
                        !removal_layers.contains(layer)
                            && self.dirty_layer_can_use_targeted_replay_nodes(
                                *layer,
                                &dirty_node_ids_before_expansion,
                                targeted_node_ids,
                            )
                    });
                if !can_use_targeted_replay_nodes {
                    self.runtime_context
                        .mark_canvas_nodes_on_layer_dirty(*layer);
                }
            }
        }
        let dirty_node_ids = self.runtime_context.dirty_canvas_node_ids();
        let has_dirty_nodes = !dirty_node_ids.is_empty();
        let has_node_removals = !removals.is_empty();
        if !has_dirty_nodes && !has_node_removals {
            for layer in &dirty_layers {
                rcs.clear(*layer);
            }
        } else if !retains_canvas_nodes {
            for layer in &dirty_layers {
                rcs.clear(*layer);
            }
        }

        let mut failed_removal_layers = Vec::new();
        for (layer, node_id) in removals {
            if !rcs.remove_node(layer, node_id) {
                self.runtime_context
                    .enqueue_canvas_node_removal(layer, node_id);
                failed_removal_layers.push(layer);
            }
        }

        // This is pretty useful during debugging - left it here since I use it often. /Sam
        // crate::api::log(&format!("tree: {:#?}", self.root_node));
        if has_dirty_nodes {
            if let Some(root_expanded_node) = &self.root_expanded_node {
                match self.build_filtered_render_plan(root_expanded_node, &dirty_node_ids) {
                    Ok(plan) => {
                        let mut stats = expanded_node::FilteredRenderStats::default();
                        root_expanded_node.recurse_render_filtered(
                            &self.runtime_context,
                            rcs,
                            &plan.render_path_nodes,
                            &plan.dirty_nodes,
                            &mut stats,
                        );
                        #[cfg(debug_assertions)]
                        log::trace!(
                            "[pax-render-filter] dirty_nodes={} path_nodes={} visited_path_nodes={} dirty_nodes_rendered={} skipped_subtrees={}",
                            plan.dirty_nodes.len(),
                            plan.render_path_nodes.len(),
                            stats.path_nodes_visited,
                            stats.dirty_nodes_rendered,
                            stats.skipped_subtrees,
                        );
                    }
                    Err(reason) => {
                        #[cfg(debug_assertions)]
                        log::trace!(
                            "[pax-render-filter] fallback={} dirty_nodes={}",
                            reason,
                            dirty_node_ids.len(),
                        );
                        root_expanded_node.recurse_render_queue(&self.runtime_context, rcs);
                    }
                }
            }
        }
        self.runtime_context.recurse_flush_queued_renders(rcs);

        self.runtime_context.clear_all_dirty_canvases();
        for layer in failed_removal_layers {
            self.runtime_context.set_canvas_dirty(layer);
        }

        for layer in dirty_layers {
            rcs.flush(layer, Rc::clone(&self.runtime_context.dirty_canvases));
        }
    }

    pub fn get_expanded_node(&self, id: ExpandedNodeIdentifier) -> Option<Rc<ExpandedNode>> {
        let val = self.runtime_context.get_expanded_node_by_eid(id).clone();
        val.map(|v| v.clone())
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        const VIEWPORT_SIZE_EPSILON: f64 = 0.001;
        let current_viewport_size = self.runtime_context.globals().viewport.get().bounds;
        if (current_viewport_size.0 - new_viewport_size.0).abs() <= VIEWPORT_SIZE_EPSILON
            && (current_viewport_size.1 - new_viewport_size.1).abs() <= VIEWPORT_SIZE_EPSILON
        {
            return;
        }

        self.runtime_context.edit_globals(|globals| {
            globals
                .viewport
                .update(|t_and_b| t_and_b.bounds = new_viewport_size);
        });
        self.runtime_context.mark_layer_canvas_plans_dirty();
        self.runtime_context.mark_occlusion_dirty();
    }

    pub fn update_layer_count(&self, rcs: &mut dyn RenderContext) {
        static LAST_LAYER_COUNT: AtomicUsize = AtomicUsize::new(0); // last-patch layer_count
        let curr_layer_count = self.runtime_context.layer_count.get();
        let old_layer_count = LAST_LAYER_COUNT.load(Ordering::Relaxed);
        if old_layer_count != curr_layer_count {
            rcs.resize_layers_to(
                curr_layer_count,
                Rc::clone(&self.runtime_context.dirty_canvases),
            );
            self.runtime_context
                .resize_canvas_layers_to(curr_layer_count);
            LAST_LAYER_COUNT.store(curr_layer_count, Ordering::Relaxed)
        }
    }

    pub fn global_dispatch_focus(&self, args: Focus) -> bool {
        let Some(root_expanded_node) = &self.root_expanded_node else {
            return false;
        };
        let mut prevent_default = false;
        root_expanded_node.recurse_visit_postorder(&mut |expanded_node| {
            prevent_default |= expanded_node.dispatch_focus(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        });
        prevent_default
    }

    pub fn global_dispatch_select_start(&self, args: SelectStart) -> bool {
        let Some(root_expanded_node) = &self.root_expanded_node else {
            return false;
        };
        let mut prevent_default = false;
        root_expanded_node.recurse_visit_postorder(&mut |expanded_node| {
            prevent_default |= expanded_node.dispatch_select_start(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        });
        prevent_default
    }

    pub fn global_dispatch_key_down(&self, args: KeyDown) -> bool {
        let Some(root_expanded_node) = &self.root_expanded_node else {
            return false;
        };
        let mut prevent_default = false;
        root_expanded_node.recurse_visit_postorder(&mut |expanded_node| {
            prevent_default |= expanded_node.dispatch_key_down(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        });
        prevent_default
    }

    pub fn global_dispatch_key_up(&self, args: KeyUp) -> bool {
        let Some(root_expanded_node) = &self.root_expanded_node else {
            return false;
        };
        let mut prevent_default = false;
        root_expanded_node.recurse_visit_postorder(&mut |expanded_node| {
            prevent_default |= expanded_node.dispatch_key_up(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        });
        prevent_default
    }

    pub fn global_dispatch_key_press(&self, args: KeyPress) -> bool {
        let Some(root_expanded_node) = &self.root_expanded_node else {
            return false;
        };
        let mut prevent_default = false;
        root_expanded_node.recurse_visit_postorder(&mut |expanded_node| {
            prevent_default |= expanded_node.dispatch_key_press(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        });
        prevent_default
    }

    pub fn global_dispatch_gyro(&self, args: Gyro) -> bool {
        let mut prevent_default = false;
        for node_id in self.runtime_context.gyro_handler_nodes() {
            let Some(expanded_node) = self.runtime_context.get_expanded_node_by_eid(node_id) else {
                continue;
            };
            prevent_default |= expanded_node.dispatch_gyro(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        }
        prevent_default
    }

    pub fn global_dispatch_accel(&self, args: Accel) -> bool {
        let mut prevent_default = false;
        for node_id in self.runtime_context.accel_handler_nodes() {
            let Some(expanded_node) = self.runtime_context.get_expanded_node_by_eid(node_id) else {
                continue;
            };
            prevent_default |= expanded_node.dispatch_accel(
                Event::new(args.clone()),
                &self.runtime_context.globals(),
                &self.runtime_context,
            );
        }
        prevent_default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{ACCEL_HANDLERS, GYRO_HANDLERS, PRE_RENDER_HANDLERS, TICK_HANDLERS};
    use crate::{InstanceNode, InstantiationArgs};
    use pax_runtime_api::pax_value::{PaxAny, PaxValue};
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TICK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static PRE_RENDER_CALLS: AtomicUsize = AtomicUsize::new(0);
    static GYRO_CALLS: AtomicUsize = AtomicUsize::new(0);
    static ACCEL_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn count_tick_handler(
        _properties: Rc<RefCell<PaxAny>>,
        _ctx: &NodeContext,
        _event: Option<PaxAny>,
    ) {
        TICK_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn count_pre_render_handler(
        _properties: Rc<RefCell<PaxAny>>,
        _ctx: &NodeContext,
        _event: Option<PaxAny>,
    ) {
        PRE_RENDER_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn count_gyro_handler(
        _properties: Rc<RefCell<PaxAny>>,
        _ctx: &NodeContext,
        _event: Option<PaxAny>,
    ) {
        GYRO_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn count_accel_handler(
        _properties: Rc<RefCell<PaxAny>>,
        _ctx: &NodeContext,
        _event: Option<PaxAny>,
    ) {
        ACCEL_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn empty_component() -> Rc<ComponentInstance> {
        <ComponentInstance as InstanceNode>::instantiate(InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
            })),
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        })
    }

    fn component_with_lifecycle_handlers() -> Rc<ComponentInstance> {
        let mut registry = HandlerRegistry::default();
        registry.handlers.insert(
            TICK_HANDLERS.to_string(),
            vec![Handler::new_inline_handler(count_tick_handler)],
        );
        registry.handlers.insert(
            PRE_RENDER_HANDLERS.to_string(),
            vec![Handler::new_inline_handler(count_pre_render_handler)],
        );

        <ComponentInstance as InstanceNode>::instantiate(InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
            })),
            handler_registry: Some(Rc::new(RefCell::new(registry))),
            children: None,
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        })
    }

    fn component_with_sensor_handlers() -> Rc<ComponentInstance> {
        let mut registry = HandlerRegistry::default();
        registry.handlers.insert(
            GYRO_HANDLERS.to_string(),
            vec![Handler::new_inline_handler(count_gyro_handler)],
        );
        registry.handlers.insert(
            ACCEL_HANDLERS.to_string(),
            vec![Handler::new_inline_handler(count_accel_handler)],
        );

        <ComponentInstance as InstanceNode>::instantiate(InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
            })),
            handler_registry: Some(Rc::new(RefCell::new(registry))),
            children: None,
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        })
    }

    fn object_field(value: PaxValue, field: &str) -> PaxValue {
        let PaxValue::Object(fields) = value else {
            panic!("expected object value");
        };
        fields
            .into_iter()
            .find_map(|(name, value)| (name == field).then_some(value))
            .unwrap_or_else(|| panic!("expected object field {field}"))
    }

    #[test]
    fn empty_engine_can_tick_and_mount_later() {
        let mut engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );

        assert!(engine.root_expanded_node.is_none());
        assert!(engine.runtime_context.get_root_expanded_node().is_none());
        assert!(engine.tick().is_empty());

        let root = engine.mount_root_component(empty_component());
        assert_eq!(
            engine.root_expanded_node.as_ref().map(|node| node.id),
            Some(root.id)
        );
        assert_eq!(
            engine
                .runtime_context
                .get_root_expanded_node()
                .map(|node| node.id),
            Some(root.id)
        );
        let _ = engine.tick();
    }

    #[test]
    fn unmount_clears_registered_root() {
        let mut engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );

        engine.mount_root_component(empty_component());
        engine.unmount();

        assert!(engine.root_expanded_node.is_none());
        assert!(engine.runtime_context.get_root_expanded_node().is_none());
    }

    #[test]
    fn globals_expose_sensor_values_to_stack_frame() {
        let engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );
        let globals = engine.runtime_context.globals();
        globals.gyro.set(Gyro {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });
        globals.accel.set(Accel {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        });

        assert_eq!(
            globals
                .stack_frame()
                .resolve_symbol("$gyro")
                .unwrap()
                .get_as_pax_value(),
            PaxValue::Object(
                vec![
                    ("x".to_string(), PaxValue::Numeric(1.0.into())),
                    ("y".to_string(), PaxValue::Numeric(2.0.into())),
                    ("z".to_string(), PaxValue::Numeric(3.0.into())),
                ]
                .into_iter()
                .collect(),
            )
        );
        assert_eq!(
            globals
                .stack_frame()
                .resolve_symbol("$accel")
                .unwrap()
                .get_as_pax_value(),
            PaxValue::Object(
                vec![
                    ("x".to_string(), PaxValue::Numeric(4.0.into())),
                    ("y".to_string(), PaxValue::Numeric(5.0.into())),
                    ("z".to_string(), PaxValue::Numeric(6.0.into())),
                ]
                .into_iter()
                .collect(),
            )
        );
    }

    #[test]
    fn lifecycle_handlers_run_once_per_tick() {
        TICK_CALLS.store(0, Ordering::SeqCst);
        PRE_RENDER_CALLS.store(0, Ordering::SeqCst);
        let mut engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );

        engine.mount_root_component(component_with_lifecycle_handlers());
        let _ = engine.tick();

        assert_eq!(TICK_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(PRE_RENDER_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sensor_handlers_dispatch_through_registered_nodes() {
        GYRO_CALLS.store(0, Ordering::SeqCst);
        ACCEL_CALLS.store(0, Ordering::SeqCst);
        let mut engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );

        assert!(!engine.global_dispatch_gyro(Gyro::default()));
        assert!(!engine.global_dispatch_accel(Accel::default()));
        engine.mount_root_component(component_with_sensor_handlers());
        assert!(!engine.global_dispatch_gyro(Gyro::default()));
        assert!(!engine.global_dispatch_accel(Accel::default()));

        assert_eq!(GYRO_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(ACCEL_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn globals_expose_clock_aliases_to_stack_frame() {
        let engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 123),
            layer_tiling::ScrollerTilingPolicy::default(),
        );
        let globals = engine.runtime_context.globals();
        let stack = globals.stack_frame();

        assert_eq!(
            stack.resolve_symbol("$frames").unwrap().get_as_pax_value(),
            PaxValue::Numeric(0.into())
        );
        assert_eq!(
            stack.resolve_symbol("$millis").unwrap().get_as_pax_value(),
            PaxValue::Numeric(123.into())
        );
        let old_frames_symbol = format!("{}{}", "$frames", "_elapsed");
        let old_millis_symbol = format!("{}{}", "$elapsed", "_millis");
        assert!(stack.resolve_symbol(&old_frames_symbol).is_none());
        assert!(stack.resolve_symbol(&old_millis_symbol).is_none());
    }

    #[test]
    fn globals_expose_target_aliases_to_stack_frame() {
        let engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Native,
            OS::IPad,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );
        let globals = engine.runtime_context.globals();
        let stack = globals.stack_frame();

        assert_eq!(
            stack.resolve_symbol("$ios").unwrap().get_as_pax_value(),
            PaxValue::Bool(true)
        );
        assert_eq!(
            stack.resolve_symbol("$ipad").unwrap().get_as_pax_value(),
            PaxValue::Bool(true)
        );
        assert_eq!(
            stack.resolve_symbol("$iphone").unwrap().get_as_pax_value(),
            PaxValue::Bool(false)
        );
        assert_eq!(
            stack.resolve_symbol("$native").unwrap().get_as_pax_value(),
            PaxValue::Bool(true)
        );
        assert_eq!(
            object_field(
                stack.resolve_symbol("$target").unwrap().get_as_pax_value(),
                "ipad"
            ),
            PaxValue::Bool(true)
        );
    }

    #[test]
    fn globals_expose_viewport_orientation_aliases_to_stack_frame() {
        let engine = PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            layer_tiling::ScrollerTilingPolicy::default(),
        );
        let globals = engine.runtime_context.globals();
        let stack = globals.stack_frame();

        assert_eq!(
            stack
                .resolve_symbol("$landscape")
                .unwrap()
                .get_as_pax_value(),
            PaxValue::Bool(true)
        );
        assert_eq!(
            stack.resolve_symbol("$major").unwrap().get_as_pax_value(),
            PaxValue::Numeric(320.0.into())
        );
        assert_eq!(
            object_field(
                stack
                    .resolve_symbol("$viewport")
                    .unwrap()
                    .get_as_pax_value(),
                "minor"
            ),
            PaxValue::Numeric(240.0.into())
        );

        globals.viewport.update(|viewport| {
            viewport.bounds = (240.0, 320.0);
        });

        assert_eq!(
            stack
                .resolve_symbol("$portrait")
                .unwrap()
                .get_as_pax_value(),
            PaxValue::Bool(true)
        );
        assert_eq!(
            stack.resolve_symbol("$minor").unwrap().get_as_pax_value(),
            PaxValue::Numeric(240.0.into())
        );
    }
}
