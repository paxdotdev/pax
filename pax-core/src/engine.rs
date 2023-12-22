use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{BTreeSet, BinaryHeap, HashMap};
use std::fmt;
use std::rc::{Rc, Weak};

use kurbo::Point;

use pax_message::NativeMessage;

use pax_runtime_api::{
    ArgsCheckboxChange, ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsKeyDown,
    ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver,
    ArgsMouseUp, ArgsScroll, ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, Axis,
    CommonProperties, Interpolatable, NodeContext, RenderContext, Size, TransitionManager,
};

use crate::declarative_macros::{handle_vtable_update, handle_vtable_update_optional};
use crate::{
    compute_tab, Affine, ComponentInstance, ExpressionContext, InstanceNode, InstanceNodePtr,
    RuntimeContext, RuntimePropertiesStackFrame, TransformAndBounds,
};

pub struct Globals {
    pub frames_elapsed: usize,
    pub viewport: TransformAndBounds,
}

/// Singleton struct storing everything related to properties computation & rendering
pub struct PaxEngine {
    pub runtime_context: RuntimeContext,
    pub root_node: Rc<ExpandedNode>,
    pub z_index_node_cache: BinaryHeap<(u32, Rc<ExpandedNode>)>,
    pub image_map: HashMap<Vec<u32>, (Box<Vec<u8>>, usize, usize)>,
}

//This trait is used strictly to side-load the `compute_properties` function onto CommonProperties,
//so that it can use the type RenderTreeContext (defined in pax_core, which depends on pax_runtime_api, which
//defines CommonProperties, and which can thus not depend on pax_core due to a would-be circular dependency.)
pub trait PropertiesComputable {
    fn compute_properties(
        &mut self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        table: &ExpressionTable,
    );
}

impl PropertiesComputable for CommonProperties {
    fn compute_properties(
        &mut self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        table: &ExpressionTable,
    ) {
        handle_vtable_update(table, stack, &mut self.width);
        handle_vtable_update(table, stack, &mut self.height);
        handle_vtable_update(table, stack, &mut self.transform);
        handle_vtable_update_optional(table, stack, self.rotate.as_mut());
        handle_vtable_update_optional(table, stack, self.scale_x.as_mut());
        handle_vtable_update_optional(table, stack, self.scale_y.as_mut());
        handle_vtable_update_optional(table, stack, self.skew_x.as_mut());
        handle_vtable_update_optional(table, stack, self.skew_y.as_mut());
        handle_vtable_update_optional(table, stack, self.anchor_x.as_mut());
        handle_vtable_update_optional(table, stack, self.anchor_y.as_mut());
        handle_vtable_update_optional(table, stack, self.x.as_mut());
        handle_vtable_update_optional(table, stack, self.y.as_mut());
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

#[cfg(debug_assertions)]
impl std::fmt::Debug for ExpandedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //see: https://users.rust-lang.org/t/reusing-an-fmt-formatter/8531/4
        //maybe this utility should be moved to a more accessible place?
        pub struct Fmt<F>(pub F)
        where
            F: Fn(&mut fmt::Formatter) -> fmt::Result;

        impl<F> fmt::Debug for Fmt<F>
        where
            F: Fn(&mut fmt::Formatter) -> fmt::Result,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                (self.0)(f)
            }
        }

        f.debug_struct("ExpandedNode")
            .field(
                "instance_node",
                &Fmt(|f| self.instance_template.resolve_debug(f, Some(self))),
            )
            .field("id_chain", &self.id_chain)
            //.field("bounds", &self.computed_tab)
            .field("common_properties", &self.common_properties.borrow())
            .field(
                "computed_expanded_properties",
                &self.computed_expanded_properties.borrow(),
            )
            .field(
                "children",
                &self.children.borrow().iter().collect::<Vec<_>>(),
            )
            .field(
                "parent",
                &self
                    .parent_expanded_node
                    .upgrade()
                    .map(|v| v.id_chain.clone()),
            )
            // .field(
            //     "slot_children",
            //     &self.expanded_and_flattened_slot_children.as_ref().map(|o| {
            //         o.iter()
            //             .map(|v| v.borrow().id_chain.clone())
            //             .collect::<Vec<_>>()
            //     }),
            // )
            .field(
                "containing_component",
                &self
                    .containing_component
                    .upgrade()
                    .map(|v| v.id_chain.clone()),
            )
            .finish()
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ComputedExpandedProperties {
    //COMPUTED_PROPERTIES: that depend on other computed properties higher up in the tree
    //
    /// Computed transform and size of this ExpandedNode
    /// Optional because a ExpandedNode is initialized with `computed_tab: None`; this is computed later
    pub computed_tab: TransformAndBounds,

    /// A copy of the computed z_index for this ExpandedNode
    pub computed_z_index: u32,

    /// A copy of the computed canvas_index for this ExpandedNode
    pub computed_canvas_index: u32,
}

