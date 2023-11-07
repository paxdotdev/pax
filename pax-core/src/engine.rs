use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use kurbo::{Point, Vec2};
use piet_common::RenderContext;

use pax_message::NativeMessage;
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

use pax_runtime_api::{ArgsCheckboxChange, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsClap, ArgsKeyDown, ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver, ArgsMouseUp, ArgsScroll, ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, CommonProperties, Interpolatable, Layer, Rotation, RuntimeContext, Size, Transform2D, TransitionManager, ZIndex, Axis};

use crate::{Affine, ComponentInstance, ComputableTransform, ExpressionContext, NodeType, InstanceNodePtr, InstanceNodePtrList, TransformAndBounds, PropertiesTreeContext, recurse_compute_properties, RuntimePropertiesStackFrame, PropertiesTreeShared};

pub struct PaxEngine<R: 'static + RenderContext> {
    pub frames_elapsed: usize,
    pub node_registry: Rc<RefCell<NodeRegistry<R>>>,
    pub expression_table: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>>,
    pub main_component: Rc<RefCell<ComponentInstance<R>>>,
    pub image_map: HashMap<Vec<u32>, (Box<Vec<u8>>, usize, usize)>,
    viewport_tab: TransformAndBounds,
}


/// Shared context for render pass recursion
pub struct RenderTreeContext<'a, R: 'static + RenderContext> {
    pub engine: &'a PaxEngine<R>,

    /// A pointer to the current expanded node, the stateful atomic unit of traversal when rendering.
    pub current_expanded_node: Rc<RefCell<ExpandedNode<R>>>,
    /// A pointer to the current instance node, the stateless, instantiated representation of a template node.
    pub current_instance_node: InstanceNodePtr<R>,
}

//Note that `#[derive(Clone)]` doesn't work because of trait bounds surrounding R, even though
//the only places R is used are trivially cloned.
impl<'a, R: 'static + RenderContext> Clone for RenderTreeContext<'a, R> {
    fn clone(&self) -> Self {
        RenderTreeContext {
            engine: self.engine, // Borrowed references are Copy, so they can be "cloned" trivially.
            // transform_global: self.transform_global.clone(),
            // transform_scroller_reset: self.transform_scroller_reset.clone(),
            // bounds: self.bounds.clone(),
            // clipping_stack: self.clipping_stack.clone(),
            // scroller_stack: self.scroller_stack.clone(),
            current_expanded_node: Rc::clone(&self.current_expanded_node),
            current_instance_node: Rc::clone(&self.current_instance_node),
        }
    }
}


macro_rules! handle_vtable_update {
    ($ptc:expr, $var:ident . $field:ident, $types_coproduct_type:ident) => {{
        let current_prop = &mut *$var.$field.as_ref().borrow_mut();
        if let Some(new_value) = $ptc.compute_vtable_value(current_prop._get_vtable_id()) {
            let new_value = if let TypesCoproduct::$types_coproduct_type(val) = new_value {
                val
            } else {
                unreachable!()
            };
            current_prop.set(new_value);
        }
    }};
}

macro_rules! handle_vtable_update_optional {
    ($rtc:expr, $var:ident . $field:ident, $types_coproduct_type:ident) => {{
        if let Some(_) = $var.$field {
            let current_prop = &mut *$var.$field.as_ref().unwrap().borrow_mut();
            if let Some(new_value) = $rtc.compute_vtable_value(current_prop._get_vtable_id()) {
                let new_value = if let TypesCoproduct::$types_coproduct_type(val) = new_value {
                    val
                } else {
                    unreachable!()
                };
                current_prop.set(new_value);
            }
        }
    }};
}

//This trait is used strictly to side-load the `compute_properties` function onto CommonProperties,
//so that it can use the type RenderTreeContext (defined in pax_core, which depends on pax_runtime_api, which
//defines CommonProperties, and which can thus not depend on pax_core due to a would-be circular dependency.)
pub trait PropertiesComputable<R: 'static + RenderContext> {
    fn compute_properties(&mut self, rtc: &mut PropertiesTreeContext<R>);
}

