use crate::{api::Property, ExpandedNodeIdentifier, TransformAndBounds};
use_RefCell!();
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::Affine;
use pax_message::NativeMessage;
use pax_runtime_api::{pax_value::PaxAny, use_RefCell, Event, Window, OS};

use crate::api::{KeyDown, KeyPress, KeyUp, NodeContext, RenderContext};
use piet::InterpolationMode;

use crate::{ComponentInstance, RuntimeContext};
use pax_runtime_api::Platform;

pub mod node_interface;
pub mod occlusion;

/// The atomic unit of rendering; also the container for each unique tuple of computed properties.
/// Represents an expanded node, that is "expanded" in the context of computed properties and repeat expansion.
/// For example, a Rectangle inside `for i in 0..3` and a `for j in 0..4` would have 12 expanded nodes representing the 12 virtual Rectangles in the
/// rendered scene graph.
/// `ExpandedNode`s are architecturally "type-blind" — while they store typed data e.g. inside `computed_properties` and `computed_common_properties`,
/// they require coordinating with their "type-aware" [`InstanceNode`] to perform operations on those properties.
mod expanded_node;
pub use expanded_node::ExpandedNode;

use self::node_interface::NodeLocal;

#[cfg(feature = "designtime")]
use {
    crate::InstanceNode,
    pax_designtime::DesigntimeManager,
    pax_runtime_api::{borrow, borrow_mut},
};

#[derive(Clone)]
pub struct Globals {
    pub frames_elapsed: Property<u64>,
    pub viewport: Property<TransformAndBounds<NodeLocal, Window>>,
    pub platform: Platform,
    pub os: OS,
    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
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
    pub root_expanded_node: Rc<ExpandedNode>,
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

struct ImgData<R: piet::RenderContext> {
    img: R::Image,
    size: (usize, usize),
}

pub struct Renderer<R: piet::RenderContext> {
    backends: HashMap<String, R>,
    image_map: HashMap<String, ImgData<R>>,
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
        if let Some(layer) = self.backends.get_mut(layer) {
            layer.fill(path, brush);
        }
    }

    fn stroke(
        &mut self,
        layer: &str,
        path: kurbo::BezPath,
        brush: &piet_common::PaintBrush,
        width: f64,
    ) {
        if let Some(layer) = self.backends.get_mut(layer) {
            layer.stroke(path, brush, width);
        }
    }

    fn save(&mut self, layer: &str) {
        if let Some(layer) = self.backends.get_mut(layer) {
            let _ = layer.save();
        }
    }

    fn transform(&mut self, layer: &str, affine: Affine) {
        if let Some(layer) = self.backends.get_mut(layer) {
            layer.transform(affine);
        }
    }

    fn clip(&mut self, layer: &str, path: kurbo::BezPath) {
        if let Some(layer) = self.backends.get_mut(layer) {
            layer.clip(path);
        }
    }

    fn restore(&mut self, layer: &str) {
        if let Some(layer) = self.backends.get_mut(layer) {
            let _ = layer.restore();
        }
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        //is this okay!? we know it's the same kind of backend no matter what layer, but it might be storing data?
        let render_context = self.backends.values_mut().next().unwrap();
        let img = render_context
            .make_image(width, height, buf, piet::ImageFormat::RgbaSeparate)
            .expect("image creation successful");
        self.image_map.insert(
            path.to_owned(),
            ImgData {
                img,
                size: (width, height),
            },
        );
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map.get(image_path).map(|img| (img.size))
    }

    fn draw_image(&mut self, layer: &str, image_path: &str, rect: kurbo::Rect) {
        let Some(data) = self.image_map.get(image_path) else {
            return;
        };
        if let Some(layer) = self.backends.get_mut(layer) {
            layer.draw_image(&data.img, rect, InterpolationMode::Bilinear);
        }
    }

    fn layers(&self) -> Vec<&str> {
        self.backends.keys().map(String::as_str).collect()
    }
}

