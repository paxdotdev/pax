use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::RangeFrom;
use std::rc::{Rc, Weak};
use kurbo::Affine;
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use piet::RenderContext;
use pax_message::NativeMessage;
use pax_runtime_api::{RuntimeContext, Timeline, Transform2D, Size, Rotation};
use crate::{ComputableTransform, ExpandedNode, ExpressionContext, InstanceNodePtr, InstanceNodePtrList, NodeRegistry, NodeType, PaxEngine, TransformAndBounds};

/// Recursive workhorse method for expanding nodes.  Visits all instance nodes in tree, stitching
/// together a tree of ExpandedNodes as it goes (mapping repeated instance nodes into multiple ExpandedNodes, for example.)
/// All properties are computed within this pass, and computed properties are stored in individual ExpandedNodes.
/// Rendering is then a function of these ExpandedNodes.
///
/// Note that `recurse_expand_nodes` could be called each frame to brute-force compute every property, but
/// once we support "dirty DAG" for tactical property updates and no-properties-computation-at-rest, then
/// this expansion should only need to happen once, at program init, or when a program definition is mutated,
/// e.g. via hot-module reloading.
pub fn recurse_expand_nodes<R: 'static + RenderContext>(ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {

    let this_instance_node = Rc::clone(&ptc.current_instance_node);
    let mut node_borrowed = this_instance_node.borrow_mut();

    //Problem: nodes that manage their own subtree expansion, like Repeat and even more problematically Component,
    //         will handle their subtree automatically through this method (at least, currently.)  The problem is,
    //         Component requires that its slot_childen are computed (in the following block of code) ***before expansion,***
    //         yet slot_children computation requires a handle to this expanded_node.  Ways to untangle:
    //         - provide a separate lifecycle method for auto vs. manual expansion of subtree (might converge on pre- and post- handlers, like before)
    //         - since we only use `this_expanded_node` at the _end_ of the slot_children expansion, to apply the flattened children to this_expanded_node (asserted as a Component)
    //           we could instead invert this and send the flattened slot children to expand_node (via `ptc`, probably) such that
    //           component can special-case the presence of that data on ptc, attaching the flattened children to itself before / as it recurses
    let this_expanded_node = node_borrowed.expand_node(ptc);

    let node_type = this_expanded_node.borrow().instance_node.borrow().get_node_type();
    if matches!(node_type, NodeType::Component) {
        ptc.current_containing_component = Some(Rc::clone(&this_expanded_node));
    }
    ptc.current_expanded_node = Some(Rc::clone(&this_expanded_node));

    // Lifecycle: `mount`
    manage_handlers_mount(ptc);

    // First compute slot_children — that is, the children templated _inside_ a component.
    // For example, in `<Stacker>for i in 0..5 { <Rectangle /> }</Stacker>`, the subtree
    // starting at `for` is the subtree of slot_children for the described instance of `Stacker`.
    // Read more about slot children at [`InstanceNode#get_slot_children`]
    if let Some(slot_children) = node_borrowed.get_slot_children() {

        //Expand children in the context of the current containing component
        let mut expanded_slot_children = vec![];
        for child in (*slot_children).borrow().iter() {
            let mut new_ptc = ptc.clone();
            new_ptc.current_instance_node = Rc::clone(child);
            new_ptc.current_expanded_node = None; // We don't want slot_children to stitch themselves to any parent; we handle that later inside Slot
            let child_expanded_node = recurse_expand_nodes(&mut new_ptc);
            expanded_slot_children.push(child_expanded_node);

        }

        //Now flatten those expanded children, ignoring (replacing with children) and node that`is_invisible_to_slot`, namely
        //[`ConditionalInstance`] and [`RepeatInstance`]
        let mut expanded_and_flattened_slot_children = vec![];
        for expanded_slot_child in expanded_slot_children {
            expanded_and_flattened_slot_children.extend(flatten_expanded_node_for_slot(&Rc::downgrade(&expanded_slot_child)));
        }

        let node_type = this_expanded_node.borrow().instance_node.borrow().get_node_type();
        assert!(matches!(node_type, NodeType::Component), ""); // `this_expanded_node`'s related `instance_node` must be of type NodeType::Component.

        this_expanded_node.borrow_mut().set_expanded_and_flattened_slot_children(expanded_and_flattened_slot_children);
    }

    let new_tab = compute_tab(ptc);
    ptc.containing_tab = new_tab;


    // Some nodes must manage their own properties computation recursion, e.g. Repeat and Conditional.  If this is such a node,
    if !this_instance_node.borrow().manages_own_subtree_for_expansion() {

        //Strictly following computation of slot children, we recurse into instance_children.
        //This ordering is required because the properties for slot children must be computed
        //in this outer context, of a containing component, before we compute properties for the inner component's context.
        //This way, we can be assured that the slot_children present on any component have already
        //been properties-computed, thus expanded by Repeat and Conditional.
        let children_to_recurse = node_borrowed.get_instance_children();

        for child in (*children_to_recurse).borrow().iter() {
            let mut new_ptc = ptc.clone();
            new_ptc.current_instance_node = Rc::clone(child);
            let child_expanded_node = recurse_expand_nodes(&mut new_ptc);

            recurse_expand_nodes(ptc);

            this_expanded_node.borrow_mut().append_child(child_expanded_node);
        }
    }

    //lifecycle: handle_native_patches — for elements with native components (for example Text, Frame, and form control elements),
    //certain native-bridge events must be triggered when changes occur, and some of those events require pre-computed `size` and `transform`.
    // node_borrowed.instance_node.borrow_mut().handle_native_patches(
    //     ptc,
    //     clipping_aware_bounds,
    //     new_scroller_normalized_accumulated_transform
    //         .as_coeffs()
    //         .to_vec(),
    //     node.borrow().z_index,
    //     subtree_depth,
    // );

    // Lifecycle: `unmount`
    manage_handlers_unmount(ptc);
    this_expanded_node
}


