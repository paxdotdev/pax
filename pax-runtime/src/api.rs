use std::rc::{Rc, Weak};

use_RefCell!();
use crate::{ExpandedNode, HandlerLocation, RuntimeContext};
use pax_runtime_api::pax_value::PaxAny;
pub use pax_runtime_api::*;
#[cfg(feature = "designtime")]
use {
    crate::api::math::Point2, crate::node_interface::NodeInterface,
    pax_designtime::DesigntimeManager, pax_manifest::UniqueTemplateNodeIdentifier,
};

#[derive(Clone)]
pub struct NodeContext {
    /// Registered handlers on the instance node
    pub(crate) component_origin: Weak<ExpandedNode>,
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
    pub slot_children_count: Property<usize>,
    /// Borrow of the RuntimeContext, used at least for exposing raycasting to userland
    #[allow(unused)]
    pub(crate) runtime_context: Rc<RuntimeContext>,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
}

impl NodeContext {
    pub fn dispatch_event(&self, identifier: &str) -> Result<(), String> {
        let component_origin = self
            .component_origin
            .upgrade()
            .ok_or_else(|| "can't dispatch from root component".to_owned())?;
        let component_origin_instance = borrow!(component_origin.instance_node);
        let registry = component_origin_instance
            .base()
            .handler_registry
            .as_ref()
            .ok_or_else(|| "no registry present".to_owned())?;

        let parent_component = component_origin
            .containing_component
            .upgrade()
            .ok_or_else(|| "can't dispatch from root (has no parent)".to_owned())?;
        let properties = borrow!(parent_component.properties);

        for handler in borrow!(registry)
            .handlers
            .get(identifier)
            .ok_or_else(|| format!("no registered handler with name \"{}\" exists", identifier))?
        {
            (handler.function)(Rc::clone(&*properties), &self, None)
        }
        Ok(())
    }
}

#[cfg(feature = "designtime")]
impl NodeContext {
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
        let expanded_nodes = self.runtime_context.get_expanded_nodes_by_global_ids(&uni);
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
