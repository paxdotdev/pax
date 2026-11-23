//! Retained paint placement inside an affine scroll surface. World geometry stays
//! available for native presentation, lighting, clipping, and input queries.
use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};
use pax_runtime_api::{Interpolatable, LocalProperty, Window};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct CanvasLocalPlacement {
    owner: Option<u32>,
    tab: Option<TransformAndBounds<NodeLocal, Window>>,
}
impl Interpolatable for CanvasLocalPlacement {}

// Affine paint on a fallback surface also bakes the world-space lighting plane.
// Projected or detached paint has additional inputs beyond its XY footprint.
#[derive(Clone, Copy, PartialEq)]
enum CanvasPaintBasis {
    SurfaceLocal,
    Affine {
        #[cfg(feature = "3d")]
        world: TransformAndBounds<NodeLocal, Window>,
    },
    AlwaysRepaint,
}

impl ExpandedNode {
    /// Wakes clip consumers when reassignment changes the canvas owner even if
    /// the layer index and transform stay equal.
    #[doc(hidden)]
    pub fn canvas_owner_dependency(&self) -> pax_runtime_api::properties::UntypedProperty {
        self.canvas_owner.untyped()
    }

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
        self.affine_canvas_domain.replace_with_computed_cutoff(
            move || {
                let transform = kurbo::Affine::from(local.get().transform);
                let affine =
                    parent_domain.get() && transform.is_finite() && transform.determinant() != 0.;
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
        );

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
        self.canvas_local_placement.replace_with_computed_cutoff(
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
        );

        self.bind_canvas_paint_placement(ctx);
    }

    // Rebind on owner changes even if the two surfaces currently have identical
    // transforms. Future owner motion must wake exactly its own paint listeners.
    pub(crate) fn bind_canvas_paint_placement(&self, ctx: &RuntimeContext) {
        let detached = self.is_render_source();
        let owner = self
            .canvas_owner
            .get()
            .and_then(|id| ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)));
        let owner_tab = owner
            .as_ref()
            .filter(|_| !detached)
            .map(|node| node.presentation_transform_and_bounds.clone());
        let local = self.canvas_local_placement.clone();
        let projected = self.presentation_transform_and_bounds.clone();
        let mut deps = vec![local.untyped(), projected.untyped()];
        deps.extend(owner_tab.iter().map(LocalProperty::untyped));
        #[cfg(feature = "3d")]
        let (spatial, presented, view, owner_domain) = {
            let spatial = self.spatial_geometry.clone();
            let presented = self.presentation_geometry.clone();
            let view = self.presentation_view.clone();
            let owner_domain = owner.as_ref().map(|n| n.affine_canvas_domain.clone());
            deps.extend([spatial.untyped(), presented.untyped(), view.untyped()]);
            deps.extend(owner_domain.iter().map(LocalProperty::untyped));
            (spatial, presented, view, owner_domain)
        };
        let profile = ctx
            .work_profiling_enabled()
            .then(|| Rc::downgrade(&ctx.work_profile));
        let id = self.id.to_u32();
        let resolve = move || {
            if let Some(tab) = local.get().tab {
                return (tab, CanvasPaintBasis::SurfaceLocal);
            }
            let _span = profile.as_ref().and_then(|p| p.upgrade()).and_then(|p| {
                crate::work_profile::WorkProfileScope::start(&p, id, "canvas world placement read")
            });
            let mut canvas = projected.get();
            if let Some(owner) = &owner_tab {
                canvas.transform = pax_runtime_api::math::Transform2::new(
                    RuntimeContext::inverse_canvas_owner_transform(owner.get()).as_coeffs(),
                ) * canvas.transform;
            }
            if detached {
                return (canvas, CanvasPaintBasis::AlwaysRepaint);
            }
            #[cfg(feature = "3d")]
            let world = {
                let world = spatial.get();
                if world.is_spatial()
                    || presented.get().is_spatial()
                    || view.get() != Default::default()
                    || owner_domain.as_ref().is_some_and(|p| !p.get())
                {
                    return (canvas, CanvasPaintBasis::AlwaysRepaint);
                }
                world
                    .orthographic_projection()
                    .expect("planar geometry retains its affine transform")
            };
            (
                canvas,
                CanvasPaintBasis::Affine {
                    #[cfg(feature = "3d")]
                    world,
                },
            )
        };

        // Keep the cutoff on the public geometry property itself. A read before
        // the effects drain must evaluate its dirty inputs. A projection of a
        // second cutoff could still be clean and return the previous owner's pose.
        let previous_basis = Cell::new(None);
        let same_basis = Rc::new(Cell::new(false));
        let evaluated_basis = same_basis.clone();
        let placement = LocalProperty::computed_with_cutoff_and_name(
            move || {
                let (tab, basis) = resolve();
                let previous = previous_basis.replace(Some(basis));
                evaluated_basis
                    .set(basis != CanvasPaintBasis::AlwaysRepaint && previous == Some(basis));
                tab
            },
            &deps,
            move |old, new| same_basis.get() && old == new,
            "canvas paint placement",
        );
        let instance = self.instance_node.borrow();
        let flags = instance.base().flags();
        let paintless = flags.layer == crate::api::Layer::DontCare
            && (flags.is_component || flags.is_slot || flags.invisible_to_slot);
        self.canvas_transform_and_bounds.replace_with_paint_cutoff(
            placement,
            &self.changed_listener,
            paintless,
        );
    }
}
