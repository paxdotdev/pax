use kurbo::{Affine, Rect, Shape};
use pax_engine::*;
use pax_runtime::api as pax_runtime_api;
use pax_runtime::api::{use_RefCell, Stroke};
use pax_runtime::api::{Layer, RenderContext};
use pax_runtime::BaseInstance;
use pax_runtime::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};

use_RefCell!();
use std::rc::Rc;

const ELLIPSE_PATH_ACCURACY: f64 = 0.01;

/// A 2D vector ellipse, which inscribes its bounding box with the specified fill and stroke.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::ellipse::EllipseInstance")]
pub struct Ellipse {
    /// Ordered outline layers above the fills, index zero topmost. Empty by default.
    pub stroke: Property<Vec<Stroke>>,
    /// Paint painted inside the ellipse.
    pub fill: Property<Vec<pax_runtime_api::Fill>>,
}

impl Default for Ellipse {
    fn default() -> Self {
        Self {
            fill: Property::new(vec![pax_runtime_api::Fill::default()]),
            stroke: Default::default(),
        }
    }
}

// Runtime instance backing `<Ellipse>`.
pub struct EllipseInstance {
    base: BaseInstance,
}

impl InstanceNode for EllipseInstance {
    fn supports_alpha_source_render(&self) -> bool {
        true
    }

    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(EllipseInstance {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: Layer::Canvas,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let tab = expanded_node.transform_and_bounds.clone();
        let (stroke, fill) = expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            (properties.stroke.clone(), properties.fill.clone())
        });

        let appearance =
            crate::common::watch_appearance(expanded_node, context, fill.clone(), stroke.clone());
        let deps = &[
            appearance.untyped(),
            tab.untyped(),
            stroke.untyped(),
            fill.untyped(),
            expanded_node.computed_opacity.untyped(),
            expanded_node.computed_opacity_scopes.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    appearance.get();
                    cloned_context.mark_canvas_node_dirty(cloned_expanded_node.id);
                    cloned_context
                        .set_canvas_dirty(cloned_expanded_node.occlusion.get().render_layer_id)
                },
                deps,
            ));
    }

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        expanded_node.with_properties_unwrapped(|p: &mut Ellipse| {
            let tab = expanded_node.transform_and_bounds.get();
            let rect = Rect::new(0.0, 0.0, tab.bounds.0, tab.bounds.1);
            Some(
                Affine::from(tab.transform)
                    * crate::common::appearance_coverage_path(
                        &kurbo::Ellipse::from_rect(rect).to_path(ELLIPSE_PATH_ACCURACY),
                        &p.fill.get(),
                        &p.stroke.get(),
                    ),
            )
        })
    }

    fn resolve_alpha_mask_paints(
        &self,
        node: &ExpandedNode,
    ) -> Vec<pax_runtime_api::AlphaMaskPaint> {
        node.with_properties_unwrapped(|p: &mut Ellipse| {
            let (w, h) = node.transform_and_bounds.get().bounds;
            crate::common::alpha_mask_paints(
                node,
                kurbo::Ellipse::from_rect(Rect::new(0.0, 0.0, w, h)).to_path(ELLIPSE_PATH_ACCURACY),
                p.fill.get(),
                p.stroke.get(),
            )
        })
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            (crate::common::appearance_coverage_alpha(
                &properties.fill.get(),
                &properties.stroke.get(),
            ) * expanded_node.computed_opacity.get())
            .clamp(0.0, 1.0)
        })
    }

    fn prepare_canvas_geometry(
        &self,
        node: &ExpandedNode,
    ) -> pax_runtime::scene_geometry::CanvasGeometry {
        let bounds = node.transform_and_bounds.get().bounds;
        let local_bounds = node.with_properties_unwrapped(|p: &mut Ellipse| {
            crate::common::stroke_coverage_bounds(
                kurbo::Rect::new(0.0, 0.0, bounds.0, bounds.1),
                &p.stroke.get(),
            )
        });
        pax_runtime::scene_geometry::CanvasGeometry {
            local_bounds: Some(local_bounds),
            path: None,
        }
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        if rc.alpha_source_layer().is_none() && !rtc.is_canvas_node_dirty(&expanded_node.id) {
            return;
        }

        let Some(scope) = crate::common::begin_bounded_canvas_node(rc, expanded_node, rtc) else {
            return;
        };

        let (width, height) = scope.bounds;
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            let rect = Rect::from_points((0.0, 0.0), (width, height));
            let ellipse = kurbo::Ellipse::from_rect(rect);
            let bez_path = ellipse.to_path(ELLIPSE_PATH_ACCURACY);
            rc.save(scope.layer_id);
            rc.transform(scope.layer_id, scope.surface_transform);
            crate::common::draw_appearance(
                rc,
                scope.layer_id,
                bez_path,
                &properties.fill.get(),
                &properties.stroke.get(),
                scope.paint_opacity,
                pax_runtime_api::PathSmoothing::None,
                0.0,
                1.0,
            );
            rc.restore(scope.layer_id);
        });
        if rc.end_node(scope.layer_id, scope.node_id) {
            rtc.clear_canvas_node_dirty(&expanded_node.id);
        }
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_e: &mut Ellipse| f.debug_struct("Ellipse").finish()),
            None => f.debug_struct("Ellipse").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
