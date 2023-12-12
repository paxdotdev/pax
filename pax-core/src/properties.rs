use crate::{
    ComputableTransform, ExpandedNode, ExpressionContext, InstanceNodePtr, InstanceNodePtrList,
    NodeRegistry, NodeType, PaxEngine, PropertiesComputable, TransformAndBounds,
};
use kurbo::Affine;
use pax_message::NativeMessage;
use pax_runtime_api::{
    Interpolatable, NodeContext, Numeric, Rotation, Size, Timeline, Transform2D, TransitionManager,
};
use piet::RenderContext;
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::RangeFrom;
use std::rc::{Rc, Weak};

/// Recursive workhorse method for expanding nodes.  Visits all instance nodes in tree, stitching
/// together a tree of ExpandedNodes as it goes (mapping repeated instance nodes into multiple ExpandedNodes, for example.)
/// All properties are computed within this pass, and computed properties are stored in individual ExpandedNodes.
/// Rendering is then a function of these ExpandedNodes.
///
/// Note that `recurse_expand_nodes` could be called each frame to brute-force compute every property, but
/// once we support "dirty DAG" for tactical property updates and no-properties-computation-at-rest, then
/// this expansion should only need to happen once, at program init, or when a program definition is mutated,
/// e.g. via hot-module reloading.
pub fn recurse_expand_nodes<R: 'static + RenderContext>(
    ptc: &mut PropertiesTreeContext<R>,
) -> Rc<RefCell<ExpandedNode<R>>> {
    let this_instance_node = Rc::clone(&ptc.current_instance_node);
    let this_expanded_node = {
        this_instance_node
            .borrow_mut()
            .expand_node_and_compute_properties(ptc)
    };

    // Compute common properties
    let common_properties = Rc::clone(&this_expanded_node.borrow_mut().get_common_properties());
    common_properties.borrow_mut().compute_properties(ptc);

    // Lifecycle: `unmount`
    manage_handlers_unmount(ptc);

    this_expanded_node
}

/// Handle node unmounting, including check for whether unmount handlers should be fired
/// (thus this function can be called on all nodes at end of properties computation
fn manage_handlers_unmount<R: 'static + RenderContext>(ptc: &mut PropertiesTreeContext<R>) {
    let id_chain: Vec<u32> = ptc
        .current_expanded_node
        .as_ref()
        .unwrap()
        .borrow()
        .id_chain
        .clone();

    let currently_mounted = matches!(
        ptc.engine
            .node_registry
            .borrow()
            .get_expanded_node(&id_chain),
        Some(_)
    );
    if ptc.marked_for_unmount && !currently_mounted {
        ptc.current_instance_node
            .clone()
            .borrow_mut()
            .handle_unmount(ptc);

        ptc.engine
            .node_registry
            .borrow_mut()
            .remove_expanded_node(&id_chain);
    }
}

/// Shared context for properties pass recursion
pub struct PropertiesTreeContext<'a, R: 'static + RenderContext> {
    pub engine: &'a PaxEngine<R>,

    /// A pointer to the node representing the current Component, for which we may be
    /// rendering some member of its template.
    pub current_containing_component: Weak<RefCell<ExpandedNode<R>>>,

    //TODOSAM try doing this
    /// A register used for passing slot children to components.  This is passed via `ptc` to satisfy sequencing concerns.
    /// Decoupling expansion from properties computation should enable removing this from `PropertiesTreeContext`
    pub expanded_and_flattened_slot_children: Option<Vec<Rc<RefCell<ExpandedNode<R>>>>>,

    /// A pointer to the current instance node
    pub current_instance_node: InstanceNodePtr<R>,

    /// A pointer to the current expanded node.  Optional only for the init case; should be populated
    /// for every node visited during properties computation.
    pub current_expanded_node: Option<Rc<RefCell<ExpandedNode<R>>>>,

    /// A pointer to the current expanded node's parent expanded node, useful at least for appending children
    pub parent_expanded_node: Option<Weak<ExpandedNode<R>>>,

    pub marked_for_unmount: bool,

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

    pub shared: Rc<RefCell<PropertiesTreeShared>>,
}

