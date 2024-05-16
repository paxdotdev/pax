use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::{borrow, pax_value::ToFromPaxAny, Size};

use crate::{
    api::math::{Point2, Space, Transform2},
    api::{Rotation, Window},
    ExpandedNode,
};

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
    pub width: Size,
    pub height: Size,
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
            x: cp.x.as_ref().map(|v| v.get()),
            y: cp.y.as_ref().map(|v| v.get()),
            local_rotation: cp
                .rotate
                .as_ref()
                .map(|p| p.get())
                .unwrap_or(Rotation::default()),
            width: cp.width.get(),
            height: cp.height.get(),
            anchor_x: cp.anchor_x.as_ref().map(|p| p.get()),
            anchor_y: cp.anchor_y.as_ref().map(|p| p.get()),
        }
    }

    pub fn origin(&self) -> Option<Point2<Window>> {
        let common_props = self.inner.get_common_properties();
        let common_props = borrow!(common_props);
        let (width, height) = self.inner.layout_properties.bounds.get();
        let p_anchor = Point2::new(
            common_props
                .anchor_x
                .as_ref()
                .map(|x| x.get().get_pixels(width))
                .unwrap_or(0.0),
            common_props
                .anchor_y
                .as_ref()
                .map(|y| y.get().get_pixels(height))
                .unwrap_or(0.0),
        );
        let origin_window = self.inner.layout_properties.transform.get() * p_anchor;
        Some(origin_window)
    }

    pub fn with_properties<V, T: ToFromPaxAny>(&self, f: impl FnOnce(&mut T) -> V) -> V {
        self.inner.with_properties_unwrapped(|tp: &mut T| f(tp))
    }

    pub fn transform(&self) -> Option<Transform2<Window, NodeLocal>> {
        Some(self.inner.layout_properties.transform.get().inverse())
    }

    pub fn bounding_points(&self) -> Option<[Point2<Window>; 4]> {
        Some(self.inner.layout_properties.corners())
    }

    pub fn parent(&self) -> Option<NodeInterface> {
        let parent = borrow!(self.inner.parent_expanded_node);
        Some(parent.upgrade()?.into())
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id)
    }
}
