

pub use pax_runtime_api::*;
use crate::RuntimeContext;

#[derive(Clone)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct NodeContext<'a> {
    /// The current global engine tick count
    pub frames_elapsed: usize,
    /// The bounds of this element's immediate container (parent) in px
    pub bounds_parent: (f64, f64),
    /// The bounds of this element in px
    pub bounds_self: (f64, f64),
    /// Borrow of the RuntimeContext, used at least for exposing raycasting to userland
    pub(crate) runtime_context: &'a RuntimeContext,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
}

#[cfg(feature = "designtime")]
impl NodeContext<'_> {
    pub fn raycast(&self, point: Point2<Window>) -> Vec<NodeInterface> {
        let expanded_nodes = self
            .runtime_context
            .get_elements_beneath_ray(point, false, vec![]);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }

    pub fn get_nodes_by_global_id(&self, uni: UniqueTemplateNodeIdentifier) -> Vec<NodeInterface> {
        let expanded_nodes = self.runtime_context.get_expanded_nodes_by_global_ids(uni);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }

    pub fn get_nodes_by_id(&self, id: &str) -> Vec<NodeInterface> {
        let expanded_nodes = self.runtime_context.get_expanded_nodes_by_id(id);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }
}