/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl PaxEngine {
    #[cfg(not(feature = "designtime"))]
    pub fn new(
        main_component_instance: Rc<ComponentInstance>,
        viewport_size: (f64, f64),
        platform: Platform,
        os: OS,
    ) -> Self {
        use crate::api::math::Transform2;
        use pax_runtime_api::{properties, Functions};
        Functions::register_all_functions();

        let frames_elapsed = Property::new(0);
        properties::register_time(&frames_elapsed);
        let globals = Globals {
            frames_elapsed,
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: viewport_size,
            }),
            platform,
            os,
        };
        let runtime_context = Rc::new(RuntimeContext::new(globals));
        let root_node =
            ExpandedNode::initialize_root(Rc::clone(&main_component_instance), &runtime_context);
        runtime_context.register_root_expanded_node(&root_node);

        PaxEngine {
            runtime_context,
            root_expanded_node: root_node,
        }
    }

    #[cfg(feature = "designtime")]
    pub fn new_with_designtime(
        designer_main_component_instance: Rc<ComponentInstance>,
        userland_main_component_instance: Rc<ComponentInstance>,
        viewport_size: (f64, f64),
        designtime: Rc<RefCell<DesigntimeManager>>,
        platform: Platform,
        os: OS,
    ) -> Self {
        use pax_runtime_api::{math::Transform2, properties, Functions};
        Functions::register_all_functions();
        let frames_elapsed = Property::new(0);
        properties::register_time(&frames_elapsed);
        let globals = Globals {
            frames_elapsed,
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: viewport_size,
            }),
            platform,
            os,
            designtime: designtime.clone(),
        };

        //Must register userland node first, because this will mount component trees (calling .mount)
        //Because InlineFrame's mount logic assumes that the "iframe" component is already registered and available
        //on runtime context, it must first be registered (here)
        let mut runtime_context = Rc::new(RuntimeContext::new(
            globals,
            userland_main_component_instance,
        ));

        let root_expanded_node = ExpandedNode::initialize_root(
            Rc::clone(&designer_main_component_instance),
            &mut runtime_context,
        );
        runtime_context.register_root_expanded_node(&root_expanded_node);

        PaxEngine {
            runtime_context,
            root_expanded_node,
        }
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
            if new_instance.base().template_node_identifier
                == borrow!(self.runtime_context.userland_frame_instance_node)
                    .base()
                    .template_node_identifier
            {
                *borrow_mut!(self.runtime_context.userland_frame_instance_node) =
                    Rc::clone(&new_instance);
            }
            node.recreate_with_new_data(new_instance.clone(), &self.runtime_context);
        }
    }

    #[cfg(feature = "designtime")]
    pub fn full_reload_userland(&mut self, new_userland_instance: Rc<dyn InstanceNode>) {
        let node = borrow!(self.runtime_context.userland_root_expanded_node)
            .as_ref()
            .map(Rc::clone)
            .unwrap();

        Rc::clone(&node).recurse_unmount(&self.runtime_context);
        node.recreate_with_new_data(new_userland_instance.clone(), &self.runtime_context);
        Rc::clone(&node).recurse_mount(&self.runtime_context);
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
        self.root_expanded_node
            .recurse_update(&mut self.runtime_context);

        let ctx = &self.runtime_context;
        occlusion::update_node_occlusion(&self.root_expanded_node, ctx);
        let time = &ctx.globals().frames_elapsed;
        time.set(time.get() + 1);

        ctx.flush_custom_events().unwrap();
        ctx.take_native_messages()
    }

    pub fn render(&mut self, rcs: &mut dyn RenderContext) {
        // This is pretty useful during debugging - left it here since I use it often. /Sam
        // crate::api::log(&format!("tree: {:#?}", self.root_node));
        self.root_expanded_node
            .recurse_render_queue(&mut self.runtime_context, rcs);
        self.runtime_context.recurse_flush_queued_renders(rcs);
    }

    pub fn get_expanded_node(&self, id: ExpandedNodeIdentifier) -> Option<Rc<ExpandedNode>> {
        let val = self.runtime_context.get_expanded_node_by_eid(id).clone();
        val.map(|v| (v.clone()))
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.runtime_context.edit_globals(|globals| {
            globals
                .viewport
                .update(|t_and_b| t_and_b.bounds = new_viewport_size);
        });
    }

    pub fn global_dispatch_key_down(&self, args: KeyDown) -> bool {
        let mut prevent_default = false;
        self.root_expanded_node
            .recurse_visit_postorder(&mut |expanded_node| {
                prevent_default |= expanded_node.dispatch_key_down(
                    Event::new(args.clone()),
                    &self.runtime_context.globals(),
                    &self.runtime_context,
                );
            });
        prevent_default
    }

    pub fn global_dispatch_key_up(&self, args: KeyUp) -> bool {
        let mut prevent_default = false;
        self.root_expanded_node
            .recurse_visit_postorder(&mut |expanded_node| {
                prevent_default |= expanded_node.dispatch_key_up(
                    Event::new(args.clone()),
                    &self.runtime_context.globals(),
                    &self.runtime_context,
                );
            });
        prevent_default
    }

    pub fn global_dispatch_key_press(&self, args: KeyPress) -> bool {
        let mut prevent_default = false;
        self.root_expanded_node
            .recurse_visit_postorder(&mut |expanded_node| {
                prevent_default |= expanded_node.dispatch_key_press(
                    Event::new(args.clone()),
                    &self.runtime_context.globals(),
                    &self.runtime_context,
                );
            });
        prevent_default
    }
}