impl PartialOrd for ExpandedNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id_chain.partial_cmp(&other.id_chain)
    }
}

impl Ord for ExpandedNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id_chain.cmp(&other.id_chain)
    }
}

impl PartialEq for ExpandedNode {
    fn eq(&self, other: &Self) -> bool {
        self.id_chain.eq(&other.id_chain)
    }
}

impl Eq for ExpandedNode {}

/// The atomic unit of rendering; also the container for each unique tuple of computed properties.
/// Represents an expanded node, that is "expanded" in the context of computed properties and repeat expansion.
/// For example, a Rectangle inside `for i in 0..3` and a `for j in 0..4` would have 12 expanded nodes representing the 12 virtual Rectangles in the
/// rendered scene graph. These nodes are addressed uniquely by id_chain (see documentation for `get_id_chain`.)
/// `ExpandedNode`s are architecturally "type-blind" — while they store typed data e.g. inside `computed_properties` and `computed_common_properties`,
/// they require coordinating with their "type-aware" [`InstanceNode`] to perform operations on those properties.
pub struct ExpandedNode {
    #[allow(dead_code)]
    /// Unique ID of this expanded node, roughly encoding an address in the tree, where the first u32 is the instance ID
    /// and the subsequent u32s represent addresses within an expanded tree via Repeat.
    pub id_chain: Vec<u32>,

    /// Pointer to the unexpanded `instance_node` underlying this ExpandedNode
    pub instance_template: InstanceNodePtr,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode directly above
    /// this one.  Used for e.g. event propagation.
    pub parent_expanded_node: Weak<ExpandedNode>,

    /// Reference to the _component for which this `ExpandedNode` is a template member._  Used at least for
    /// getting a reference to slot_children for `slot`.  `Option`al because the very root instance node (root component, root instance node)
    /// has a corollary "root component expanded node."  That very root expanded node _does not have_ a containing ExpandedNode component,
    /// thus `containing_component` is `Option`al.
    pub containing_component: Weak<ExpandedNode>,

    /// Persistent clone of the state of the [`PropertiesTreeShared#runtime_properties_stack`] at the time that this node was expanded (this is expected to remain immutable
    /// through the lifetime of the program after the initial expansion; however, if that constraint changes, this should be
    /// explicitly updated to accommodate.)
    pub stack: Rc<RuntimePropertiesStackFrame>,

    //TODO replace these two with BTreeSet?
    /// Pointers to the ExpandedNode beneath this one.  Used for e.g. rendering recursion.
    children: RefCell<BTreeSet<Rc<ExpandedNode>>>,

    /// Each ExpandedNode has a unique "stamp" of computed properties
    pub properties: Rc<RefCell<dyn Any>>,

    /// Each ExpandedNode has unique, computed `CommonProperties`
    common_properties: Rc<RefCell<CommonProperties>>,

    pub computed_expanded_properties: RefCell<Option<ComputedExpandedProperties>>,
    // Persistent clone of the state of the [`PropertiesTreeShared#clipping_stack`] at the time this node was expanded.
    // A snapshot of the clipping stack above this element at the time of properties-computation
    // pub clipping_stack: Vec<Vec<u32>>,

    // Persistent clone of the state of the [`PropertiesTreeShared#scroller_stack`] at the time this node was expanded.
    // A snapshot of the scroller stack above this element at the time of properties-computation
    // pub scroller_stack: Vec<Vec<u32>>,

    // /// For component instances only, tracks the expanded + flattened slot_children
    // expanded_and_flattened_slot_children: Option<Vec<Rc<ExpandedNode>>>,
}

