use crate::{api::Property, ExpandedNodeIdentifier};
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use kurbo::Affine;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::{NativeMessage, OcclusionPatch};
use pax_runtime_api::{math::Transform2, pax_value::PaxAny, OS};

use crate::api::{KeyDown, KeyPress, KeyUp, Layer, NodeContext, OcclusionLayerGen, RenderContext};
use piet::InterpolationMode;

use crate::{
    ComponentInstance, ExpressionContext, InstanceNode, RuntimeContext, RuntimePropertiesStackFrame,
};
use pax_runtime_api::Platform;

pub mod node_interface;

/// The atomic unit of rendering; also the container for each unique tuple of computed properties.
/// Represents an expanded node, that is "expanded" in the context of computed properties and repeat expansion.
/// For example, a Rectangle inside `for i in 0..3` and a `for j in 0..4` would have 12 expanded nodes representing the 12 virtual Rectangles in the
/// rendered scene graph.
/// `ExpandedNode`s are architecturally "type-blind" — while they store typed data e.g. inside `computed_properties` and `computed_common_properties`,
/// they require coordinating with their "type-aware" [`InstanceNode`] to perform operations on those properties.
mod expanded_node;
pub use expanded_node::ExpandedNode;

#[cfg(feature = "designtime")]
use {
    self::node_interface::NodeLocal,
    pax_designtime::DesigntimeManager,
    pax_runtime_api::{properties, Window},
};

use self::expanded_node::LayoutProperties;

#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Globals {
    pub frames_elapsed: Property<u64>,
    pub viewport: LayoutProperties,
    pub platform: Platform,
    pub os: OS,
    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
}

/// Singleton struct storing everything related to properties computation & rendering
pub struct PaxEngine {
    pub runtime_context: Rc<RefCell<RuntimeContext>>,
    pub root_node: Rc<ExpandedNode>,
    main_component_instance: Rc<ComponentInstance>,
}

pub enum HandlerLocation {
    Inline,
    Component,
}

pub struct Handler {
    pub function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    pub location: HandlerLocation,
}

impl Handler {
    pub fn new_inline_handler(
        function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    ) -> Self {
        Handler {
            function,
            location: HandlerLocation::Inline,
        }
    }

    pub fn new_component_handler(
        function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    ) -> Self {
        Handler {
            function,
            location: HandlerLocation::Component,
        }
    }
}

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

pub struct Renderer<R: piet::RenderContext> {
    pub backends: HashMap<String, R>,
    pub image_map: HashMap<String, R::Image>,
}

impl<R: piet::RenderContext> Renderer<R> {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
            image_map: HashMap::new(),
        }
    }

    pub fn add_context(&mut self, id: &str, context: R) {
        self.backends.insert(id.to_owned(), context);
    }

    pub fn remove_context(&mut self, id: &str) {
        self.backends.remove(id);
    }

    pub fn image_loaded(&self, path: &str) -> bool {
        self.image_map.contains_key(path)
    }
}

impl<R: piet::RenderContext> crate::api::RenderContext for Renderer<R> {
    fn fill(&mut self, layer: &str, path: kurbo::BezPath, brush: &piet_common::PaintBrush) {
        self.backends.get_mut(layer).unwrap().fill(path, brush);
    }

    fn stroke(
        &mut self,
        layer: &str,
        path: kurbo::BezPath,
        brush: &piet_common::PaintBrush,
        width: f64,
    ) {
        self.backends
            .get_mut(layer)
            .unwrap()
            .stroke(path, brush, width);
    }

    fn save(&mut self, layer: &str) {
        self.backends
            .get_mut(layer)
            .unwrap()
            .save()
            .expect("failed to save piet state");
    }

    fn transform(&mut self, layer: &str, affine: Affine) {
        self.backends.get_mut(layer).unwrap().transform(affine);
    }

    fn clip(&mut self, layer: &str, path: kurbo::BezPath) {
        self.backends.get_mut(layer).unwrap().clip(path);
    }

    fn restore(&mut self, layer: &str) {
        self.backends
            .get_mut(layer)
            .unwrap()
            .restore()
            .expect("failed to restore piet state");
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        //is this okay!? we know it's the same kind of backend no matter what layer, but it might be storing data?
        let render_context = self.backends.values_mut().next().unwrap();
        let img = render_context
            .make_image(width, height, buf, piet::ImageFormat::RgbaSeparate)
            .expect("image creation successful");
        self.image_map.insert(path.to_owned(), img);
    }