/// Helper function that accepts a
fn flatten_expanded_node_for_slot<R: 'static + RenderContext>(node: &Weak<RefCell<ExpandedNode<R>>>) -> Vec<Rc<RefCell<ExpandedNode<R>>>> {
    let mut result = vec![];

    let is_invisible_to_slot = {
        let node_upgraded = node.upgrade().unwrap();
        let node_borrowed = node_upgraded.borrow();
        let instance_node_borrowed = node_borrowed.instance_node.borrow();
        instance_node_borrowed.is_invisible_to_slot()
    };
    if is_invisible_to_slot {
        // If the node is invisible, recurse on its children
        let node_upgraded = node.upgrade().unwrap();
        for child in node_upgraded.borrow().children_expanded_nodes.iter() {
            result.extend(flatten_expanded_node_for_slot(&child));
        }
    } else {
        // If the node is visible, add it to the result
        result.push(node.upgrade().unwrap());
    }

    result
}

/// For the `current_expanded_node` attached to `ptc`, calculates and returns a new [`crate::rendering::TransformAndBounds`]
/// Intended as a helper method to be called during properties computation, for creating a new `tab` to attach to
/// `ptc` for downstream calculations.
fn compute_tab<R: 'static + RenderContext>(ptc: &mut PropertiesTreeContext<R>) -> TransformAndBounds {
    let node = Rc::clone(&ptc.current_expanded_node.as_ref().unwrap());

    //get the size of this node (calc'd or otherwise) and use
    //it as the new accumulated bounds: both for this node's children (their parent container bounds)
    //and for this node itself (e.g. for specifying the size of a Rectangle node)
    let new_accumulated_bounds_and_current_node_size = node
        .borrow_mut()
        .compute_size_within_bounds(ptc.containing_tab.bounds);

    let mut node_size: (f64, f64) = (0.0, 0.0);

    let node_transform_property_computed = {
        let node_borrowed = ptc.current_expanded_node.as_ref().unwrap().borrow_mut();

        let computed_transform2d_matrix = node_borrowed
            .get_common_properties().borrow()
            .transform
            .get()
            .compute_transform2d_matrix(new_accumulated_bounds_and_current_node_size.clone(), ptc.containing_tab.bounds);

        computed_transform2d_matrix
    };

    // From a combination of the sugared TemplateNodeDefinition properties like `width`, `height`, `x`, `y`, `scale_x`, etc.
    let desugared_transform = {
        //Extract common_properties, pack into Transform2D, decompose / compute, and combine with node_computed_transform
        let node_borrowed = ptc.current_expanded_node.as_ref().unwrap().borrow();
        let comm = node_borrowed.get_common_properties();
        let comm = comm.borrow();
        let mut desugared_transform2d = Transform2D::default();

        let translate = [
            if let Some(ref val) = comm.x {
                val.get().clone()
            } else {
                Size::ZERO()
            },
            if let Some(ref val) = comm.y {
                val.get().clone()
            } else {
                Size::ZERO()
            },
        ];
        desugared_transform2d.translate = Some(translate);

        let anchor = [
            if let Some(ref val) = comm.anchor_x {
                val.get().clone()
            } else {
                Size::ZERO()
            },
            if let Some(ref val) = comm.anchor_y {
                val.get().clone()
            } else {
                Size::ZERO()
            },
        ];
        desugared_transform2d.anchor = Some(anchor);

        let scale = [
            if let Some(ref val) = comm.scale_x {
                val.get().clone()
            } else {
                Size::Percent(pax_runtime_api::Numeric::from(100.0))
            },
            if let Some(ref val) = comm.scale_y {
                val.get().clone()
            } else {
                Size::Percent(pax_runtime_api::Numeric::from(100.0))
            },
        ];
        desugared_transform2d.scale = Some(scale);

        let skew = [
            if let Some(ref val) = comm.skew_x {
                val.get().get_as_float()
            } else {
                0.0
            },
            if let Some(ref val) = comm.skew_y {
                val.get().get_as_float()
            } else {
                0.0
            },
        ];
        desugared_transform2d.skew = Some(skew);

        let rotate = if let Some(ref val) = comm.rotate {
            val.get().clone()
        } else {
            Rotation::ZERO()
        };
        desugared_transform2d.rotate = Some(rotate);

        desugared_transform2d.compute_transform2d_matrix(new_accumulated_bounds_and_current_node_size.clone(), ptc.containing_tab.bounds)
    };

    let new_accumulated_transform =
        ptc.containing_tab.transform * desugared_transform * node_transform_property_computed;

    // let new_scroller_normalized_accumulated_transform =
    //     accumulated_scroller_normalized_transform
    //         * desugared_transform
    //         * node_transform_property_computed;

    // rtc.transform_scroller_reset = new_scroller_normalized_accumulated_transform.clone();

    TransformAndBounds {
        transform: new_accumulated_transform,
        bounds: new_accumulated_bounds_and_current_node_size,
    }

}

