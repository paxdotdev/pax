//! Wheel cancellation intent and its retained, input-only web projection.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use pax_message::{NativeMessage, WheelRegion, WheelRegionsPatch};
use pax_runtime_api::{Layer, WheelEvents};

use crate::{ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};

#[derive(Default)]
pub(crate) struct WheelRegions {
    scopes: BTreeSet<u32>,
    published: BTreeMap<u32, WheelRegion>,
}

impl ExpandedNode {
    /// Whether the normal template event route permits scoped wheel cancellation.
    /// Passive descendants do not revoke an enclosing handler's capability.
    pub fn allows_wheel_cancellation(&self) -> bool {
        if self.get_common_properties().borrow().wheel_events.get()
            == Some(WheelEvents::Cancellable)
        {
            return true;
        }
        self.template_parent.upgrade().is_some_and(|parent| parent.allows_wheel_cancellation())
    }
}

impl RuntimeContext {
    pub(crate) fn set_wheel_scope(&self, id: u32, policy: Option<WheelEvents>) {
        let mut state = self.wheel_regions.borrow_mut();
        if policy == Some(WheelEvents::Cancellable) {
            state.scopes.insert(id);
        } else {
            state.scopes.remove(&id);
        }
    }

    pub(crate) fn remove_wheel_region(&self, id: u32) {
        let mut state = self.wheel_regions.borrow_mut();
        state.scopes.remove(&id);
        if state.published.remove(&id).is_some() {
            // Remove input leaves before their native Frame/Scroller is deleted.
            self.enqueue_native_message(NativeMessage::WheelRegionsUpdate(WheelRegionsPatch {
                remove: vec![id], ..Default::default()
            }));
        }
    }

    pub(crate) fn update_wheel_regions(&self) {
        if !self.globals().target.web {
            return;
        }
        let mut state = self.wheel_regions.borrow_mut();
        if state.scopes.is_empty() && state.published.is_empty() {
            return;
        }
        // Only visit opted-in subtrees, not the page or its unrelated animations.
        // Values are compared before publishing; native scrolling does not change
        // these unscrolled transforms and produces no region patches.
        let mut pending: Vec<Rc<ExpandedNode>> = state.scopes.iter()
            .filter_map(|id| self.get_expanded_node_by_eid(ExpandedNodeIdentifier(*id)))
            .collect();
        let mut visited = BTreeSet::new();
        let mut next = BTreeMap::new();
        while let Some(node) = pending.pop() {
            if !visited.insert(node.id.to_u32()) || node.attached.get() == 0 || node.is_render_source()
                || node.transition_phase.get() == pax_manifest::cartridge_generation::TRANSITION_PHASE_EXIT
            {
                continue;
            }
            pending.extend(node.presentation_children(self));
            let instance = node.instance_node.borrow();
            let cp = node.get_common_properties();
            if instance.base().flags().invisible_to_raycasting
                || cp.borrow()._raycastable.get() == Some(false)
                || !node.allows_wheel_cancellation()
            {
                continue;
            }
            let tab = node.transform_and_bounds.get();
            let transform = tab.transform.coeffs();
            if tab.bounds.0 <= 0.0 || tab.bounds.1 <= 0.0
                || !tab.bounds.0.is_finite() || !tab.bounds.1.is_finite()
                || !transform.iter().all(|v| v.is_finite())
            {
                continue;
            }
            let placement = node.occlusion.get();
            let native = instance.materializes_native_surface(&node)
                || matches!(instance.base().flags().layer, Layer::Native | Layer::NativeNonOccluding);
            next.insert(node.id.to_u32(), WheelRegion {
                id: node.id.to_u32(), parent_frame: placement.parent_frame,
                z_index: placement.z_index, transform,
                size_x: tab.bounds.0, size_y: tab.bounds.1, native,
            });
        }
        let patch = WheelRegionsPatch {
            upsert: next.iter().filter(|(id, value)| state.published.get(id) != Some(value))
                .map(|(_, value)| value.clone()).collect(),
            remove: state.published.keys().filter(|id| !next.contains_key(id)).copied().collect(),
        };
        state.published = next;
        if !patch.upsert.is_empty() || !patch.remove.is_empty() {
            self.enqueue_native_message(NativeMessage::WheelRegionsUpdate(patch));
        }
    }
}