/// Whereas `ptc` is cloned for each new call site, giving each state of computation its own "sandbox" for e.g. writing
/// current pointers without overwriting others, some state within `ptc` needs to be a shared singleton.  `PropertiesTreeShared` is intended
/// to be wrapped in an `Rc<RefCell<>>`, so that it may be cloned along with `ptc` while preserving a reference to the same shared, mutable state.
pub struct PropertiesTreeShared {
    /// Queue for native "CRUD" message (e.g. TextCreate), populated during properties
    /// computation and passed across native bridge each tick after canvas rendering
    pub native_message_queue: VecDeque<NativeMessage>,
}

impl<'a, R: 'static + RenderContext> Clone for PropertiesTreeContext<'a, R> {
    fn clone(&self) -> Self {
        Self {
            expanded_and_flattened_slot_children: self.expanded_and_flattened_slot_children.clone(),
            engine: &self.engine,
            current_containing_component: self.current_containing_component.clone(),
            current_instance_node: Rc::clone(&self.current_instance_node),
            current_expanded_node: self.current_expanded_node.clone(),
            parent_expanded_node: self.parent_expanded_node.clone(),
            marked_for_unmount: self.marked_for_unmount,
            runtime_properties_stack: self.runtime_properties_stack.clone(),
            clipping_stack: self.clipping_stack.clone(),
            scroller_stack: self.scroller_stack.clone(),
            shared: Rc::clone(&self.shared),
        }
    }
}

