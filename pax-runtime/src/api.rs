use std::{
    rc::{Rc, Weak},
    time::Instant,
};

use_RefCell!();
use crate::{
    node_interface::NodeLocal, ExpandedNode, RuntimeContext, RuntimePropertiesStackFrame,
    TransformAndBounds,
};

use math::Transform2;
use pax_runtime_api::math::Point2;
pub use pax_runtime_api::*;

#[cfg(feature = "designtime")]
use {
    crate::node_interface::NodeInterface, pax_designtime::DesigntimeManager,
    pax_manifest::UniqueTemplateNodeIdentifier,
};

#[derive(Clone)]
pub struct NodeContext {
    /// slot index of this node in its container
    pub slot_index: Property<Option<usize>>,
    /// Stack frame of this component, used to look up stores
    pub local_stack_frame: Rc<RuntimePropertiesStackFrame>,
    /// Reference to the ExpandedNode of the component containing this node
    pub containing_component: Weak<ExpandedNode>,
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
    /// The transform of this node in the global coordinate space
    pub node_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
    /// Slot children of this node
    pub slot_children: Property<Vec<Rc<ExpandedNode>>>,
    /// A property that can be depended on to dirty when a slot child is attached
    pub slot_children_attached_listener: Property<()>,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
    pub(crate) get_elapsed_millis: Rc<dyn Fn() -> u128>,
}

impl NodeContext {
    pub fn push_local_store<T: Store>(&self, store: T) {
        self.local_stack_frame.insert_stack_local_store(store);
    }

    pub fn peek_local_store<T: Store, V>(&self, f: impl FnOnce(&mut T) -> V) -> Result<V, String> {
        self.local_stack_frame.peek_stack_local_store(f)
    }

    pub fn local_point(&self, p: Point2<Window>) -> Point2<NodeLocal> {
        self.node_transform_and_bounds.as_transform().inverse() * p
    }

    /// Get std::time::Instant::now()
    pub fn elapsed_time_millis(&self) -> u128 {
        (self.get_elapsed_millis)()
    }

    pub fn navigate_to(&self, url: &str, target: NavigationTarget) {
        self.runtime_context
            .enqueue_native_message(NativeMessage::Navigate(NavigationPatch {
                url: url.to_string(),
                target: match target {
                    NavigationTarget::Current => "current",
                    NavigationTarget::New => "new",
                }
                .to_string(),
            }))
    }

    pub fn dispatch_event(&self, identifier: &'static str) -> Result<(), String> {
        let component_origin = self
            .containing_component
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
    pub fn raycast(&self, point: Point2<Window>, hit_invisible: bool) -> Vec<NodeInterface> {
        let expanded_nodes =
            self.runtime_context
                .get_elements_beneath_ray(point, false, vec![], hit_invisible);
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

    pub fn get_userland_root_expanded_node(&self) -> Option<NodeInterface> {
        #[cfg(feature = "designtime")]
        let expanded_node = self.runtime_context.get_userland_root_expanded_node()?;
        #[cfg(not(feature = "designtime"))]
        let expanded_node = self.runtime_context.get_root_expanded_node()?;
        Some(expanded_node.into())
    }

    pub fn get_root_expanded_node(&self) -> Option<NodeInterface> {
        let expanded_node = self.runtime_context.get_root_expanded_node()?;
        Some(expanded_node.into())
    }

    pub fn get_nodes_by_id(&self, id: &str) -> Vec<NodeInterface> {
        let expanded_nodes = self.runtime_context.get_expanded_nodes_by_id(id);
        expanded_nodes
            .into_iter()
            .map(Into::<NodeInterface>::into)
            .collect()
    }
}
