use crate::api::TextInput;
use crate::node_interface::NodeLocal;
use pax_runtime_api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Interpolatable, Percent, Property, Variable,
};

use crate::api::math::Point2;
use crate::constants::{
    BUTTON_CLICK_HANDLERS, CHECKBOX_CHANGE_HANDLERS, CLAP_HANDLERS, CLICK_HANDLERS,
    CONTEXT_MENU_HANDLERS, DOUBLE_CLICK_HANDLERS, DROP_HANDLERS, KEY_DOWN_HANDLERS,
    KEY_PRESS_HANDLERS, KEY_UP_HANDLERS, MOUSE_DOWN_HANDLERS, MOUSE_MOVE_HANDLERS,
    MOUSE_OUT_HANDLERS, MOUSE_OVER_HANDLERS, MOUSE_UP_HANDLERS, SCROLL_HANDLERS,
    TEXTBOX_CHANGE_HANDLERS, TEXTBOX_INPUT_HANDLERS, TEXT_INPUT_HANDLERS, TOUCH_END_HANDLERS,
    TOUCH_MOVE_HANDLERS, TOUCH_START_HANDLERS, WHEEL_HANDLERS,
};
use_RefCell!();
use crate::{ExpandedNodeIdentifier, Globals, LayoutProperties, TransformAndBounds};
use core::fmt;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::api::{
    ButtonClick, CheckboxChange, Clap, Click, CommonProperties, ContextMenu, DoubleClick, Drop,
    Event, KeyDown, KeyPress, KeyUp, MouseDown, MouseMove, MouseOut, MouseOver, MouseUp,
    NodeContext, RenderContext, Scroll, Size, TextboxChange, TextboxInput, TouchEnd, TouchMove,
    TouchStart, Wheel, Window,
};

use crate::{
    compute_tab, ComponentInstance, HandlerLocation, InstanceNode, InstanceNodePtr, RuntimeContext,
    RuntimePropertiesStackFrame,
};

#[derive(Clone)]
pub struct ExpandedNode {
    #[allow(dead_code)]
    /// Unique ID of this expanded node, roughly encoding an address in the tree, where the first u32 is the instance ID
    /// and the subsequent u32s represent addresses within an expanded tree via Repeat.
    pub id: ExpandedNodeIdentifier,

    /// Pointer to the unexpanded `instance_node` underlying this ExpandedNode
    pub instance_node: RefCell<InstanceNodePtr>,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode
    /// redered directly above this one.
    pub render_parent: RefCell<Weak<ExpandedNode>>,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode
    /// in the template directy aboe this one.
    pub template_parent: Weak<ExpandedNode>,

    /// Id of closest frame present in the node tree.
    /// included as a parameter on AnyCreatePatch when
    /// creating a native element to know what clipping context
    /// to attach to
    pub parent_frame: Property<Option<ExpandedNodeIdentifier>>,

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
    pub children: Property<Vec<Rc<ExpandedNode>>>,

    /// A list of mounted children that need to be dismounted when children is recalculated
    pub mounted_children: RefCell<Vec<Rc<ExpandedNode>>>,

    /// Each ExpandedNode has a unique "stamp" of computed properties
    pub properties: RefCell<Rc<RefCell<PaxAny>>>,

    /// Each ExpandedNode has unique, computed `CommonProperties`
    common_properties: RefCell<Rc<RefCell<CommonProperties>>>,
    /// Set by chassis, for for example text nodes that get resize info from an interrupt
    /// if a node doesn't have fixed bounds(width/height specified), this value is used instead.
    pub rendered_size: Property<Option<(f64, f64)>>,

    /// The layout information (width, height, transform) used to render this node.
    /// computed property based on parent bounds + common properties
    pub transform_and_bounds: Property<TransformAndBounds<NodeLocal, Window>>,

    /// For component instances only, tracks the expanded slot_children in its
    /// non-collapsed form (repeat and conditionals still present). This allows
    /// repeat/conditionals to update their children (handled in component.rs
    /// update_children method)
    pub expanded_slot_children: RefCell<Option<Vec<Rc<ExpandedNode>>>>,
    /// Flattened version of the above, where repeat/conditionals are removed
    /// recursively and replaced by their children. This is re-computed each
    /// frame from the non-collapsed expanded_slot_children after they have
    /// been updated.
    pub expanded_and_flattened_slot_children: Property<Vec<Rc<ExpandedNode>>>,
    // Number of expanded and flattened slot children
    pub flattened_slot_children_count: Property<usize>,

