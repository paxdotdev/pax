//! Scoped queries over mounted application instances.

use std::{
    collections::HashSet,
    fmt,
    rc::{Rc, Weak},
};

use pax_manifest::SelectorExpr;
use pax_runtime_api::pax_value::PaxValue;

use crate::{ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};

#[cfg(test)]
pub(crate) mod tests;

/// A failed node lookup or tree operation. Failed moves leave the tree unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeError {
    /// The declaration's lifetime has ended, or the application is unavailable.
    Unmounted,
    /// A selector could not be parsed.
    InvalidSelector(String),
    /// Exactly one node was requested, but a different number matched.
    ExpectedOne(usize),
    /// A position is outside the selected collection.
    IndexOutOfBounds { index: usize, len: usize },
    /// The supplied node is not a member of this collection.
    NotAChild,
    /// Nodes from different application runtimes cannot be transferred.
    DifferentApplication,
    /// The operation would create a cycle in the presentation tree.
    Cycle,
    /// This node cannot receive externally transferred content.
    InvalidDestination,
    /// The authored placement is no longer mounted or available.
    HomeUnavailable,
    /// Animated moves require a finite, nondegenerate starting footprint.
    InvalidMoveGeometry,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unmounted => write!(f, "node reference is no longer available"),
            Self::InvalidSelector(s) => write!(f, "invalid node selector: {s}"),
            Self::ExpectedOne(n) => write!(f, "expected one node, found {n}"),
            Self::IndexOutOfBounds { index, len } => write!(
                f,
                "child index {index} is outside collection of length {len}"
            ),
            Self::NotAChild => write!(f, "node is not an active child of this container"),
            Self::DifferentApplication => {
                write!(f, "cannot move nodes between application runtimes")
            }
            Self::Cycle => write!(f, "move would create a presentation cycle"),
            Self::InvalidDestination => write!(f, "node cannot receive transferred content"),
            Self::HomeUnavailable => write!(f, "the node's authored placement is unavailable"),
            Self::InvalidMoveGeometry => write!(
                f,
                "animated move requires a finite, nondegenerate footprint"
            ),
        }
    }
}
impl std::error::Error for NodeError {}

/// A reference to one declaration-owned instance. Explicit moves preserve its
/// mount. An abandoned transfer can be recovered into a fresh mount; ordinary
/// declaration removal permanently invalidates this reference.
#[derive(Clone)]
pub struct NodeRef {
    pub(crate) node: Weak<ExpandedNode>,
    pub(crate) context: Weak<RuntimeContext>,
    pub(crate) epoch: u64,
    id: ExpandedNodeIdentifier,
}

impl fmt::Debug for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRef")
            .field("id", &self.id)
            .field("generation", &self.epoch)
            .finish()
    }
}

impl PartialEq for NodeRef {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.epoch == other.epoch && self.context.ptr_eq(&other.context)
    }
}
impl Eq for NodeRef {}

/// The string or integer identity used by an existing keyed repeat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKey {
    String(String),
    Integer(i64),
}
impl From<&str> for NodeKey {
    fn from(v: &str) -> Self {
        Self::String(v.into())
    }
}
impl From<String> for NodeKey {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}
impl From<i64> for NodeKey {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}
impl From<i32> for NodeKey {
    fn from(v: i32) -> Self {
        Self::Integer(v.into())
    }
}

/// An ordered snapshot of node references. Queries never silently select the
/// first match or change the identity of a previously returned reference.
#[derive(Clone, Debug)]
pub struct NodeQuery {
    nodes: Vec<NodeRef>,
    error: Option<NodeError>,
}

/// Easing configuration for one explicit move. The same destination methods are
/// available as on `NodeRef`; an abandoned node remounts without a flight origin.
#[must_use]
pub struct AnimatedNodeMove {
    node: NodeRef,
    duration: pax_runtime_api::Duration,
    easing: pax_runtime_api::EasingCurve,
}

