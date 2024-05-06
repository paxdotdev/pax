use std::{cell::RefCell, rc::Rc};

use crate::RuntimeContext;
pub use pax_runtime_api::*;
#[cfg(feature = "designtime")]
use {
    crate::api::math::Point2, crate::node_interface::NodeInterface,
    pax_designtime::DesigntimeManager, pax_manifest::UniqueTemplateNodeIdentifier,
};

#[derive(Clone)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct NodeContext {
    /// The current global engine tick count
    pub frames_elapsed: Property<u64>,
    /// The bounds of this element's immediate container (parent) in px
    pub bounds_parent: Property<(f64, f64)>,
    /// The bounds of this element in px
    pub bounds_self: Property<(f64, f64)>,
    /// Current platform (Web/Native) this app is running on
    pub platform: Platform,
    /// Current os (Android/Windows/Mac/Linux) this app is running on
    pub os: OS,
    /// The number of slot children provided to this component template
    pub slot_children: usize,
    /// Borrow of the RuntimeContext, used at least for exposing raycasting to userland
    #[allow(unused)]
    pub(crate) runtime_context: Rc<RefCell<RuntimeContext>>,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
}

#[cfg(feature = "designtime")]
impl NodeContext {
    pub fn raycast(&self, point: Point2<Window>) -> Vec<NodeInterface> {
        let rc = self.runtime_context.borrow();
        let expanded_nodes = rc.get_elements_beneath_ray(point, false, vec![]);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }

    pub fn get_nodes_by_global_id(&self, uni: UniqueTemplateNodeIdentifier) -> Vec<NodeInterface> {
        let rc = self.runtime_context.borrow();
        let expanded_nodes = rc.get_expanded_nodes_by_global_ids(&uni);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }

    pub fn get_nodes_by_id(&self, id: &str) -> Vec<NodeInterface> {
        let rc = self.runtime_context.borrow();
        let expanded_nodes = rc.get_expanded_nodes_by_id(id);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }
}
