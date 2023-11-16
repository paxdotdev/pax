use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use kurbo::{Point, Vec2};
use piet_common::RenderContext;

use pax_message::NativeMessage;

use pax_runtime_api::{
    ArgsCheckboxChange, ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsKeyDown,
    ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver,
    ArgsMouseUp, ArgsScroll, ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, Axis,
    CommonProperties, Interpolatable, Layer, Numeric, Rotation, NodeContext, Size, Transform2D,
    TransitionManager, ZIndex,
};

use crate::{recurse_expand_nodes, Affine, ComponentInstance, ComputableTransform, ExpressionContext, InstanceNodePtr, InstanceNodePtrList, NodeType, PropertiesTreeContext, PropertiesTreeShared, RuntimePropertiesStackFrame, TransformAndBounds, handle_vtable_update, handle_vtable_update_optional, RenderTreeContext, recurse_compute_layout, recurse_render};

/// Singleton struct storing everything related to properties computation & rendering
pub struct PaxEngine<R: 'static + RenderContext> {
    pub frames_elapsed: usize,
    pub node_registry: Rc<RefCell<NodeRegistry<R>>>,
    pub expression_table: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> Box<dyn Any>>>,
    pub main_component: Rc<RefCell<ComponentInstance<R>>>,
    pub image_map: HashMap<Vec<u32>, (Box<Vec<u8>>, usize, usize)>,
    pub viewport_tab: TransformAndBounds,
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

pub struct HandlerRegistry {
    pub scroll_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsScroll)>,
    pub clap_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsClap)>,
    pub touch_start_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsTouchStart)>,
    pub touch_move_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsTouchMove)>,
    pub touch_end_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsTouchEnd)>,
    pub key_down_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsKeyDown)>,
    pub key_up_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsKeyUp)>,
    pub key_press_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsKeyPress)>,
    pub checkbox_change_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsCheckboxChange)>,
    pub click_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsClick)>,
    pub mouse_down_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseDown)>,
    pub mouse_up_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseUp)>,
    pub mouse_move_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseMove)>,
    pub mouse_over_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseOver)>,
    pub mouse_out_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseOut)>,
    pub double_click_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsDoubleClick)>,
    pub context_menu_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsContextMenu)>,
    pub wheel_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsWheel)>,
    pub pre_render_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext)>,
    pub mount_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext)>,
}

impl Default for HandlerRegistry {
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

    /// Pointer to the unexpanded `instance_node` underlying this ExpandedNode
    pub instance_node: InstanceNodePtr<R>,

    /// Computed transform and size of this ExpandedNode
    /// Optional because a ExpandedNode is initialized with `computed_tab: None`; this is computed later
    pub computed_tab: Option<TransformAndBounds>,

    /// A copy of the computed z_index for this ExpandedNode
    pub computed_z_index: Option<u32>,

    /// A copy of the NodeContext appropriate for this ExpandedNode
    pub computed_node_context: Option<NodeContext>,

    /// Reference to the _component for which this `ExpandedNode` is a template member._  Used at least for
    /// getting a reference to slot_children for `slot`.  `Option`al because the very root instance node (root component, root instance node)
    /// has a corollary "root component expanded node."  That very root expanded node _does not have_ a containing ExpandedNode component,
    /// thus `containing_component` is `Option`al.
    pub containing_component: Option<Rc<RefCell<ExpandedNode<R>>>>,

    /// Persistent clone of the state of the [`PropertiesTreeShared#runtime_properties_stack`] at the time that this node was expanded (this is expected to remain immutable
    /// through the lifetime of the program after the initial expansion; however, if that constraint changes, this should be
    /// explicitly updated to accommodate.)
    pub runtime_properties_stack: Vec<Rc<RefCell<RuntimePropertiesStackFrame>>>,

    /// Persistent clone of the state of the [`PropertiesTreeShared#clipping_stack`] at the time this node was expanded.
    /// A snapshot of the clipping stack above this element at the time of properties-computation
    pub clipping_stack: Vec<Vec<u32>>,

