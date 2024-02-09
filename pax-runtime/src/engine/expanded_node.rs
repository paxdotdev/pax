use crate::constants::{
    BUTTON_CLICK_HANDLERS, CHECKBOX_CHANGE_HANDLERS, CLAP_HANDLERS, CLICK_HANDLERS,
    CONTEXT_MENU_HANDLERS, DOUBLE_CLICK_HANDLERS, KEY_DOWN_HANDLERS, KEY_PRESS_HANDLERS,
    KEY_UP_HANDLERS, MOUSE_DOWN_HANDLERS, MOUSE_MOVE_HANDLERS, MOUSE_OUT_HANDLERS,
    MOUSE_OVER_HANDLERS, MOUSE_UP_HANDLERS, SCROLL_HANDLERS, TEXTBOX_CHANGE_HANDLERS,
    TOUCH_END_HANDLERS, TOUCH_MOVE_HANDLERS, TOUCH_START_HANDLERS, WHEEL_HANDLERS,
};
use crate::Globals;
use core::fmt;
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use kurbo::Point;

use crate::api::{
    ArgsButtonClick, ArgsCheckboxChange, ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick,
    ArgsKeyDown, ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut,
    ArgsMouseOver, ArgsMouseUp, ArgsScroll, ArgsTextboxChange, ArgsTouchEnd, ArgsTouchMove,
    ArgsTouchStart, ArgsWheel, Axis, CommonProperties, NodeContext, RenderContext, Size,
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
    pub instance_node: InstanceNodePtr,

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

    /// Properties that are currently re-computed each frame before rendering.
    /// Only contains computed_tab atm. Might be possible to retire if tab comp
    /// would be part of render pass?
    pub layout_properties: RefCell<Option<LayoutProperties>>,

    /// For component instances only, tracks the expanded slot_children in it's
    /// non-collapsed form (repeat and conditionals still present). This allows
    /// repeat/conditionals to update their children (handled in component.rs
    /// update_children method)
    pub expanded_slot_children: RefCell<Option<Vec<Rc<ExpandedNode>>>>,
    /// Flattened version of the above, where repeat/conditionals are removed
    /// recursively and replaced by their children. This is re-computed each
    /// frame from the non-collapsed expanded_slot_children after they have
    /// been updated.
    pub expanded_and_flattened_slot_children: RefCell<Option<Vec<Rc<ExpandedNode>>>>,

    /// Flag that is > 0 if this node is part of the root tree. If it is,
    /// updates to this nodes children also marks them as attached (+1), triggering
    /// mount and dismount on addition/removal. This is needed mainly for slot,
    /// since an entire "shadow tree" needs to be expanded and updated for
    /// each slot child, but only the ones that have a "connected" slot should
    /// trigger mount/dismount updates
    pub attached: RefCell<u32>,

    /// Occlusion layer for this node. Used by canvas elements to decide what canvas to draw on, and
    /// by native elements to move to the correct native layer.
    pub occlusion_id: RefCell<u32>,
}

