use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::Property;
use pax_runtime_api::{borrow, pax_value::ToFromPaxAny, Interpolatable};

use crate::{
    api::{math::Space, Window},
    ExpandedNode, LayoutProperties, TransformAndBounds,
};
use crate::{ExpandedNodeIdentifier, InstanceFlags};

impl Interpolatable for NodeInterface {}

impl PartialEq for NodeInterface {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id.eq(&other.inner.id)
    }
}

impl Eq for NodeInterface {}

impl PartialOrd for NodeInterface {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.id.partial_cmp(&other.inner.id)
    }
}

impl Ord for NodeInterface {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.id.cmp(&other.inner.id)
    }
}

#[derive(Clone)]
/// Designer/runtime inspection handle for an expanded node.
pub struct NodeInterface {
    inner: Rc<ExpandedNode>,
}

impl From<Rc<ExpandedNode>> for NodeInterface {
    fn from(expanded_node: Rc<ExpandedNode>) -> Self {
        Self {
            inner: expanded_node,
        }
    }
}

impl std::fmt::Debug for NodeInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeInterface({:?})", self.inner)
    }
}

/// Marker coordinate space for a node's local layout space.
pub struct NodeLocal;

impl Space for NodeLocal {}

impl NodeInterface {
    /// Compiler-global template id for this node, if it originated from a template node.
    pub fn global_id(&self) -> Option<UniqueTemplateNodeIdentifier> {
        let instance_node = borrow!(self.inner.instance_node);
        let base = instance_node.base();
        base.template_node_identifier.clone()
    }

    /// Runtime-expanded id for this concrete node.
    pub fn engine_id(&self) -> ExpandedNodeIdentifier {
        self.inner.id.clone()
    }

    /// Current layout properties after common-property collection.
    pub fn layout_properties(&self) -> LayoutProperties {
        self.inner.layout_properties().get()
    }

    /// Auto-sized bounds reported by a native or text-backed node.
    pub fn auto_size(&self) -> Option<(f64, f64)> {
        self.inner.rendered_size.get()
    }

    /// Borrow the node's typed property object if it has the requested type.
    pub fn with_properties<V, T: ToFromPaxAny>(&self, f: impl FnOnce(&mut T) -> V) -> Option<V> {
        self.inner.try_with_properties_unwrapped(|tp: &mut T| f(tp))
    }

    /// Check whether the node's property object is of type `T`.
    pub fn is_of_type<T: ToFromPaxAny>(&self) -> bool {
        self.inner
            .try_with_properties_unwrapped::<T, _>(|_| ())
            .is_some()
    }

    /// Static flags from the node's instance.
    pub fn instance_flags(&self) -> InstanceFlags {
        let instance_node = borrow!(self.inner.instance_node);
        let base = instance_node.base();
        base.flags().clone()
    }

    /// Test the template `id` common property.
    pub fn has_id(&self, id: &str) -> bool {
        let cp = borrow!(self.inner.common_properties);
        let cp = borrow!(&*cp);
        cp.id.read(|i| i.as_ref().is_some_and(|i| i == id))
    }

    /// Reactive transform-and-bounds property for this node.
    pub fn transform_and_bounds(&self) -> Property<TransformAndBounds<NodeLocal, Window>> {
        self.inner.transform_and_bounds.clone()
    }

    /// Parent in render traversal order.
    pub fn render_parent(&self) -> Option<NodeInterface> {
        let parent = borrow!(self.inner.render_parent);
        Some(parent.upgrade()?.into())
    }

    /// Containing component for template scoping and slot ownership.
    pub fn containing_component(&self) -> Option<NodeInterface> {
        Some(self.inner.containing_component.upgrade()?.into())
    }

    /// Parent in template ownership order.
    pub fn template_parent(&self) -> Option<NodeInterface> {
        Some(self.inner.template_parent.upgrade()?.into())
    }

    /// True if this node is below `node` in the template-parent chain.
    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id)
    }

    /// Mounted child nodes.
    pub fn children(&self) -> Vec<NodeInterface> {
        let children = borrow!(self.inner.mounted_children);
        (&*children)
            .into_iter()
            .map(Rc::clone)
            .map(Into::into)
            .collect()
    }

    /// Reactive count of slot children after repeat/conditional flattening.
    pub fn flattened_slot_children_count(&self) -> Property<usize> {
        self.inner.flattened_slot_children_count.clone()
    }
}