macro_rules! dispatch_event_handler {
    ($fn_name:ident, $arg_type:ty, $handler_field:ident) => {
        pub fn $fn_name(&self, args: $arg_type, globals: &Globals) {
            if let Some(registry) = self.instance_template.base().get_handler_registry() {
                let handlers = &(*registry).borrow().$handler_field;
                let component_properties = if let Some(cc) = self.containing_component.upgrade() {
                    Rc::clone(&cc.properties)
                } else {
                    Rc::clone(&self.properties)
                };

                let comp_props = self.computed_expanded_properties.borrow();
                let bounds_self = comp_props.as_ref().unwrap().computed_tab.bounds;
                let bounds_parent = self
                    .parent_expanded_node
                    .upgrade()
                    .map(|parent| {
                        let comp_props = parent.computed_expanded_properties.borrow();
                        let bounds_parent = comp_props.as_ref().unwrap().computed_tab.bounds;
                        bounds_parent
                    })
                    .unwrap_or(globals.viewport.bounds);
                let context = NodeContext {
                    bounds_self,
                    bounds_parent,
                    frames_elapsed: globals.frames_elapsed,
                };
                handlers.iter().for_each(|handler| {
                    handler(Rc::clone(&component_properties), &context, args.clone());
                });
            }

            if let Some(parent) = &self.parent_expanded_node.upgrade() {
                parent.$fn_name(args, globals);
            }
        }
    };
}

impl ExpandedNode {
    pub fn new(
        template: Rc<dyn InstanceNode>,
        env: Rc<RuntimePropertiesStackFrame>,
        context: &mut RuntimeContext,
        parent_expanded_node: Weak<ExpandedNode>,
        containing_component: Weak<ExpandedNode>,
    ) -> Rc<Self> {
        let properties = (&template.base().instance_prototypical_properties_factory)();
        let common_properties = (&template
            .base()
            .instance_prototypical_common_properties_factory)();

        let root = Rc::new(ExpandedNode {
            id_chain: vec![context.gen_uid().0],
            instance_template: Rc::clone(&template),
            properties,
            common_properties,
            stack: env,

            parent_expanded_node,
            containing_component,

            children: RefCell::new(BTreeSet::new()),
            computed_expanded_properties: RefCell::new(None),
        });

        root.initialize_children(context);
        root.mount(context);
        root
    }

    fn initialize_children(self: &Rc<Self>, context: &mut RuntimeContext) {
        self.update(context);
        // Containers such as Frame, Group, etc don't recompute children based on values
        // and need an initial explicit call to recompute_children to populate theirs.
        // Others such as conditionals recognized that their state had changed above when
        // calling update and don't need to recompute again.
        if self.children.borrow().is_empty() {
            Rc::clone(&self.instance_template).recompute_children(&self, context);
        }
    }

    fn native_patches(&self, context: &mut RuntimeContext) {
        self.instance_template.handle_native_patches(self, context);
    }

    // This method will not need to exist when dirty-dag updates are
    // a thing. recompute_children can instead be reactively called when
    // certain values are changed
    pub fn recurse_update(self: &Rc<Self>, context: &mut RuntimeContext) {
        self.update(context);
        self.native_patches(context);
        for child in self.children.borrow().iter() {
            child.recurse_update(context);
        }
    }

    //Pre-order traversal that computes
    pub fn update(self: &Rc<Self>, context: &mut RuntimeContext) {
        // Here everything type independent is computed
        let viewport = self
            .parent_expanded_node
            .upgrade()
            .and_then(|p| {
                let props = p.computed_expanded_properties.borrow();
                props.as_ref().map(|c| c.computed_tab.clone())
            })
            .unwrap_or(context.globals().viewport.clone());

        *self.computed_expanded_properties.borrow_mut() = Some(ComputedExpandedProperties {
            computed_tab: compute_tab(self, &viewport),
            //TODO fill these in
            computed_z_index: 0,
            computed_canvas_index: 0,
        });

        self.get_common_properties()
            .borrow_mut()
            .compute_properties(&self.stack, context.expression_table());

        Rc::clone(&self.instance_template).update(&self, context);
    }

    //TODO how to render to different layers here?
    pub fn render(&self, context: &mut RuntimeContext, rc: &mut Box<dyn RenderContext>) {
        if let Some(ref registry) = self.instance_template.base().handler_registry {
            for handler in &registry.borrow().pre_render_handlers {
                handler(Rc::clone(&self.properties), &self.get_node_context(context))
            }
        }
        for child in self.children.borrow().iter().rev() {
            child.render(context, rc);
        }
        self.instance_template.render(&self, context, rc);
    }

    pub fn get_children_detatched(
        self: &Rc<Self>,
        templates: impl Iterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &mut RuntimeContext,
    ) {
        // Direct parent is none, direct containing component is none
        // Do update properties, but quiet down patches (ie don't send create/native patches events)
        // (should native_patch updates be seen as a render operation?)
    }