impl<R: 'static + RenderContext> PropertiesComputable<R> for CommonProperties {
    fn compute_properties(&mut self, ptc: &mut PropertiesTreeContext<R>) {
        handle_vtable_update!(ptc, self.width, Size);
        handle_vtable_update!(ptc, self.height, Size);
        handle_vtable_update!(ptc, self.transform, Transform2D);
        handle_vtable_update_optional!(ptc, self.rotate, Rotation);
        handle_vtable_update_optional!(ptc, self.scale_x, Size);
        handle_vtable_update_optional!(ptc, self.scale_y, Size);
        handle_vtable_update_optional!(ptc, self.skew_x, Numeric);
        handle_vtable_update_optional!(ptc, self.skew_y, Numeric);
        handle_vtable_update_optional!(ptc, self.anchor_x, Size);
        handle_vtable_update_optional!(ptc, self.anchor_y, Size);
        handle_vtable_update_optional!(ptc, self.x, Size);
        handle_vtable_update_optional!(ptc, self.y, Size);
    }
}

impl<'a, R: RenderContext> RenderTreeContext<'a, R> {


    pub fn compute_eased_value<T: Clone + Interpolatable>(
        &self,
        transition_manager: Option<&mut TransitionManager<T>>,
    ) -> Option<T> {
        if let Some(tm) = transition_manager {
            if tm.queue.len() > 0 {
                let current_transition = tm.queue.get_mut(0).unwrap();
                if let None = current_transition.global_frame_started {
                    current_transition.global_frame_started = Some(self.engine.frames_elapsed);
                }
                let progress = (1.0 + self.engine.frames_elapsed as f64
                    - current_transition.global_frame_started.unwrap() as f64)
                    / (current_transition.duration_frames as f64);
                return if progress >= 1.0 {
                    //NOTE: we may encounter float imprecision here, consider `progress >= 1.0 - EPSILON` for some `EPSILON`
                    let new_value = current_transition.curve.interpolate(
                        &current_transition.starting_value,
                        &current_transition.ending_value,
                        progress,
                    );
                    tm.value = Some(new_value.clone());

                    tm.queue.pop_front();
                    self.compute_eased_value(Some(tm))
                } else {
                    let new_value = current_transition.curve.interpolate(
                        &current_transition.starting_value,
                        &current_transition.ending_value,
                        progress,
                    );
                    tm.value = Some(new_value.clone());
                    tm.value.clone()
                };
            } else {
                return tm.value.clone();
            }
        }
        None
    }
}

pub struct HandlerRegistry<R: 'static + RenderContext> {
    pub scroll_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsScroll)>,
    pub clap_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsClap)>,
    pub touch_start_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsTouchStart)>,
    pub touch_move_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsTouchMove)>,
    pub touch_end_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsTouchEnd)>,
    pub key_down_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsKeyDown)>,
    pub key_up_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsKeyUp)>,
    pub key_press_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsKeyPress)>,
    pub checkbox_change_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsCheckboxChange)>,
    pub click_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsClick)>,
    pub mouse_down_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsMouseDown)>,
    pub mouse_up_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsMouseUp)>,
    pub mouse_move_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsMouseMove)>,
    pub mouse_over_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsMouseOver)>,
    pub mouse_out_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsMouseOut)>,
    pub double_click_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsDoubleClick)>,
    pub context_menu_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsContextMenu)>,
    pub wheel_handlers: Vec<fn(InstanceNodePtr<R>, RuntimeContext, ArgsWheel)>,
    pub pre_render_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, RuntimeContext)>,
    pub mount_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, RuntimeContext)>,
}

impl<R: 'static + RenderContext> Default for HandlerRegistry<R> {
    fn default() -> Self {
        HandlerRegistry {
            scroll_handlers: Vec::new(),
            clap_handlers: Vec::new(),
            touch_start_handlers: Vec::new(),
            touch_move_handlers: Vec::new(),
            touch_end_handlers: Vec::new(),
            key_down_handlers: Vec::new(),
            key_up_handlers: Vec::new(),
            key_press_handlers: Vec::new(),
            click_handlers: Vec::new(),
            mouse_down_handlers: Vec::new(),
            mouse_up_handlers: Vec::new(),
            mouse_move_handlers: Vec::new(),
            mouse_over_handlers: Vec::new(),
            mouse_out_handlers: Vec::new(),
            double_click_handlers: Vec::new(),
            context_menu_handlers: Vec::new(),
            wheel_handlers: Vec::new(),
            pre_render_handlers: Vec::new(),
            mount_handlers: Vec::new(),
            checkbox_change_handlers: Vec::new(),
        }
    }
}

