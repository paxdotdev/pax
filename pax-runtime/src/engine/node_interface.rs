use std::rc::Rc;

use crate::{
    api::Window,
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

impl Space for NodeLocal {}

impl NodeInterface {
    pub fn global_id(&self) -> (String, usize) {
        let base = self.inner.instance_node.base();
        (base.component_type_id.to_owned(), base.template_node_id)
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
        let origin_window: Point2<Window> = (tab.transform * p_anchor).cast_space();
        Some(origin_window)
    }

    pub fn transform(&self) -> Option<Transform2<Window, NodeLocal>> {
        let up_lp = self.inner.layout_properties.borrow_mut();
        if let Some(lp) = up_lp.as_ref() {
            Some(lp.computed_tab.transform.inverse().cast_spaces())
        } else {
            None
        }
    }

    pub fn bounding_points(&self) -> Option<[Point2<Window>; 4]> {
        let lp = self.inner.layout_properties.borrow();
        if let Some(layout) = lp.as_ref() {
            Some(layout.computed_tab.corners().map(Point2::cast_space))
        } else {
            None
        }
    }

    pub fn is_descendant_of(&self, node: &NodeInterface) -> bool {
        self.inner.is_descendant_of(&node.inner.id_chain)
    }
}