impl AnimatedNodeMove {
    fn run(
        self,
        operation: impl FnOnce(&NodeRef) -> Result<(), NodeError>,
    ) -> Result<(), NodeError> {
        use pax_runtime_api::math::{Transform2, Vector2};
        let (node, context) = self.node.resolve()?;
        if node.attached.get() == 0 {
            return operation(&self.node);
        }
        let footprint = node.transform_and_bounds.get();
        if footprint.bounds.0 <= 1e-9
            || footprint.bounds.1 <= 1e-9
            || !footprint.bounds.0.is_finite()
            || !footprint.bounds.1.is_finite()
            || !footprint
                .transform
                .inverse()
                .coeffs()
                .iter()
                .all(|v| v.is_finite())
        {
            return Err(NodeError::InvalidMoveGeometry);
        }
        let scroll: Transform2<pax_runtime_api::Window> = Transform2::new(
            context
                .presentation_scroll_transform_for_node(&node)
                .as_coeffs(),
        );
        let origin = crate::presentation::MoveOrigin {
            transform: scroll
                * footprint.transform
                * Transform2::scale_sep(Vector2::new(footprint.bounds.0, footprint.bounds.1)),
            bounds: footprint.bounds,
        };
        let departing = context.departing_flight(&node);
        operation(&self.node)?;
        node.move_progress.set(0.0);
        node.move_origin.set(Some(origin));
        node.move_progress.ease_to(1.0, self.duration, self.easing);
        context.begin_move_flight(&node, departing, self.duration);
        Ok(())
    }
    /// Animate a move immediately before this sibling.
    pub fn move_to_insert_before(self, sibling: &NodeRef) -> Result<(), NodeError> {
        self.run(|node| node.move_to_insert_before(sibling))
    }
    /// Animate a move immediately after this sibling.
    pub fn move_to_insert_after(self, sibling: &NodeRef) -> Result<(), NodeError> {
        self.run(|node| node.move_to_insert_after(sibling))
    }
    /// Animate a move to the end of this parent's content.
    pub fn move_to_append_child(self, parent: &NodeRef) -> Result<(), NodeError> {
        self.run(|node| node.move_to_append_child(parent))
    }
    /// Animate a move to a final zero-based content index.
    pub fn move_to_insert_child(self, parent: &NodeRef, index: usize) -> Result<(), NodeError> {
        self.run(|node| node.move_to_insert_child(parent, index))
    }
    /// Animate a return to the declaration's current placement.
    pub fn move_to_home(self) -> Result<(), NodeError> {
        self.run(NodeRef::move_to_home)
    }
}

impl NodeRef {
    pub(crate) fn new(node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) -> Self {
        Self {
            node: Rc::downgrade(node),
            context: Rc::downgrade(context),
            epoch: node.reference_epoch.get(),
            id: node.id,
        }
    }

    pub(crate) fn resolve(&self) -> Result<(Rc<ExpandedNode>, Rc<RuntimeContext>), NodeError> {
        let node = self.node.upgrade().ok_or(NodeError::Unmounted)?;
        let context = self.context.upgrade().ok_or(NodeError::Unmounted)?;
        if node.reference_epoch.get() != self.epoch
            || (node.attached.get() == 0 && !context.is_detached(node.id))
        {
            return Err(NodeError::Unmounted);
        }
        Ok((node, context))
    }

    /// Runtime identity of this instance; valid only for its application lifetime.
    pub fn engine_id(&self) -> ExpandedNodeIdentifier {
        self.id
    }

    /// Configure easing for one move. Motion starts from the current presented
    /// footprint (including an interrupted move) and follows destination layout.
    /// Crossing Overlay contributions temporarily presents above the higher
    /// endpoint, below unrelated higher Overlays, until the animation completes.
    pub fn animated(
        &self,
        duration: impl Into<pax_runtime_api::Duration>,
        easing: pax_runtime_api::EasingCurve,
    ) -> AnimatedNodeMove {
        AnimatedNodeMove {
            node: self.clone(),
            duration: duration.into(),
            easing,
        }
    }

    /// Whether this handle is awaiting explicit recovery after its borrower unmounted.
    pub fn is_detached(&self) -> Result<bool, NodeError> {
        let (node, context) = self.resolve()?;
        Ok(context.is_detached(node.id))
    }

    /// Return to the declaration's current placement. A live visitor keeps its
    /// mount and state; an abandoned node starts a fresh mount. Repeat reordering
    /// is respected. If already home, this is a no-op.
    pub fn move_to_home(&self) -> Result<(), NodeError> {
        let (node, context) = self.resolve()?;
        context.move_home(&node)
    }

    /// Move this instance immediately before an active sibling. A live instance
    /// retains its mount; an abandoned instance starts a fresh mount.
    pub fn move_to_insert_before(&self, sibling: &Self) -> Result<(), NodeError> {
        self.move_relative(sibling, false)
    }