    pub fn attach_children(self: &Rc<Self>, children: &[Rc<ExpandedNode>]) {
        // Fire mount events
        // Set containing component down in the hierarchy up to component types
        // Set parent of direct children to this
        // Add them to the children set of this component
    }

    pub fn set_children(
        self: &Rc<Self>,
        templates: impl Iterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &mut RuntimeContext,
    ) {
        let containing_component = if self.instance_template.base().flags().is_component {
            Rc::downgrade(&self)
        } else {
            Weak::clone(&self.containing_component)
        };
        let mut expanded_children = self.children.borrow_mut();

        while let Some(node) = expanded_children.pop_first() {
            node.unmount(context);
        }
        // TODO run unmount handlers for these children here

        expanded_children.clear();
        for (template, env) in templates {
            expanded_children.insert(Self::new(
                template,
                env,
                context,
                Rc::downgrade(self),
                Weak::clone(&containing_component),
            ));
        }
    }

    /// Manages unpacking an Rc<RefCell<dyn Any>>, downcasting into
    /// the parameterized `target_type`, and executing a provided closure `body` in the
    /// context of that unwrapped variant (including support for mutable operations),
    /// the closure is executed.  Used at least by calculating properties in `expand_node` and
    /// passing `&mut self` into event handlers (where the typed `self` is retrieved from an instance of `dyn Any`)
    pub fn with_properties_unwrapped<T: 'static, R>(
        &self,
        callback: impl FnOnce(&mut T) -> R,
    ) -> R {
        // Borrow the contents of the RefCell mutably.
        let mut borrowed = self.properties.borrow_mut();

        // Downcast the unwrapped value to the specified `target_type` (or panic)
        let mut unwrapped_value = if let Some(val) = borrowed.downcast_mut::<T>() {
            val
        } else {
            panic!() //Failed to downcast
        };
        callback(&mut unwrapped_value)
    }

    fn mount(&self, context: &mut RuntimeContext) {
        self.instance_template.handle_mount(self, context);
        if let Some(ref registry) = self.instance_template.base().handler_registry {
            for handler in &registry.borrow().mount_handlers {
                handler(Rc::clone(&self.properties), &self.get_node_context(context))
            }
        }
    }

    fn unmount(&self, context: &mut RuntimeContext) {
        while let Some(child) = self.children.borrow_mut().pop_first() {
            child.unmount(context);
        }
        self.instance_template.handle_unmount(self, context);
    }

    pub fn get_node_context(&self, context: &RuntimeContext) -> NodeContext {
        let globals = context.globals();
        let computed_props = self.computed_expanded_properties.borrow();
        let bounds_self = computed_props
            .as_ref()
            .expect("node has been updated")
            .computed_tab
            .bounds;
        let parent = self.parent_expanded_node.upgrade();
        let bounds_parent = parent
            .as_ref()
            .and_then(|p| {
                let props = p.computed_expanded_properties.borrow();
                props.as_ref().map(|v| v.computed_tab.bounds)
            })
            .unwrap_or(globals.viewport.bounds);
        NodeContext {
            frames_elapsed: globals.frames_elapsed,
            bounds_self,
            bounds_parent,
        }
    }

    pub fn get_common_properties(&self) -> Rc<RefCell<CommonProperties>> {
        Rc::clone(&self.common_properties)
    }

    /// Determines whether the provided ray, orthogonal to the view plane,
    /// intersects this `ExpandedNode`.
    pub fn ray_cast_test(&self, ray: &(f64, f64)) -> bool {
        // Don't vacuously hit for `invisible_to_raycasting` nodes
        if self
            .instance_template
            .base()
            .flags()
            .invisible_to_raycasting
        {
            return false;
        }

        let props = self.computed_expanded_properties.borrow();
        let computed_tab = &props.as_ref().unwrap().computed_tab;

        let inverted_transform = computed_tab.transform.inverse();
        let transformed_ray = inverted_transform * Point { x: ray.0, y: ray.1 };

        let relevant_bounds = computed_tab.bounds;

        //Default implementation: rectilinear bounding hull
        let res = transformed_ray.x > 0.0
            && transformed_ray.y > 0.0
            && transformed_ray.x < relevant_bounds.0
            && transformed_ray.y < relevant_bounds.1;

        res
    }

    /// Returns the size of this node, or `None` if this node
    /// doesn't have a size (e.g. `Group`)
    pub fn get_size(&self) -> (Size, Size) {
        self.instance_template.get_size(self)
    }

    /// Returns the size of this node in pixels, requiring this node's containing bounds
    /// for calculation of `Percent` values
    pub fn get_size_computed(&self, bounds: (f64, f64)) -> (f64, f64) {
        let size = self.get_size();
        (
            size.0.evaluate(bounds, Axis::X),
            size.1.evaluate(bounds, Axis::Y),
        )
    }

    /// Used at least by ray-casting; only nodes that clip content (and thus should
    /// not allow outside content to respond to ray-casting) should return a value
    pub fn get_clipping_size(&self) -> Option<(Size, Size)> {
        None
    }

    /// Returns the clipping bounds of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    pub fn get_clipping_size_computed(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_clipping_size() {
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

    dispatch_event_handler!(dispatch_scroll, ArgsScroll, scroll_handlers);
    dispatch_event_handler!(dispatch_clap, ArgsClap, clap_handlers);
    dispatch_event_handler!(dispatch_touch_start, ArgsTouchStart, touch_start_handlers);

    dispatch_event_handler!(dispatch_touch_move, ArgsTouchMove, touch_move_handlers);
    dispatch_event_handler!(dispatch_touch_end, ArgsTouchEnd, touch_end_handlers);
    dispatch_event_handler!(dispatch_key_down, ArgsKeyDown, key_down_handlers);
    dispatch_event_handler!(dispatch_key_up, ArgsKeyUp, key_up_handlers);
    dispatch_event_handler!(dispatch_key_press, ArgsKeyPress, key_press_handlers);
    dispatch_event_handler!(
        dispatch_checkbox_change,
        ArgsCheckboxChange,
        checkbox_change_handlers
    );
    dispatch_event_handler!(dispatch_mouse_down, ArgsMouseDown, mouse_down_handlers);
    dispatch_event_handler!(dispatch_mouse_up, ArgsMouseUp, mouse_up_handlers);
    dispatch_event_handler!(dispatch_mouse_move, ArgsMouseMove, mouse_move_handlers);
    dispatch_event_handler!(dispatch_mouse_over, ArgsMouseOver, mouse_over_handlers);
    dispatch_event_handler!(dispatch_mouse_out, ArgsMouseOut, mouse_out_handlers);
    dispatch_event_handler!(
        dispatch_double_click,
        ArgsDoubleClick,
        double_click_handlers
    );
    dispatch_event_handler!(
        dispatch_context_menu,
        ArgsContextMenu,
        context_menu_handlers
    );
    dispatch_event_handler!(dispatch_click, ArgsClick, click_handlers);
    dispatch_event_handler!(dispatch_wheel, ArgsWheel, wheel_handlers);
}