/// The atomic unit of rendering; also the container for each unique tuple of computed properties.
/// Represents an expanded node, that is "expanded" in the context of computed properties and repeat expansion.
/// For example, a Rectangle inside `for i in 0..3` and a `for j in 0..4` would have 12 expanded nodes representing the 12 virtual Rectangles in the
/// rendered scene graph. These nodes are addressed uniquely by id_chain (see documentation for `get_id_chain`.)
/// `ExpandedNode`s are architecturally "type-blind" — while they store typed data e.g. inside `computed_properties` and `computed_common_properties`,
/// they require coordinating with their "type-aware" [`InstanceNode`] to perform operations on those properties.
pub struct ExpandedNode<R: 'static + RenderContext> {
    #[allow(dead_code)]
    /// Unique ID of this expanded node, roughly encoding an address in the tree, where the first u32 is the instance ID
    /// and the subsequent u32s represent addresses within an expanded tree via Repeat.
    pub id_chain: Vec<u32>,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode directly above
    /// this one.  Used for e.g. event propagation.
    pub parent_expanded_node: Option<Weak<RefCell<ExpandedNode<R>>>>,

    /// Pointers to the ExpandedNode beneath this one.  Used for e.g. rendering recursion.
    pub children_expanded_nodes: Vec<Weak<RefCell<ExpandedNode<R>>>>,

    /// Pointer to the unexpanded `instance_node` underlying this ExpandedNode
    pub instance_node: InstanceNodePtr<R>,

    /// Computed transform and size of this ExpandedNode
    pub tab: TransformAndBounds,

    /// A copy of the calculated z_index for this ExpandedNode
    pub z_index: u32,

    /// A copy of the RuntimeContext appropriate for this ExpandedNode
    pub node_context: RuntimeContext,

    /// A snapshot of the clipping stack above this element at the time of properties-computation
    pub ancestral_clipping_ids: Vec<Vec<u32>>,

    /// A snapshot of the scroller stack above this element at the time of properties-computation
    pub ancestral_scroller_ids: Vec<Vec<u32>>,

    /// Each ExpandedNode has a unique "stamp" of computed properties
    computed_properties: Rc<RefCell<PropertiesCoproduct>>,

    /// Each ExpandedNode has unique, computed `CommonProperties`
    computed_common_properties: Rc<RefCell<CommonProperties>>,
}


