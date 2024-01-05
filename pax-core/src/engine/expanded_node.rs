use crate::Globals;
use core::fmt;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use kurbo::Point;

use pax_runtime_api::{
    ArgsCheckboxChange, ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsKeyDown,
    ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver,
    ArgsMouseUp, ArgsScroll, ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, Axis,
    CommonProperties, NodeContext, RenderContext, Size,
};

use crate::{
    compute_tab, ComponentInstance, InstanceNode, InstanceNodePtr, PropertiesComputable,
    RuntimeContext, RuntimePropertiesStackFrame, TransformAndBounds,
};

pub struct ExpandedNode {
    #[allow(dead_code)]
    /// Unique ID of this expanded node, roughly encoding an address in the tree, where the first u32 is the instance ID
    /// and the subsequent u32s represent addresses within an expanded tree via Repeat.
    pub id_chain: Vec<u32>,

    /// Pointer to the unexpanded `instance_node` underlying this ExpandedNode
    pub instance_template: InstanceNodePtr,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode directly above
    /// this one.  Used for e.g. event propagation.
    pub parent_expanded_node: RefCell<Weak<ExpandedNode>>,

    /// Reference to the _component for which this `ExpandedNode` is a template member._  Used at least for
    /// getting a reference to slot_children for `slot`.  `Option`al because the very root instance node (root component, root instance node)
    /// has a corollary "root component expanded node."  That very root expanded node _does not have_ a containing ExpandedNode component,
    /// thus `containing_component` is `Option`al.
    pub containing_component: Weak<ExpandedNode>,

    /// Persistent clone of the state of the [`PropertiesTreeShared#runtime_properties_stack`] at the time that this node was expanded (this is expected to remain immutable
    /// through the lifetime of the program after the initial expansion; however, if that constraint changes, this should be
    /// explicitly updated to accommodate.)
    pub stack: Rc<RuntimePropertiesStackFrame>,

    /// Pointers to the ExpandedNode beneath this one.  Used for e.g. rendering recursion.
    children: RefCell<Vec<Rc<ExpandedNode>>>,

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

    // TODO this should probably be moved to ComponentProperties (need to modify codegen)
    /// For component instances only, tracks the expanded + flattened slot_children
    pub expanded_slot_children: RefCell<Option<Vec<Rc<ExpandedNode>>>>,
    pub expanded_and_flattened_slot_children: RefCell<Option<Vec<Rc<ExpandedNode>>>>,

    /// Flag that is true if this node is part of the root tree. If it is,
    /// updates to this nodes children also marks them as attached, triggering
    /// mount and dismount on addition/removal. This is needed mainly for slot,
    /// since an entire "shadow tree" needs to be expanded and updated for
    /// each slot child, but only the ones that have a "connected" slot should
    /// trigger mount/dismount updates
    attached: RefCell<bool>,

    // TODO this should be component prop as well?
    /// Flag that signifies that a node has done the initial expansion. Used for
    /// the default implementation of update_children on InstanceNodes that only
    /// expand once (all except for repeat/conditional)
    pub done_initial_expansion_of_children: RefCell<bool>,

    //Occlusion layer for this node. Used by canvas elements to decide what canvas to draw on, and
    // by native elements to move to the correct native layer.
    pub occlusion_id: RefCell<u32>,
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
                    .borrow()
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

            if let Some(parent) = &self.parent_expanded_node.borrow().upgrade() {
                parent.$fn_name(args, globals);
            }
        }
    };
}

impl ExpandedNode {
    pub fn root(template: Rc<ComponentInstance>, context: &mut RuntimeContext) -> Rc<Self> {
        let root_env =
            RuntimePropertiesStackFrame::new(Rc::new(RefCell::new(())) as Rc<RefCell<dyn Any>>);
        let root_node = ExpandedNode::new(template, root_env, context, Weak::new());
        root_node.recurse_mount(context);
        root_node
    }

    fn new(
        template: Rc<dyn InstanceNode>,
        env: Rc<RuntimePropertiesStackFrame>,
        context: &mut RuntimeContext,
        containing_component: Weak<ExpandedNode>,
    ) -> Rc<Self> {
        let properties = (&template.base().instance_prototypical_properties_factory)();
        let common_properties = (&template
            .base()
            .instance_prototypical_common_properties_factory)();

        let node = Rc::new(ExpandedNode {
            id_chain: vec![context.gen_uid().0],
            instance_template: Rc::clone(&template),
            attached: RefCell::new(false),
            properties,
            common_properties,
            stack: env,
            parent_expanded_node: Default::default(),
            containing_component,

            children: RefCell::new(Vec::new()),
            computed_expanded_properties: RefCell::new(None),
            expanded_slot_children: Default::default(),
            expanded_and_flattened_slot_children: Default::default(),
            done_initial_expansion_of_children: Default::default(),
            occlusion_id: RefCell::new(0),
        });

        node.update_children(context);
        node
    }

    pub fn create_children_detatched(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &mut RuntimeContext,
    ) -> Vec<Rc<ExpandedNode>> {
        let containing_component = if self.instance_template.base().flags().is_component {
            Rc::downgrade(&self)
        } else {
            Weak::clone(&self.containing_component)
        };

        let mut children = Vec::new();

        for (template, env) in templates {
            children.push(Self::new(
                template,
                env,
                context,
                Weak::clone(&containing_component),
            ));
        }
        children
    }

    // OBS this does not set the current parent or in other ways modify the stack
    pub fn attach_children(
        self: &Rc<Self>,
        new_children: Vec<Rc<ExpandedNode>>,
        context: &mut RuntimeContext,
    ) {
        let mut curr_children = self.children.borrow_mut();
        //TODO here we could probably check intersection between old and new children (to avoid unmount + mount)
        if *self.attached.borrow() {
            for child in curr_children.iter() {
                child.recurse_unmount(context);
            }
            for child in new_children.iter() {
                child.recurse_mount(context);
            }
        }
        for child in new_children.iter() {
            *child.parent_expanded_node.borrow_mut() = Rc::downgrade(self);
        }
        *curr_children = new_children;
    }