pub struct Renderer<R> {
    pub backend: R,
}

impl<R: piet::RenderContext> pax_runtime_api::RenderContext for Renderer<R> {
    fn fill(&mut self, path: kurbo::BezPath, brush: &piet_common::PaintBrush) {
        self.backend.fill(path, brush);
    }

    fn stroke(&mut self, path: kurbo::BezPath, brush: &piet_common::PaintBrush, width: f64) {
        self.backend.stroke(path, brush, width);
    }

    fn save(&mut self) {
        self.backend.save().expect("failed to save piet state");
    }

    fn clip(&mut self, path: kurbo::BezPath) {
        self.backend.clip(path);
    }

    fn restore(&mut self) {
        self.backend
            .restore()
            .expect("failed to restore piet state");
    }
}

pub struct ExpressionTable {
    pub table: HashMap<usize, Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>>,
}

impl ExpressionTable {
    pub fn compute_vtable_value(
        &self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        vtable_id: usize,
    ) -> Box<dyn Any> {
        if let Some(evaluator) = self.table.get(&vtable_id) {
            let stack_frame = Rc::clone(stack);
            let ec = ExpressionContext { stack_frame };
            (**evaluator)(ec)
        } else {
            panic!() //unhandled error if an invalid id is passed or if vtable is incorrectly initialized
        }
    }

