use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::Property;
use pax_runtime_api::{borrow, pax_value::ToFromPaxAny, Interpolatable};

use crate::{
    api::{math::Space, Window},
    ExpandedNode, LayoutProperties, TransformAndBounds,
};

impl Interpolatable for NodeInterface {}

impl PartialEq for NodeInterface {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id.eq(&other.inner.id)
    }
}

#[derive(Clone)]
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

pub struct NodeLocal;

impl Space for NodeLocal {}

impl NodeInterface {
    pub fn global_id(&self) -> Option<UniqueTemplateNodeIdentifier> {
        let instance_node = borrow!(self.inner.instance_node);
        let base = instance_node.base();
        base.template_node_identifier.clone()
    }

    pub fn layout_properties(&self) -> LayoutProperties {
        self.inner.layout_properties().get()
    }

    pub fn with_properties<V, T: ToFromPaxAny>(&self, f: impl FnOnce(&mut T) -> V) -> V {
        self.inner.with_properties_unwrapped(|tp: &mut T| f(tp))
    }

    pub fn is_of_type<T: ToFromPaxAny>(&self) -> bool {
        self.inner
            .try_with_properties_unwrapped::<T, _>(|_| ())
            .is_some()
    }

    pub fn transform_and_bounds(&self) -> Property<TransformAndBounds<NodeLocal, Window>> {
        self.inner.transform_and_bounds.clone()
    }

    pub fn render_parent(&self) -> Option<NodeInterface> {
        let parent = borrow!(self.inner.render_parent);
        Some(parent.upgrade()?.into())
    }

    pub fn containing_component(&self) -> Option<NodeInterface> {
        Some(self.inner.containing_component.upgrade()?.into())
    }

    pub fn template_parent(&self) -> Option<NodeInterface> {
        Some(self.inner.template_parent.upgrade()?.into())
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id)
    }
}
