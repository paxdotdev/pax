//! Native content's retained chart is the nearest native Scroller, independent of
//! vector layer allocation. Frame clips do not change this coordinate space.
use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};
use pax_runtime_api::{Interpolatable, LocalProperty, Window};
use std::rc::Rc;

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct NativeLocalPlacement {
    owner: Option<u32>,
    tab: Option<TransformAndBounds<NodeLocal, Window>>,
}
impl Interpolatable for NativeLocalPlacement {}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PlacementDependency {
    Local(u32, TransformAndBounds<NodeLocal, Window>),
    #[cfg(feature = "3d")]
    World(crate::SpatialGeometry),
    #[cfg(not(feature = "3d"))]
    World(TransformAndBounds<NodeLocal, Window>),
}
impl Default for PlacementDependency {
    fn default() -> Self {
        Self::World(Default::default())
    }
}
impl Interpolatable for PlacementDependency {}

impl ExpandedNode {
    pub(crate) fn retained_native_placement(
        &self,
    ) -> Option<(u32, TransformAndBounds<NodeLocal, Window>)> {
        let local = self.native_local_placement.get();
        local.owner.zip(local.tab)
    }

    // Cut off host-only changes before input/coverage effects, not after their
    // world-space projections and path serialization have already run.
    pub(crate) fn retained_placement_dependency(
        &self,
        eligible: LocalProperty<bool>,
    ) -> LocalProperty<PlacementDependency> {
        let local = self.native_local_placement.clone();
        #[cfg(feature = "3d")]
        let world = self.presentation_geometry.clone();
        #[cfg(not(feature = "3d"))]
        let world = self.presentation_transform_and_bounds.clone();
        let deps = [local.untyped(), world.untyped(), eligible.untyped()];
        LocalProperty::computed_with_cutoff(
            move || {
                if eligible.get() {
                    let local = local.get();
                    if let Some((owner, tab)) = local.owner.zip(local.tab) {
                        return PlacementDependency::Local(owner, tab);
                    }
                }
                PlacementDependency::World(world.get())
            },
            &deps,
            PartialEq::eq,
        )
    }
    pub(crate) fn bind_native_placement(self: &Rc<Self>) {
        let parent = self.render_parent_node();
        let scroller = parent
            .as_ref()
            .filter(|p| p.instance_node.borrow().resolve_scroll_offset(p).is_some())
            .map(|p| p.id.to_u32());
        let parent = parent
            .map(|p| p.native_local_placement.clone())
            .unwrap_or_default();
        let local = self.local_transform_and_bounds.clone();
        let domain = self.affine_canvas_domain.clone();
        let origin = self.move_origin.clone();
        let progress = self.move_progress.clone();
        let detached = self.is_render_source() || self.move_in_flight.get();
        let deps = [
            parent.untyped(),
            local.untyped(),
            domain.untyped(),
            origin.untyped(),
            progress.untyped(),
        ];
        self.native_local_placement
            .replace_with(LocalProperty::computed_with_cutoff(
                move || {
                    let parent = parent.get();
                    let owner = scroller.or(parent.owner);
                    let mut result = NativeLocalPlacement { owner, tab: None };
                    // Same-domain moves retain their origin after easing ends.
                    // At the exact destination pose, local placement is valid
                    // again; interrupted/overshooting motion stays conservative.
                    let moving = origin.get().is_some() && progress.get() != 1.;
                    if detached || owner.is_none() || !domain.get() || moving {
                        return result;
                    }
                    let local = local.get();
                    result.tab = if scroller.is_some() {
                        Some(local.cast_spaces())
                    } else {
                        parent.tab.map(|parent| TransformAndBounds {
                            transform: parent.transform * local.transform,
                            bounds: local.bounds,
                        })
                    };
                    result
                },
                &deps,
                NativeLocalPlacement::eq,
            ));
    }

    /// Retained native placement: an optional Scroller ID and a transform
    /// in that host's content coordinates. With no owner, the matrix uses the
    /// existing world/projected native convention. Chassis must honor both modes.
    /// Affine host movement does not invalidate the local result; spatial,
    /// detached and flight placements retain the conservative world path.
    pub fn native_placement(
        self: &Rc<Self>,
        ctx: &Rc<RuntimeContext>,
    ) -> LocalProperty<(Option<u32>, Vec<f64>)> {
        let local = self.native_local_placement.clone();
        #[cfg(feature = "3d")]
        let geometry = self.presentation_geometry().untyped();
        #[cfg(not(feature = "3d"))]
        let geometry = self.presentation_transform_and_bounds.untyped();
        let deps = [local.untyped(), geometry];
        let node = Rc::downgrade(self);
        let context = Rc::downgrade(ctx);
        let fallback = LocalProperty::computed_with_cutoff(
            move || {
                let local = local.get();
                if local.tab.is_some() {
                    return None;
                }
                let matrix = node.upgrade().zip(context.upgrade()).map_or_else(
                    || vec![0.; 6],
                    |(node, ctx)| ctx.native_presentation_matrix_for_node(&node),
                );
                Some(matrix)
            },
            &deps,
            PartialEq::eq,
        );
        let local = self.native_local_placement.clone();
        let deps = [local.untyped(), fallback.untyped()];
        LocalProperty::computed_with_cutoff(
            move || {
                let local = local.get();
                if let Some(tab) = local.tab {
                    (
                        local.owner,
                        kurbo::Affine::from(tab.transform).as_coeffs().to_vec(),
                    )
                } else {
                    (None, fallback.get().unwrap_or_else(|| vec![0.; 6]))
                }
            },
            &deps,
            PartialEq::eq,
        )
    }
}