macro_rules! dispatch_event_handler {
    ($fn_name:ident, $arg_type:ty, $handler_key:ident) => {
        pub fn $fn_name(&self, args: $arg_type, globals: &Globals) {
            if let Some(registry) = self.instance_node.base().get_handler_registry() {
                let component_properties = if let Some(cc) = self.containing_component.upgrade() {
                    Rc::clone(&cc.properties)
                } else {
                    Rc::clone(&self.properties)
                };

                let comp_props = self.layout_properties.borrow();
                let bounds_self = comp_props.as_ref().unwrap().computed_tab.bounds;
                let bounds_parent = self
                    .parent_expanded_node
                    .borrow()
                    .upgrade()
                    .map(|parent| {
                        let comp_props = parent.layout_properties.borrow();
                        let bounds_parent = comp_props.as_ref().unwrap().computed_tab.bounds;
                        bounds_parent
                    })
                    .unwrap_or(globals.viewport.bounds);
                let context = NodeContext {
                    bounds_self,
                    bounds_parent,
                    frames_elapsed: globals.frames_elapsed,
                    #[cfg(feature = "designtime")]
                    designtime: globals.designtime.clone(),
                };

                let borrowed_registry = &(*registry).borrow();
                    if let Some(handlers) = borrowed_registry.handlers.get($handler_key) {
                        handlers.iter().for_each(|handler| {
                            handler(
                                Rc::clone(&component_properties),
                                &context,
                                Some(Box::new(args.clone()) as Box<dyn Any>),
                            );
                        });
                    };
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
        let root_node = Self::new(template, root_env, context, Weak::new());
        Rc::clone(&root_node).recurse_mount(context);
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

        Rc::new(ExpandedNode {
            id_chain: vec![context.gen_uid().0],
            instance_node: Rc::clone(&template),
            attached: RefCell::new(0),
            properties,
            common_properties,
            stack: env,
            parent_expanded_node: Default::default(),
            containing_component,

            children: RefCell::new(Vec::new()),
            layout_properties: RefCell::new(None),
            expanded_slot_children: Default::default(),
            expanded_and_flattened_slot_children: Default::default(),
            occlusion_id: RefCell::new(0),
        })
    }

    pub fn create_children_detached(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &mut RuntimeContext,
    ) -> Vec<Rc<ExpandedNode>> {
        let containing_component = if self.instance_node.base().flags().is_component {
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

    pub fn attach_children(
        self: &Rc<Self>,
        new_children: Vec<Rc<ExpandedNode>>,
        context: &mut RuntimeContext,
    ) {
        let mut curr_children = self.children.borrow_mut();
        //TODO here we could probably check intersection between old and new children (to avoid unmount + mount)
        if *self.attached.borrow() > 0 {
            for child in curr_children.iter() {
                Rc::clone(child).recurse_unmount(context);
            }
            for child in new_children.iter() {
                Rc::clone(child).recurse_mount(context);
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
        let new_children = self.create_children_detached(templates, context);
        self.attach_children(new_children, context);
    }

    /// This method recursively updates all node properties. When dirty-dag exists, this won't
    /// need to be here since all property dependencies can be set up and removed during mount/unmount
    pub fn recurse_update(self: &Rc<Self>, context: &mut RuntimeContext) {
        self.get_common_properties()
            .borrow_mut()
            .compute_properties(&self.stack, context.expression_table());

        let viewport = self
            .parent_expanded_node
            .borrow()
            .upgrade()
            .and_then(|p| {
                let props = p.layout_properties.borrow();
                props.as_ref().map(|c| c.computed_tab.clone())
            })
            .unwrap_or(context.globals().viewport.clone());

        *self.layout_properties.borrow_mut() = Some(LayoutProperties {
            computed_tab: compute_tab(self, &viewport),
        });

        if let Some(ref registry) = self.instance_node.base().handler_registry {
            for handler in registry
                    .borrow()
                    .handlers
                    .get("tick")
                    .unwrap_or(&Vec::new())
                {
                    handler(
                        Rc::clone(&self.properties),
                        &self.get_node_context(context),
                        None,
                    )
                }
        }
        Rc::clone(&self.instance_node).update(&self, context);
        if *self.attached.borrow() > 0 {
            self.instance_node.handle_native_patches(self, context);
        }
        if let Some(ref registry) = self.instance_node.base().handler_registry {
            for handler in registry
                .borrow()
                .handlers
                .get("pre_render")
                .unwrap_or(&Vec::new())
            {
                handler(
                    Rc::clone(&self.properties),
                    &self.get_node_context(context),
                    None,
                )
            }
        }
        for child in self.children.borrow().iter() {
            child.recurse_update(context);
        }
    }

    fn recurse_mount(self: Rc<Self>, context: &mut RuntimeContext) {
        if *self.attached.borrow() == 0 {
            *self.attached.borrow_mut() += 1;
            context.lookup.insert(self.id_chain[0], Rc::clone(&self));
            self.instance_node.handle_mount(&self, context);
            if let Some(ref registry) = self.instance_node.base().handler_registry {
                    for handler in registry
                        .borrow()
                        .handlers
                        .get("mount")
                        .unwrap_or(&Vec::new())
                    {
                        handler(
                            Rc::clone(&self.properties),
                            &self.get_node_context(context),
                            None,
                        )
                    }
                
            }
        }
        for child in self.children.borrow().iter() {
            Rc::clone(child).recurse_mount(context);
        }
    }

    fn recurse_unmount(self: Rc<Self>, context: &mut RuntimeContext) {
        for child in self.children.borrow().iter() {
            Rc::clone(child).recurse_unmount(context);
        }
        if *self.attached.borrow() == 1 {
            *self.attached.borrow_mut() -= 1;
            context.lookup.remove(&self.id_chain[0]);
            self.instance_node.handle_unmount(&self, context);
        }
    }

    pub fn recurse_render(&self, context: &mut RuntimeContext, rcs: &mut dyn RenderContext) {
        for child in self.children.borrow().iter().rev() {
            child.recurse_render(context, rcs);
        }
        self.instance_node.render(&self, context, rcs);
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
        let computed_props = self.layout_properties.borrow();
        let bounds_self = computed_props
            .as_ref()
            .map(|v| v.computed_tab.bounds)
            .unwrap_or(globals.viewport.bounds);
        let parent = self.parent_expanded_node.borrow().upgrade();
        let bounds_parent = parent
            .as_ref()
            .and_then(|p| {
                let props = p.layout_properties.borrow();
                props.as_ref().map(|v| v.computed_tab.bounds)
            })
            .unwrap_or(globals.viewport.bounds);
        NodeContext {
            frames_elapsed: globals.frames_elapsed,
            bounds_self,
            bounds_parent,
            #[cfg(feature = "designtime")]
            designtime: globals.designtime.clone(),
        }
    }

    pub fn get_common_properties(&self) -> Rc<RefCell<CommonProperties>> {
        Rc::clone(&self.common_properties)
    }

    /// Determines whether the provided ray, orthogonal to the view plane,
    /// intersects this `ExpandedNode`.
    pub fn ray_cast_test(&self, ray: &(f64, f64)) -> bool {
        // Don't vacuously hit for `invisible_to_raycasting` nodes
        if self.instance_node.base().flags().invisible_to_raycasting {
            return false;
        }

        let props = self.layout_properties.borrow();
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
        self.instance_node.get_size(self)
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

    dispatch_event_handler!(
        dispatch_scroll,
        ArgsScroll,
        SCROLL_HANDLERS
    );
    dispatch_event_handler!(dispatch_clap, ArgsClap, CLAP_HANDLERS);
    dispatch_event_handler!(
        dispatch_touch_start,
        ArgsTouchStart,
        TOUCH_START_HANDLERS
    );

    dispatch_event_handler!(
        dispatch_touch_move,
        ArgsTouchMove,
        TOUCH_MOVE_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_touch_end,
        ArgsTouchEnd,
        TOUCH_END_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_key_down,
        ArgsKeyDown,
        KEY_DOWN_HANDLERS
    );
    dispatch_event_handler!(dispatch_key_up, ArgsKeyUp, KEY_UP_HANDLERS);
    dispatch_event_handler!(
        dispatch_key_press,
        ArgsKeyPress,
        KEY_PRESS_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_checkbox_change,
        ArgsCheckboxChange,
        CHECKBOX_CHANGE_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_textbox_change,
        ArgsTextboxChange,
        TEXTBOX_CHANGE_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_button_click,
        ArgsButtonClick,
        BUTTON_CLICK_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_mouse_down,
        ArgsMouseDown,
        MOUSE_DOWN_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_mouse_up,
        ArgsMouseUp,
        MOUSE_UP_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_mouse_move,
        ArgsMouseMove,
        MOUSE_MOVE_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_mouse_over,
        ArgsMouseOver,
        MOUSE_OVER_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_mouse_out,
        ArgsMouseOut,
        MOUSE_OUT_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_double_click,
        ArgsDoubleClick,
        DOUBLE_CLICK_HANDLERS
    );
    dispatch_event_handler!(
        dispatch_context_menu,
        ArgsContextMenu,
        CONTEXT_MENU_HANDLERS
    );
    dispatch_event_handler!(dispatch_click, ArgsClick, CLICK_HANDLERS);
    dispatch_event_handler!(dispatch_wheel, ArgsWheel, WHEEL_HANDLERS);
}

/// Properties that are currently re-computed each frame before rendering.
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct LayoutProperties {
    /// Computed transform and size of this ExpandedNode
    pub computed_tab: TransformAndBounds,
}

/// Given some InstanceNodePtrList, distill away all "slot-invisible" nodes (namely, `if` and `for`)
/// and return another InstanceNodePtrList with a flattened top-level list of nodes.
fn flatten_expanded_nodes_for_slot(nodes: &[Rc<ExpandedNode>]) -> Vec<Rc<ExpandedNode>> {
    let mut result = vec![];
    for node in nodes {
        if node.instance_node.base().flags().invisible_to_slot {
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
                &Fmt(|f| self.instance_node.resolve_debug(f, Some(self))),
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