/// Handle node unmounting, including check for whether unmount handlers should be fired
/// (thus this function can be called on all nodes at end of properties computation
fn manage_handlers_unmount<R: 'static + RenderContext>(ptc: &mut PropertiesTreeContext<R>) {

    if ptc.marked_for_unmount {
        ptc.current_instance_node.clone().borrow_mut().handle_unmount(ptc);

        ptc.engine.node_registry
            .borrow_mut()
            .remove_expanded_node(&ptc.current_expanded_node.clone().unwrap().borrow().id_chain);
    }
}

/// Helper method to fire `mount` event if this is this expandednode's first frame
/// (or first frame remounting, if previously mounted then unmounted.)
/// Note that this must happen after initial `compute_properties`, which performs the
/// necessary side-effect of creating the `self` that must be passed to handlers.
fn manage_handlers_mount<R: 'static + RenderContext>(ptc: &mut PropertiesTreeContext<R>) {
    {
        let mut node_registry = (*ptc.engine.node_registry).borrow_mut();

        let id_chain = <Vec<u32> as AsRef<Vec<u32>>>::as_ref(&ptc.current_expanded_node.clone().unwrap().borrow().id_chain).clone();
        if !node_registry.is_mounted(&id_chain) {
            //Fire primitive-level mount lifecycle method
            let mut instance_node = Rc::clone(&ptc.current_instance_node);
            instance_node.borrow_mut().handle_mount(ptc);

            //Fire registered mount events
            let registry = (*ptc.current_instance_node).borrow().get_handler_registry();
            if let Some(registry) = registry {
                //grab Rc of properties from stack frame; pass to type-specific handler
                //on instance in order to dispatch cartridge method
                for handler in (*registry).borrow().mount_handlers.iter() {
                    handler(
                        ptc.current_expanded_node.clone().unwrap().borrow_mut().get_properties(),
                        ptc.current_expanded_node.clone().unwrap().borrow().node_context.clone(),
                    );
                }
            }
            node_registry.mark_mounted(id_chain);
        }
    }
}