    /// Persistent clone of the state of the [`PropertiesTreeShared#scroller_stack`] at the time this node was expanded.
    /// A snapshot of the scroller stack above this element at the time of properties-computation
    pub scroller_stack: Vec<Vec<u32>>,

    /// For component instances only, tracks the expanded + flattened slot_children
    expanded_and_flattened_slot_children: Option<Vec<Rc<RefCell<ExpandedNode<R>>>>>,

    /// Each ExpandedNode has a unique "stamp" of computed properties
    computed_properties: Rc<RefCell<dyn Any>>,

    /// Each ExpandedNode has unique, computed `CommonProperties`
    computed_common_properties: Rc<RefCell<CommonProperties>>,

    /// Pointers to the ExpandedNode beneath this one.  Used for e.g. rendering recursion.
    children_expanded_nodes: Vec<Rc<RefCell<ExpandedNode<R>>>>,

    /// Constant-time lookup for presence of children expanded nodes; maintained duplicatively of children_expanded_nodes
    /// and used for performant checking-for-presence-before-inserting of children_nodes.
    /// Note that this only checks for presence, not for ordering.  If we support
    /// changing the index of children at any point (e.g. possibly via `key` as a feature of `RepeatInstance`) then this should be
    /// updated to be order-aware.
    children_expanded_nodes_set: HashSet<Vec<u32>>,
}