    /// Move this instance immediately after an active sibling. An abandoned
    /// instance starts a fresh mount.
    pub fn move_to_insert_after(&self, sibling: &Self) -> Result<(), NodeError> {
        self.move_relative(sibling, true)
    }

    /// Move this instance to the end of a destination's active content children.
    /// Moves affect runtime presentation only, never the serialized program.
    pub fn move_to_append_child(&self, parent: &Self) -> Result<(), NodeError> {
        let children = parent.content_children().all()?;
        let len = children.iter().filter(|child| *child != self).count();
        self.move_to_insert_child(parent, len)
    }

    /// Move to a final zero-based child index, after removing this instance from
    /// the sequence if it is already a child. The remaining sequence's `len`
    /// appends; larger indices fail.
    /// Failed moves leave placement unchanged. Destination loss ends the mount
    /// but retains a recovery entry; original-owner loss invalidates the handle.
    pub fn move_to_insert_child(&self, parent: &Self, index: usize) -> Result<(), NodeError> {
        let (node, context) = self.resolve()?;
        let (parent, parent_context) = parent.resolve()?;
        if !Rc::ptr_eq(&context, &parent_context) {
            return Err(NodeError::DifferentApplication);
        }
        context.move_node(&node, &parent, index)
    }

    fn move_relative(&self, sibling: &Self, after: bool) -> Result<(), NodeError> {
        let (_, context) = self.resolve()?;
        let (target, target_context) = sibling.resolve()?;
        if !Rc::ptr_eq(&context, &target_context) {
            return Err(NodeError::DifferentApplication);
        }
        if self == sibling {
            return Ok(());
        }
        let parent = content_parent(&target).ok_or(NodeError::InvalidDestination)?;
        let mut children = content_children(&parent, &context);
        children.retain(|child| child.id != self.id);
        let index = children
            .iter()
            .position(|child| child.id == target.id)
            .ok_or(NodeError::NotAChild)?;
        self.move_to_insert_child(&Self::new(&parent, &context), index + usize::from(after))
    }

    /// Find descendants matching an ID, class, or type selector. Traversal crosses
    /// component boundaries and follows current destination order, not depth
    /// or the temporary presentation of an animated transfer.
    pub fn query(&self, selector: &str) -> NodeQuery {
        NodeQuery::from_nodes(vec![self.clone()]).query(selector)
    }

    /// Active content children, with implementation-only control-flow and slot
    /// wrappers flattened. Exit-retained nodes are excluded; culled nodes remain.
    pub fn content_children(&self) -> NodeQuery {
        match self.resolve() {
            Ok((node, context)) => NodeQuery::from_nodes(
                content_children(&node, &context)
                    .iter()
                    .map(|n| Self::new(n, &context))
                    .collect(),
            ),
            Err(e) => NodeQuery::failed(e),
        }
    }

    /// The next active sibling, or `None` at the end of the content sequence.
    pub fn next(&self) -> Result<Option<Self>, NodeError> {
        self.neighbor(1)
    }

    /// The previous active sibling, or `None` at the start of the content sequence.
    pub fn previous(&self) -> Result<Option<Self>, NodeError> {
        self.neighbor(-1)
    }

    fn neighbor(&self, offset: isize) -> Result<Option<Self>, NodeError> {
        let (node, context) = self.resolve()?;
        let Some(parent) = content_parent(&node) else {
            return Ok(None);
        };
        let children = content_children(&parent, &context);
        let index = children
            .iter()
            .position(|n| n.id == node.id)
            .ok_or(NodeError::NotAChild)?;
        let Some(index) = index.checked_add_signed(offset) else {
            return Ok(None);
        };
        Ok(children.get(index).map(|n| Self::new(n, &context)))
    }
}

impl NodeQuery {
    pub(crate) fn from_nodes(nodes: Vec<NodeRef>) -> Self {
        Self { nodes, error: None }
    }
    pub(crate) fn failed(error: NodeError) -> Self {
        Self {
            nodes: vec![],
            error: Some(error),
        }
    }

    /// Narrow to matching descendants of the selected nodes, deduplicated in
    /// traversal order. Invalid selectors are reported by the terminal operation.
    pub fn query(self, selector: &str) -> Self {
        if self.error.is_some() {
            return self;
        }
        let selector = match SelectorExpr::parse(selector) {
            Ok(s) => s,
            Err(e) => return Self::failed(NodeError::InvalidSelector(e)),
        };
        let mut result = vec![];
        let mut seen = HashSet::new();
        for source in self.nodes {
            let (node, context) = match source.resolve() {
                Ok(v) => v,
                Err(e) => return Self::failed(e),
            };
            let mut pending = active_children(&node, &context);
            pending.reverse();
            while let Some(child) = pending.pop() {
                if !seen.insert(child.id) {
                    continue;
                }
                if child.selector_metadata.borrow().matches(&selector) {
                    result.push(NodeRef::new(&child, &context));
                }
                pending.extend(active_children(&child, &context).into_iter().rev());
            }
        }
        Self::from_nodes(result)
    }