impl<R: 'static + RenderContext> ExpandedNode<R> {
    pub fn upsert_with_prototypical_properties(ptc: &mut PropertiesTreeContext<R>, prototypical_properties: Rc<RefCell<PropertiesCoproduct>>, prototypical_common_properties: Rc<RefCell<CommonProperties>>) -> Rc<RefCell<Self>> {
        let id_chain = ptc.get_id_chain();
        let expanded_node = if let Some(already_registered_node) = ptc.engine.node_registry.borrow().get_expanded_node(&id_chain) {
            Rc::clone(already_registered_node)
        } else {
            let new_expanded_node = Rc::new(RefCell::new(ExpandedNode {
                id_chain: id_chain.clone(),
                parent_expanded_node: None,
                children_expanded_nodes: vec![],
                instance_node: Rc::clone(&ptc.current_instance_node),
                tab: ptc.tab.clone(),
                z_index: 0,
                node_context: ptc.distill_userland_node_context(),
                computed_properties: prototypical_properties,
                computed_common_properties: prototypical_common_properties,
                ancestral_clipping_ids: vec![],
                ancestral_scroller_ids: vec![]
            }));
            ptc.engine.node_registry.borrow_mut().expanded_node_map.insert(id_chain, Rc::clone(&new_expanded_node));
            new_expanded_node
        };
        expanded_node
    }

    pub fn append_child(&mut self, child: Rc<RefCell<ExpandedNode<R>>>) {
        self.children_expanded_nodes.push(Rc::downgrade(&child));
    }

    pub fn get_properties(&self) -> Rc<RefCell<PropertiesCoproduct>> {
        //need to refactor signature and pass in id_chain + either rtc + registry or just registry
        Rc::clone(&self.computed_properties)
    }

    pub fn get_common_properties(&self) -> Rc<RefCell<CommonProperties>> {
        Rc::clone(&self.computed_common_properties)
    }

    /// Determines whether the provided ray, orthogonal to the view plane,
    /// intersects this `ExpandedNode`.
    pub fn ray_cast_test(&self, ray: &(f64, f64)) -> bool {

        //short-circuit fail for Group and other size-None elements.
        //This doesn't preclude event handlers on Groups and size-None elements --
        //it just requires the event to "bubble".  otherwise, `Component A > Component B` will
        //never allow events to be bound to `B` — they will be vacuously intercepted by `A`
        if let None = self.get_size() {
            return false;
        }

        let inverted_transform = self.tab.transform.inverse();
        let transformed_ray = inverted_transform * Point { x: ray.0, y: ray.1 };

        // let relevant_bounds = match self.tab.clipping_bounds {
        //     None => self.tab.bounds,
        //     Some(cp) => cp,
        // };
        let relevant_bounds = self.tab.bounds;

        //Default implementation: rectilinear bounding hull
        transformed_ray.x > 0.0
            && transformed_ray.y > 0.0
            && transformed_ray.x < relevant_bounds.0
            && transformed_ray.y < relevant_bounds.1
    }

    /// Used at least by ray-casting; only nodes that clip content (and thus should
    /// not allow outside content to respond to ray-casting) should return true
    pub fn get_clipping_bounds(&self) -> Option<(Size, Size)> {
        None
    }

    /// Returns the size of this node, or `None` if this node
    /// doesn't have a size (e.g. `Group`)
    pub fn get_size(&self) -> Option<(Size, Size)> {
        Some((
            self.get_common_properties().borrow()
                .width
                .as_ref()
                .borrow()
                .get()
                .clone(),
            self.get_common_properties().borrow()
                .height
                .as_ref()
                .borrow()
                .get()
                .clone(),
        ))
    }

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    pub fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_size() {
            None => bounds,
            Some(size_raw) => (
                size_raw.0.evaluate(bounds, Axis::X),
                size_raw.1.evaluate(bounds, Axis::Y),
            ),
        }
    }

    /// Returns the clipping bounds of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    pub fn compute_clipping_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_clipping_bounds() {
            None => bounds,
            Some(size_raw) => (
                size_raw.0.evaluate(bounds, Axis::X),
                size_raw.1.evaluate(bounds, Axis::Y),
            ),
        }
    }

    /// Returns the scroll offset from a Scroller component
    /// Used by the engine to transform its children
    pub fn get_scroll_offset(&mut self) -> (f64, f64) {
        // (0.0, 0.0)
        todo!("patch into an ExpandedNode-friendly way to track this state");
        // Perhaps we simply add scroll_offset_x and scroll_offset_y globally to `ExpandedNode`?  The alternative
        // seems to be to store them inside PropertiesCoproduct and deal with un/wrapping.
    }

    pub fn dispatch_scroll(&self, args_scroll: ArgsScroll) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().scroll_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_scroll.clone(),
                );
            });
        }
        (*self.instance_node)
            .borrow_mut()
            .handle_scroll(args_scroll.clone());
        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_scroll(args_scroll);
        }
    }

    pub fn dispatch_clap(&self, args_clap: ArgsClap) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().clap_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_clap.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_clap(args_clap);
        }
    }

    pub fn dispatch_touch_start(&self, args_touch_start: ArgsTouchStart) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().touch_start_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_touch_start.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_touch_start(args_touch_start);
        }
    }

    pub fn dispatch_touch_move(&self, args_touch_move: ArgsTouchMove) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().touch_move_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_touch_move.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_touch_move(args_touch_move);
        }
    }

    pub fn dispatch_touch_end(&self, args_touch_end: ArgsTouchEnd) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().touch_end_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_touch_end.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_touch_end(args_touch_end);
        }
    }

    pub fn dispatch_key_down(&self, args_key_down: ArgsKeyDown) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().key_down_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_key_down.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_key_down(args_key_down);
        }
    }

    pub fn dispatch_key_up(&self, args_key_up: ArgsKeyUp) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().key_up_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_key_up.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_key_up(args_key_up);
        }
    }

    pub fn dispatch_key_press(&self, args_key_press: ArgsKeyPress) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().key_press_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_key_press.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_key_press(args_key_press);
        }
    }

    pub fn dispatch_click(&self, args_click: ArgsClick) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().click_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_click.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_click(args_click);
        }
    }

    pub fn dispatch_checkbox_change(&self, args_change: ArgsCheckboxChange) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().checkbox_change_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_change.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_checkbox_change(args_change);
        }
    }

    pub fn dispatch_mouse_down(&self, args_mouse_down: ArgsMouseDown) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().mouse_down_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_mouse_down.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_mouse_down(args_mouse_down);
        }
    }

    pub fn dispatch_mouse_up(&self, args_mouse_up: ArgsMouseUp) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().mouse_up_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_mouse_up.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_mouse_up(args_mouse_up);
        }
    }

    pub fn dispatch_mouse_move(&self, args_mouse_move: ArgsMouseMove) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().mouse_move_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_mouse_move.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_mouse_move(args_mouse_move);
        }
    }

    pub fn dispatch_mouse_over(&self, args_mouse_over: ArgsMouseOver) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().mouse_over_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_mouse_over.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_mouse_over(args_mouse_over);
        }
    }

    pub fn dispatch_mouse_out(&self, args_mouse_out: ArgsMouseOut) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().mouse_out_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_mouse_out.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_mouse_out(args_mouse_out);
        }
    }

    pub fn dispatch_double_click(&self, args_double_click: ArgsDoubleClick) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().double_click_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_double_click.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_double_click(args_double_click);
        }
    }

    pub fn dispatch_context_menu(&self, args_context_menu: ArgsContextMenu) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().context_menu_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_context_menu.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_context_menu(args_context_menu);
        }
    }

    pub fn dispatch_wheel(&self, args_wheel: ArgsWheel) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().wheel_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.instance_node),
                    self.node_context.clone(),
                    args_wheel.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent.upgrade().unwrap().borrow().dispatch_wheel(args_wheel);
        }
    }
}