impl<R: 'static + RenderContext> ExpandedNode<R> {
    pub fn get_children_expanded_nodes(&self) -> &Vec<Rc<RefCell<ExpandedNode<R>>>> {
        &self.children_expanded_nodes
    }

    // Appends the passed `child_expanded_node` to be a child of this ExpandedNode, after first ensuring this node
    // was not already registered as a child (to avoid duplicates.)  This is especially important in a world
    // where we expand nodes every tick (pre-dirty-DAG) and this check might be able to be retired when we expand exactly once
    // per instance tree.
    pub fn append_child_expanded_node(
        &mut self,
        child_expanded_node: Rc<RefCell<ExpandedNode<R>>>,
    ) {
        //check if expanded node is already a child of this node (and no-op if it is)
        let cenb = child_expanded_node.borrow();
        let id_chain_ref = &cenb.id_chain;

        if !self.children_expanded_nodes_set.contains(id_chain_ref) {
            let id_chain = id_chain_ref.clone();

            drop(cenb); // satisfy borrow checker, now that we have our cloned id_chain
            self.children_expanded_nodes_set.insert(id_chain);
            self.children_expanded_nodes.push(child_expanded_node);
        }
    }

    // Register expanded & flattened slot_children on a Component that received them, so that they
    // may be referred to by a `slot` inside that component's template.
    pub fn set_expanded_and_flattened_slot_children(
        &mut self,
        expanded_and_flattened_slot_children: Option<Vec<Rc<RefCell<ExpandedNode<R>>>>>,
    ) {
        self.expanded_and_flattened_slot_children = expanded_and_flattened_slot_children;
    }

    pub fn get_expanded_and_flattened_slot_children(
        &self,
    ) -> &Option<Vec<Rc<RefCell<ExpandedNode<R>>>>> {
        &self.expanded_and_flattened_slot_children
    }

    pub fn get_or_create_with_prototypical_properties(
        ptc: &mut PropertiesTreeContext<R>,
        prototypical_properties: &Rc<RefCell<dyn Any>>,
        prototypical_common_properties: &Rc<RefCell<CommonProperties>>,
    ) -> Rc<RefCell<Self>> {
        let id_chain = ptc.get_id_chain(ptc.current_instance_id);
        let expanded_node = if let Some(already_registered_node) = ptc
            .engine
            .node_registry
            .borrow()
            .get_expanded_node(&id_chain)
        {
            Rc::clone(already_registered_node)
        } else {
            let new_expanded_node = Rc::new(RefCell::new(ExpandedNode {
                id_chain: id_chain.clone(),
                parent_expanded_node: None,
                children_expanded_nodes: vec![],
                instance_node: Rc::clone(&ptc.current_instance_node),
                containing_component: ptc.current_containing_component.clone(),

                computed_properties: todo!("deep clone"),// Rc::clone(&prototypical_properties),
                computed_common_properties: todo!("deep clone"),// Rc::clone(&prototypical_common_properties),

                expanded_and_flattened_slot_children: None,
                children_expanded_nodes_set: HashSet::new(),

                // Initialize the following to `None`, will assign values during `recurse_compute_layout`
                computed_z_index: None,
                computed_node_context: None,
                computed_tab: None,

                // Clone the following stacks from `ptc`
                clipping_stack: ptc.get_current_clipping_ids(),
                scroller_stack: ptc.get_current_scroller_ids(),
                runtime_properties_stack: ptc.clone_runtime_stack(),
            }));

            new_expanded_node
        };
        ptc.engine
            .node_registry
            .borrow_mut()
            .expanded_node_map
            .insert(id_chain, Rc::clone(&expanded_node));

        //Side-effect: attach an Rc pointer for the current expanded_node to `ptc`.
        ptc.current_expanded_node = Some(Rc::clone(&expanded_node));

        expanded_node
    }

    pub fn get_properties(&self) -> Rc<RefCell<dyn Any>> {
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
        // if let None = self.get_size() {
        //     return false;
        // }
        // todo!("hook back up vacuous hit testing");
        //
        let inverted_transform = self.computed_tab.as_ref().unwrap().transform.inverse();
        let transformed_ray = inverted_transform * Point { x: ray.0, y: ray.1 };

        // let relevant_bounds = match self.tab.clipping_bounds {
        //     None => self.tab.bounds,
        //     Some(cp) => cp,
        // };
        let relevant_bounds = self.computed_tab.as_ref().unwrap().bounds;

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
    pub fn get_size(&self) -> (Size, Size) {
        self.instance_node.borrow().get_size(self)
    }

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    pub fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        let size = self.get_size();
        (
            size.0.evaluate(bounds, Axis::X),
            size.1.evaluate(bounds, Axis::Y),
        )
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
    }

    pub fn dispatch_scroll(&self, args_scroll: ArgsScroll) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().scroll_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_scroll.clone(),
                );
            });
        }
        (*self.instance_node)
            .borrow_mut()
            .handle_scroll(args_scroll.clone());
        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_scroll(args_scroll);
        }
    }

    pub fn dispatch_clap(&self, args_clap: ArgsClap) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().clap_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_touch_end.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_touch_end(args_touch_end);
        }
    }

    pub fn dispatch_key_down(&self, args_key_down: ArgsKeyDown) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().key_down_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_key_down.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_key_down(args_key_down);
        }
    }

    pub fn dispatch_key_up(&self, args_key_up: ArgsKeyUp) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().key_up_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_key_up.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_key_up(args_key_up);
        }
    }

    pub fn dispatch_key_press(&self, args_key_press: ArgsKeyPress) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().key_press_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_key_press.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_key_press(args_key_press);
        }
    }

    pub fn dispatch_click(&self, args_click: ArgsClick) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().click_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_click.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_click(args_click);
        }
    }

    pub fn dispatch_checkbox_change(&self, args_change: ArgsCheckboxChange) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().checkbox_change_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_mouse_up.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_mouse_up(args_mouse_up);
        }
    }

    pub fn dispatch_mouse_move(&self, args_mouse_move: ArgsMouseMove) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().mouse_move_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_mouse_out.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_mouse_out(args_mouse_out);
        }
    }

    pub fn dispatch_double_click(&self, args_double_click: ArgsDoubleClick) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            let handlers = &(*registry).borrow().double_click_handlers;
            handlers.iter().for_each(|handler| {
                handler(
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
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
                    Rc::clone(&self.get_properties()),
                    &self.computed_node_context.clone().unwrap(),
                    args_wheel.clone(),
                );
            });
        }

        if let Some(parent) = &self.parent_expanded_node {
            parent
                .upgrade()
                .unwrap()
                .borrow()
                .dispatch_wheel(args_wheel);
        }
    }
}