    /// Flag that is > 0 if this node is part of the root tree. If it is,
    /// updates to this nodes children also marks them as attached (+1), triggering
    /// mount and dismount on addition/removal. This is needed mainly for slot,
    /// since an entire "shadow tree" needs to be expanded and updated for
    /// each slot child, but only the ones that have a "connected" slot should
    /// trigger mount/dismount updates
    pub attached: Cell<u32>,

    /// Occlusion layer for this node. Used by canvas elements to decide what canvas to draw on, and
    /// by native elements to move to the correct native layer.
    // occlusionID (clanvas/native layer) + z-index
    pub occlusion: Cell<Occlusion>,

    /// A map of all properties available on this expanded node.
    /// Used by the RuntimePropertiesStackFrame to resolve symbols.
    pub properties_scope: RefCell<HashMap<String, Variable>>,

    /// The flattened index of this node in its container (if this container
    /// cares about slot children, ex: component, path).
    pub slot_index: Property<Option<usize>>,

    /// property used to "freeze" (stop firing tick) on a node and all it's children
    pub suspended: Property<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Occlusion {
    pub occlusion_layer_id: u32,
    pub z_index: i32,
    // this is used to perform last patches logic,
    // and is updated to reflect this nodes parent_frame
    // when occlusion is calculated
    pub parent_frame: Option<u32>,
}

impl ImplToFromPaxAny for ExpandedNode {}
impl Interpolatable for ExpandedNode {}

macro_rules! dispatch_event_handler {
    ($fn_name:ident, $arg_type:ty, $handler_key:ident, $recurse:expr) => {
        pub fn $fn_name(
            &self,
            event: Event<$arg_type>,
            globals: &Globals,
            ctx: &Rc<RuntimeContext>,
        ) -> bool {
            if let Some(registry) = borrow!(self.instance_node).base().get_handler_registry() {
                let borrowed_registry = &borrow!(*registry);
                if let Some(handlers) = borrowed_registry.handlers.get($handler_key) {
                    if handlers.len() > 0 {
                        let component_properties =
                            if let Some(cc) = self.containing_component.upgrade() {
                                Rc::clone(&*borrow!(cc.properties))
                            } else {
                                Rc::clone(&*borrow!(self.properties))
                            };

                        let context = self.get_node_context(ctx);
                        handlers.iter().for_each(|handler| {
                            let properties = if let HandlerLocation::Component = &handler.location {
                                Rc::clone(&*borrow!(self.properties))
                            } else {
                                Rc::clone(&component_properties)
                            };
                            (handler.function)(
                                Rc::clone(&properties),
                                &context,
                                Some(event.clone().to_pax_any()),
                            );
                        });
                    }
                };
            }

            if $recurse {
                if let Some(parent) = self.template_parent.upgrade() {
                    return parent.$fn_name(event, globals, ctx);
                }
            }
            event.cancelled()
        }
    };
}

impl ExpandedNode {
    pub fn root(template: Rc<ComponentInstance>, ctx: &Rc<RuntimeContext>) -> Rc<Self> {
        let root_env = RuntimePropertiesStackFrame::new(
            HashMap::new(),
            Rc::new(RefCell::new(().to_pax_any())),
        );
        let root_node = Self::new(template, root_env, ctx, Weak::new(), Weak::new());
        Rc::clone(&root_node).recurse_mount(ctx);
        let globals = ctx.globals();
        let container_transform_and_bounds = globals.viewport.clone();
        let layout_properties = root_node.layout_properties();
        let transform_and_bounds = compute_tab(
            layout_properties,
            Property::default(),
            container_transform_and_bounds,
        );
        root_node
            .transform_and_bounds
            .replace_with(transform_and_bounds);
        root_node
    }