pub struct NodeRegistry<R: 'static + RenderContext> {
    ///Allows look up of an `InstanceNodePtr` by instance id
    instance_node_map: HashMap<u32, InstanceNodePtr<R>>,

    ///Allows look up of an `ExpandedNode` by id_chain
    expanded_node_map: HashMap<Vec<u32>, Rc<RefCell<ExpandedNode<R>>>>,

    ///Tracks which `ExpandedNode`s are currently mounted -- if id is present in set, is mounted
    mounted_set: HashSet<Vec<u32>>,

    ///Tracks which `ExpandedNode`s are marked for unmounting.  Actual unmounting must happen at the correct time
    ///in the properties/rendering lifecycle, so this set is a primitive message queue: "write now, process later."
    ///Despite the unordered nature of the set, unmounting is strongly ordered; `ExpandedNode`s check for the presence of their
    ///own ID in this set during recursion, triggering unmount if so.
    marked_for_unmount_set: HashSet<Vec<u32>>,

    ///Stateful range iterator allowing us to retrieve the next value to mint as an instance id
    instance_uid_gen: std::ops::RangeFrom<u32>,
}

impl<R: 'static + RenderContext> NodeRegistry<R> {
    pub fn new() -> Self {
        Self {
            mounted_set: HashSet::new(),
            marked_for_unmount_set: HashSet::new(),
            instance_node_map: HashMap::new(),
            expanded_node_map: HashMap::new(),
            instance_uid_gen: 0..,
        }
    }

    /// Mint a new, monotonically increasing id for use in creating new instance nodes
    pub fn mint_instance_id(&mut self) -> u32 {
        self.instance_uid_gen.next().unwrap()
    }

    /// Add an instance to the NodeRegistry, incrementing its Rc count and giving it a canonical home
    pub fn register(&mut self, instance_id: u32, node: InstanceNodePtr<R>) {
        self.instance_node_map.insert(instance_id, node);
    }

    pub fn remove_expanded_node(&mut self, id_chain: &Vec<u32>) {
        self.expanded_node_map.remove(id_chain);
    }

    /// Retrieve an ExpandedNode by its id_chain from the encapsulated `expanded_node_map`
    pub fn get_expanded_node(&self, id_chain: &Vec<u32>) -> Option<&Rc<RefCell<ExpandedNode<R>>>> {
        self.expanded_node_map.get(id_chain)
    }

    /// Returns ExpandedNodes ordered by z-index descending; used at least by ray casting
    pub fn get_expanded_nodes_sorted_by_z_index_desc(&self) -> Vec<Rc<RefCell<ExpandedNode<R>>>> {
        let mut values: Vec<_> = self.expanded_node_map.values().cloned().collect();
        values.sort_by(|a, b| b.borrow().z_index.cmp(&a.borrow().z_index));
        values
    }

    /// Remove an instance from the instance_node_map.  This roughly only decrements the `Rc` surrounding
    /// the instance and is exposed to enable complete deletion of an Rc where the final reference may have been in the instance_node_map.
    pub fn deregister(&mut self, instance_id: u32) {
        self.instance_node_map.remove(&instance_id);
    }

    /// Mark an ExpandedNode as mounted, so that `mount` handlers will not fire on subsequent frames
    pub fn mark_mounted(&mut self, id_chain: Vec<u32>) {
        self.mounted_set.insert(id_chain);
    }

    /// Evaluates whether an ExpandedNode has been marked mounted, for use in determining whether to fire a `mount` handler
    pub fn is_mounted(&self, id_chain: &Vec<u32>) -> bool {
        self.mounted_set.contains(id_chain)
    }

    /// Mark an instance node for unmounting, which will happen during the upcoming tick
    pub fn mark_for_unmount(&mut self, id_chain: Vec<u32>) {
        self.marked_for_unmount_set.insert(id_chain);
    }

}


