use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;

use pax_properties_coproduct::{PropertiesCoproduct};
use crate::{HandlerRegistry, RenderNodePtr, RenderNodePtrList, RenderTreeContext};

use pax_runtime_api::{Timeline};


//
// pub trait Dispatcher<'a, T> {
//     fn get_handler_registry(&self) -> &'a HandlerRegistry<T>;
//
//     fn dispatch_event(&mut self, args: ArgsCoproduct) {
//         //first: what kind of event is this?  does this element have registered handlers for that event?
//         //only if so, then unwrap `&self.properties` into cast_properties, invoke handler(s)
//         match args {
//             ArgsCoproduct::Tick(mut args_cast) => {
//                 //are there tick handlers registered?
//                 if self.get_handler_registry().tick_handlers.len() > 0 {
//                     for handler in self.get_handler_registry().tick_handlers.iter() {
//                         handler(&self.unwrap_properties(), args_cast.clone())
//                     }
//                 }
//             },
//             ArgsCoproduct::Click(args_cast) => {
//                 //are there click handlers registered?
//                 if self.get_handler_registry().click_handlers.len() > 0 {
//                     for handler in self.get_handler_registry().click_handlers.iter() {
//                         handler(&self.unwrap_properties(), args_cast.clone())
//                     }
//                 }
//             },
//             //TODO: Support additional handlers by adding them here.
//         }
//     }
//
//     //to be written manually for primitives, codegenned for pax components
//     fn unwrap_properties(&mut self) -> T;
// }


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
    /// that may be handled by `Placeholder` and a scope that includes
    pub fn push_stack_frame(&mut self, adoptees: RenderNodePtrList, scope: Box<Scope>, timeline: Option<Rc<RefCell<Timeline>>>, should_skip_adoption: bool) {

        let parent = self.peek_stack_frame();

        //TODO: track index/map for `nth_adoptee` to optimize hot-running lookup logic

        self.stack.push(
            Rc::new(RefCell::new(
                StackFrame::new(adoptees, Rc::new(RefCell::new(*scope)), parent, timeline, should_skip_adoption)
            ))
        );
    }

}


/// `Scope` attaches to stack frames to provide an evaluation context + relevant data access
/// for features like Expressions.
///
/// The stored values that are DI'ed into expressions are held in these scopes,
/// e.g. `index` and `datum` for `Repeat`.
pub struct Scope {
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    // TODO: children, parent, etc.
}






/// Data structure for a single frame of our runtime stack, including
/// a reference to its parent frame, a list of `adoptees` for
/// prospective [`Placeholder`] consumption, and a `Scope` for
/// runtime evaluation, e.g. of Expressions.  StackFrames also track
/// timeline playhead position.
pub struct StackFrame
{
    adoptees: RenderNodePtrList,
    scope: Rc<RefCell<Scope>>,
    parent: Option<Rc<RefCell<StackFrame>>>,
    timeline: Option<Rc<RefCell<Timeline>>>,
    should_skip_adoption: bool,
}

