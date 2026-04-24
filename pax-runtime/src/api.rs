use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};

use_RefCell!();
use crate::{
    node_interface::NodeLocal, ExpandedNode, LayoutHull, RuntimeContext,
    RuntimePropertiesStackFrame, TransformAndBounds,
};

pub use pax_runtime_api::*;
use pax_runtime_api::{cursor::CursorStyle, math::Point2, properties::UntypedProperty};

use crate::node_interface::NodeInterface;
#[cfg(feature = "designtime")]
use {
    pax_designtime::{
        messages::{UserlandSourceUpdateRequest, UserlandSourceUpdateResponse},
        DesigntimeManager,
    },
    pax_manifest::UniqueTemplateNodeIdentifier,
};

#[derive(Clone)]
/// Runtime context passed into user component lifecycle methods and event handlers.
///
/// Child-related fields intentionally separate semantic payload from engine
/// transport:
///
/// - `projected_children` is the raw transport family used by `Slot`
/// - `received_children` is the normalized semantic payload that this node
///   should treat as content from its caller
/// - `retained_received_children` are former received children kept alive only
///   so `@out` transitions can finish
///
/// A node's own private template or primitive-assembled structure is
/// intentionally not surfaced here as a first-class "child family" for
/// container consumers.
pub struct NodeContext {
    pub expanded_node: Weak<ExpandedNode>,
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
    /// Measured bounds resolved by the chassis or container layout for this node.
    pub measured_size: Property<Option<(f64, f64)>>,
    /// Node-local subtree layout hull published by the engine for container measurement.
    pub subtree_layout_hull: Property<LayoutHull>,
    /// Current platform (Web/Native) this app is running on
    pub platform: Platform,
    /// Current os (Android/Windows/Mac/Linux) this app is running on
    pub os: OS,
    /// The number of projected children available to this node.
    ///
    /// This is the raw transport count used by slot-driven implementations.
    /// Container-style consumers usually want `received_children_count`
    /// instead.
    pub projected_children_count: Property<usize>,
    /// Borrow of the RuntimeContext, used at least for exposing raycasting to userland
    pub(crate) runtime_context: Rc<RuntimeContext>,
    /// The transform of this node in the global coordinate space
    pub node_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
    /// Children projected into this node from the containing component.
    ///
    /// Projection is an engine transport mechanism. Consumers that want the
    /// semantic payload owned by this node should prefer `received_children`.
    pub projected_children: Property<Vec<Rc<ExpandedNode>>>,
    /// A structural invalidation signal for projected children.
    pub projected_children_changed: Property<()>,
    /// Semantic payload children received by this node from its caller.
    ///
    /// This is the canonical "content" view for container-style logic. It
    /// excludes private encapsulated implementation children and also excludes
    /// exit-retained payload nodes, which instead appear in
    /// `retained_received_children`.
    pub received_children: Property<Vec<Rc<ExpandedNode>>>,
    /// Convenience count derived from `received_children`.
    pub received_children_count: Property<usize>,
    /// A structural invalidation signal for `received_children`.
    ///
    /// Prefer this or `received_children` itself for structural subscriptions
    /// that must react to reorders as well as insertions and removals.
    pub received_children_changed: Property<()>,
    /// Received children retained only so exit transitions can finish.
    ///
    /// These are no longer part of the active semantic payload, but some
    /// containers still need to place them as ghosts or overlays while their
    /// `@out` transitions run.
    pub retained_received_children: Property<Vec<Rc<ExpandedNode>>>,
    /// A structural invalidation signal for `retained_received_children`.
    pub retained_received_children_changed: Property<()>,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
    pub(crate) get_elapsed_millis: Rc<dyn Fn() -> u128>,
}

impl NodeContext {
    /// Push component-local state onto the runtime stack for descendants to find.
    pub fn push_local_store<T: Store>(&self, store: T) {
        self.local_stack_frame.insert_stack_local_store(store);
    }

    /// Borrow the nearest stack-local store of type `T`.
    pub fn peek_local_store<T: Store, V>(&self, f: impl FnOnce(&mut T) -> V) -> Result<V, String> {
        self.local_stack_frame.peek_stack_local_store(f)
    }

    /// Convert a window-space point into this node's local coordinate space.
    pub fn local_point(&self, p: Point2<Window>) -> Point2<NodeLocal> {
        self.node_transform_and_bounds.as_transform().inverse() * p
    }

    /// Return the interface for this node's containing component, when present.
    pub fn get_node_interface(&self) -> Option<NodeInterface> {
        Weak::upgrade(&self.containing_component).map(|v| v.into())
    }

    /// Milliseconds elapsed according to the chassis-provided clock.
    pub fn elapsed_time_millis(&self) -> u128 {
        (self.get_elapsed_millis)()
    }

    /// Attach a dependency subscription whose callback runs when any dependency dirties.
    pub fn subscribe(&self, dependencies: &[UntypedProperty], f: impl Fn() + 'static) {
        match self.expanded_node.upgrade() {
            Some(expanded_node) => {
                let subscription_prop =
                    self.runtime_context
                        .register_node_effect(expanded_node.id, dependencies, f);
                borrow_mut!(expanded_node.subscriptions).push(subscription_prop);
            }
            None => log::warn!("couldn't add subscription: node doesn't exist anymore"),
        }
    }

    /// Remove all subscriptions registered on this node.
    pub fn clear_subscriptions(&self) {
        match self.expanded_node.upgrade() {
            Some(expanded_node) => borrow_mut!(expanded_node.subscriptions).clear(),
            None => log::warn!("couldn't clear subscriptions: node doesn't exist anymore"),
        }
    }

    /// Ask the chassis to navigate to a URL.
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

    /// Queue a named custom event from this component for dispatch at the end of the tick.
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

    /// Ask the chassis to display the requested cursor over the app surface.
    pub fn set_cursor(&self, cursor: CursorStyle) {
        self.runtime_context
            .enqueue_native_message(NativeMessage::SetCursor(SetCursorPatch {
                cursor: cursor.to_string(),
            }));
    }

    /// Request a screenshot capture from the chassis, keyed by caller-provided id.
    pub fn screenshot(&self, id: u32) {
        self.runtime_context
            .enqueue_native_message(NativeMessage::Screenshot(ScreenshotPatch {
                id,
                scale: None,
            }));
    }

    /// Shared map where completed screenshot captures are published by id.
    pub fn get_screenshot_map(&self) -> Rc<RefCell<HashMap<u32, ScreenshotData>>> {
        self.runtime_context.get_screenshot_map()
    }
}

#[cfg(feature = "designtime")]
impl NodeContext {
    /// Send updated project source to the design server for designtime-only application.
    pub fn submit_userland_source_update(
        &self,
        request_id: String,
        path: String,
        contents: String,
    ) -> Result<(), String> {
        self.designtime
            .borrow_mut()
            .send_userland_source_update(UserlandSourceUpdateRequest {
                request_id,
                path,
                contents,
            })
            .map_err(|err| err.to_string())
    }

    /// Drain any queued responses from designtime source update requests.
    pub fn take_userland_source_update_responses(&self) -> Vec<UserlandSourceUpdateResponse> {
        self.designtime
            .borrow_mut()
            .take_userland_source_update_responses()
    }

    pub fn raycast(&self, point: Point2<Window>, hit_invisible: bool) -> Vec<NodeInterface> {
        let expanded_nodes = self.runtime_context.get_elements_beneath_ray(
            self.runtime_context.get_userland_root_expanded_node(),
            point,
            false,
            vec![],
            hit_invisible,
        );
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