/// Shared context for properties pass recursion
pub struct PropertiesTreeContext<'a, R: 'static + RenderContext> {

    pub engine: &'a PaxEngine<R>,

    /// A pointer to the node representing the current Component, for which we may be
    /// rendering some member of its template.
    pub current_containing_component: Option<Rc<RefCell<ExpandedNode<R>>>>,

    /// A pointer to the current instance node
    pub current_instance_node: InstanceNodePtr<R>,

    /// A pointer to the current expanded node.  Optional only for the init case; should be populated
    /// for every node visited during properties computation.
    pub current_expanded_node: Option<Rc<RefCell<ExpandedNode<R>>>>,

    /// A pointer to the current expanded node's parent expanded node, useful at least for appending children
    pub parent_expanded_node: Option<Weak<ExpandedNode<R>>>,

    pub marked_for_unmount: bool,

    pub shared: Rc<RefCell<PropertiesTreeShared>>,

    pub containing_tab: TransformAndBounds,
}

/// Whereas `ptc` is cloned for each new call site, giving each state of computation its own "sandbox" for e.g. writing
/// current pointers without overwriting others, some state within `ptc` needs to be shared-mutable.  `PropertiesTreeShared` is intended
/// to be wrapped in an `Rc<RefCell<>>`, so that it may be cloned along with `ptc` while preserving a reference to the same shared, mutable state.
pub struct PropertiesTreeShared {

    /// Runtime stack managed for computing properties, for example for resolving symbols like `self.foo` or `i` (from `for i in 0..5`).
    /// Stack offsets are resolved statically during computation.  For example, if `self.foo` is statically determined to be offset by 2 frames,
    /// then at runtime it is expected that `self.foo` can be resolved 2 frames up from the top of this stack.
    /// (Mismatches between the static compile-time stack and this runtime stack would result in an unrecoverable panic.)
    pub runtime_properties_stack: Vec<Rc<RefCell<RuntimePropertiesStackFrame>>>,

    /// Tracks the native ids (id_chain)s of clipping instances
    /// When a node is mounted, it may consult the clipping stack to see which clipping instances are relevant to it
    /// This list of `id_chain`s is passed along with `**Create`, in order to associate with the appropriate clipping elements on the native side
    pub clipping_stack: Vec<Vec<u32>>,

    /// Similar to clipping stack but for scroller containers
    pub scroller_stack: Vec<Vec<u32>>,

    /// Iterator for tracking current z-index; expected to be reset at the beginning of every properties computation pass
    pub z_index_gen: RangeFrom<isize>,

    /// Queue for native "CRUD" message (e.g. TextCreate), populated during properties
    /// computation and passed across native bridge each tick after canvas rendering
    pub native_message_queue: VecDeque<NativeMessage>,
}

impl<'a, R: 'static + RenderContext> Clone for PropertiesTreeContext<'a, R> {
    fn clone(&self) -> Self {
        Self {
            engine: &self.engine.clone(),
            current_containing_component: self.current_containing_component.clone(),
            current_instance_node: Rc::clone(&self.current_instance_node),
            current_expanded_node: self.current_expanded_node.clone(),
            parent_expanded_node: self.parent_expanded_node.clone(),
            marked_for_unmount: self.marked_for_unmount,
            shared: Rc::clone(&self.shared),
            containing_tab: self.containing_tab.clone(),
        }
    }
}

