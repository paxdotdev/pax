use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use piet::RenderContext;
use pax_message::NativeMessage;
use pax_runtime_api::Timeline;
use crate::{ExpandedNode, ExpressionContext, InstanceNodePtr, InstanceNodePtrList, NodeRegistry};

/// Shared context for properties pass recursion
pub struct PropertiesTreeContext {
    /// Queue for native "CRUD" message (e.g. TextCreate), populated during properties
    /// computation and passed across native bridge each tick
    pub native_message_queue: VecDeque<NativeMessage>,

    pub timeline_playhead_position: usize,
    pub current_z_index: u32,


    /// A pointer to the node representing the current Component, for which we may be
    /// rendering some member of its template.
    pub current_containing_component: InstanceNodePtr<R>,
    /// A clone of current_containing_component#get_slot_children, stored alongside current_containing_component
    /// to manage borrowing & data access
    pub current_containing_component_slot_children: InstanceNodePtrList<R>,
    /// A pointer to the current instance node
    pub current_instance_node: InstanceNodePtr<R>,
    /// A pointer to the current expanded node
    pub current_expanded_node: Option<Rc<RefCell<ExpandedNode<R>>>>,
    /// A pointer to the current expanded node's parent expanded node
    pub parent_expanded_node: Option<Weak<ExpandedNode<R>>>,

}

impl PropertiesTreeContext {
    // NOTE: this value could be cached on stackframes, registered & cached during engine rendertree traversal (specifically: when stackframes are pushed)
    //       This would make id_chain resolution essentially free, O(1) instead of O(log(n))
    //       Profile first to understand the impact before optimizing
    pub fn get_list_of_repeat_indicies_from_stack(&self) -> Vec<u32> {
        let mut indices: Vec<u32> = vec![];

        self.runtime_properties_stack.iter().for_each(|frame_wrapped| {
            if let PropertiesCoproduct::RepeatItem(_datum, i) =
                &*(*(*(*frame_wrapped).borrow_mut()).properties).borrow()
            {
                indices.push(*i as u32)
            }
        });
        indices
    }

    //return current state of native message queue, passing in a freshly initialized queue for next frame
    pub fn take_native_message_queue(&mut self) -> VecDeque<NativeMessage> {
        std::mem::take(&mut self.native_message_queue)
    }

    pub fn enqueue_native_message(&mut self, msg: NativeMessage) {
        self.native_message_queue.push_back(msg);
    }

    /// Return a pointer to the top StackFrame on the stack,
    /// without mutating the stack or consuming the value
    pub fn peek_stack_frame(&mut self) -> Option<Rc<RefCell<RuntimePropertiesStackFrame>>> {
        if self.runtime_properties_stack.len() > 0 {
            Some(Rc::clone(&self.runtime_properties_stack[&self.runtime_properties_stack.len() - 1]))
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
    /// that may be handled by `Slot` and a scope that includes the PropertiesCoproduct of the associated Component
    pub fn push_stack_frame(
        &mut self,
        properties: Rc<RefCell<PropertiesCoproduct>>,
        timeline: Option<Rc<RefCell<Timeline>>>,
    ) {
        let parent = self.peek_stack_frame().as_ref().map(Rc::downgrade);

        self.runtime_properties_stack.push(Rc::new(RefCell::new(RuntimePropertiesStackFrame::new(
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
    pub fn get_id_chain(&self, id: u32) -> Vec<u32> {
        let mut indices = (*self.runtime)
            .borrow()
            .get_list_of_repeat_indicies_from_stack();
        indices.insert(0, id);
        indices
    }


    pub fn compute_vtable_value(&self, vtable_id: Option<usize>) -> Option<TypesCoproduct> {
        if let Some(id) = vtable_id {
            if let Some(evaluator) = self.engine.expression_table.get(&id) {
                let ec = ExpressionContext {
                    engine: self.engine,
                    stack_frame: Rc::clone(
                        &(*self.runtime).borrow_mut().peek_stack_frame().unwrap(),
                    ),
                };
                return Some((**evaluator)(ec));
            }
        } //FUTURE: for timelines: else if present in timeline vtable...

        None
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
    //     (*slot_child_borrowed.get_rendering_children())
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