    fn new(
        template: Rc<dyn InstanceNode>,
        env: Rc<RuntimePropertiesStackFrame>,
        context: &Rc<RuntimeContext>,
        containing_component: Weak<ExpandedNode>,
        parent: Weak<ExpandedNode>,
    ) -> Rc<Self> {
        let properties = (&template.base().instance_prototypical_properties_factory)(env.clone());
        let common_properties =
            (&template
                .base()
                .instance_prototypical_common_properties_factory)(env.clone());

        let mut property_scope = borrow!(*common_properties).retrieve_property_scope();

        if let Some(scope) = &template.base().properties_scope_factory {
            property_scope.extend(scope(properties.clone()));
        }

        let id = context.gen_uid();
        let res = Rc::new(ExpandedNode {
            id,
            stack: env,
            instance_node: RefCell::new(Rc::clone(&template)),
            attached: Cell::new(0),
            properties: RefCell::new(properties),
            common_properties: RefCell::new(common_properties),
            rendered_size: Property::default(),

            // these two refer to their rendering parent, not their
            // template parent
            render_parent: Default::default(),
            parent_frame: Default::default(),
            template_parent: parent,

            containing_component,
            children: Property::new_with_name(
                Vec::new(),
                &format!("node children (node id: {})", id.0),
            ),
            mounted_children: RefCell::new(Vec::new()),
            transform_and_bounds: Property::new(TransformAndBounds::default()),
            expanded_slot_children: Default::default(),
            expanded_and_flattened_slot_children: Default::default(),
            flattened_slot_children_count: Property::new(0),
            occlusion: Default::default(),
            properties_scope: RefCell::new(property_scope),
            slot_index: Property::default(),
            suspended: Property::new(false),
        });
        res
    }

    pub fn recreate_with_new_data(
        self: &Rc<Self>,
        template: Rc<dyn InstanceNode>,
        context: &Rc<RuntimeContext>,
    ) {
        Rc::clone(self).recurse_unmount(context);
        let new_expanded_node = Self::new(
            template.clone(),
            Rc::clone(&self.stack),
            context,
            Weak::clone(&self.containing_component),
            Weak::clone(&self.template_parent),
        );
        *borrow_mut!(self.instance_node) = Rc::clone(&*borrow!(new_expanded_node.instance_node));
        *borrow_mut!(self.properties) = Rc::clone(&*borrow!(new_expanded_node.properties));
        *borrow_mut!(self.properties_scope) = borrow!(new_expanded_node.properties_scope).clone();
        *borrow_mut!(self.common_properties) =
            Rc::clone(&*borrow!(new_expanded_node.common_properties));
        self.occlusion.set(Default::default());

        Rc::clone(self).recurse_mount(context);
        Rc::clone(self).recurse_update(context);
        self.bind_to_parent_bounds();
    }

    /// Returns whether this node is a descendant of the ExpandedNode described by `other_expanded_node_id` (id)
    /// Currently requires traversing linked list of ancestory, incurring a O(log(n)) cost for a tree of `n` elements.
    /// This could be mitigated with caching/memoization, perhaps by storing a HashSet on each ExpandedNode describing its ancestory chain.
    pub fn is_descendant_of(&self, other_expanded_node_id: &ExpandedNodeIdentifier) -> bool {
        if let Some(parent) = borrow!(self.render_parent).upgrade() {
            // We have a parent — if it matches the ID, this node is indeed an ancestor of other_expanded_node_id.  Otherwise, recurse upward.
            if parent.id.eq(other_expanded_node_id) {
                true
            } else {
                parent.is_descendant_of(other_expanded_node_id)
            }
        } else {
            false
        }
    }