impl<'a, R: 'static + RenderContext> PropertiesTreeContext<'a, R> {

    pub fn push_clipping_stack_id(&mut self, id_chain: Vec<u32>) {
        self.shared.borrow_mut().clipping_stack.push(id_chain);
    }

    pub fn pop_clipping_stack_id(&mut self) {
        self.shared.borrow_mut().clipping_stack.pop();
    }

    pub fn get_current_clipping_ids(&self) -> Vec<Vec<u32>> {
        self.shared.borrow_mut().clipping_stack.clone()
    }

    pub fn push_scroller_stack_id(&mut self, id_chain: Vec<u32>) {
        self.shared.borrow_mut().scroller_stack.push(id_chain);
    }

    pub fn pop_scroller_stack_id(&mut self) {
        self.shared.borrow_mut().scroller_stack.pop();
    }

    pub fn get_current_scroller_ids(&self) -> Vec<Vec<u32>> {
        self.shared.borrow_mut().scroller_stack.clone()
    }

    pub fn get_list_of_repeat_indicies_from_stack(&self) -> Vec<u32> {
        let mut indices: Vec<u32> = vec![];

        self.shared.borrow_mut().runtime_properties_stack.iter().for_each(|frame_wrapped| {
            if let PropertiesCoproduct::RepeatItem(_datum, i) =
                &*(*(*(*frame_wrapped).borrow_mut()).properties).borrow()
            {
                indices.push(*i as u32)
            }
        });
        indices
    }

    pub fn distill_userland_node_context(&self) -> RuntimeContext {
        RuntimeContext {
            bounds_parent: self.containing_tab.bounds,
            frames_elapsed: self.engine.frames_elapsed,
        }
    }

    //return current state of native message queue, passing in a freshly initialized queue for next frame
    pub fn take_native_message_queue(&mut self) -> VecDeque<NativeMessage> {
        std::mem::take(&mut self.shared.borrow_mut().native_message_queue)
    }

    pub fn enqueue_native_message(&mut self, msg: NativeMessage) {
        self.shared.borrow_mut().native_message_queue.push_back(msg);
    }

    /// Return a pointer to the top StackFrame on the stack,
    /// without mutating the stack or consuming the value
    pub fn peek_stack_frame(&self) -> Option<Rc<RefCell<RuntimePropertiesStackFrame>>> {
        if self.shared.borrow_mut().runtime_properties_stack.len() > 0 {
            Some(Rc::clone(&self.shared.borrow_mut().runtime_properties_stack[&self.shared.borrow_mut().runtime_properties_stack.len() - 1]))
        } else {
            None
        }
    }

    /// Remove the top element from the stack.  Currently does
    /// nothing with the value of the popped StackFrame.
    pub fn pop_stack_frame(&mut self) {
        self.shared.borrow_mut().runtime_properties_stack.pop(); //NOTE: handle value here if needed
    }

    /// Add a new frame to the stack, passing a list of slot_children
    /// that may be handled by `Slot` and a scope that includes the PropertiesCoproduct of the associated Component
    pub fn push_stack_frame(
        &mut self,
        properties: Rc<RefCell<PropertiesCoproduct>>,
        timeline: Option<Rc<RefCell<Timeline>>>,
    ) {
        let parent = self.peek_stack_frame().as_ref().map(Rc::downgrade);

        self.shared.borrow_mut().runtime_properties_stack.push(Rc::new(RefCell::new(RuntimePropertiesStackFrame::new(
            properties,
            parent,
            timeline,
        ))));
    }

    /// Get an `id_chain` for this element, a `Vec<u64>` used collectively as a single unique ID across native bridges.
    ///
    /// The need for this emerges from the fact that `Repeat`ed elements share a single underlying
    /// `instance`, where that instantiation happens once at init-time — specifically, it does not happen
    /// when `Repeat`ed elements are added and removed to the render tree.  10 apparent rendered elements may share the same `instance_id` -- which doesn't work as a unique key for native renderers
    /// that are expected to render and update 10 distinct elements.
    ///
    /// Thus, the `id_chain` is used as a unique key, first the `instance_id` (which will increase monotonically through the lifetime of the program),
    /// then each RepeatItem index through a traversal of the stack frame.  Thus, each virtually `Repeat`ed element
    /// gets its own unique ID in the form of an "address" through any nested `Repeat`-ancestors.
    pub fn get_id_chain(&self) -> Vec<u32> {
        let id= self.current_instance_node.clone().borrow().get_instance_id();
        let mut indices = (&self.get_list_of_repeat_indicies_from_stack()).clone();
        indices.insert(0, id);
        indices
    }

    pub fn compute_vtable_value(&self, vtable_id: usize) -> TypesCoproduct {
        if let Some(evaluator) = self.engine.expression_table.get(&vtable_id) {
            let ec = ExpressionContext {
                engine: self.engine,
                stack_frame: Rc::clone(
                    &self.peek_stack_frame().unwrap(),
                ),
            };
            (**evaluator)(ec)
        }else{
            panic!()//unhandled error if an invalid id is passed or if vtable is incorrectly initialized
        }
    }
}

