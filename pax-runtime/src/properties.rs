use crate::api::math::Point2;
use crate::api::Window;
use pax_lang::interpreter::IdentifierResolver;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::NativeMessage;
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::properties::UntypedProperty;
use pax_runtime_api::{borrow, borrow_mut, use_RefCell, Interpolatable, PaxValue, Store, Variable};
use_RefCell!();
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::{ExpandedNode, Globals};

impl Interpolatable for ExpandedNodeIdentifier {}

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
    node_cache: RefCell<NodeCache>,
    queued_custom_events: RefCell<Vec<(Rc<ExpandedNode>, &'static str)>>,
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
        self.eid_to_node.remove(&node.id);
        if let Some(uni) = &borrow!(node.instance_node).base().template_node_identifier {
            self.uni_to_eid
                .entry(uni.clone())
                .or_default()
                .retain(|&n| n != node.id);
        }
    }
}

impl RuntimeContext {
    pub fn new(globals: Globals) -> Self {
        Self {
            next_uid: Cell::new(ExpandedNodeIdentifier(0)),
            messages: RefCell::new(Vec::new()),
            globals: RefCell::new(globals),
            root_node: RefCell::new(Weak::new()),
            node_cache: RefCell::new(NodeCache::new()),
            queued_custom_events: Default::default(),
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
                common_props.id.get().is_some_and(|i| i == id)
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
        hit_invisible: bool,
    ) -> Vec<Rc<ExpandedNode>> {
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        let root_node = borrow!(self.root_node).upgrade().unwrap();
        let mut to_process = vec![root_node];
        while let Some(node) = to_process.pop() {
            let hit = node.ray_cast_test(ray);
            if hit {
                if hit_invisible
                    || !borrow!(node.instance_node)
                        .base()
                        .flags()
                        .invisible_to_raycasting
                {
                    //We only care about the topmost node getting hit, and the element
                    //pool is ordered by z-index so we can just resolve the whole
                    //calculation when we find the first matching node
                    if limit_one {
                        return vec![node];
                    }
                    accum.push(Rc::clone(&node));
                }
            }

            if hit || !borrow!(node.instance_node).clips_content(&node) {
                to_process.extend(node.children.get().iter().cloned().rev())
            }
        }
        accum
    }

    /// Alias for `get_elements_beneath_ray` with `limit_one = true`
    pub fn get_topmost_element_beneath_ray(&self, ray: Point2<Window>) -> Option<Rc<ExpandedNode>> {
        let res = self.get_elements_beneath_ray(ray, true, vec![], false);
        Some(
            res.into_iter()
                .next()
                .unwrap_or(borrow!(self.root_node).upgrade().unwrap()),
        )
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

    pub fn queue_custom_event(&self, source_expanded_node: Rc<ExpandedNode>, name: &'static str) {
        let mut queued_custom_events = borrow_mut!(self.queued_custom_events);
        queued_custom_events.push((source_expanded_node, name));
    }

    pub fn flush_custom_events(self: &Rc<Self>) -> Result<(), String> {
        let mut queued_custom_event = borrow_mut!(self.queued_custom_events);
        let to_flush: Vec<_> = std::mem::take(queued_custom_event.as_mut());
        for (target, ident) in to_flush {
            target.dispatch_custom_event(ident, self)?;
        }
        Ok(())
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
    symbols_within_frame: HashMap<String, Variable>,
    local_stores: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,
    properties: Rc<RefCell<PaxAny>>,
    parent: Weak<RuntimePropertiesStackFrame>,
}

impl RuntimePropertiesStackFrame {
    pub fn new(
        symbols_within_frame: HashMap<String, Variable>,
        properties: Rc<RefCell<PaxAny>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            symbols_within_frame,
            properties,
            local_stores: Default::default(),
            parent: Weak::new(),
        })
    }

    pub fn push(
        self: &Rc<Self>,
        symbols_within_frame: HashMap<String, Variable>,
        properties: &Rc<RefCell<PaxAny>>,
    ) -> Rc<Self> {
        Rc::new(RuntimePropertiesStackFrame {
            symbols_within_frame,
            local_stores: Default::default(),
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

    pub fn insert_stack_local_store<T: Store>(&self, store: T) {
        let type_id = TypeId::of::<T>();
        borrow_mut!(self.local_stores).insert(type_id, Box::new(store));
    }

    pub fn peek_stack_local_store<T: Store, V>(
        self: &Rc<Self>,
        f: impl FnOnce(&mut T) -> V,
    ) -> Result<V, String> {
        let mut current = Rc::clone(self);
        let type_id = TypeId::of::<T>();

        while !borrow!(current.local_stores).contains_key(&type_id) {
            current = current
                .parent
                .upgrade()
                .ok_or_else(|| format!("couldn't find store in local stack"))?;
        }
        let v = {
            let mut stores = borrow_mut!(current.local_stores);
            let store = stores.get_mut(&type_id).unwrap().downcast_mut().unwrap();
            f(store)
        };
        Ok(v)
    }

    pub fn resolve_symbol_as_erased_property(&self, symbol: &str) -> Option<UntypedProperty> {
        if let Some(e) = self.symbols_within_frame.get(symbol) {
            Some(e.clone().get_untyped_property().clone())
        } else {
            self.parent
                .upgrade()?
                .resolve_symbol_as_erased_property(symbol)
        }
    }

    pub fn resolve_symbol_as_pax_value(&self, symbol: &str) -> Option<PaxValue> {
        if let Some(e) = self.symbols_within_frame.get(symbol) {
            Some(e.get_as_pax_value())
        } else {
            self.parent.upgrade()?.resolve_symbol_as_pax_value(symbol)
        }
    }

    pub fn get_properties(&self) -> Rc<RefCell<PaxAny>> {
        Rc::clone(&self.properties)
    }
}

impl IdentifierResolver for RuntimePropertiesStackFrame {
    fn resolve(&self, name: String) -> Result<PaxValue, String> {
        self.resolve_symbol_as_pax_value(&name)
            .ok_or_else(|| format!("Could not resolve symbol {}", name))
    }
}

/// Data structure used for dynamic injection of values
/// into Expressions, maintaining a pointer e.g. to the current
/// stack frame to enable evaluation of properties & dependencies
pub struct ExpressionContext {
    pub stack_frame: Rc<RuntimePropertiesStackFrame>,
}
