//! Coalesce inherited pose changes before framework geometry preparation.
//!
//! Local content effects keep their own DAG inputs. This first stage still
//! prepares current world records for drawing and native compositing; it does
//! not retain screen-space masks or introduce a shared GPU transform hierarchy.
use crate::{ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};
use pax_runtime_api::{Layer, LocalProperty};
use std::{collections::HashSet, rc::Rc};

pub(crate) struct RetainedMotion {
    pub dirty: HashSet<ExpandedNodeIdentifier>,
    pub enabled: bool,
    pub notifications: u64,
    pub visited: u64,
    pub placements: u64,
}
impl Default for RetainedMotion {
    fn default() -> Self {
        Self {
            dirty: HashSet::new(),
            enabled: true,
            notifications: 0,
            visited: 0,
            placements: 0,
        }
    }
}

impl ExpandedNode {
    pub(crate) fn bind_retained_motion(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let instance = self.instance_node.borrow();
        self.motion_retained.set(
            ctx.retained_motion.borrow().enabled
                && instance.supports_retained_motion()
                && !self.is_render_source()
                && !instance.is_overlay()
                && matches!(
                    instance.base().flags().layer,
                    Layer::Canvas | Layer::DontCare
                ),
        );
        // A non-retained ancestor may still move retained descendants. Observe
        // local pose at every tree node, never inherited world placement here.
        if !ctx.retained_motion.borrow().enabled {
            self.motion_listener.replace_with(LocalProperty::default());
            return;
        }
        let pose = self.local_spatial_geometry.clone();
        let camera = self.own_camera_view.clone();
        let origin = self.move_origin.clone();
        let progress = self.move_progress.clone();
        // A flight is detached from its destination's presentation subtree,
        // but its interpolated pose still follows that destination and camera.
        // Only the flight root needs this world-dependent notification.
        let flight = self
            .move_in_flight
            .get()
            .then(|| self.presentation_geometry.clone());
        let viewport = self
            .render_parent_node()
            .is_none()
            .then(|| ctx.globals().viewport);
        let mut deps = vec![
            pose.untyped(),
            camera.untyped(),
            origin.untyped(),
            progress.untyped(),
        ];
        if let Some(viewport) = &viewport {
            deps.push(viewport.untyped());
        }
        if let Some(flight) = &flight {
            deps.push(flight.untyped());
        }
        let node = Rc::downgrade(self);
        let context = Rc::downgrade(ctx);
        self.motion_listener.replace_with(LocalProperty::computed(
            move || {
                pose.get();
                camera.get();
                origin.get();
                progress.get();
                if let Some(flight) = &flight {
                    flight.get();
                }
                if let Some(viewport) = &viewport {
                    viewport.get();
                }
                let (Some(node), Some(ctx)) = (node.upgrade(), context.upgrade()) else {
                    return;
                };
                if node.attached.get() > 0 && ctx.retained_motion.borrow().enabled {
                    let mut state = ctx.retained_motion.borrow_mut();
                    state.notifications += 1;
                    state.dirty.insert(node.id);
                }
            },
            &deps,
        ));
        ctx.register_expanded_node_effect_property_named(
            self,
            &self.motion_listener,
            "local pose notification",
        );
    }
}

impl RuntimeContext {
    pub(crate) fn flush_retained_motion(&self) {
        let dirty = std::mem::take(&mut self.retained_motion.borrow_mut().dirty);
        if dirty.is_empty() {
            return;
        }
        let started = self.work_profile_active().then(web_time::Instant::now);
        let profile_node = dirty.iter().next().unwrap().to_u32();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        // A changed child beneath another changed root is covered by that root's
        // traversal. No equality of world matrices is needed to discover reuse.
        for id in &dirty {
            let Some(node) = self.get_expanded_node_by_eid(*id) else {
                continue;
            };
            let mut ancestor = node.presentation_parent_node();
            let mut covered = false;
            while let Some(parent) = ancestor {
                if dirty.contains(&parent.id) {
                    covered = true;
                    break;
                }
                ancestor = parent.presentation_parent_node();
            }
            if !covered {
                stack.push(node);
            }
        }
        let mut placements = 0;
        while let Some(node) = stack.pop() {
            if !visited.insert(node.id) || node.attached.get() == 0 {
                continue;
            }
            if node.motion_retained.get() {
                // Existing affine Scroller charts already retain local input and
                // coverage. Their local-placement subscriptions handle inner edits.
                let local_chart = node.retained_native_placement().is_some();
                if !local_chart || !node.instance_node.borrow().geometry_is_scroller_local() {
                    self.mark_node_input_dirty(node.id.to_u32());
                }
                if !local_chart || !node.local_compositing.get() {
                    self.mark_node_occlusion_placement_dirty(node.id.to_u32());
                }
                if node.instance_node.borrow().base().flags().layer == Layer::Canvas
                    && !(node.has_surface_local_canvas_placement()
                        && node.instance_node.borrow().geometry_is_scroller_local())
                {
                    self.mark_canvas_placement_dirty(&node);
                    self.set_canvas_dirty(node.occlusion.get().render_layer_id);
                }
                placements += 1;
            }
            stack.extend(self.placed_children(&node, node.mounted_children.borrow().clone()));
            stack.extend(node.sidecar_children.borrow().iter().cloned());
        }
        let mut state = self.retained_motion.borrow_mut();
        state.visited += visited.len() as u64;
        state.placements += placements;
        drop(state);
        if let Some(started) = started {
            self.record_node_work(
                profile_node,
                "retained motion propagation",
                started.elapsed().as_secs_f64() * 1000.,
            );
        }
    }
}

#[cfg(test)]
mod tests;