    pub fn set_children(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &mut RuntimeContext,
    ) {
        let new_children = self.create_children_detatched(templates, context);
        self.attach_children(new_children, context);
    }

    fn native_patches(&self, context: &mut RuntimeContext) {
        self.instance_template.handle_native_patches(self, context);
    }

    pub fn recurse_update(self: &Rc<Self>, context: &mut RuntimeContext) {
        self.recurse_update_children(context);
        self.recurse_update_native_patches(context);
    }

    // This method will not need to exist when dirty-dag updates are
    // a thing.
    pub fn recurse_update_children(self: &Rc<Self>, context: &mut RuntimeContext) {
        self.update_children(context);
        for child in self.children.borrow().iter() {
            child.recurse_update_children(context);
        }
    }

    pub fn update_children(self: &Rc<Self>, context: &mut RuntimeContext) {
        self.get_common_properties()
            .borrow_mut()
            .compute_properties(&self.stack, context.expression_table());

        let viewport = self
            .parent_expanded_node
            .borrow()
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

        if let Some(ref registry) = self.instance_template.base().handler_registry {
            for handler in &registry.borrow().pre_render_handlers {
                handler(Rc::clone(&self.properties), &self.get_node_context(context))
            }
        }
        Rc::clone(&self.instance_template).update_children(&self, context);
    }

    pub fn recurse_update_native_patches(&self, context: &mut RuntimeContext) {
        self.native_patches(context);
        for child in self.children.borrow().iter() {
            child.recurse_update_native_patches(context);
        }
    }

    //TODO how to render to different layers here?
    pub fn recurse_render(
        &self,
        context: &mut RuntimeContext,
        rcs: &mut HashMap<String, Box<dyn RenderContext>>,
    ) {
        for child in self.children.borrow().iter().rev() {
            child.recurse_render(context, rcs);
        }
        let rc = &mut rcs
            .get_mut(&self.occlusion_id.borrow().to_string())
            .expect("occlusion ind present");
        self.instance_template.render(&self, context, rc);
    }

    fn recurse_mount(&self, context: &mut RuntimeContext) {
        self.mount(context);
        for child in self.children.borrow().iter() {
            child.recurse_mount(context);
        }
    }

    fn mount(&self, context: &mut RuntimeContext) {
        assert_eq!(*self.attached.borrow(), false);
        *self.attached.borrow_mut() = true;
        self.instance_template.handle_mount(self, context);
        if let Some(ref registry) = self.instance_template.base().handler_registry {
            for handler in &registry.borrow().mount_handlers {
                handler(Rc::clone(&self.properties), &self.get_node_context(context))
            }
        }
    }

    fn recurse_unmount(&self, context: &mut RuntimeContext) {
        for child in self.children.borrow().iter() {
            child.recurse_unmount(context);
        }
        self.unmount(context);
    }

    fn unmount(&self, context: &mut RuntimeContext) {
        assert_eq!(*self.attached.borrow(), true);
        *self.attached.borrow_mut() = false;
        self.instance_template.handle_unmount(self, context);
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

    pub fn recurse_visit_postorder<T>(
        self: &Rc<Self>,
        func: &impl Fn(&Rc<Self>, &mut T),
        val: &mut T,
    ) {
        for child in self.children.borrow().iter().rev() {
            child.recurse_visit_postorder(func, val);
        }
        func(self, val);
    }

    pub fn get_node_context(&self, context: &RuntimeContext) -> NodeContext {
        let globals = context.globals();
        let computed_props = self.computed_expanded_properties.borrow();
        let bounds_self = computed_props
            .as_ref()
            .map(|v| v.computed_tab.bounds)
            .unwrap_or(globals.viewport.bounds);
        let parent = self.parent_expanded_node.borrow().upgrade();
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

    pub fn compute_flattened_slot_children(&self) {
        if let Some(slot_children) = self.expanded_slot_children.borrow().as_ref() {
            *self.expanded_and_flattened_slot_children.borrow_mut() =
                Some(flatten_expanded_nodes_for_slot(&slot_children));
        }
    }

    pub fn do_initial_expansion_of_children(&self) -> bool {
        let mut do_it = self.done_initial_expansion_of_children.borrow_mut();
        let old = *do_it;
        *do_it = true;
        !old
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
// Given some InstanceNodePtrList, distill away all "slot-invisible" nodes (namely, `if` and `for`)
// and return another InstanceNodePtrList with a flattened top-level list of nodes.
// Helper function that accepts a
fn flatten_expanded_nodes_for_slot(nodes: &[Rc<ExpandedNode>]) -> Vec<Rc<ExpandedNode>> {
    let mut result = vec![];
    for node in nodes {
        if node.instance_template.base().flags().invisible_to_slot {
            result.extend(flatten_expanded_nodes_for_slot(
                node.children
                    .borrow()
                    .clone()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .as_slice(),
            ));
        } else {
            result.push(Rc::clone(&node))
        }
    }
    result
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
            // .field("common_properties", &self.common_properties.try_borrow())
            // .field(
            //     "computed_expanded_properties",
            //     &self.computed_expanded_properties.try_borrow(),
            // )
            .field(
                "children",
                &self.children.try_borrow().iter().collect::<Vec<_>>(),
            )
            .field(
                "parent",
                &self
                    .parent_expanded_node
                    .borrow()
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
            .field("occlusion_id", &self.occlusion_id.borrow())
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