/// Data structure for a single frame of our runtime stack, including
/// a reference to its parent frame and `properties` for
/// runtime evaluation, e.g. of Expressions.  `RuntimePropertiesStackFrame`s also track
/// timeline playhead position.
///
/// `Component`s push `RuntimePropertiesStackFrame`s before computing properties and pop them after computing, thus providing a
/// hierarchical store of node-relevant data that can be bound to symbols in expressions.
pub struct RuntimePropertiesStackFrame {
    properties: Rc<RefCell<PropertiesCoproduct>>,
    parent: Option<Weak<RefCell<RuntimePropertiesStackFrame>>>,
    timeline: Option<Rc<RefCell<Timeline>>>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(
        properties: Rc<RefCell<PropertiesCoproduct>>,
        parent: Option<Weak<RefCell<RuntimePropertiesStackFrame>>>,
        timeline: Option<Rc<RefCell<Timeline>>>,
    ) -> Self {
        RuntimePropertiesStackFrame {
            properties,
            parent,
            timeline,
        }
    }

    pub fn get_timeline_playhead_position(&self) -> usize {
        match &self.timeline {
            None => {
                //if this stackframe doesn't carry a timeline, then refer
                //to the parent stackframe's timeline (and recurse)
                match &self.parent {
                    Some(parent_frame) => (*parent_frame.upgrade().unwrap())
                        .borrow()
                        .get_timeline_playhead_position(),
                    None => 0,
                }
            }
            Some(timeline) => (**timeline).borrow().playhead_position,
        }
    }

    /// Traverses stack recursively `n` times to retrieve ancestor;
    /// useful for runtime lookups for identifiers, where `n` is the statically known offset determined by the Pax compiler
    /// when resolving a symbol
    pub fn peek_nth(&self, n: isize) -> Option<Rc<RefCell<RuntimePropertiesStackFrame>>> {
        if n == 0 {
            //0th ancestor is self; handle by caller since caller owns the Rc containing `self`
            None
        } else {
            self.recurse_peek_nth(n, 0)
        }
    }

    fn recurse_peek_nth(&self, n: isize, depth: isize) -> Option<Rc<RefCell<RuntimePropertiesStackFrame>>> {
        let new_depth = depth + 1;
        let parent = self.parent.as_ref().unwrap();
        if new_depth == n {
            return Some(parent.upgrade().unwrap());
        }
        (*parent.upgrade().unwrap())
            .borrow()
            .recurse_peek_nth(n, new_depth)
    }

    pub fn get_properties(&self) -> Rc<RefCell<PropertiesCoproduct>> {
        Rc::clone(&self.properties)
    }
}

/// Given a vec of instance nodes (TODO: or expanded nodes?), process them recursively into a single flat list of `ExpandedNodes`,
/// such that they can be "redirected" by Slot at render-time.  In particular, this handles `if` and `for`,
/// which should be invisible to slot.  E.g. `for i in 0..10 { <Rectangle /> }` would flatten into 10 `Rectangle` nodes.
pub fn flatten_invisible_slot_children_recursive<R: 'static + RenderContext>(
    slot_children: Vec<InstanceNodePtr<R>>,
) -> Vec<InstanceNodePtr<R>> {
    todo!("handle multiple roots");
    // let slot_child_borrowed = (**slot_child).borrow_mut();
    // if slot_child_borrowed.is_invisible_to_slot() {
    //     (*slot_child_borrowed.get_instance_children())
    //         .borrow()
    //         .iter()
    //         .map(|top_level_child_node| {
    //             Runtime::flatten_invisible_slot_children_recursive(top_level_child_node, rtc)
    //         })
    //         .flatten()
    //         .collect()
    // } else {
    //     vec![Rc::clone(slot_child)]
    // }
}