use crate::constants::{PRE_RENDER_HANDLERS, TICK_HANDLERS};
use crate::{
    api::Property, ExpandedNodeIdentifier, RuntimePropertiesStackFrame, TransformAndBounds,
};
use_RefCell!();
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use pax_message::NativeMessage;
use pax_runtime_api::{
    pax_value::PaxAny, use_RefCell, Event, Focus, SelectStart, Variable, Window, OS,
};

use crate::api::{KeyDown, KeyPress, KeyUp, NodeContext, RenderContext};

use crate::{ComponentInstance, RuntimeContext};
use pax_runtime_api::Platform;

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
    RuntimeSettingsLayer, RuntimeSettingsSignatureEntry, RuntimeSettingsSource,
};

use self::node_interface::NodeLocal;

#[cfg(feature = "designtime")]
use {
    crate::InstanceNode,
    pax_designtime::DesigntimeManager,
    pax_runtime_api::{borrow, borrow_mut},
};

#[derive(Clone)]
/// Engine-wide reactive globals exposed to every component frame.
pub struct Globals {
    pub frames_elapsed: Property<u64>,
    pub viewport: Property<TransformAndBounds<NodeLocal, Window>>,
    pub browser_allows_scroller_vector_layers: Property<bool>,
    pub browser_allows_nested_scroller_vector_layers: Property<bool>,
    pub platform: Platform,
    pub os: OS,
    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
    pub get_elapsed_millis: Rc<dyn Fn() -> u128>,
}

impl Globals {
    /// Build the root stack frame containing `$mobile`, `$desktop`, `$viewport`, and `$frames_elapsed`.
    pub fn stack_frame(&self) -> Rc<RuntimePropertiesStackFrame> {
        let mobile = Property::new(self.os.is_mobile());
        let desktop = Property::new(self.os.is_desktop());

        let cloned_viewport = self.viewport.clone();
        let deps = [cloned_viewport.untyped()];
        let viewport = Property::computed(
            move || {
                let viewport = cloned_viewport.get();
                pax_runtime_api::Viewport {
                    width: viewport.bounds.0,
                    height: viewport.bounds.1,
                }
            },
            &deps,
        );

        let mobile_var = Variable::new_from_typed_property(mobile);
        let desktop_var = Variable::new_from_typed_property(desktop);
        let viewport_var = Variable::new_from_typed_property(viewport);
        let frames_elapsed_var = Variable::new_from_typed_property(self.frames_elapsed.clone());

        let global_scope = vec![
            ("$mobile".to_string(), mobile_var),
            ("$desktop".to_string(), desktop_var),
            ("$viewport".to_string(), viewport_var),
            ("$frames_elapsed".to_string(), frames_elapsed_var),
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
            .field("frames_elapsed", &self.frames_elapsed)
            .field("viewport", &self.viewport)
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

        let frames_elapsed = Property::new(0);
        properties::register_time(&frames_elapsed);
        Globals {
            frames_elapsed,
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: viewport_size,
            }),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform,
            os,
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

        let frames_elapsed = Property::new(0);
        properties::register_time(&frames_elapsed);
        Globals {
            frames_elapsed,
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: viewport_size,
            }),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform,
            os,
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
            root_expanded_node.recurse_sync_import_settings(&self.runtime_context);
        }

        let ctx = &self.runtime_context;
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
        let time = &ctx.globals().frames_elapsed;
        time.set(time.get() + 1);

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
        let has_dirty_nodes = self.runtime_context.has_dirty_canvas_nodes();
        let has_node_removals = !removals.is_empty();
        if !has_dirty_nodes && !has_node_removals {
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
        if let Some(root_expanded_node) = &self.root_expanded_node {
            root_expanded_node.recurse_render_queue(&self.runtime_context, rcs);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InstanceNode, InstantiationArgs};
    use pax_runtime_api::pax_value::{PaxAny, PaxValue};
    use std::cell::RefCell;

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
}
