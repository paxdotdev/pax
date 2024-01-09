use pax_message::NativeMessage;
use pax_runtime_api::Numeric;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{ExpressionTable, Globals};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uid(pub u32);

/// Shared context for properties pass recursion
pub struct RuntimeContext {
    next_uid: Uid,
    messages: Vec<NativeMessage>,
    globals: Globals,
    expression_table: ExpressionTable,
}

impl RuntimeContext {
    pub fn new(expression_table: ExpressionTable, globals: Globals) -> Self {
        Self {
            next_uid: Uid(0),
            messages: Vec::new(),
            globals,
            expression_table,
        }
    }

    pub fn gen_uid(&mut self) -> Uid {
        self.next_uid.0 += 1;
        self.next_uid
    }

    pub fn enqueue_native_message(&mut self, message: NativeMessage) {
        self.messages.push(message)
    }

    pub fn take_native_messages(&mut self) -> Vec<NativeMessage> {
        std::mem::take(&mut self.messages)
    }

    pub fn globals(&self) -> &Globals {
        &self.globals
    }

    pub fn globals_mut(&mut self) -> &mut Globals {
        &mut self.globals
    }

    pub fn expression_table(&self) -> &ExpressionTable {
        &self.expression_table
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
    parent: Option<Rc<RuntimePropertiesStackFrame>>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(properties: Rc<RefCell<dyn Any>>) -> Rc<Self> {
        Rc::new(Self {
            properties,
            parent: None,
        })
    }

    pub fn push(self: &Rc<Self>, properties: &Rc<RefCell<dyn Any>>) -> Rc<Self> {
        Rc::new(RuntimePropertiesStackFrame {
            parent: Some(Rc::clone(&self)),
            properties: Rc::clone(properties),
        })
    }

    pub fn pop(self: &Rc<Self>) -> Option<Rc<Self>> {
        self.parent.clone()
    }

    /// Traverses stack recursively `n` times to retrieve ancestor;
    /// useful for runtime lookups for identifiers, where `n` is the statically known offset determined by the Pax compiler
    /// when resolving a symbol
    pub fn peek_nth(self: &Rc<Self>, n: isize) -> Option<Rc<RefCell<dyn Any>>> {
        let mut curr = Rc::clone(self);
        for _ in 0..n {
            curr = Rc::clone(curr.parent.as_ref()?);
        }
        Some(Rc::clone(&curr.properties))
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