impl StackFrame {
    pub fn new(adoptees: RenderNodePtrList, scope: Rc<RefCell<Scope>>, parent: Option<Rc<RefCell<StackFrame>>>, timeline: Option<Rc<RefCell<Timeline>>>, should_skip_adoption: bool) -> Self {
        StackFrame {
            adoptees: Rc::clone(&adoptees),
            scope,
            parent,
            timeline,
            should_skip_adoption,
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

    // pub fn pop_adoptee(&mut self) -> Option<RenderNodePtr> {
    //     //pop adoptee from local stackframe.
    //     //if not present, recurse to parent.
    //     let mut adoptees_borrowed = (*&self.adoptees).borrow_mut();
    //     match adoptees_borrowed.pop() {
    //         Some(adoptee) => {
    //             Some(adoptee)
    //         },
    //         None => {
    //             //recurse to parent
    //             match &self.parent {
    //                 Some(parent) => {
    //                     (*parent).borrow_mut().pop_adoptee()
    //                 },
    //                 None => {
    //                     //no adoptees; no parent; nada
    //                     None
    //                 }
    //             }
    //         }
    //     }
    // }

    fn recurse_get_adoptees(maybe_parent: &Option<Rc<RefCell<StackFrame>>>) -> Option<RenderNodePtrList> {
        match maybe_parent {
            Some(parent) => {
                if (**parent).borrow().should_skip_adoption {
                    StackFrame::recurse_get_adoptees(&(**parent).borrow().parent)
                } else {
                    Some(Rc::clone(&(**parent).borrow().adoptees))
                }
            },
            None => {
                None
            }
        }
    }

    pub fn nth_adoptee(&self, i: usize) -> Option<RenderNodePtr> {

        //first, determine which frame we should draw adoptees from


        let adoptees = if self.should_skip_adoption {
            StackFrame::recurse_get_adoptees(&self.parent)
        } else {
            Some(Rc::clone(&self.adoptees))
        };


        // match adoptees {
        //
        // }

        // let mut frame = self;
        // loop {
        //     if !frame.should_skip_adoption {
        //         //frame is now correct
        //         break;
        //     } else {
        //         frame = match &frame.parent {
        //             Some(parent) => {
        //                 &(**parent).borrow()
        //             },
        //             None => {
        //                 //no parent, no adoptees
        //                 return None;
        //             }
        //         }
        //     }
        // };

        todo!()

        // let appropriate_frame = if &self.should_skip_adoption {
        //     let ancestor = &self.parent;
        //
        //     let ancestor = match &self.parent {
        //
        //     }
        // } else {
        //
        // }
        //find list of adoptees on appropriate stackframe
        // - this means dumb upward traversal, or perhaps adding a flag for `skip_adoption` to ComponentInstance => StackFrame
        //walk that list linearly; for each node, if it is `should_flatten`, then query its children and continue the indexed walk (recurse this expansion for top-level `should_flatten` nodes only.)
        //once `n` is reached, return the node; if there are fewer than `n` walkable nodes, return None
        //can be optimized by memoization; StackFrames are reset every tick but can be memoized in the scope of:
        //1. a given frame, so that subsequent lookups for a given frame are optimized, and/or
        //2. detecting graph mutations, only recalculating when mutations occur
    }

    pub fn has_adoptees(&self) -> bool {
        (*self.adoptees).borrow().len() > 0
    }

    /// Returns the adoptees attached to this stack frame, if present.
    /// Otherwise, recurses up the stack return ancestors' adoptees if found
    /// TODO:  if this logic is problematic, e.g. descendants are grabbing ancestors' adoptees
    ///        inappropriately, then we could adjust this logic to:
    ///        grab direct parent's adoptees instead of current node's,
    ///        but only if current node is a `should_flatten` node like `Repeat`
    // pub fn get_adoptees(&self) -> RenderNodePtrList {
    //
    //
    //     //try surgically flattening as necessary, recombining
    //     if self.has_adoptees() {
    //
    //         //this is expensive and should be revisited.  Perhaps take a stab
    //         //at a RenderNodePtr refactor, better abstracting single node from list (e.g. always list? sometimes with n=1 elements)
    //
    //         let flattened_list = (*&self.adoptees).borrow().iter().map(|node|{
    //             if (*node).borrow().should_flatten() {
    //                 (*(*node).borrow().get_rendering_children()).borrow().clone()
    //             } else {
    //                 vec![Rc::clone(node)]
    //             }
    //         }).flatten().collect();
    //
    //         Rc::new(RefCell::new(flattened_list))
    //     }else {
    //         Rc::clone(&empty_adoptees)
    //     }
        // else {
        //     match &self.parent {
        //         Some(parent_frame) => {
        //             parent_frame.borrow().get_adoptees()
        //         },
        //         None => Rc::new(RefCell::new(vec![]))
        //     }
        // }

    // }

    pub fn get_scope(&self) -> Rc<RefCell<Scope>> {
        Rc::clone(&self.scope)
    }
}