    pub fn create_children_detached(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &Rc<RuntimeContext>,
        template_parent: &Weak<ExpandedNode>,
    ) -> Vec<Rc<ExpandedNode>> {
        let containing_component = if borrow!(self.instance_node).base().flags().is_component {
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
                Weak::clone(&template_parent),
            ));
        }
        children
    }

    pub fn attach_children(
        self: &Rc<Self>,
        new_children: Vec<Rc<ExpandedNode>>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
    ) -> Vec<Rc<ExpandedNode>> {
        let mut curr_children = borrow_mut!(self.mounted_children);
        //TODO here we could probably check intersection between old and new children (to avoid unmount + mount)

        for child in new_children.iter() {
            // set parent and connect up viewport bounds to new parent
            *borrow_mut!(child.render_parent) = Rc::downgrade(self);
            // set frame clipping reference
            let parent_frame = parent_frame.clone();
            let deps = [parent_frame.untyped()];
            child
                .parent_frame
                .replace_with(Property::computed(move || parent_frame.get(), &deps));

            // suspension is used in the designer to turn of/on tick/update
            let cp = self.get_common_properties();
            let self_suspended = borrow!(cp)._suspended.clone();
            let parent_suspended = self.suspended.clone();
            let deps = [parent_suspended.untyped(), self_suspended.untyped()];
            child.suspended.replace_with(Property::computed(
                move || {
                    self_suspended
                        .get()
                        .unwrap_or_else(|| parent_suspended.get())
                },
                &deps,
            ));
        }
        if self.attached.get() > 0 {
            for child in curr_children.iter() {
                Rc::clone(child).recurse_unmount(context);
            }
            for child in new_children.iter() {
                Rc::clone(child).recurse_mount(context);
                child.bind_to_parent_bounds();
            }
        }
        *curr_children = new_children.clone();
        new_children
    }

    fn bind_to_parent_bounds(self: &Rc<Self>) {
        let parent = borrow!(self.render_parent).upgrade().unwrap();
        let parent_transform_and_bounds = parent.transform_and_bounds.clone();
        let common_props = borrow!(self.common_properties);
        let extra_transform = borrow!(common_props).transform.clone();

        let transform_and_bounds = compute_tab(
            self.layout_properties(),
            extra_transform,
            parent_transform_and_bounds,
        );
        self.transform_and_bounds.replace_with(transform_and_bounds);
    }

    pub fn generate_children(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
    ) -> Vec<Rc<ExpandedNode>> {
        let new_children = self.create_children_detached(templates, context, &Rc::downgrade(&self));
        let res = self.attach_children(new_children, context, parent_frame);
        res
    }

    /// This method recursively updates all node properties. When dirty-dag exists, this won't
    /// need to be here since all property dependencies can be set up and removed during mount/unmount
    pub fn recurse_update(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
            if !self.suspended.get() {
                for handler in borrow!(registry)
                    .handlers
                    .get("tick")
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }
        }
        Rc::clone(&*borrow!(self.instance_node)).update(&self, context);
        if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
            if !self.suspended.get() {
                for handler in borrow!(registry)
                    .handlers
                    .get("pre_render")
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }
        }
        for child in self.children.get().iter() {
            child.recurse_update(context);
        }
    }

    pub fn recurse_mount(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if self.attached.get() == 0 {
            self.attached.set(self.attached.get() + 1);
            context.add_to_cache(&self);
            borrow!(self.instance_node)
                .clone()
                .handle_mount(&self, context);
            if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
                for handler in borrow!(registry)
                    .handlers
                    .get("mount")
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }
            // Mount slot children and children AFTER mounting self
            if let Some(slot_children) = borrow!(self.expanded_slot_children).as_ref() {
                for slot_child in slot_children {
                    slot_child.recurse_mount(context);
                }
            }
            // this is needed to reslove slot connections in a single tick (otherwise
            // self.compute_flattened_slot_children() isn't run)
            self.recurse_update(context);
        }
    }

    pub fn recurse_unmount(self: Rc<Self>, context: &Rc<RuntimeContext>) {
        // WARNING: do NOT make recurse_unmount result in expr evaluation,
        // in this case: do not refer to self.children expression.
        // expr evaluation in this context can trigger get's of "old data", ie try to get
        // an index of a for loop source that doesn't exist anymore
        if self.attached.get() == 1 {
            self.attached.set(self.attached.get() - 1);
            context.remove_from_cache(&self);
            for child in borrow!(self.mounted_children).iter() {
                Rc::clone(child).recurse_unmount(context);
            }
            borrow!(self.instance_node).handle_unmount(&self, context);
            if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
                for handler in borrow!(registry)
                    .handlers
                    .get("unmount")
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }
            // Needed because occlusion updates are only sent on diffs so we reset it when unmounting
            self.occlusion.set(Default::default());
        }
    }

    pub fn recurse_render(&self, ctx: &Rc<RuntimeContext>, rcs: &mut dyn RenderContext) {
        borrow!(self.instance_node).handle_pre_render(&self, ctx, rcs);
        for child in self.children.get().iter().rev() {
            child.recurse_render(ctx, rcs);
        }
        borrow!(self.instance_node).render(&self, ctx, rcs);
        borrow!(self.instance_node).handle_post_render(&self, ctx, rcs);
    }

    /// Manages unpacking an Rc<RefCell<PaxValue>>, downcasting into
    /// the parameterized `target_type`, and executing a provided closure `body` in the
    /// context of that unwrapped variant (including support for mutable operations),
    /// the closure is executed.  Used at least by calculating properties in `expand_node` and
    /// passing `&mut self` into event handlers (where the typed `self` is retrieved from an instance of `PaxValue`)
    pub fn with_properties_unwrapped<T: ToFromPaxAny, R>(
        &self,
        callback: impl FnOnce(&mut T) -> R,
    ) -> R {
        self.try_with_properties_unwrapped(callback)
            .expect("properties not of expected type")
    }

    pub fn try_with_properties_unwrapped<T: ToFromPaxAny, R>(
        &self,
        callback: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        // Borrow the contents of the RefCell mutably.
        let properties = borrow_mut!(self.properties);
        let mut borrowed = borrow_mut!(properties);
        // Downcast the unwrapped value to the specified `target_type` (or panic)
        let Ok(mut val) = T::mut_from_pax_any(&mut *borrowed) else {
            return None;
        };
        Some(callback(&mut val))
    }

    pub fn recurse_visit_postorder(self: &Rc<Self>, func: &mut impl FnMut(&Rc<Self>)) {
        // NOTE: This is to make sure that the slot children list is updated before trying to access children,
        // to make stacker/scroller behave correctly when number of children is dynamic. (ex: tree view in designer)
        self.compute_flattened_slot_children();
        for child in self.children.get().iter().rev() {
            child.recurse_visit_postorder(func)
        }
        func(self);
    }

    pub fn get_node_context<'a>(&'a self, ctx: &Rc<RuntimeContext>) -> NodeContext {
        let globals = ctx.globals();
        let t_and_b = self.transform_and_bounds.clone();
        let deps = [t_and_b.untyped()];
        let bounds_self = Property::computed(move || t_and_b.get().bounds, &deps);
        let t_and_b_parent = if let Some(parent) = borrow!(self.render_parent).upgrade() {
            parent.transform_and_bounds.clone()
        } else {
            globals.viewport.clone()
        };
        let deps = [t_and_b_parent.untyped()];
        let bounds_parent = Property::computed(move || t_and_b_parent.get().bounds, &deps);

        let slot_children_count = if borrow!(self.instance_node).base().flags().is_component {
            self.flattened_slot_children_count.clone()
        } else {
            self.containing_component
                .upgrade()
                .map(|v| v.flattened_slot_children_count.clone())
                .unwrap_or_default()
        };

        NodeContext {
            slot_index: self.slot_index.clone(),
            local_stack_frame: Rc::clone(&self.stack),
            component_origin: Weak::clone(&self.containing_component),
            frames_elapsed: globals.frames_elapsed.clone(),
            bounds_self,
            bounds_parent,
            runtime_context: ctx.clone(),
            platform: globals.platform.clone(),
            os: globals.os.clone(),
            slot_children_count,
            node_transform_and_bounds: self.transform_and_bounds.get(),
            #[cfg(feature = "designtime")]
            designtime: globals.designtime.clone(),
        }
    }

    pub fn get_common_properties(&self) -> Rc<RefCell<CommonProperties>> {
        Rc::clone(&*borrow!(self.common_properties))
    }

    /// Determines whether the provided ray, orthogonal to the view plane,
    /// intersects this `ExpandedNode`.
    pub fn ray_cast_test(&self, ray: Point2<Window>) -> bool {
        let cp = borrow!(self.common_properties);

        // skip raycast if false
        if !borrow!(&*cp)._raycastable.get().unwrap_or(true) {
            return false;
        }
        let t_and_b = self.transform_and_bounds.get();

        let inverted_transform = t_and_b.transform.inverse();
        let transformed_ray = inverted_transform * ray;
        let (width, height) = t_and_b.bounds;
        //Default implementation: rectilinear bounding hull
        let res = transformed_ray.x > 0.0
            && transformed_ray.y > 0.0
            && transformed_ray.x < width
            && transformed_ray.y < height;
        res
    }

    /// Returns the scroll offset from a Scroller component
    /// Used by the engine to transform its children
    pub fn get_scroll_offset(&mut self) -> (f64, f64) {
        // (0.0, 0.0)
        todo!("patch into an ExpandedNode-friendly way to track this state");
    }

    pub fn compute_flattened_slot_children(&self) {
        // All of this should ideally be reactively updated,
        // but currently doesn't exist a way to "listen to"
        // an entire node tree, and generate the flattened list
        // only when changed.
        if let Some(slot_children) = borrow!(self.expanded_slot_children).as_ref() {
            let new_flattened = flatten_expanded_nodes_for_slot(&slot_children);
            let old_and_new_filtered_same =
                self.expanded_and_flattened_slot_children.read(|flattened| {
                    flattened
                        .iter()
                        .map(|n| n.id)
                        .eq(new_flattened.iter().map(|n| n.id))
                });

            if !old_and_new_filtered_same {
                self.flattened_slot_children_count.set(new_flattened.len());
                self.expanded_and_flattened_slot_children.set(new_flattened);
                for (i, slot_child) in self
                    .expanded_and_flattened_slot_children
                    .get()
                    .iter()
                    .enumerate()
                {
                    if slot_child.slot_index.get() != Some(i) {
                        slot_child.slot_index.set(Some(i));
                    };
                }
            }
        }
    }

    dispatch_event_handler!(dispatch_scroll, Scroll, SCROLL_HANDLERS, true);
    dispatch_event_handler!(dispatch_clap, Clap, CLAP_HANDLERS, true);
    dispatch_event_handler!(dispatch_touch_start, TouchStart, TOUCH_START_HANDLERS, true);

    dispatch_event_handler!(dispatch_touch_move, TouchMove, TOUCH_MOVE_HANDLERS, true);
    dispatch_event_handler!(dispatch_touch_end, TouchEnd, TOUCH_END_HANDLERS, true);
    dispatch_event_handler!(dispatch_key_down, KeyDown, KEY_DOWN_HANDLERS, false);
    dispatch_event_handler!(dispatch_key_up, KeyUp, KEY_UP_HANDLERS, false);
    dispatch_event_handler!(dispatch_key_press, KeyPress, KEY_PRESS_HANDLERS, false);
    dispatch_event_handler!(
        dispatch_checkbox_change,
        CheckboxChange,
        CHECKBOX_CHANGE_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_textbox_change,
        TextboxChange,
        TEXTBOX_CHANGE_HANDLERS,
        true
    );
    dispatch_event_handler!(dispatch_text_input, TextInput, TEXT_INPUT_HANDLERS, true);
    dispatch_event_handler!(
        dispatch_textbox_input,
        TextboxInput,
        TEXTBOX_INPUT_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_button_click,
        ButtonClick,
        BUTTON_CLICK_HANDLERS,
        true
    );
    dispatch_event_handler!(dispatch_mouse_down, MouseDown, MOUSE_DOWN_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_up, MouseUp, MOUSE_UP_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_move, MouseMove, MOUSE_MOVE_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_over, MouseOver, MOUSE_OVER_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_out, MouseOut, MOUSE_OUT_HANDLERS, true);
    dispatch_event_handler!(
        dispatch_double_click,
        DoubleClick,
        DOUBLE_CLICK_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_context_menu,
        ContextMenu,
        CONTEXT_MENU_HANDLERS,
        true
    );
    dispatch_event_handler!(dispatch_click, Click, CLICK_HANDLERS, true);
    dispatch_event_handler!(dispatch_wheel, Wheel, WHEEL_HANDLERS, true);
    dispatch_event_handler!(dispatch_drop, Drop, DROP_HANDLERS, true);

    pub fn dispatch_custom_event(
        &self,
        identifier: &str,
        ctx: &Rc<RuntimeContext>,
    ) -> Result<(), String> {
        let component_origin_instance = borrow!(self.instance_node);
        let registry = component_origin_instance
            .base()
            .handler_registry
            .as_ref()
            .ok_or_else(|| "no registry present".to_owned())?;

        let parent_component = self
            .containing_component
            .upgrade()
            .ok_or_else(|| "can't dispatch from root (has no parent)".to_owned())?;
        let properties = borrow!(parent_component.properties);

        for handler in borrow!(registry)
            .handlers
            .get(identifier)
            .expect("presence should have been checked when added to custom_event_queue")
        {
            (handler.function)(Rc::clone(&*properties), &self.get_node_context(ctx), None)
        }
        Ok(())
    }

    // fired if the chassis thinks this element should be a different size (in pixels).
    // usually, this is something the engine has asked for, ie. text nodes
    // changing size.
    pub fn chassis_resize_request(self: &Rc<ExpandedNode>, width: f64, height: f64) {
        self.rendered_size.set(Some((width, height)));
    }

    /// Helper method that returns a collection of common properties
    /// related to layout (position, size, scale, anchor, etc),
    pub fn layout_properties(self: &Rc<ExpandedNode>) -> Property<LayoutProperties> {
        let common_props = self.get_common_properties();
        let common_props = borrow!(common_props);
        let cp_width = common_props.width.clone();
        let cp_height = common_props.height.clone();
        let cp_transform = common_props.transform.clone();
        let cp_anchor_x = common_props.anchor_x.clone();
        let cp_anchor_y = common_props.anchor_y.clone();
        let cp_scale_x = common_props.scale_x.clone();
        let cp_scale_y = common_props.scale_y.clone();
        let cp_skew_x = common_props.skew_x.clone();
        let cp_skew_y = common_props.skew_y.clone();
        let cp_rotate = common_props.rotate.clone();
        let cp_x = common_props.x.clone();
        let cp_y = common_props.y.clone();
        let rendered_size = self.rendered_size.clone();
        let deps = [
            cp_width.untyped(),
            cp_height.untyped(),
            cp_transform.untyped(),
            cp_anchor_x.untyped(),
            cp_anchor_y.untyped(),
            cp_scale_x.untyped(),
            cp_scale_y.untyped(),
            cp_skew_x.untyped(),
            cp_skew_y.untyped(),
            cp_rotate.untyped(),
            cp_x.untyped(),
            cp_y.untyped(),
            rendered_size.untyped(),
        ];

        Property::computed(
            move || {
                // Used for auto sized text, might be used for other things later
                let fallback = rendered_size.get();
                let (w_fallback, h_fallback) = match fallback {
                    Some((wf, hf)) => (Some(wf), Some(hf)),
                    None => (None, None),
                };

                LayoutProperties {
                    x: cp_x.get(),
                    y: cp_y.get(),
                    width: cp_width
                        .get()
                        .or(w_fallback.map(|v| Size::Pixels(v.into()))),
                    height: cp_height
                        .get()
                        .or(h_fallback.map(|v| Size::Pixels(v.into()))),
                    rotate: cp_rotate.get(),
                    // TODO make the common prop only accept percent
                    scale_x: cp_scale_x
                        .get()
                        .map(|v| Percent((100.0 * v.expect_percent()).into())),
                    scale_y: cp_scale_y
                        .get()
                        .map(|v| Percent((100.0 * v.expect_percent()).into())),
                    anchor_x: cp_anchor_x.get(),
                    anchor_y: cp_anchor_y.get(),
                    skew_x: cp_skew_x.get(),
                    skew_y: cp_skew_y.get(),
                }
            },
            &deps,
        )
    }
}

