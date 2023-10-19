use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use pax_properties_coproduct::PropertiesCoproduct;
use pax_runtime_api::Timeline;
use piet::RenderContext;

use crate::{RenderNodePtr, RenderNodePtrList, RenderTreeContext};

/// `Runtime` is a container for data and logic needed by the `Engine`,
/// explicitly aside from rendering.  For example, this is a home
/// for logic that manages scopes and stack frames.
pub struct Runtime<R: 'static + RenderContext> {
    stack: Vec<Rc<RefCell<StackFrame<R>>>>,

    /// Tracks the native ids (id_chain)s of clipping instances
    /// When a node is mounted, it may consult the clipping stack to see which clipping instances are relevant to it
    /// This list of `id_chain`s is passed along with `**Create`, in order to associate with the appropriate clipping elements on the native side
    clipping_stack: Vec<Vec<u32>>,
    /// Similar to clipping stack but for scroller containers
    scroller_stack: Vec<Vec<u32>>,
    native_message_queue: VecDeque<pax_message::NativeMessage>,
}

impl<R: 'static + RenderContext> Runtime<R> {
    pub fn new() -> Self {
        Runtime {
            stack: vec![],
            clipping_stack: vec![],
            scroller_stack: vec![],
            native_message_queue: VecDeque::new(),
        }
    }

    // NOTE: this value could be cached on stackframes, registered & cached during engine rendertree traversal (specifically: when stackframes are pushed)
    //       This would make id_chain resolution essentially free, O(1) instead of O(log(n))
    //       Profile first to understand the impact before optimizing
    pub fn get_list_of_repeat_indicies_from_stack(&self) -> Vec<u32> {
        let mut indices: Vec<u32> = vec![];

        self.stack.iter().for_each(|frame_wrapped| {
            if let PropertiesCoproduct::RepeatItem(_datum, i) =
                &*(*(*(*frame_wrapped).borrow_mut()).properties).borrow()
            {
                indices.push(*i as u32)
            }
        });
        indices
    }

    //return current state of native message queue, passing in a freshly initialized queue for next frame
    pub fn take_native_message_queue(&mut self) -> VecDeque<pax_message::NativeMessage> {
        std::mem::take(&mut self.native_message_queue)
    }

    pub fn enqueue_native_message(&mut self, msg: pax_message::NativeMessage) {
        self.native_message_queue.push_back(msg);
    }

    /// Return a pointer to the top StackFrame on the stack,
    /// without mutating the stack or consuming the value
    pub fn peek_stack_frame(&mut self) -> Option<Rc<RefCell<StackFrame<R>>>> {
        if self.stack.len() > 0 {
            Some(Rc::clone(&self.stack[&self.stack.len() - 1]))
        } else {
            None
        }
    }

    /// Remove the top element from the stack.  Currently does
    /// nothing with the value of the popped StackFrame.
    pub fn pop_stack_frame(&mut self) {
        self.stack.pop(); //NOTE: handle value here if needed
    }

    /// Add a new frame to the stack, passing a list of slot_children
    /// that may be handled by `Slot` and a scope that includes the PropertiesCoproduct of the associated Component
    pub fn push_stack_frame(
        &mut self,
        flattened_slot_children: RenderNodePtrList<R>,
        properties: Rc<RefCell<PropertiesCoproduct>>,
        timeline: Option<Rc<RefCell<Timeline>>>,
    ) {
        let parent = self.peek_stack_frame().as_ref().map(Rc::downgrade);

        self.stack.push(Rc::new(RefCell::new(StackFrame::new(
            flattened_slot_children,
            properties,
            parent,
            timeline,
        ))));
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

    /// Handles special-cases like `for`/`Repeat`, where properties for the
    /// control flow primitive need to be computed out-of-lifecycle, and where nested child elements
    /// need to be treated as top-level elements.
    /// For example, given `<Stacker><Ellipse />for i in (0..3){ <Rectangle /> }</Stacker>`,
    /// without this special handling `Stacker` will receive only two slot_children: the `Ellipse` and the `Repeat` node
    /// created by `for`.  In other words `for`s children need to be treated as `<Stacker>`s children,
    /// and this processing allows that to happpen.
    /// Note that this must be recursive to handle nested cases of flattening, for example nested `for` loops
    #[allow(non_snake_case)]
    pub fn process__should_flatten__slot_children_recursive(
        slot_child: &RenderNodePtr<R>,
        rtc: &mut RenderTreeContext<R>,
    ) -> Vec<RenderNodePtr<R>> {
        let slot_child_borrowed = (**slot_child).borrow_mut();
        if slot_child_borrowed.should_flatten() {
            (*slot_child_borrowed.get_rendering_children())
                .borrow()
                .iter()
                .map(|top_level_child_node| {
                    Runtime::process__should_flatten__slot_children_recursive(top_level_child_node, rtc)
                })
                .flatten()
                .collect()
        } else {
            vec![Rc::clone(slot_child)]
        }
    }
}

/// Data structure for a single frame of our runtime stack, including
/// a reference to its parent frame, a list of `slot_children` for
/// prospective [`Slot`] consumption, and `properties` for
/// runtime evaluation, e.g. of Expressions.  StackFrames also track
/// timeline playhead position.
///
/// `Component`s push StackFrames when mounting and pop them when unmounting, thus providing a
/// hierarchical store of node-relevant data that can be bound to symbols, e.g. in expressions.
/// Note that `RepeatItem`s also push `StackFrame`s, because `RepeatItem` uses a `Component` internally.
pub struct StackFrame<R: 'static + RenderContext> {
    slot_children: RenderNodePtrList<R>,
    properties: Rc<RefCell<PropertiesCoproduct>>,
    parent: Option<Weak<RefCell<StackFrame<R>>>>,
    timeline: Option<Rc<RefCell<Timeline>>>,
}

impl<R: 'static + RenderContext> StackFrame<R> {
    pub fn new(
        slot_children: RenderNodePtrList<R>,
        properties: Rc<RefCell<PropertiesCoproduct>>,
        parent: Option<Weak<RefCell<StackFrame<R>>>>,
        timeline: Option<Rc<RefCell<Timeline>>>,
    ) -> Self {
        StackFrame {
            slot_children,
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

    // Traverses stack recursively `n` times to retrieve ancestor;
    // useful for runtime lookups for identifiers
    pub fn peek_nth(&self, n: isize) -> Option<Rc<RefCell<StackFrame<R>>>> {
        if n == 0 {
            //0th ancestor is self; handle by caller since caller owns the Rc containing `self`
            None
        } else {
            self.recurse_peek_nth(n, 0)
        }
    }

    fn recurse_peek_nth(&self, n: isize, depth: isize) -> Option<Rc<RefCell<StackFrame<R>>>> {
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

    pub fn get_unflattened_slot_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.slot_children)
    }

    pub fn nth_slot_child(&self, n: usize) -> Option<RenderNodePtr<R>> {
        match (*self.slot_children).borrow().get(n) {
            Some(i) => Some(Rc::clone(i)),
            None => None,
        }
    }

    pub fn has_slot_children(&self) -> bool {
        (*self.slot_children).borrow().len() > 0
    }
}