    /// Filter by the nearest owning keyed iteration. Narrow the collection first
    /// when unrelated or nested repeats can contain the same key.
    pub fn key(self, key: impl Into<NodeKey>) -> Self {
        if self.error.is_some() {
            return self;
        }
        let encoded = match key.into() {
            NodeKey::String(v) => format!("string:{v}"),
            NodeKey::Integer(v) => format!("int:{v}"),
        };
        let mut nodes = vec![];
        for handle in self.nodes {
            let (node, _) = match handle.resolve() {
                Ok(v) => v,
                Err(e) => return Self::failed(e),
            };
            let matched = node
                .stack
                .resolve_symbol("@repeat_key")
                .map(|v| matches!(v.get_as_pax_value(), PaxValue::String(s) if s == encoded))
                .unwrap_or(false);
            if matched {
                nodes.push(handle);
            }
        }
        Self::from_nodes(nodes)
    }

    fn validate(&self) -> Result<(), NodeError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        for node in &self.nodes {
            node.resolve()?;
        }
        Ok(())
    }

    /// Number of nodes in this snapshot. Ended declaration lifetimes are errors.
    pub fn len(&self) -> Result<usize, NodeError> {
        self.validate()?;
        Ok(self.nodes.len())
    }
    /// Whether this snapshot contains no nodes.
    pub fn is_empty(&self) -> Result<bool, NodeError> {
        Ok(self.len()? == 0)
    }
    /// Resolve exactly one match, rejecting absent or ambiguous selections.
    pub fn one(self) -> Result<NodeRef, NodeError> {
        self.validate()?;
        if self.nodes.len() != 1 {
            return Err(NodeError::ExpectedOne(self.nodes.len()));
        }
        Ok(self.nodes.into_iter().next().unwrap())
    }
    /// Select a zero-based position in this snapshot, independently of repeat keys.
    pub fn nth(&self, index: usize) -> Result<NodeRef, NodeError> {
        self.validate()?;
        self.nodes
            .get(index)
            .cloned()
            .ok_or(NodeError::IndexOutOfBounds {
                index,
                len: self.nodes.len(),
            })
    }
    /// Find the position of an exact instance within this snapshot.
    pub fn index_of(&self, node: &NodeRef) -> Result<usize, NodeError> {
        self.validate()?;
        node.resolve()?;
        self.nodes
            .iter()
            .position(|n| n == node)
            .ok_or(NodeError::NotAChild)
    }
    /// Resolve all matches in their traversal order.
    pub fn all(self) -> Result<Vec<NodeRef>, NodeError> {
        self.validate()?;
        Ok(self.nodes)
    }
}

pub(crate) fn active_children(
    node: &Rc<ExpandedNode>,
    context: &RuntimeContext,
) -> Vec<Rc<ExpandedNode>> {
    node.logical_children(context)
        .into_iter()
        .filter(|n| {
            n.attached.get() > 0
                && !n.is_render_source()
                && n.transition_phase.get()
                    != pax_manifest::cartridge_generation::TRANSITION_PHASE_EXIT
        })
        .collect()
}

pub(crate) fn transparent(node: &ExpandedNode) -> bool {
    let instance = node.instance_node.borrow();
    let flags = instance.base().flags();
    flags.invisible_to_slot || flags.is_slot
}

pub(crate) fn content_children(
    node: &Rc<ExpandedNode>,
    context: &RuntimeContext,
) -> Vec<Rc<ExpandedNode>> {
    let mut result = vec![];
    for child in active_children(node, context) {
        if transparent(&child) {
            result.extend(content_children(&child, context));
        } else {
            result.push(child);
        }
    }
    result
}

pub(crate) fn content_parent(node: &ExpandedNode) -> Option<Rc<ExpandedNode>> {
    let mut parent = node.render_parent.borrow().upgrade();
    while parent.as_ref().is_some_and(|n| transparent(n)) {
        parent = parent.unwrap().render_parent.borrow().upgrade();
    }
    parent
}
