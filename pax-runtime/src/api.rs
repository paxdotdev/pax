use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};

use_RefCell!();
use crate::{
    node_interface::NodeLocal, ExpandedNode, LayoutHull, RuntimeContext,
    RuntimePropertiesStackFrame, TransformAndBounds,
};
use kurbo::Affine;

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

/// This runtime owner stays on the UI thread. Transfer properties or delivery
/// handles to workers instead.
///
/// ```compile_fail
/// fn requires_send<T: Send>() {}
/// requires_send::<pax_runtime::api::NodeContext>();
/// ```
#[derive(Clone)]
/// Runtime context passed into user component lifecycle methods and event handlers.
///
/// For an inline binding, Rust `self` belongs to the component that authored
/// the binding, while this context describes the child receiving the call.
/// Bounds, expression stack access, subscriptions, and async lifetime remain target-local.
/// Component-level bindings receive their own component's state and context.
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
    pub slot_index: LocalProperty<Option<usize>>,
    pub expression_stack: Rc<RuntimePropertiesStackFrame>,
    pub(crate) store_generation: u64,
    /// Reference to the ExpandedNode of the component containing this node
    pub containing_component: Weak<ExpandedNode>,
    /// The current global engine frame count.
    pub elapsed_frames: LocalProperty<u64>,
    /// The current global engine wall-clock time in milliseconds.
    pub elapsed_millis: LocalProperty<u64>,
    /// Current device orientation sensor reading.
    pub gyro: LocalProperty<Gyro>,
    /// Current device accelerometer reading.
    pub accel: LocalProperty<Accel>,
    /// The bounds of this element's immediate container (parent) in px
    pub bounds_parent: LocalProperty<(f64, f64)>,
    /// The bounds of this element in px
    pub bounds_self: LocalProperty<(f64, f64)>,
    /// Measured bounds resolved by the chassis or container layout for this node.
    pub measured_size: LocalProperty<Option<(f64, f64)>>,
    /// Node-local subtree layout hull published by the engine for container measurement.
    pub subtree_layout_hull: LocalProperty<LayoutHull>,
    /// Current platform (Web/Native) this app is running on
    pub platform: Platform,
    /// Current os (Android/Windows/Mac/Linux) this app is running on
    pub os: OS,
    /// Derived target facts for platform/OS checks.
    pub target: LocalProperty<TargetInfo>,
    /// Derived viewport facts for size and orientation checks.
    pub viewport: LocalProperty<Viewport>,
    /// The number of projected children available to this node.
    ///
    /// This is the raw transport count used by slot-driven implementations.
    /// Container-style consumers usually want `received_children_count`
    /// instead.
    pub projected_children_count: LocalProperty<usize>,
    /// Borrow of the RuntimeContext, used at least for exposing raycasting to userland
    pub(crate) runtime_context: Rc<RuntimeContext>,
    /// The transform of this node in the global coordinate space
    pub node_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
    /// Children projected into this node from the containing component.
    ///
    /// Projection is an engine transport mechanism. Consumers that want the
    /// semantic payload owned by this node should prefer `received_children`.
    pub projected_children: LocalProperty<Vec<Rc<ExpandedNode>>>,
    /// A structural invalidation signal for projected children.
    pub projected_children_changed: LocalProperty<()>,
    /// Semantic payload children received by this node from its caller.
    ///
    /// This is the canonical "content" view for container-style logic. It
    /// excludes private encapsulated implementation children and also excludes
    /// exit-retained payload nodes, which instead appear in
    /// `retained_received_children`.
    pub received_children: LocalProperty<Vec<Rc<ExpandedNode>>>,
    /// Convenience count derived from `received_children`.
    pub received_children_count: LocalProperty<usize>,
    /// A structural invalidation signal for `received_children`.
    ///
    /// Prefer this or `received_children` itself for structural subscriptions
    /// that must react to reorders as well as insertions and removals.
    /// Unchanged child identities and order cut off propagation, except when a
    /// child remounts or its parent bindings change. Subscribe to child geometry
    /// separately for size or position updates.
    pub received_children_changed: LocalProperty<()>,
    /// Received children retained only so exit transitions can finish.
    ///
    /// These are no longer part of the active semantic payload, but some
    /// containers still need to place them as ghosts or overlays while their
    /// `@out` transitions run.
    pub retained_received_children: LocalProperty<Vec<Rc<ExpandedNode>>>,
    /// A structural invalidation signal for `retained_received_children`.
    pub retained_received_children_changed: LocalProperty<()>,

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
    pub(crate) get_elapsed_millis: Rc<dyn Fn() -> u128>,
}

impl NodeContext {
    /// Lifetime of this exact context node, ending at actual unmount. Retain a
    /// component mount scope when later handlers receive child-control contexts.
    /// A saved context cannot acquire the scope of a later mount of that node.
    pub fn async_scope(&self) -> Result<AsyncScope, AsyncError> {
        self.expanded_node
            .upgrade()
            .filter(|node| node.stores.generation() == self.store_generation)
            .ok_or(AsyncError::Closed)?
            .async_scope(&self.runtime_context)
    }

    /// Access the owning application lifetime, cancellation and error reporting.
    pub fn application(&self) -> AppContext {
        self.runtime_context.application.context()
    }

    /// Loading state and supported actions for a node in the loading component.
    /// Returns `None` in the main application, after unmount, or after loading
    /// closes. Descendants inherit the same context as `@loading` in Pax.
    pub fn loading(&self) -> Option<LoadingContext> {
        self.expanded_node
            .upgrade()
            .filter(|node| node.stores.generation() == self.store_generation)?;
        self.expression_stack
            .loading()
            .filter(|loading| loading.snapshot().is_some())
    }