    fn draw_image(&mut self, layer: &str, image_path: &str, rect: kurbo::Rect) {
        let Some(img) = self.image_map.get(image_path) else {
            return;
        };
        self.backends
            .get_mut(layer)
            .unwrap()
            .draw_image(img, rect, InterpolationMode::Bilinear);
    }

    fn layers(&self) -> Vec<&str> {
        self.backends.keys().map(String::as_str).collect()
    }
}

pub struct ExpressionTable {
    pub table: HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>>,
}

#[cfg(debug_assertions)]
impl std::fmt::Debug for ExpressionTable {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

impl ExpressionTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn compute_vtable_value(
        &self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        vtable_id: usize,
    ) -> PaxAny {
        if let Some(evaluator) = self.table.get(&vtable_id) {
            let stack_frame = Rc::clone(stack);
            let ec = ExpressionContext { stack_frame };
            (**evaluator)(ec)
        } else {
            panic!() //unhandled error if an invalid id is passed or if vtable is incorrectly initialized
        }
    }
}

/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl PaxEngine {
    #[cfg(not(feature = "designtime"))]
    pub fn new(
        main_component_instance: Rc<ComponentInstance>,
        expression_table: ExpressionTable,
        viewport_size: (f64, f64),
        platform: Platform,
        os: OS,
    ) -> Self {
        use pax_runtime_api::properties;

        let frames_elapsed = Property::new(0);
        properties::register_time(&frames_elapsed);
        let globals = Globals {
            frames_elapsed,
            viewport: LayoutProperties {
                transform: Property::new(Transform2::identity()),
                bounds: Property::new(viewport_size),
            },
            platform,
            os,
        };

        let runtime_context = Rc::new(RefCell::new(RuntimeContext::new(expression_table, globals)));

        let root_node = ExpandedNode::root(Rc::clone(&main_component_instance), &runtime_context);

        PaxEngine {
            runtime_context,
            root_node,
            main_component_instance,
        }
    }

    #[cfg(feature = "designtime")]
    pub fn new_with_designtime(
        main_component_instance: Rc<ComponentInstance>,
        expression_table: ExpressionTable,
        viewport_size: (f64, f64),
        designtime: Rc<RefCell<DesigntimeManager>>,
        platform: Platform,
        os: OS,
    ) -> Self {
        use pax_runtime_api::math::Transform2;
        let frames_elapsed = Property::new(0);
        properties::register_time(&frames_elapsed);
        let globals = Globals {
            frames_elapsed,
            viewport: LayoutProperties {
                transform: Property::new(Transform2::identity()),
                bounds: Property::new(viewport_size),
            },
            platform,
            os,
            designtime: designtime.clone(),
        };

        let mut runtime_context =
            Rc::new(RefCell::new(RuntimeContext::new(expression_table, globals)));

        let root_node =
            ExpandedNode::root(Rc::clone(&main_component_instance), &mut runtime_context);

        PaxEngine {
            runtime_context,
            root_node,
            main_component_instance,
        }
    }

    /// Replace an instance node in the main component's template
    pub fn replace_main_template_instance_node(&mut self, new_instance: Rc<dyn InstanceNode>) {
        for temp in self.main_component_instance.template.borrow().iter() {
            replace_instance_node_at(&temp, &new_instance);
        }

        fn replace_instance_node_at(
            parent: &Rc<dyn InstanceNode>,
            new_instance: &Rc<dyn InstanceNode>,
        ) {
            let mut instance_nodes = parent.base().get_instance_children().borrow_mut();
            for node in instance_nodes.iter_mut() {
                if node.base().template_node_identifier
                    == new_instance.base().template_node_identifier
                {
                    *node = Rc::clone(&new_instance);
                } else {
                    replace_instance_node_at(node, new_instance)
                }
            }
        }
    }

    pub fn remount_main_template_expanded_node(&mut self, new_instance: Rc<dyn InstanceNode>) {
        let unique_id = new_instance
            .base()
            .template_node_identifier
            .clone()
            .expect("new instance node has unique identifier");
        Self::recurse_remount_main_template_expanded_node(
            &self.root_node,
            &unique_id,
            &mut self.runtime_context,
        );
    }

    /// Remounts an expanded node (&siblings) in the main component's template
    pub fn recurse_remount_main_template_expanded_node(
        parent: &Rc<ExpandedNode>,
        id: &UniqueTemplateNodeIdentifier,
        ctx: &Rc<RefCell<RuntimeContext>>,
    ) {
        if parent.children.get().iter().any(|node| {
            node.instance_node
                .borrow()
                .base()
                .template_node_identifier
                .as_ref()
                .is_some_and(|i| i == id)
        }) {
            // OBS: HACK: this is not general, works for non-for loop/if nodes only
            // to do more generally, split expanded_node.update into prop updates and
            // regen of children steps
            let env = Rc::clone(&parent.stack);
            let parent_template = parent.instance_node.borrow();
            let children = parent_template.base().get_instance_children().borrow();
            let new_templates = children.clone().into_iter().zip(iter::repeat(env));
            parent.generate_children(new_templates, ctx);
            parent.recurse_update(ctx);
        } else {
            for child in parent.children.get().iter() {
                Self::recurse_remount_main_template_expanded_node(child, id, ctx);
            }
        }
    }

    pub fn partial_update_expanded_node(&mut self, new_instance: Rc<dyn InstanceNode>) {
        // update the expanded nodes that just got a new instance node
        let unique_id = new_instance
            .base()
            .template_node_identifier
            .clone()
            .expect("new instance node has unique identifier");

        let nodes = self
            .runtime_context
            .borrow()
            .get_expanded_nodes_by_global_ids(&unique_id);
        for node in nodes {
            node.recreate_with_new_data(
                new_instance.clone(),
                self.runtime_context.borrow().expression_table(),
            );
        }
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
        self.root_node.recurse_update(&mut self.runtime_context);

        // 2. LAYER-IDS, z-index list creation Will always be recomputed each
        // frame. Nothing intensive is to be done here.
        {
            self.runtime_context.borrow_mut().z_index_node_cache.clear();
            fn assign_z_indicies(n: &Rc<ExpandedNode>, state: &mut Vec<Rc<ExpandedNode>>) {
                state.push(Rc::clone(&n));
            }

            self.root_node.recurse_visit_postorder(
                &assign_z_indicies,
                &mut self.runtime_context.borrow_mut().z_index_node_cache,
            );
        }

        // Occlusion
        let mut occlusion_ind = OcclusionLayerGen::new(None);
        let cache = self.runtime_context.borrow().z_index_node_cache.clone();
        for node in cache.iter() {
            let layer = node.instance_node.borrow().base().flags().layer;
            occlusion_ind.update_z_index(layer);
            let new_occlusion_ind = occlusion_ind.get_level();
            let mut curr_occlusion_ind = node.occlusion_id.borrow_mut();
            if layer == Layer::Native && *curr_occlusion_ind != new_occlusion_ind {
                self.runtime_context.borrow_mut().enqueue_native_message(
                    pax_message::NativeMessage::OcclusionUpdate(OcclusionPatch {
                        id: node.id.to_u32(),
                        occlusion_layer_id: new_occlusion_ind,
                    }),
                );
            }
            *curr_occlusion_ind = new_occlusion_ind;
        }

        let mut ctx = self.runtime_context.borrow_mut();
        let time = &ctx.globals().frames_elapsed;

        time.set(time.get() + 1);

        ctx.take_native_messages()
    }

    pub fn render(&mut self, rcs: &mut dyn RenderContext) {
        // This is pretty useful during debugging - left it here since I use it often. /Sam
        // crate::api::log(&format!("tree: {:#?}", self.root_node));

        self.root_node
            .recurse_render(&mut self.runtime_context, rcs);
    }

    pub fn get_expanded_node(&self, id: ExpandedNodeIdentifier) -> Option<Rc<ExpandedNode>> {
        let binding = self.runtime_context.borrow();
        let val = binding.node_cache.get(&id).clone();
        val.map(|v| (v.clone()))
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.runtime_context
            .borrow_mut()
            .globals_mut()
            .viewport
            .bounds
            .set(new_viewport_size);
    }

    pub fn global_dispatch_key_down(&self, args: KeyDown) {
        self.root_node.recurse_visit_postorder(
            &|expanded_node, _| {
                expanded_node.dispatch_key_down(
                    args.clone(),
                    self.runtime_context.borrow().globals(),
                    &self.runtime_context,
                );
            },
            &mut (),
        );
    }

    pub fn global_dispatch_key_up(&self, args: KeyUp) {
        self.root_node.recurse_visit_postorder(
            &|expanded_node, _| {
                expanded_node.dispatch_key_up(
                    args.clone(),
                    self.runtime_context.borrow().globals(),
                    &self.runtime_context,
                );
            },
            &mut (),
        );
    }

    pub fn global_dispatch_key_press(&self, args: KeyPress) {
        self.root_node.recurse_visit_postorder(
            &|expanded_node, _| {
                expanded_node.dispatch_key_press(
                    args.clone(),
                    self.runtime_context.borrow().globals(),
                    &self.runtime_context,
                );
            },
            &mut (),
        );
    }
}
