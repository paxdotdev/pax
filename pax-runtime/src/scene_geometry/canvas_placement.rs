//! Retained paint placement inside an affine scroll surface. World geometry stays
//! available for native presentation, lighting, clipping, and input queries.
use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};
use pax_runtime_api::{Interpolatable, LocalProperty, Window};
use std::rc::Rc;

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct CanvasLocalPlacement {
    owner: Option<u32>,
    tab: Option<TransformAndBounds<NodeLocal, Window>>,
}
impl Interpolatable for CanvasLocalPlacement {}

impl ExpandedNode {
    /// Whether ordinary paint is composed directly in an allocated affine
    /// scroller surface. Native placement and world-space queries stay separate.
    pub fn has_surface_local_canvas_placement(&self) -> bool {
        self.canvas_local_placement.get().tab.is_some()
    }

    pub(crate) fn bind_canvas_placement(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let parent = self.render_parent_node();
        let parent_domain = parent
            .as_ref()
            .map(|p| p.affine_canvas_domain.clone())
            .unwrap_or_else(|| LocalProperty::new(true));
        let local = self.local_transform_and_bounds.clone();
        let deps = vec![parent_domain.untyped(), local.untyped()];
        #[cfg(feature = "3d")]
        let spatial = {
            let common = self.get_common_properties();
            let common = common.borrow();
            (
                common.rotate_x.clone(),
                common.rotate_y.clone(),
                common.z.clone(),
                self.own_camera_view.clone(),
            )
        };
        #[cfg(feature = "3d")]
        let deps = {
            let mut deps = deps;
            deps.extend([
                spatial.0.untyped(),
                spatial.1.untyped(),
                spatial.2.untyped(),
                spatial.3.untyped(),
            ]);
            deps
        };
        self.affine_canvas_domain
            .replace_with(LocalProperty::computed_with_cutoff_and_name(
                move || {
                    let transform = kurbo::Affine::from(local.get().transform);
                    let affine = parent_domain.get()
                        && transform.is_finite()
                        && transform.determinant() != 0.;
                    #[cfg(feature = "3d")]
                    let affine = affine
                        && spatial.0.get().unwrap_or_default().get_as_radians() == 0.
                        && spatial.1.get().unwrap_or_default().get_as_radians() == 0.
                        && spatial.2.get().unwrap_or_default().to_float() == 0.
                        && spatial.3.get().is_none();
                    affine
                },
                &deps,
                bool::eq,
                "canvas affine domain",
            ));

        let parent_id = parent.as_ref().map(|p| p.id.to_u32());
        let parent = parent
            .map(|p| p.canvas_local_placement.clone())
            .unwrap_or_default();
        let owner = self.canvas_owner.clone();
        let domain = self.affine_canvas_domain.clone();
        let local = self.local_transform_and_bounds.clone();
        let origin = self.move_origin.clone();
        let detached = self.is_render_source() || self.move_in_flight.get();
        let deps = [
            parent.untyped(),
            owner.untyped(),
            domain.untyped(),
            local.untyped(),
            origin.untyped(),
        ];
        self.canvas_local_placement
            .replace_with(LocalProperty::computed_with_cutoff_and_name(
                move || {
                    let owner = owner.get();
                    let mut result = CanvasLocalPlacement { owner, tab: None };
                    if detached || owner.is_none() || !domain.get() || origin.get().is_some() {
                        return result;
                    }
                    let local = local.get();
                    // Reset at the actual allocated surface, not at every authored
                    // Scroller: some backends share layers with nested scrollers.
                    result.tab = if owner == parent_id {
                        Some(local.cast_spaces())
                    } else {
                        let parent = parent.get();
                        (parent.owner == owner)
                            .then_some(parent.tab)
                            .flatten()
                            .map(|parent| TransformAndBounds {
                                transform: parent.transform * local.transform,
                                bounds: local.bounds,
                            })
                    };
                    result
                },
                &deps,
                CanvasLocalPlacement::eq,
                "canvas local placement",
            ));

        // Paint and clip listeners must hear a world change even when their surface
        // has not drawn yet. Template components and control-flow wrappers have no
        // paint of their own. Install their same cutoff only if someone reads it.
        let instance = self.instance_node.borrow();
        let flags = instance.base().flags();
        let wrapper = flags.layer == crate::api::Layer::DontCare
            && (flags.is_component || flags.is_slot || flags.invisible_to_slot);
        drop(instance);
        let local = self.canvas_local_placement.clone();
        let world = self.presentation_transform_and_bounds.clone();
        let make_fallback = move || {
            let local = local.clone();
            let world = world.clone();
            let deps = [local.untyped(), world.untyped()];
            LocalProperty::computed_with_cutoff_and_name(
                move || local.get().tab.is_none().then(|| world.get()),
                &deps,
                |old, new| old.is_none() && new.is_none(),
                "canvas world fallback",
            )
        };
        let fallback = if wrapper {
            LocalProperty::default()
        } else {
            make_fallback()
        };
        let deferred = std::cell::Cell::new(wrapper);
        let local = self.canvas_local_placement.clone();
        let context = Rc::downgrade(ctx);
        let node = Rc::downgrade(self);
        // Surface ownership is updated by occlusion assignment. Depending
        // on the complete order record would turn z-order-only work into paint
        // work, or require settling a new cutoff before retained replay.
        let deps = [local.untyped(), fallback.untyped()];
        self.canvas_transform_and_bounds
            .replace_with(LocalProperty::computed(
                move || {
                    if deferred.replace(false) {
                        fallback.replace_with(make_fallback());
                    }
                    if let Some(tab) = local.get().tab {
                        return tab;
                    }
                    let tab = fallback.get().unwrap_or_default();
                    let (Some(ctx), Some(node)) = (context.upgrade(), node.upgrade()) else {
                        return tab;
                    };
                    let _span =
                        ctx.profile_node_work(node.id.to_u32(), "canvas world placement read");
                    TransformAndBounds {
                        transform: pax_runtime_api::math::Transform2::new(
                            ctx.world_to_canvas_transform_for_node(&node).as_coeffs(),
                        ) * tab.transform,
                        bounds: tab.bounds,
                    }
                },
                &deps,
            ));
    }
}
