use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::Property;
use pax_runtime_api::{borrow, pax_value::ToFromPaxAny, Interpolatable, Percent};

use crate::{
    api::{
        math::{Point2, Space},
        Window,
    },
    ExpandedNode, LayoutProperties, TransformAndBounds,
};

impl Interpolatable for NodeInterface {}
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

    pub fn layout_properties(&self) -> LayoutProperties {
        borrow!(self.inner.layout_properties).clone()
    }

    pub fn parent(&self) -> Option<NodeInterface> {
        let parent = borrow!(self.inner.parent_expanded_node);
        Some(parent.upgrade()?.into())
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id)
    }
}
