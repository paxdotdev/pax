//! Reactive placement dependencies. Values contain no mounted-node ownership;
//! bindings use weak references and are retired with the node's property scope.
use crate::{ExpandedNode, RuntimeContext};
use kurbo::{Affine, BezPath};
use pax_runtime_api::{Interpolatable, LocalProperty};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScrollLineage {
    pub affine: Affine,
    #[cfg(feature = "3d")]
    pub world: [f64; 3],
    #[cfg(feature = "3d")]
    pub depth: f64,
    #[cfg(feature = "3d")]
    pub native: Option<pax_runtime_api::math::PlanarProjection>,
}

impl ScrollLineage {
    pub fn identity() -> Self {
        Self {
            affine: Affine::IDENTITY,
            #[cfg(feature = "3d")]
            native: Some(Default::default()),
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollState {
    pub inherited: ScrollLineage,
    pub children: ScrollLineage,
    pub step: Affine,
    #[cfg(feature = "3d")]
    pub depth_step: f64,
}
impl Default for ScrollState {
    fn default() -> Self {
        Self {
            inherited: ScrollLineage::identity(),
            children: ScrollLineage::identity(),
            step: Affine::IDENTITY,
            #[cfg(feature = "3d")]
            depth_step: 0.,
        }
    }
}
impl Interpolatable for ScrollState {}

#[derive(Clone, Default)]
pub(crate) struct ClipGeometry(pub Option<Rc<BezPath>>);
impl Interpolatable for ClipGeometry {}

#[derive(Default)]
pub(crate) struct PreparationWork {
    pub scroll: Cell<u64>,
    pub clip: Cell<u64>,
}

impl ExpandedNode {
    pub(crate) fn bind_presentation_inputs(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let parent = self
            .presentation_parent_node()
            .map(|p| p.scroll_state.clone())
            .unwrap_or_default();
        let instance = self.instance_node.borrow();
        let mut scroll_deps: Vec<_> = self
            .properties_scope
            .borrow()
            .iter()
            .filter(|(name, _)| instance.property_requires_input_recompute(name))
            .map(|(_, p)| p.get_untyped_property().clone())
            .collect();
        scroll_deps.push(parent.untyped());
        scroll_deps.push(self.presentation_transform_and_bounds.untyped());
        #[cfg(feature = "3d")]
        scroll_deps.extend([
            self.spatial_geometry.untyped(),
            self.presentation_view.untyped(),
        ]);
        let weak = Rc::downgrade(self);
        let context = Rc::downgrade(ctx);
        let work = ctx.presentation_work();
        self.scroll_state.replace_with(LocalProperty::computed(
            move || {
                let inherited = parent.get().children;
                let (Some(node), Some(ctx)) = (weak.upgrade(), context.upgrade()) else {
                    return ScrollState::default();
                };
                let mut state = ScrollState {
                    inherited,
                    children: inherited,
                    step: Affine::IDENTITY,
                    #[cfg(feature = "3d")]
                    depth_step: 0.,
                };
                if node.instance_node.borrow().scrolls_content(&node) {
                    work.scroll.set(work.scroll.get() + 1);
                    let offset = ctx.scroller_presentation_offset(&node);
                    state.step = ctx.compute_scroller_presentation_transform(&node, offset);
                    state.children.affine = inherited.affine * state.step;
                    #[cfg(feature = "3d")]
                    {
                        let (x, y) = offset;
                        let world = node.spatial_geometry.get();
                        let matrix = world.world_transform().coeffs();
                        for axis in 0..3 {
                            state.children.world[axis] -= matrix[axis] * x + matrix[4 + axis] * y;
                        }
                        let projected = node.presentation_geometry.get();
                        if projected.is_spatial() {
                            let matrix = projected.world_transform().coeffs();
                            state.depth_step = -matrix[2] * x - matrix[6] * y;
                            state.children.depth += state.depth_step;
                        }
                        if x != 0. || y != 0. {
                            use pax_runtime_api::math::{PlanarProjection, Transform3, Vector3};
                            let p = inherited.world;
                            let moved = crate::SpatialGeometry::new(
                                Transform3::translate(Vector3::new(p[0], p[1], p[2]))
                                    * world.world_transform(),
                                world.bounds,
                            );
                            state.children.native =
                                node.presentation_view.get().plane(moved).and_then(|plane| {
                                    let plane = plane.mapping();
                                    Some(
                                        plane
                                            * PlanarProjection::from(Affine::translate((-x, -y)))
                                            * plane.inverse()?
                                            * inherited.native?,
                                    )
                                });
                        }
                    }
                }
                state
            },
            &scroll_deps,
        ));

        let mut clip_deps: Vec<_> = self
            .properties_scope
            .borrow()
            .iter()
            .filter(|(name, _)| instance.property_requires_clip_recompute(name))
            .map(|(_, p)| p.get_untyped_property().clone())
            .collect();
        clip_deps.push(self.layout_bounds.untyped());
        let weak = Rc::downgrade(self);
        let work = ctx.presentation_work();
        self.local_clip_geometry
            .replace_with(LocalProperty::computed(
                move || {
                    let Some(node) = weak.upgrade() else {
                        return ClipGeometry::default();
                    };
                    let path = node.instance_node.borrow().resolve_local_clip_path(&node);
                    if path.is_some() {
                        work.clip.set(work.clip.get() + 1);
                    }
                    ClipGeometry(path.map(Rc::new))
                },
                &clip_deps,
            ));
        clip_deps.extend([
            self.local_clip_geometry.untyped(),
            self.presentation_transform_and_bounds.untyped(),
        ]);
        #[cfg(feature = "3d")]
        clip_deps.extend([
            self.spatial_geometry.untyped(),
            self.presentation_view.untyped(),
        ]);
        let local = self.local_clip_geometry.clone();
        let weak = Rc::downgrade(self);
        let work = ctx.presentation_work();
        self.world_clip_geometry
            .replace_with(LocalProperty::computed(
                move || {
                    let Some(node) = weak.upgrade() else {
                        return ClipGeometry::default();
                    };
                    let path = if let Some(local) = local.get().0 {
                        node.presentation_projection()
                            .map(|p| p.project_path(&local, None))
                    } else {
                        let path = node.instance_node.borrow().resolve_effect_clip_path(&node);
                        if path.is_some() {
                            work.clip.set(work.clip.get() + 1);
                        }
                        path
                    };
                    ClipGeometry(path.map(Rc::new))
                },
                &clip_deps,
            ));
    }
}
