use std::rc::Rc;

use crate::{
    api::{Rotation, Window},
    math::{Point2, Space, Transform2},
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
    pub fn global_id(&self) -> (String, usize) {
        let base = self.inner.instance_node.base();
        (base.component_type_id.to_owned(), base.template_node_id)
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

    pub fn transform(&self) -> Option<Transform2<Window, NodeLocal>> {
        let up_lp = self.inner.layout_properties.borrow_mut();
        if let Some(lp) = up_lp.as_ref() {
            Some(lp.computed_tab.transform.inverse())
        } else {
            None
        }
    }

    pub fn bounding_points(&self) -> Option<[Point2<Window>; 4]> {
        let lp = self.inner.layout_properties.borrow();
        if let Some(layout) = lp.as_ref() {
            Some(layout.computed_tab.corners())
        } else {
            None
        }
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id_chain)
    }
}