    pub fn compute_eased_value<T: Clone + Interpolatable>(
        &self,
        transition_manager: Option<&mut TransitionManager<T>>,
        globals: &Globals,
    ) -> Option<T> {
        if let Some(tm) = transition_manager {
            if tm.queue.len() > 0 {
                let current_transition = tm.queue.get_mut(0).unwrap();
                if let None = current_transition.global_frame_started {
                    current_transition.global_frame_started = Some(globals.frames_elapsed);
                }
                let progress = (1.0 + globals.frames_elapsed as f64
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
                    self.compute_eased_value(Some(tm), globals)
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

/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl PaxEngine {
    pub fn new(
        main_component_instance: Rc<ComponentInstance>,
        expression_table: ExpressionTable,
        logger: pax_runtime_api::PlatformSpecificLogger,
        viewport_size: (f64, f64),
    ) -> Self {
        pax_runtime_api::register_logger(logger);

        let globals = Globals {
            frames_elapsed: 0,
            viewport: TransformAndBounds {
                transform: Affine::default(),
                bounds: viewport_size,
            },
        };
        let root_env =
            RuntimePropertiesStackFrame::new(Rc::new(RefCell::new(())) as Rc<RefCell<dyn Any>>);

        let mut runtime_context = RuntimeContext::new(expression_table, globals);

        let root_node = ExpandedNode::new(
            main_component_instance,
            root_env,
            &mut runtime_context,
            Weak::new(),
            Weak::new(),
        );

        PaxEngine {
            runtime_context,
            root_node,
            image_map: HashMap::new(),
            z_index_node_cache: BinaryHeap::new(),
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
    pub fn tick(
        &mut self,
        rcs: &mut HashMap<String, Box<dyn RenderContext>>,
    ) -> Vec<NativeMessage> {
        //
        // 1. UPDATE NODES (properties, etc.). This part we should be able to
        // completely remove once reactive properties dirty-dag is a thing.
        //
        self.root_node.recurse_update(&mut self.runtime_context);

        //
        // 2. LAYER-IDS, z-index list creation Will always be recomputed each
        // frame. Nothing intensive is to be done here. Info is not stored on
        // the nodes, but in a separate datastructure.
        //
        // let mut z_index_gen = 0..;
        // let mut z_index = LayerId::new(None);
        self.z_index_node_cache.clear();
        fn assign_z_indicies(
            n: &Rc<ExpandedNode>,
            i: &mut u32,
            cache: &mut BinaryHeap<(u32, Rc<ExpandedNode>)>,
        ) {
            for child in n.children.borrow().iter().rev() {
                assign_z_indicies(child, i, cache);
            }
            cache.push((*i, Rc::clone(&n)));
        }
        assign_z_indicies(&self.root_node, &mut 0, &mut self.z_index_node_cache);

        //
        // 3. RENDER
        // Render as a function of the now-computed ExpandedNode tree.
        //
        for (_, node) in &self.z_index_node_cache {
            node.render(
                &mut self.runtime_context,
                &mut rcs.values_mut().next().unwrap(),
            );
        }
        //TODOSAMS redo rendering logic as a method call on the root expanded node as well

        self.runtime_context.take_native_messages()
    }

    /// Simple 2D raycasting: the coordinates of the ray represent a
    /// ray running orthogonally to the view plane, intersecting at
    /// the specified point `ray`.  Areas outside of clipping bounds will
    /// not register a `hit`, nor will elements that suppress input events.
    pub fn get_topmost_element_beneath_ray(&self, ray: (f64, f64)) -> Option<Rc<ExpandedNode>> {
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        //Instead of storing a pointer to `last_rtc`, we should store a custom
        //struct with exactly the fields we need for ray-casting

        let mut ret: Option<Rc<ExpandedNode>> = None;
        for (_, node) in self.z_index_node_cache.iter().rev().skip(1) {
            if node.ray_cast_test(&ray) {
                //We only care about the topmost node getting hit, and the element
                //pool is ordered by z-index so we can just resolve the whole
                //calculation when we find the first matching node

                let mut ancestral_clipping_bounds_are_satisfied = true;
                let mut parent: Option<Rc<ExpandedNode>> = node.parent_expanded_node.upgrade();

                loop {
                    if let Some(unwrapped_parent) = parent {
                        if let Some(_) = unwrapped_parent.get_clipping_size() {
                            ancestral_clipping_bounds_are_satisfied =
                            //clew
                                (*unwrapped_parent).ray_cast_test(&ray);
                            break;
                        }
                        parent = unwrapped_parent.parent_expanded_node.upgrade();
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

    pub fn get_focused_element(&self) -> Option<Rc<ExpandedNode>> {
        let (x, y) = self.runtime_context.globals().viewport.bounds;
        self.get_topmost_element_beneath_ray((x / 2.0, y / 2.0))
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.runtime_context.globals_mut().viewport.bounds = new_viewport_size;
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