impl<'a, R: 'static + RenderContext> PropertiesTreeContext<'a, R> {
    pub fn clone_runtime_stack(&self) -> Vec<Rc<RefCell<RuntimePropertiesStackFrame>>> {
        self.runtime_properties_stack.clone()
    }

    pub fn push_clipping_stack_id(&mut self, id_chain: Vec<u32>) {
        self.clipping_stack.push(id_chain);
    }

    pub fn pop_clipping_stack_id(&mut self) {
        self.clipping_stack.pop();
    }

    pub fn get_current_clipping_ids(&self) -> Vec<Vec<u32>> {
        self.clipping_stack.clone()
    }

    pub fn push_scroller_stack_id(&mut self, id_chain: Vec<u32>) {
        self.scroller_stack.push(id_chain);
    }

    pub fn pop_scroller_stack_id(&mut self) {
        self.scroller_stack.pop();
    }

    pub fn get_current_scroller_ids(&self) -> Vec<Vec<u32>> {
        self.scroller_stack.clone()
    }

    pub fn get_list_of_repeat_indicies_from_stack(&self) -> Vec<u32> {
        let mut indices: Vec<u32> = vec![];

        self.runtime_properties_stack
            .iter()
            .for_each(|frame_wrapped| {
                let frame_rc_cloned = frame_wrapped.clone();
                let frame_refcell_borrowed = frame_rc_cloned.borrow();
                let properties_rc_cloned = Rc::clone(&frame_refcell_borrowed.properties);
                let mut properties_refcell_borrowed = properties_rc_cloned.borrow_mut();

                if let Some(mut ri) =
                    properties_refcell_borrowed.downcast_mut::<crate::RepeatItem>()
                {
                    indices.push(ri.i as u32)
                }
            });
        indices
    }

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
        let len = *&self.runtime_properties_stack.len();
        if len > 0 {
            Some(Rc::clone(&self.runtime_properties_stack[len - 1]))
        } else {
            None
        }
    }

    /// Remove the top element from the stack.  Currently does
    /// nothing with the value of the popped StackFrame.
    pub fn pop_stack_frame(&mut self) {
        self.runtime_properties_stack.pop(); //NOTE: handle value here if needed
    }

    /// Add a new frame to the stack, passing a list of slot_children
    /// that may be handled by `Slot` and a scope that includes the `dyn Any` properties of the associated Component
    pub fn push_stack_frame(&mut self, properties: Rc<RefCell<dyn Any>>) {
        let parent = self.peek_stack_frame().as_ref().map(Rc::clone);

        self.runtime_properties_stack.push(Rc::new(RefCell::new(
            RuntimePropertiesStackFrame::new(properties, parent),
        )));
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
    pub fn get_id_chain(&self, instance_id: u32) -> Vec<u32> {
        let mut indices = (&self.get_list_of_repeat_indicies_from_stack()).clone();
        indices.insert(0, instance_id);
        indices
    }

    pub fn compute_vtable_value(&self, vtable_id: usize) -> Box<dyn Any> {
        if let Some(evaluator) = self.engine.expression_table.get(&vtable_id) {
            let expanded_node = &self.current_expanded_node.as_ref().unwrap().borrow();
            let stack_frame = Rc::clone(
                expanded_node
                    .runtime_properties_stack
                    .get(expanded_node.runtime_properties_stack.len() - 1)
                    .unwrap(),
            );

            let ec = ExpressionContext {
                engine: self.engine,
                stack_frame,
            };
            (**evaluator)(ec)
        } else {
            panic!() //unhandled error if an invalid id is passed or if vtable is incorrectly initialized
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
    properties: Rc<RefCell<dyn Any>>,
    parent: Option<Rc<RefCell<RuntimePropertiesStackFrame>>>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(
        properties: Rc<RefCell<dyn Any>>,
        parent: Option<Rc<RefCell<RuntimePropertiesStackFrame>>>,
    ) -> Self {
        RuntimePropertiesStackFrame { properties, parent }
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

    fn recurse_peek_nth(
        &self,
        n: isize,
        depth: isize,
    ) -> Option<Rc<RefCell<RuntimePropertiesStackFrame>>> {
        let new_depth = depth + 1;
        let parent = self.parent.as_ref().unwrap();
        if new_depth == n {
            return Some(parent.clone());
        }
        (*parent.clone()).borrow().recurse_peek_nth(n, new_depth)
    }

    pub fn get_properties(&self) -> Rc<RefCell<dyn Any>> {
        Rc::clone(&self.properties)
    }
}

pub fn get_numeric_from_wrapped_properties(wrapped: Rc<RefCell<dyn Any>>) -> Numeric {
    //"u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f64"
    let wrapped_borrowed = wrapped.borrow();
    if let Some(unwrapped_u8) = wrapped_borrowed.downcast_ref::<u8>() {
        Numeric::from(*unwrapped_u8)
    } else if let Some(unwrapped_u16) = wrapped_borrowed.downcast_ref::<u16>() {
        Numeric::from(*unwrapped_u16)
    } else if let Some(unwrapped_u32) = wrapped_borrowed.downcast_ref::<u32>() {
        Numeric::from(*unwrapped_u32)
    } else if let Some(unwrapped_u64) = wrapped_borrowed.downcast_ref::<u64>() {
        Numeric::from(*unwrapped_u64)
    } else if let Some(unwrapped_u128) = wrapped_borrowed.downcast_ref::<u128>() {
        Numeric::from(*unwrapped_u128)
    } else if let Some(unwrapped_usize) = wrapped_borrowed.downcast_ref::<usize>() {
        Numeric::from(*unwrapped_usize)
    } else if let Some(unwrapped_i8) = wrapped_borrowed.downcast_ref::<i8>() {
        Numeric::from(*unwrapped_i8)
    } else if let Some(unwrapped_i16) = wrapped_borrowed.downcast_ref::<i16>() {
        Numeric::from(*unwrapped_i16)
    } else if let Some(unwrapped_i32) = wrapped_borrowed.downcast_ref::<i32>() {
        Numeric::from(*unwrapped_i32)
    } else if let Some(unwrapped_i64) = wrapped_borrowed.downcast_ref::<i64>() {
        Numeric::from(*unwrapped_i64)
    } else if let Some(unwrapped_i128) = wrapped_borrowed.downcast_ref::<i128>() {
        Numeric::from(*unwrapped_i128)
    } else if let Some(unwrapped_isize) = wrapped_borrowed.downcast_ref::<isize>() {
        Numeric::from(*unwrapped_isize)
    } else if let Some(unwrapped_f64) = wrapped_borrowed.downcast_ref::<f64>() {
        Numeric::from(*unwrapped_f64)
    } else {
        panic!("Non-Numeric passed; tried to coerce into Numeric")
    }
}