    /// Convert a window-space point into normalized local coordinates, including any
    /// camera projection and presentation offsets inherited from ancestor scrollers.
    /// `(0, 0)` is the local origin and `(1, 1)` is the opposite layout corner.
    /// A clipped or noninvertible perspective mapping returns nonfinite coordinates.
    pub fn local_point(&self, p: Point2<Window>) -> Point2<NodeLocal> {
        if let Some(node) = self
            .expanded_node
            .upgrade()
            .filter(|n| n.has_perspective_camera())
        {
            let bounds = node.transform_and_bounds.get().bounds;
            return self
                .runtime_context
                .presented_local_point(&node, kurbo::Point::new(p.x, p.y))
                .map(|p| Point2::new(p.x / bounds.0, p.y / bounds.1))
                .unwrap_or_else(|| Point2::new(f64::NAN, f64::NAN));
        }
        let (node_transform, presentation_transform) = self
            .expanded_node
            .upgrade()
            .map(|node| {
                (
                    node.presentation_transform_and_bounds.get(),
                    self.runtime_context
                        .presentation_scroll_transform_for_node(&node),
                )
            })
            .unwrap_or((self.node_transform_and_bounds, Affine::IDENTITY));
        local_point_with_presentation_transform(node_transform, presentation_transform, p)
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
    ///
    /// On web targets, same-origin navigation in the current tab can be handled
    /// through the browser History API and routed back into Pax without a full
    /// page reload. Cargo metadata's
    /// `[package.metadata.pax.web].server_owned_prefixes` delegates matching paths
    /// to ordinary browser navigation before changing application history.
    /// Other targets use the active chassis navigation behavior.
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

    /// Queue a bubbling `@submit` from this context's node at the end of the tick.
    ///
    /// Custom controls can use this from their activation handler without
    /// re-entering borrowed component state. The payload has a Programmatic
    /// trigger. Values are read at delivery time, and detached sources are
    /// ignored. This neither performs validation nor dismisses the keyboard.
    pub fn submit(&self) {
        if let Some(node) = self.expanded_node.upgrade() {
            self.runtime_context.queue_submit(node);
        }
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

fn local_point_with_presentation_transform(
    node_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
    presentation_transform: Affine,
    point: Point2<Window>,
) -> Point2<NodeLocal> {
    node_transform_and_bounds.as_transform().inverse() * (presentation_transform.inverse() * point)
}

#[cfg(test)]
mod local_point_tests {
    use super::*;
    use pax_runtime_api::math::{Transform2, Vector2};

    #[test]
    fn local_point_accounts_for_ancestor_scroller_presentation_offset() {
        let node = TransformAndBounds {
            transform: Transform2::<NodeLocal, Window>::translate(Vector2::new(50.0, 400.0)),
            bounds: (200.0, 100.0),
        };
        let presentation_transform = Affine::translate((0.0, -300.0));

        let local = local_point_with_presentation_transform(
            node,
            presentation_transform,
            Point2::new(150.0, 140.0),
        );

        assert!((local.x - 0.5).abs() < 1e-9);
        assert!((local.y - 0.4).abs() < 1e-9);
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

    /// Queries the userland tree at a window-space point, in front-to-back order.
    /// Does not dispatch hover events, capture input, advance time, or drain callbacks.
    /// `hit_invisible` includes normally hidden/exit targets, but retains clipping
    /// and `_raycastable` checks. Paint opacity alone does not disable a hit.
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

impl NodeContext {
    /// Reference to this application's mounted root, excluding designtime chrome.
    /// Use `query` on the result to address nodes outside the current component.
    pub fn app_root(&self) -> Result<crate::NodeRef, crate::NodeError> {
        self.current_query_node()
            .ok_or(crate::NodeError::Unmounted)?;
        #[cfg(feature = "designtime")]
        let root = self
            .runtime_context
            .get_userland_root_expanded_node()
            .or_else(|| self.runtime_context.get_root_expanded_node());
        #[cfg(not(feature = "designtime"))]
        let root = self.runtime_context.get_root_expanded_node();
        let root = root
            .filter(|n| n.attached.get() > 0)
            .ok_or(crate::NodeError::Unmounted)?;
        Ok(crate::NodeRef::new(&root, &self.runtime_context))
    }

    fn current_query_node(&self) -> Option<Rc<ExpandedNode>> {
        self.expanded_node.upgrade().filter(|node| {
            node.attached.get() > 0 && node.stores.generation() == self.store_generation
        })
    }

    fn node_collection_owner(&self) -> Option<Rc<ExpandedNode>> {
        let node = self.current_query_node()?;
        let is_component = node.instance_node.borrow().base().flags().is_component;
        if is_component {
            Some(node)
        } else {
            self.containing_component.upgrade()
        }
    }

    /// Abandoned transfers declared by this component. Entries are unmounted;
    /// moving one explicitly starts a fresh mount. The snapshot owns no lifetime.
    pub fn detached_nodes(&self) -> crate::NodeQuery {
        self.node_collection_owner()
            .map(|owner| self.runtime_context.detached_nodes(&owner))
            .unwrap_or_else(|| crate::NodeQuery::failed(crate::NodeError::Unmounted))
    }

    /// Live transfers hosted in this component's authored subtree but owned
    /// outside it. Return these in `@pre_unmount` to preserve their mounted state.
    pub fn visiting_nodes(&self) -> crate::NodeQuery {
        self.node_collection_owner()
            .map(|host| self.runtime_context.visiting_nodes(&host))
            .unwrap_or_else(|| crate::NodeQuery::failed(crate::NodeError::Unmounted))
    }
}

impl pax_runtime_api::async_runtime::AsyncScopeSource for NodeContext {
    fn work_scope(&self) -> Result<AsyncScope, AsyncError> {
        self.async_scope()
    }
}