/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl<R: 'static + RenderContext> PaxEngine<R> {
    pub fn new(
        main_component_instance: Rc<RefCell<ComponentInstance<R>>>,
        expression_table: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>>,
        logger: pax_runtime_api::PlatformSpecificLogger,
        viewport_size: (f64, f64),
        node_registry: Rc<RefCell<NodeRegistry<R>>>,
    ) -> Self {
        pax_runtime_api::register_logger(logger);
        PaxEngine {
            frames_elapsed: 0,
            node_registry,
            expression_table,
            main_component: main_component_instance,
            viewport_tab: TransformAndBounds {
                transform: Affine::default(),
                bounds: viewport_size,
                // clipping_bounds: Some(viewport_size),
            },
            image_map: HashMap::new(),
        }
    }

    /// Enact primary workhorse methods of a tick:
    /// 1. Compute properties recursively, expanding control flow and storing computed properties in `ExpandedNodes`.
    /// 2. Render the computed `ExpandedNode`s.
    fn compute_properties_and_render(
        &self,
        rcs: &mut HashMap<String, R>,
    ) -> Vec<NativeMessage> {

        //Broadly:
        // 1. compute properties; recurse entire instance tree and evaluate ExpandedNodes, stitching
        //    together parent/child relationships between ExpandedNodes along the way.
        // 2. render:
        //     a. find lowest node (last child of last node), accumulating transform along the way
        //     b. start rendering, from lowest node on-up

        let root_component_instance: InstanceNodePtr<R> = self.main_component.clone();
        let mut z_index = ZIndex::new(None);

        // COMPUTE PROPERTIES
        let mut ptc = PropertiesTreeContext {
            engine: &self,
            timeline_playhead_position: 0,
            current_containing_component: Rc::clone(&root_component_instance),
            current_containing_component_slot_children: Rc::new(RefCell::new(vec![])),
            current_instance_node: Rc::clone(&root_component_instance),
            current_expanded_node: None,
            parent_expanded_node: None,
            tab: TransformAndBounds {
                bounds: self.viewport_tab.bounds,
                transform: Affine::default(),
                // clipping_bounds: None,
            },
            transform_scroller_reset: Default::default(),
            marked_for_unmount: false,
            shared: Rc::new(RefCell::new(
                PropertiesTreeShared {
                    clipping_stack: vec![],
                    scroller_stack: vec![],
                    native_message_queue: Default::default(),
                    runtime_properties_stack: vec![],
                    z_index_gen: 0..,
                }
            )),
        };
        let root_expanded_node = recurse_compute_properties(&mut ptc);

        // RENDER
        let mut rtc = RenderTreeContext {
            engine: &self,
            // transform_global: Affine::default(),
            // transform_scroller_reset: Affine::default(),
            // bounds: self.viewport_tab.bounds,
            // clipping_stack: vec![],
            // scroller_stack: vec![],
            current_expanded_node: Rc::clone(&root_expanded_node),
            current_instance_node: Rc::clone(&root_expanded_node.borrow().instance_node)
        };

        self.recurse_render(
            &mut rtc,
            rcs,
            &mut z_index,
            false,
        );
        //reset the marked_for_unmount set
        self.node_registry.borrow_mut().marked_for_unmount_set = HashSet::new();

        let native_render_queue = ptc.take_native_message_queue();
        native_render_queue.into()
    }


    /// Helper method to fire `pre_render` handlers for the node attached to the `rtc`
    fn manage_handlers_pre_render(rtc: &mut RenderTreeContext<R>) {
        //fire `pre_render` handlers
        let node = Rc::clone(&rtc.current_expanded_node);
        let registry = (*rtc.current_expanded_node).borrow().instance_node.borrow().get_handler_registry();
        if let Some(registry) = registry {
            for handler in (*registry).borrow().pre_render_handlers.iter() {
                handler(
                    node.borrow_mut().get_properties(),
                    node.borrow().node_context.clone(),
                );
            }
        }
    }

    fn recurse_render(
        &self,
        rtc: &mut RenderTreeContext<R>,
        rcs: &mut HashMap<String, R>,
        z_index_info: &mut ZIndex,
        marked_for_unmount: bool,
    ) {
        //Recurse:

        //  - fire lifecycle events for this node
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...

        // let accumulated_transform = rtc.transform_global;
        // let accumulated_scroller_normalized_transform = rtc.transform_scroller_reset;
        let accumulated_bounds = rtc.current_expanded_node.borrow().tab.bounds;
        let node = Rc::clone(&rtc.current_expanded_node);

        rtc.current_instance_node = Rc::clone(&node.borrow().instance_node);

        //scroller IDs are used by chassis, for identifying native scrolling containers
        let scroller_ids = rtc.current_expanded_node.borrow().ancestral_scroller_ids.clone();
        let scroller_id = match scroller_ids.last() {
            None => None,
            Some(v) => Some(v.clone()),
        };
        let canvas_id = ZIndex::assemble_canvas_id(scroller_id.clone(), rtc.current_expanded_node.borrow().z_index);

        Self::manage_handlers_pre_render(rtc);

        let mut subtree_depth = 0;

        //keep recursing through children
        let mut child_z_index_info = z_index_info.clone();
        if z_index_info.get_current_layer() == Layer::Scroller {
            let id_chain = node.borrow().id_chain.clone();
            child_z_index_info = ZIndex::new(Some(id_chain));
            // let (scroll_offset_x, scroll_offset_y) = node.borrow_mut().get_scroll_offset();
            // let mut reset_transform = Affine::default();
            // reset_transform =
            //     reset_transform.then_translate(Vec2::new(scroll_offset_x, scroll_offset_y));
            // rtc.transform_scroller_reset = reset_transform.clone();
        }

        &node.borrow_mut().children_expanded_nodes
            .iter()
            .rev()
            .for_each(|expanded_node| {
                //note that we're iterating starting from the last child, for z-index (.rev())
                let mut new_rtc = rtc.clone();
                // if it's a scroller reset the z-index context for its children
                self.recurse_render(
                    &mut new_rtc,
                    rcs,
                    &mut child_z_index_info.clone(),
                    marked_for_unmount,
                );
                //FUTURE: for dependency management, return computed values from subtree above

                subtree_depth = subtree_depth.max(child_z_index_info.get_level());
            });

        let is_viewport_culled = !node.borrow().tab.intersects(&self.viewport_tab);

        let clipping = node
            .borrow_mut()
            .compute_clipping_within_bounds(accumulated_bounds);
        let clipping_bounds = match node.borrow_mut().get_clipping_bounds() {
            None => None,
            Some(_) => Some(clipping),
        };

        // let clipping_aware_bounds = if let Some(cb) = clipping_bounds {
        //     cb
        // } else {
        //     new_accumulated_bounds
        // };

        if let Some(rc) = rcs.get_mut(&canvas_id) {
            //lifecycle: render
            //this is this node's time to do its own rendering, aside
            //from the rendering of its children. Its children have already been rendered.
            if !is_viewport_culled {
                node.borrow_mut().instance_node.borrow_mut().handle_render(rtc, rc);
            }
        } else {
            if let Some(rc) = rcs.get_mut("0") {
                if !is_viewport_culled {
                    node.borrow_mut().instance_node.borrow_mut().handle_render(rtc, rc);
                }
            }
        }

        //lifecycle: post_render
        node.borrow_mut().instance_node.borrow_mut().handle_post_render(rtc, rcs);

    }

    /// Simple 2D raycasting: the coordinates of the ray represent a
    /// ray running orthogonally to the view plane, intersecting at
    /// the specified point `ray`.  Areas outside of clipping bounds will
    /// not register a `hit`, nor will elements that suppress input events.
    pub fn get_topmost_element_beneath_ray(
        &self,
        ray: (f64, f64),
    ) -> Option<Rc<RefCell<ExpandedNode<R>>>> {
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        //Instead of storing a pointer to `last_rtc`, we should store a custom
        //struct with exactly the fields we need for ray-casting

        //Need:
        // - Cached computed transform `: Affine`
        // - Pointer to parent:
        //     for bubbling, i.e. propagating event
        //     for finding ancestral clipping containers

        let mut nodes_ordered: Vec<Rc<RefCell<ExpandedNode<R>>>> = (*self.node_registry)
            .borrow()
            .get_expanded_nodes_sorted_by_z_index_desc();

        // remove root element that is moved to top during reversal
        nodes_ordered.remove(0);

        // let ray = Point {x: ray.0,y: ray.1};
        let mut ret: Option<Rc<RefCell<ExpandedNode<R>>>> = None;
        for node in nodes_ordered {
            if (*node).borrow().ray_cast_test(&ray)
            {
                //We only care about the topmost node getting hit, and the element
                //pool is ordered by z-index so we can just resolve the whole
                //calculation when we find the first matching node

                let mut ancestral_clipping_bounds_are_satisfied = true;
                let mut parent: Option<Rc<RefCell<ExpandedNode<R>>>> = node
                    .borrow()
                    .parent_expanded_node
                    .as_ref()
                    .and_then(|weak| weak.upgrade());

                loop {
                    if let Some(unwrapped_parent) = parent {
                        if let Some(_) = (*unwrapped_parent).borrow().get_clipping_bounds()
                        {
                            ancestral_clipping_bounds_are_satisfied = (*unwrapped_parent)
                                .borrow().ray_cast_test(&ray);
                            break;
                        }
                        parent = unwrapped_parent
                            .borrow()
                            .parent_expanded_node
                            .as_ref()
                            .and_then(|weak| weak.upgrade());
                    } else {
                        break;
                    }
                }

                if ancestral_clipping_bounds_are_satisfied {
                    ret = Some(Rc::clone(&node));
                    break;
                }
            }
        }

        ret
    }

    pub fn get_focused_element(&self) -> Option<Rc<RefCell<ExpandedNode<R>>>> {
        let (x, y) = self.viewport_tab.bounds;
        self.get_topmost_element_beneath_ray((x / 2.0, y / 2.0))
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_tab.bounds = new_viewport_size;
    }

    /// Workhorse method to advance rendering and property calculation by one discrete tick
    /// Will be executed synchronously up to 240 times/second.
    pub fn tick(&mut self, rcs: &mut HashMap<String, R>) -> Vec<NativeMessage> {
        let native_render_queue = self.compute_properties_and_render(rcs);
        self.frames_elapsed = self.frames_elapsed + 1;
        native_render_queue
    }

    pub fn load_image(
        &mut self,
        id_chain: Vec<u32>,
        image_data: Vec<u8>,
        width: usize,
        height: usize,
    ) {
        self.image_map
            .insert(id_chain, (Box::new(image_data), width, height));
    }
}
