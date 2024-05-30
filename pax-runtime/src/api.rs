use std::rc::{Rc, Weak};

use_RefCell!();
use crate::{ExpandedNode, RuntimeContext, RuntimePropertiesStackFrame};
pub use pax_runtime_api::*;

#[cfg(feature = "designtime")]
use {
    crate::api::math::Point2, crate::node_interface::NodeInterface, crate::HandlerLocation,
    pax_designtime::DesigntimeManager, pax_manifest::UniqueTemplateNodeIdentifier,
};

#[derive(Clone)]
pub struct NodeContext {
    /// Stack frame of this component, used to look up stores
    pub(crate) local_stack_frame: Rc<RuntimePropertiesStackFrame>,
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
    pub(crate) runtime_context: Rc<RuntimeContext>,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
}

impl NodeContext {
    pub fn push_local_store<T: Store>(&self, store: T) {
        self.local_stack_frame.insert_stack_local_store(store);
    }

    pub fn peek_local_store<T: Store, V>(&self, f: impl FnOnce(&mut T) -> V) -> Result<V, String> {
        self.local_stack_frame.peek_stack_local_store(f)
    }

    pub fn dispatch_event(&self, identifier: &'static str) -> Result<(), String> {
        let component_origin = self
            .component_origin
            .upgrade()
            .ok_or_else(|| "can't dispatch from root component".to_owned())?;

        // Check that this is a valid custom event to trigger
        {
            let component_origin_instance = borrow!(component_origin.instance_node);
            let registry = component_origin_instance
                .base()
                .handler_registry
                .as_ref()
                .ok_or_else(|| "no registry present".to_owned())?;
            borrow!(registry).handlers.get(identifier).ok_or_else(|| {
                format!("no registered handler with name \"{}\" exists", identifier)
            })?;
        }

        // ok now we know it's a valid thing to dispatch, queue it for end of tick
        self.runtime_context
            .queue_custom_event(Rc::clone(&component_origin), identifier);

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
