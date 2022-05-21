use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{VecDeque};
use std::ops::Deref;
use std::rc::Rc;

use piet::RenderContext;
use pax_properties_coproduct::{PropertiesCoproduct};
use pax_runtime_api::{Timeline};

use crate::{HandlerRegistry, RenderNodePtr, RenderNodePtrList, RenderTreeContext};


/// `Runtime` is a container for data and logic needed by the `Engine`,
/// explicitly aside from rendering.  For example, this is a home
/// for logic that manages scopes and stack frames.
pub struct Runtime<R: 'static + RenderContext> {
    stack: Vec<Rc<RefCell<StackFrame<R>>>>,

    /// Tracks the native ids (id_chain)s of clipping instances
    /// When a node is mounted, it may consult the clipping stack to see which clipping instances are relevant to it
    /// This list of `id_chain`s is passed along with `**Create`, in order to associate with the appropriate clipping elements on the native side
    clipping_stack: Vec<Vec<u64>>,
    native_message_queue: VecDeque<pax_message::NativeMessage>
}

impl<R: 'static + RenderContext> Runtime<R> {
    pub fn new() -> Self {
        Runtime {
            stack: vec![],
            clipping_stack: vec![],
            native_message_queue: VecDeque::new(),
        }
    }

    // TODO: this value could be cached on stackframes, registered & cached during engine rendertree traversal (specifically: when stackframes are pushed)
    //       This would make id_chain resolution essentially free, O(1) instead of O(log(n))
    //       Profile first to understand the impact before optimizing
    pub fn get_list_of_repeat_indicies_from_stack(&self) -> Vec<u64> {
        let mut indices: Vec<u64> = vec![];

        self.stack.iter().for_each(|frame_wrapped|{
            if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*(*frame_wrapped).borrow_mut()).borrow().properties).borrow() {
                indices.push(*i as u64)
            }
        });
        indices
    }

    //return current state of native message queue, passing in a freshly initialized queue for next frame
    pub fn swap_native_message_queue(&mut self) -> VecDeque<pax_message::NativeMessage> {
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
        }else{
            None
        }
    }

    /// Remove the top element from the stack.  Currently does
    /// nothing with the value of the popped StackFrame.
    pub fn pop_stack_frame(&mut self){
        self.stack.pop(); //TODO: handle value here if needed
    }

    /// Add a new frame to the stack, passing a list of adoptees
    /// that may be handled by `Slot` and a scope that includes the PropertiesCoproduct of the associated Component
    pub fn push_stack_frame(&mut self, flattened_adoptees: RenderNodePtrList<R>, properties: Rc<RefCell<PropertiesCoproduct>>, timeline: Option<Rc<RefCell<Timeline>>>) {
        let parent = self.peek_stack_frame();

        self.stack.push(
            Rc::new(RefCell::new(
                StackFrame::new(flattened_adoptees, properties, parent, timeline)
            ))
        );
    }

    pub fn push_clipping_stack_id(&mut self, id_chain: Vec<u64>) {
        self.clipping_stack.push(id_chain);
    }

    pub fn pop_clipping_stack_id(&mut self) {
        self.clipping_stack.pop();
    }

    pub fn get_current_clipping_ids(&self) -> Vec<Vec<u64>> {
        self.clipping_stack.clone()
    }

    /// Handles special-cases like `for`/`Repeat`, where properties for the
    /// control flow primitive need to be computed out-of-lifecycle, and where nested child elements
    /// need to be treated as top-level elements.
    /// For example, for `<Stacker><Ellipse />for i in (0..3){ <Rectangle /> }</Stacker>`,
    /// without this special handling `Stacker` will receive only two adoptees: the `Ellipse` and the `Repeat` node
    /// created by `for`.  In other words `for`s children need to be treated as `<Stacker>`s children,
    /// and this processing allows that to happpen.
    /// Note that this must be recursive to handle nested cases of flattening, for example nested `for` loops
    pub fn process__should_flatten__adoptees_recursive(adoptee: &RenderNodePtr<R>, rtc: &mut RenderTreeContext<R>) -> Vec<RenderNodePtr<R>> {
        let mut adoptee_borrowed = (**adoptee).borrow_mut();
        if adoptee_borrowed.should_flatten() {
            //1. this is an `if` or `for` (etc.) — it needs its properties computed
            //   in order for its children to be correct
            adoptee_borrowed.compute_properties(rtc);
            //2. recurse into top-level should_flatten() nodes
            (*adoptee_borrowed.get_rendering_children()).borrow().iter().map(|top_level_child_node|{
                Runtime::process__should_flatten__adoptees_recursive(top_level_child_node, rtc)
            }).flatten().collect()
            //TODO: probably worth optimizing (pending profiling.)  Lots of allocation happening here -- flattening and collecting `Vec`s is probably not
            //the most efficient possible approach, and this is fairly hot-running code.
        } else {
            vec![Rc::clone(adoptee)]
        }
    }
}


/// Data structure for a single frame of our runtime stack, including
/// a reference to its parent frame, a list of `adoptees` for
/// prospective [`Slot`] consumption, and `properties` for
/// runtime evaluation, e.g. of Expressions.  StackFrames also track
/// timeline playhead position.
///
/// `Component`s push StackFrames when mounting and pop them when unmounting, thus providing a
/// hierarchical store of node-relevant data that can be bound to symbols, e.g. in expressions.
/// Note that `RepeatItem`s also push `StackFrame`s, because `RepeatItem` uses a `Component` internally.
pub struct StackFrame<R: 'static + RenderContext>
{
    adoptees: RenderNodePtrList<R>,
    properties: Rc<RefCell<PropertiesCoproduct>>,
    parent: Option<Rc<RefCell<StackFrame<R>>>>,
    timeline: Option<Rc<RefCell<Timeline>>>,
}

impl<R: 'static + RenderContext> StackFrame<R> {
    pub fn new(adoptees: RenderNodePtrList<R>, properties: Rc<RefCell<PropertiesCoproduct>>, parent: Option<Rc<RefCell<StackFrame<R>>>>, timeline: Option<Rc<RefCell<Timeline>>>) -> Self {
        StackFrame {
            adoptees,
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
                    Some(parent_frame) => {
                        (**parent_frame).borrow().get_timeline_playhead_position()
                    },
                    None => 0
                }
            },
            Some(timeline) => {
                (**timeline).borrow().playhead_position
            }
        }
    }

    // Traverses stack recursively `n` times to retrieve
    // Unchecked: will throw a runtime error if there are fewer than `n` descendants to traverse.
    pub fn nth_descendant(&self, n: isize) -> Rc<RefCell<StackFrame<R>>> {
        assert!(n > 0);
        self.nth_descendant_recursive(n, 0)
    }

    fn nth_descendant_recursive(&self, n: isize, depth: isize) -> Rc<RefCell<StackFrame<R>>> {
        let new_depth = depth + 1;
        let parent = self.parent.as_ref().unwrap();
        if new_depth == n {
            return Rc::clone(parent);
        }
        parent.deref().borrow().nth_descendant_recursive(n, new_depth)
    }

    pub fn get_properties(&self) -> Rc<RefCell<PropertiesCoproduct>> {
        Rc::clone(&self.properties)
    }

    pub fn get_unflattened_adoptees(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.adoptees)
    }

    pub fn nth_adoptee(&self, n: usize) -> Option<RenderNodePtr<R>> {
        match (*self.adoptees).borrow().get(n) {
            Some(i) => {Some(Rc::clone(i))}
            None => {None}
        }
    }

    pub fn has_adoptees(&self) -> bool {
        (*self.adoptees).borrow().len() > 0
    }

}
