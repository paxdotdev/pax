use crate::api::math::Point2;
use crate::api::Window;
use crate::numeric::Numeric;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::NativeMessage;
use pax_runtime_api::properties::UntypedProperty;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::{any::Any, collections::HashMap};

use crate::{ExpandedNode, ExpressionTable, Globals};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ExpandedNodeIdentifier(pub u32);

impl ExpandedNodeIdentifier {
    // used for sending identifiers to chassis
    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Default)]
pub struct NodeCache {
    pub lookup: HashMap<u32, ExpandedNode>,
}

/// Shared context for properties pass recursion
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct RuntimeContext {
    next_uid: ExpandedNodeIdentifier,
    messages: Vec<NativeMessage>,
    globals: Globals,
    expression_table: Rc<ExpressionTable>,
    pub z_index_node_cache: Vec<Rc<ExpandedNode>>,
    pub node_cache: HashMap<ExpandedNodeIdentifier, Rc<ExpandedNode>>,
    pub uni_to_eid: HashMap<UniqueTemplateNodeIdentifier, Vec<ExpandedNodeIdentifier>>,
}

impl RuntimeContext {
    pub fn new(expression_table: ExpressionTable, globals: Globals) -> Self {
        Self {
            next_uid: ExpandedNodeIdentifier(0),
            messages: Vec::new(),
            globals,
            expression_table: Rc::new(expression_table),
            z_index_node_cache: vec![],
            node_cache: HashMap::default(),
            uni_to_eid: HashMap::default(),
        }
    }

    /// Finds all ExpandedNodes with the CommonProperty#id matching the provided string
    pub fn get_expanded_nodes_by_id(&self, id: &str) -> Vec<Rc<ExpandedNode>> {
        //v0 limitation: currently an O(n) lookup cost (could be made O(1) with an id->expandednode cache)
        self.node_cache
            .values()
            .filter(|val| {
                if let Some(other_id) = &val.get_common_properties().borrow().id {
                    other_id.get() == id
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    /// Finds all ExpandedNodes with corresponding UniqueTemplateNodeIdentifier
    pub fn get_expanded_nodes_by_global_ids(
        &self,
        uni: &UniqueTemplateNodeIdentifier,
    ) -> Vec<Rc<ExpandedNode>> {
        self.uni_to_eid
            .get(uni)
            .map(|eids| {
                let mut nodes = vec![];
                for e in eids {
                    nodes.extend(
                        self.node_cache
                            .get(e)
                            .map(|node| vec![Rc::clone(node)])
                            .unwrap_or_default(),
                    )
                }
                nodes
            })
            .unwrap_or_default()
    }

    /// Simple 2D raycasting: the coordinates of the ray represent a
    /// ray running orthogonally to the view plane, intersecting at
    /// the specified point `ray`.  Areas outside of clipping bounds will
    /// not register a `hit`, nor will elements that suppress input events.
    pub fn get_elements_beneath_ray(
        &self,
        ray: Point2<Window>,
        limit_one: bool,
        mut accum: Vec<Rc<ExpandedNode>>,
    ) -> Vec<Rc<ExpandedNode>> {
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        for node in self.z_index_node_cache.iter().rev().skip(1) {
            if node.ray_cast_test(ray) {
                //We only care about the topmost node getting hit, and the element
                //pool is ordered by z-index so we can just resolve the whole
                //calculation when we find the first matching node

                let mut ancestral_clipping_bounds_are_satisfied = true;
                let mut parent: Option<Rc<ExpandedNode>> =
                    node.parent_expanded_node.borrow().upgrade();

                loop {
                    if let Some(unwrapped_parent) = parent {
                        if let Some(_) = unwrapped_parent.get_clipping_size() {
                            ancestral_clipping_bounds_are_satisfied =
                                (*unwrapped_parent).ray_cast_test(ray);
                            break;
                        }
                        parent = unwrapped_parent.parent_expanded_node.borrow().upgrade();
                    } else {
                        break;
                    }
                }

                if ancestral_clipping_bounds_are_satisfied {
                    accum.push(Rc::clone(&node));
                    if limit_one {
                        return accum;
                    }
                }
            }
        }
        accum
    }

    /// Alias for `get_elements_beneath_ray` with `limit_one = true`
    pub fn get_topmost_element_beneath_ray(&self, ray: Point2<Window>) -> Option<Rc<ExpandedNode>> {
        let res = self.get_elements_beneath_ray(ray, true, vec![]);
        if res.len() == 0 {
            None
        } else if res.len() == 1 {
            Some(res.get(0).unwrap().clone())
        } else {
            unreachable!() //bug in limit_one logic
        }
    }

    pub fn gen_uid(&mut self) -> ExpandedNodeIdentifier {
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

    pub fn expression_table(&self) -> Rc<ExpressionTable> {
        self.expression_table.clone()
    }
}

/// Data structure for a single frame of our runtime stack, including
/// a reference to its parent frame and `properties` for
/// runtime evaluation, e.g. of Expressions.  `RuntimePropertiesStackFrame`s also track
/// timeline playhead position.
///
/// `Component`s push `RuntimePropertiesStackFrame`s before computing properties and pop them after computing, thus providing a
/// hierarchical store of node-relevant data that can be bound to symbols in expressions.
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct RuntimePropertiesStackFrame {
    symbols_within_frame: HashMap<String, UntypedProperty>,
    properties: Rc<RefCell<dyn Any>>,
    parent: Weak<RuntimePropertiesStackFrame>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(
        symbols_within_frame: HashMap<String, UntypedProperty>,
        properties: Rc<RefCell<dyn Any>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            symbols_within_frame,
            properties,
            parent: Weak::new(),
        })
    }

    pub fn push(
        self: &Rc<Self>,
        symbols_within_frame: HashMap<String, UntypedProperty>,
        properties: &Rc<RefCell<dyn Any>>,
    ) -> Rc<Self> {
        Rc::new(RuntimePropertiesStackFrame {
            symbols_within_frame,
            parent: Rc::downgrade(&self),
            properties: Rc::clone(properties),
        })
    }

    pub fn pop(self: &Rc<Self>) -> Option<Rc<Self>> {
        self.parent.upgrade()
    }

    /// Traverses stack recursively `n` times to retrieve ancestor;
    /// useful for runtime lookups for identifiers, where `n` is the statically known offset determined by the Pax compiler
    /// when resolving a symbol
    pub fn peek_nth(self: &Rc<Self>, n: isize) -> Option<Rc<RefCell<dyn Any>>> {
        let mut curr = Rc::clone(self);
        for _ in 0..n {
            curr = curr.parent.upgrade()?;
        }
        Some(Rc::clone(&curr.properties))
    }

    pub fn resolve_symbol(&self, symbol: &str) -> Option<Rc<RefCell<dyn Any>>> {
        if let Some(_) = self.symbols_within_frame.get(symbol) {
            Some(Rc::clone(&self.properties))
        } else {
            self.parent.upgrade()?.resolve_symbol(symbol)
        }
    }

    pub fn resolve_symbol_as_erased_property(&self, symbol: &str) -> Option<UntypedProperty> {
        if let Some(e) = self.symbols_within_frame.get(symbol) {
            Some(e.clone())
        } else {
            self.parent
                .upgrade()?
                .resolve_symbol_as_erased_property(symbol)
        }
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

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ExpressionContext {
    pub stack_frame: Rc<RuntimePropertiesStackFrame>,
}
