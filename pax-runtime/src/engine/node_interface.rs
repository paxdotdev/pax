use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::{borrow, pax_value::ToFromPaxAny, Interpolatable, Percent, Size};

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
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub local_rotation: Option<Rotation>,
    pub scale_x: Option<Percent>,
    pub scale_y: Option<Percent>,
    pub anchor_x: Option<Size>,
    pub anchor_y: Option<Size>,
    pub skew_x: Option<Rotation>,
    pub skew_y: Option<Rotation>,
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
        // Common props should most likely contain the types
        // that is coerced into here directly instead of having them converted
        Properties {
            x: cp.x.get(),
            y: cp.y.get(),
            local_rotation: cp.rotate.get(),
            width: cp.width.get(),
            height: cp.height.get(),
            anchor_x: cp.anchor_x.get(),
            anchor_y: cp.anchor_y.get(),
            scale_x: cp
                .scale_x
                .get()
                .map(|v| Percent((100.0 * v.expect_percent()).into())),
            scale_y: cp
                .scale_y
                .get()
                .map(|v| Percent((100.0 * v.expect_percent()).into())),
            skew_x: cp.skew_x.get().map(|v| Rotation::Radians(v.into())),
            skew_y: cp.skew_y.get().map(|v| Rotation::Radians(v.into())),
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
