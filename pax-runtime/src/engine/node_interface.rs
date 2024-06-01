use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::{borrow, pax_value::ToFromPaxAny, Interpolatable, Size};

use crate::{
    api::math::{Point2, Space},
    api::{Rotation, Window},
    ExpandedNode,
};

use super::expanded_node::LayoutProperties;

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

#[derive(Clone, Default)]
pub struct Properties {
    pub x: Option<Size>,
    pub y: Option<Size>,
    pub local_rotation: Rotation,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub anchor_x: Option<Size>,
    pub anchor_y: Option<Size>,
}

impl Space for NodeLocal {}

impl NodeInterface {
    pub fn global_id(&self) -> Option<UniqueTemplateNodeIdentifier> {
        let instance_node = borrow!(self.inner.instance_node);
        let base = instance_node.base();
        base.template_node_identifier.clone()
    }

    pub fn common_properties(&self) -> Properties {
        let cp = self.inner.get_common_properties();
        let cp = borrow!(cp);
        Properties {
            x: cp.x.get(),
            y: cp.y.get(),
            local_rotation: cp.rotate.get().unwrap_or(Rotation::default()),
            width: cp.width.get(),
            height: cp.height.get(),
            anchor_x: cp.anchor_x.get(),
            anchor_y: cp.anchor_y.get(),
        }
    }

    pub fn origin(&self) -> Option<Point2<Window>> {
        let common_props = self.inner.get_common_properties();
        let common_props = borrow!(common_props);
        let (width, height) = self.inner.layout_properties.bounds.get();
        let p_anchor = Point2::new(
            common_props
                .anchor_x
                .get()
                .map(|x| x.get_pixels(width))
                .unwrap_or(0.0),
            common_props
                .anchor_y
                .get()
                .map(|y| y.get_pixels(height))
                .unwrap_or(0.0),
        );
        let origin_window = self.inner.layout_properties.transform.get() * p_anchor;
        Some(origin_window)
    }

    pub fn with_properties<V, T: ToFromPaxAny>(&self, f: impl FnOnce(&mut T) -> V) -> V {
        self.inner.with_properties_unwrapped(|tp: &mut T| f(tp))
    }

    pub fn layout_properties(&self) -> LayoutProperties {
        self.inner.layout_properties.clone()
    }

    pub fn parent(&self) -> Option<NodeInterface> {
        let parent = borrow!(self.inner.parent_expanded_node);
        Some(parent.upgrade()?.into())
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id)
    }
}