/// Given some InstanceNodePtrList, distill away all "slot-invisible" nodes (namely, `if` and `for`)
/// and return another InstanceNodePtrList with a flattened top-level list of nodes.
fn flatten_expanded_nodes_for_slot(nodes: &[Rc<ExpandedNode>]) -> Vec<Rc<ExpandedNode>> {
    let mut result: Vec<Rc<ExpandedNode>> = vec![];
    for node in nodes {
        if borrow!(node.instance_node).base().flags().invisible_to_slot {
            result.extend(flatten_expanded_nodes_for_slot(
                node.children
                    .get()
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
                &Fmt(|f| borrow!(self.instance_node).resolve_debug(f, Some(self))),
            )
            .field("id", &self.id)
            .field("common_properties", &borrow!(self.common_properties))
            .field("transform_and_bounds", &self.transform_and_bounds)
            .field("children", &self.children.get().iter().collect::<Vec<_>>())
            .field(
                "slot_children",
                &self
                    .expanded_and_flattened_slot_children
                    .get()
                    .iter()
                    .map(|v| v.id)
                    .collect::<Vec<_>>(),
            )
            .field("occlusion_id", &self.occlusion.get())
            .field(
                "containing_component",
                &self.containing_component.upgrade().map(|v| v.id.clone()),
            )
            .finish()
    }
}
