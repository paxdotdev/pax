use crate::api::math::Point2;
use crate::api::Window;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::NativeMessage;
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::properties::UntypedProperty;
use pax_runtime_api::{borrow, borrow_mut, use_RefCell};
use_RefCell!();
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::{ExpandedNode, ExpressionTable, Globals};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpandedNodeIdentifier(pub u32);

impl ExpandedNodeIdentifier {
    // used for sending identifiers to chassis
    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

/// Shared context for properties pass recursion
pub struct RuntimeContext {
    next_uid: Cell<ExpandedNodeIdentifier>,
    messages: RefCell<Vec<NativeMessage>>,
    globals: RefCell<Globals>,
    root_node: RefCell<Weak<ExpandedNode>>,
    expression_table: Rc<ExpressionTable>,
    node_cache: RefCell<NodeCache>,
}

struct NodeCache {
    eid_to_node: HashMap<ExpandedNodeIdentifier, Rc<ExpandedNode>>,
    uni_to_eid: HashMap<UniqueTemplateNodeIdentifier, Vec<ExpandedNodeIdentifier>>,
}

impl NodeCache {
    fn new() -> Self {
        Self {
            eid_to_node: Default::default(),
            uni_to_eid: Default::default(),
        }
    }

    // Add this node to all relevant constant lookup cache structures
    fn add_to_cache(&mut self, node: &Rc<ExpandedNode>) {
        self.eid_to_node.insert(node.id, Rc::clone(&node));
        let uni = borrow!(node.instance_node)
            .base()
            .template_node_identifier
            .clone();
        if let Some(uni) = uni {
            self.uni_to_eid.entry(uni).or_default().push(node.id);
        }
    }

    // Remove this node from all relevant constant lookup cache structures
    fn remove_from_cache(&mut self, node: &Rc<ExpandedNode>) {
        if let Some(uni) = &borrow!(node.instance_node).base().template_node_identifier {
            self.uni_to_eid.remove(uni);
        }
    }
}

impl RuntimeContext {
    pub fn new(expression_table: ExpressionTable, globals: Globals) -> Self {
        Self {
            next_uid: Cell::new(ExpandedNodeIdentifier(0)),
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            expression_table: Rc::new(expression_table),
            root_node: RefCell::new(Weak::new()),
            node_cache: RefCell::new(NodeCache::new()),
        }
    }

    pub fn register_root_node(&self, root: &Rc<ExpandedNode>) {
        *borrow_mut!(self.root_node) = Rc::downgrade(root);
    }

    pub fn add_to_cache(&self, node: &Rc<ExpandedNode>) {
        borrow_mut!(self.node_cache).add_to_cache(node);
    }

    pub fn remove_from_cache(&self, node: &Rc<ExpandedNode>) {
        borrow_mut!(self.node_cache).remove_from_cache(node);
    }

    pub fn get_expanded_node_by_eid(&self, id: ExpandedNodeIdentifier) -> Option<Rc<ExpandedNode>> {
        borrow!(self.node_cache).eid_to_node.get(&id).cloned()
    }

    /// Finds all ExpandedNodes with the CommonProperty#id matching the provided string
    pub fn get_expanded_nodes_by_id(&self, id: &str) -> Vec<Rc<ExpandedNode>> {
        //v0 limitation: currently an O(n) lookup cost (could be made O(1) with an id->expandednode cache)
        borrow!(self.node_cache)
            .eid_to_node
            .values()
            .filter(|val| {
                let common_props = val.get_common_properties();
                let common_props = borrow!(common_props);
                if let Some(other_id) = &common_props.id {
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
        let node_cache = borrow!(self.node_cache);
        node_cache
            .uni_to_eid
            .get(uni)
            .map(|eids| {
                let mut nodes = vec![];
                for e in eids {
                    nodes.extend(
                        node_cache
                            .eid_to_node
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

        let root_node = borrow!(self.root_node).upgrade().unwrap();
        root_node.recurse_visit_postorder(&mut |node| {
            if node.ray_cast_test(ray) {
                //We only care about the topmost node getting hit, and the element
                //pool is ordered by z-index so we can just resolve the whole
                //calculation when we find the first matching node

                let mut ancestral_clipping_bounds_are_satisfied = true;
                let mut parent: Option<Rc<ExpandedNode>> =
                    borrow!(node.parent_expanded_node).upgrade();

                loop {
                    if let Some(unwrapped_parent) = parent {
                        if let Some(_) = unwrapped_parent.get_clipping_size() {
                            ancestral_clipping_bounds_are_satisfied =
                                (*unwrapped_parent).ray_cast_test(ray);
                            break;
                        }
                        parent = borrow!(unwrapped_parent.parent_expanded_node).upgrade();
                    } else {
                        break;
                    }
                }

                if ancestral_clipping_bounds_are_satisfied {
                    accum.push(Rc::clone(&node));
                }
            }
        });
        // TODO this isn't efficient, should make a iterator impl for expanded node instead
        accum.reverse();
        accum.pop();
        if limit_one {
            accum.truncate(1);
        }
        accum
    }

    /// Alias for `get_elements_beneath_ray` with `limit_one = true`
    pub fn get_topmost_element_beneath_ray(&self, ray: Point2<Window>) -> Option<Rc<ExpandedNode>> {
        let res = self.get_elements_beneath_ray(ray, true, vec![]);
        let res = if res.len() == 0 {
            None
        } else if res.len() == 1 {
            Some(res.get(0).unwrap().clone())
        } else {
            unreachable!() //bug in limit_one logic
        };
        res
    }

    pub fn gen_uid(&self) -> ExpandedNodeIdentifier {
        let val = self.next_uid.get();
        let next_val = ExpandedNodeIdentifier(val.0 + 1);
        self.next_uid.set(next_val);
        val
    }

    pub fn enqueue_native_message(&self, message: NativeMessage) {
        borrow_mut!(self.messages).push(message)
    }

    pub fn take_native_messages(&self) -> Vec<NativeMessage> {
        let mut messages = borrow_mut!(self.messages);
        std::mem::take(&mut *messages)
    }

    pub fn globals(&self) -> Globals {
        borrow!(self.globals).clone()
    }

    pub fn edit_globals(&self, f: impl Fn(&mut Globals)) {
        let mut globals = borrow_mut!(self.globals);
        f(&mut globals);
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

pub struct RuntimePropertiesStackFrame {
    symbols_within_frame: HashMap<String, UntypedProperty>,
    properties: Rc<RefCell<PaxAny>>,
    parent: Weak<RuntimePropertiesStackFrame>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(
        symbols_within_frame: HashMap<String, UntypedProperty>,
        properties: Rc<RefCell<PaxAny>>,
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
        properties: &Rc<RefCell<PaxAny>>,
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
    pub fn peek_nth(self: &Rc<Self>, n: isize) -> Option<Rc<RefCell<PaxAny>>> {
        let mut curr = Rc::clone(self);
        for _ in 0..n {
            curr = curr.parent.upgrade()?;
        }
        Some(Rc::clone(&curr.properties))
    }

    pub fn resolve_symbol(&self, symbol: &str) -> Option<Rc<RefCell<PaxAny>>> {
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

    pub fn get_properties(&self) -> Rc<RefCell<PaxAny>> {
        Rc::clone(&self.properties)
    }
}

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext {
    pub stack_frame: Rc<RuntimePropertiesStackFrame>,
}
