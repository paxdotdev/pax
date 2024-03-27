use std::rc::Rc;

use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::math::Vector2;

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

pub struct Properties {
    pub local_rotation: Rotation,
}

impl Space for NodeLocal {}

impl NodeInterface {
    pub fn global_id(&self) -> Option<UniqueTemplateNodeIdentifier> {
        let instance_node = self.inner.instance_node.borrow();
        let base = instance_node.base();
        base.template_node_identifier.clone()
    }

    pub fn properties(&self) -> Properties {
        let cp = self.inner.get_common_properties();
        let rot = cp
            .borrow()
            .rotate
            .as_ref()
            .map(|p| p.get().clone())
            .unwrap_or(Rotation::default());
        Properties {
            local_rotation: rot,
        }
    }

    pub fn origin(&self) -> Option<Point2<Window>> {
        let common_props = self.inner.get_common_properties();
        let common_props = common_props.borrow();
        let lp = self.inner.layout_properties.borrow();
        let tab = &lp.as_ref()?.computed_tab;
        let p_anchor = Point2::new(
            common_props
                .anchor_x
                .as_ref()
                .map(|x| x.get().get_pixels(tab.bounds.0))
                .unwrap_or(0.0),
            common_props
                .anchor_y
                .as_ref()
                .map(|y| y.get().get_pixels(tab.bounds.1))
                .unwrap_or(0.0),
        );
        let origin_window = tab.transform * p_anchor;
        Some(origin_window)
    }

    pub fn local_origin(&self) -> Option<Point2<NodeLocal>> {
        let common_props = self.inner.get_common_properties();
        let common_props = common_props.borrow();
        let lp = self.inner.layout_properties.borrow();
        let tab = &lp.as_ref()?.computed_tab;
        let p_anchor = Point2::new(
            common_props
                .anchor_x
                .as_ref()
                // map 0.0 to 1.0
                .map(|x| x.get().get_pixels(tab.bounds.0) / tab.bounds.0)
                .unwrap_or(0.0),
            common_props
                .anchor_y
                .as_ref()
                .map(|y| y.get().get_pixels(tab.bounds.1) / tab.bounds.1)
                .unwrap_or(0.0),
        );
        Some(p_anchor)
    }

    pub fn transform(&self) -> Option<Transform2<Window, NodeLocal>> {
        let up_lp = self.inner.layout_properties.borrow_mut();
        if let Some(lp) = up_lp.as_ref() {
            Some(lp.computed_tab.transform.inverse())
        } else {
            None
        }
    }

    pub fn bounds(&self) -> Option<Transform2<NodeLocal, Window>> {
        let up_lp = self.inner.layout_properties.borrow_mut();
        if let Some(lp) = up_lp.as_ref() {
            let (w, h) = lp.computed_tab.bounds;
            let res = lp.computed_tab.transform * Transform2::scale_sep(Vector2::new(w, h));
            Some(res)
        } else {
            None
        }
    }

    pub fn parent(&self) -> Option<NodeInterface> {
        let parent = self.inner.parent_expanded_node.borrow();
        Some(parent.upgrade()?.into())
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id_chain)
    }
}
