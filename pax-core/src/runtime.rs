use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use pax_properties_coproduct::{PropertiesCoproduct};
use pax_runtime_api::{Timeline};

use crate::{HandlerRegistry, RenderNodePtr, RenderNodePtrList, RenderTreeContext};

/// `Runtime` is a container for data and logic needed by the `Engine`,
/// explicitly aside from rendering.  For example, this is a home
/// for logic that manages scopes and stack frames.
pub struct Runtime {
    stack: Vec<Rc<RefCell<StackFrame>>>,
    logger: fn(&str),
}

impl Runtime {
    pub fn new(logger: fn(&str)) -> Self {
        Runtime {
            stack: Vec::new(),
            logger,
        }
    }

    pub fn log(&self, message: &str) {
        (&self.logger)(message);
    }

    /// Return a pointer to the top StackFrame on the stack,
    /// without mutating the stack or consuming the value
    pub fn peek_stack_frame(&mut self) -> Option<Rc<RefCell<StackFrame>>> {
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
    pub fn push_stack_frame(&mut self, expanded_adoptees: RenderNodePtrList, properties: Rc<RefCell<PropertiesCoproduct>>, timeline: Option<Rc<RefCell<Timeline>>>) {
        let parent = self.peek_stack_frame();

        self.stack.push(
            Rc::new(RefCell::new(
                StackFrame::new(expanded_adoptees, properties, parent, timeline)
            ))
        );
    }

    /// Handles special-cases like `@for`/`Repeat`, where properties for the
    /// control flow primitive need to be computed out-of-lifecycle, and where nested child elements
    /// need to be treated as top-level elements.
    /// For example, for `<Spread><Ellipse />@for i in (0..3){ <Rectangle /> }</Spread>`,
    /// without this special handling `Spread` will receive only two adoptees: the `Ellipse` and the `Repeat` node
    /// created by `@for`.  In other words `@for`s children need to be treated as `<Spread>`s children,
    /// and this processing allows that to happpen.
    /// Note that this must be recursive to handle nested cases of flattening, for example nested `@for` loops
    pub fn process__should_flatten__adoptees_recursive(adoptee: &RenderNodePtr, rtc: &mut RenderTreeContext) -> Vec<RenderNodePtr> {
        let mut adoptee_borrowed = (**adoptee).borrow_mut();
        if adoptee_borrowed.should_flatten() {
            //1. compute properties
            adoptee_borrowed.compute_properties(rtc);
            //2. recurse into top-level should_flatten() nodes
            (*adoptee_borrowed.get_rendering_children()).borrow().iter().map(|top_level_child_node|{
                Runtime::process__should_flatten__adoptees_recursive(top_level_child_node, rtc)
            }).flatten().collect()
            //TODO: optimize.  Lots of allocation happening here -- flattening and collecting `Vec`s is probably not
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
pub struct StackFrame
{
    adoptees: RenderNodePtrList,
    properties: Rc<RefCell<PropertiesCoproduct>>,
    parent: Option<Rc<RefCell<StackFrame>>>,
    timeline: Option<Rc<RefCell<Timeline>>>,
}

impl StackFrame {
    pub fn new(adoptees: RenderNodePtrList, properties: Rc<RefCell<PropertiesCoproduct>>, parent: Option<Rc<RefCell<StackFrame>>>, timeline: Option<Rc<RefCell<Timeline>>>) -> Self {
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
    // TODO: more elegant error handling?
    pub fn nth_descendant(&self, n: isize) -> Rc<RefCell<StackFrame>> {
        if n == 0 {
            unreachable!("nth_descendant")
        }
        self.nth_descendant_recursive(n, 0)
    }

    fn nth_descendant_recursive(&self, n: isize, depth: isize) -> Rc<RefCell<StackFrame>> {
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

    pub fn get_unexpanded_adoptees(&self) -> RenderNodePtrList {
        Rc::clone(&self.adoptees)
    }

    pub fn nth_adoptee(&self, n: usize) -> Option<RenderNodePtr> {
        match (*self.adoptees).borrow().get(n) {
            Some(i) => {Some(Rc::clone(i))}
            None => {None}
        }
    }

    pub fn has_adoptees(&self) -> bool {
        (*self.adoptees).borrow().len() > 0
    }

}