pub struct NodeRegistry<R: 'static + RenderContext> {
    /// Allows look up of an `InstanceNodePtr` by instance id
    instance_node_map: HashMap<u32, InstanceNodePtr<R>>,

    /// Allows look up of an `ExpandedNode` by id_chain
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
        values.sort_by(|a, b| b.borrow().computed_z_index.cmp(&a.borrow().computed_z_index));
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

    /// Evaluates whether an ExpandedNode has been marked mounted, for use in determining whether to fire a `mount` handler
    pub fn is_marked_for_unmount(&self, id_chain: &Vec<u32>) -> bool {
        self.marked_for_unmount_set.contains(id_chain)
    }

    /// Mark an instance node for unmounting, which will happen during the upcoming tick
    pub fn mark_for_unmount(&mut self, id_chain: Vec<u32>) {
        self.marked_for_unmount_set.insert(id_chain);
    }

    /// Remove from marked_for_unmount_set
    pub fn revert_mark_for_unmount(&mut self, id_chain: &Vec<u32>) {
        self.marked_for_unmount_set.remove(id_chain);
    }
}

/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl<R: 'static + RenderContext> PaxEngine<R> {
    pub fn new(
        main_component_instance: Rc<RefCell<ComponentInstance<R>>>,
        expression_table: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> Box<dyn Any>>>,
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

    /// Primary workhorse methods of a tick
    /// Broadly:
    /// 1. expand nodes & compute properties; recurse entire instance tree and evaluate ExpandedNodes, stitching
    ///    together parent/child relationships between ExpandedNodes along the way.
    /// 2. compute rendering properties (z-index & TransformAndBounds) by visiting ExpandedNode tree
    ///    in rendering order, writing computed rendering-specific values to ExpandedNodes
    /// 3. render:
    ///     a. find lowest node (last child of last node)
    ///     b. start rendering, from lowest node on-up, throughout tree
    fn compute_properties_and_render(&self, rcs: &mut HashMap<String, R>) -> Vec<NativeMessage> {

        let root_component_instance: InstanceNodePtr<R> = self.main_component.clone();
        let mut z_index = ZIndex::new(None);

        // EXPAND NODES & COMPUTE ALL PROPERTIES
        let mut ptc = PropertiesTreeContext {
            engine: &self,
            current_containing_component: None,
            current_instance_node: Rc::clone(&root_component_instance),
            current_instance_id: root_component_instance.borrow().get_instance_id(),
            current_expanded_node: None,
            parent_expanded_node: None,
            marked_for_unmount: false,
            clipping_stack: vec![],
            scroller_stack: vec![],
            runtime_properties_stack: vec![],
            shared: Rc::new(RefCell::new(PropertiesTreeShared {
                native_message_queue: Default::default(),
            })),
            expanded_and_flattened_slot_children: None,
        };
        let root_expanded_node = recurse_expand_nodes(&mut ptc);


        let mut z_index_gen = 0..;
        // COMPUTE LAYOUT
        // (Visits ExpandedNodes in rendering order and calculates + writes z-index and tab to each ExpandedNode)
        // This could be cordoned off to specific subtrees based on dirtiness-awareness in the future
        recurse_compute_layout(
            &self,
            &root_expanded_node,
            &TransformAndBounds {
                bounds: self.viewport_tab.bounds,
                transform: Affine::default(),
            },
            &mut z_index_gen,
        );

        // RENDER
        // Generally side-effect-free aside from the rendering itself, express the current ExpandedNode tree as pixels.
        let mut rtc = RenderTreeContext {
            engine: &self,
            current_expanded_node: Rc::clone(&root_expanded_node),
            current_instance_node: Rc::clone(&root_expanded_node.borrow().instance_node),
        };

        recurse_render(&mut rtc, rcs, &mut z_index, false);

        //reset the marked_for_unmount set
        rtc.engine.node_registry.borrow_mut().marked_for_unmount_set = HashSet::new();
        let native_render_queue = ptc.take_native_message_queue();
        native_render_queue.into()
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
            if (*node).borrow().ray_cast_test(&ray) {
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
                        if let Some(_) = (*unwrapped_parent).borrow().get_clipping_bounds() {
                            ancestral_clipping_bounds_are_satisfied =
                                (*unwrapped_parent).borrow().ray_cast_test(&ray);
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
