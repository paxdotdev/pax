use kurbo::{Affine, Rect, Shape};
use pax_engine::*;
use pax_runtime::api::{use_RefCell, Stroke};
use pax_runtime::api::{Fill, Layer, Material, RenderContext};
use pax_runtime::BaseInstance;
use pax_runtime::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};

use crate::common::{begin_bounded_canvas_node, mark_canvas_node_dirty_on_render_change};
use_RefCell!();
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

const ELLIPSE_PATH_ACCURACY: f64 = 0.01;

/// A 2D vector ellipse, which inscribes its bounding box with the specified fill and stroke.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::ellipse::EllipseInstance")]
pub struct Ellipse {
    /// Stroke drawn around the ellipse.
    pub stroke: Property<Stroke>,
    /// Fill painted inside the ellipse.
    pub fill: Property<Fill>,
    /// Light-reactive surface response.
    pub material: Property<Material>,
}

// Runtime instance backing `<Ellipse>`.
pub struct EllipseInstance {
    base: BaseInstance,
}

impl InstanceNode for EllipseInstance {
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
        let (stroke, fill, material) =
            expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
                (
                    properties.stroke.clone(),
                    properties.fill.clone(),
                    properties.material.clone(),
                )
            });

        let deps = &[
            tab.untyped(),
            stroke.untyped(),
            fill.untyped(),
            material.untyped(),
            expanded_node.computed_opacity.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();
        let last_render_signature = Rc::new(RefCell::new(None));
        let stroke_for_dirty = stroke.clone();
        let fill_for_dirty = fill.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let style_hash =
                        stroke_fill_style_hash(&stroke_for_dirty.get(), &fill_for_dirty.get());
                    mark_canvas_node_dirty_on_render_change(
                        &last_render_signature,
                        &cloned_expanded_node,
                        &cloned_context,
                        style_hash,
                    );
                },
                deps,
            ));
    }

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        let tab = expanded_node.transform_and_bounds.get();
        let (width, height) = tab.bounds;
        let rect = Rect::from_points((0.0, 0.0), (width, height));
        let ellipse = kurbo::Ellipse::from_rect(rect);
        Some(Affine::from(tab.transform) * ellipse.to_path(ELLIPSE_PATH_ACCURACY))
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            (properties.fill.get().coverage_alpha_0_1() * expanded_node.computed_opacity.get())
                .clamp(0.0, 1.0)
        })
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        if !rtc.is_canvas_node_dirty(&expanded_node.id) {
            return;
        }

        let Some(scope) = begin_bounded_canvas_node(rc, expanded_node, rtc) else {
            return;
        };

        let (width, height) = scope.bounds;
        expanded_node.with_properties_unwrapped(|properties: &mut Ellipse| {
            let rect = Rect::from_points((0.0, 0.0), (width, height));
            let ellipse = kurbo::Ellipse::from_rect(rect);
            let bez_path = ellipse.to_path(ELLIPSE_PATH_ACCURACY);
            let opacity = expanded_node.computed_opacity.get();
            let fill = properties.fill.get();
            let stroke = properties.stroke.get();
            let material = properties.material.get();
            rc.save(scope.layer_id);
            rc.transform(scope.layer_id, scope.surface_transform);
            rc.fill_with_material_and_opacity(
                scope.layer_id,
                bez_path.clone(),
                &fill,
                &material,
                opacity,
            );

            //hack to address "phantom stroke" bug on Web
            let width: f64 = stroke.width.get().expect_pixels().to_float();

            if width > f64::EPSILON {
                rc.stroke_with_material_and_opacity(
                    scope.layer_id,
                    bez_path,
                    &stroke,
                    &material,
                    opacity,
                );
            }
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

fn stroke_fill_style_hash(stroke: &Stroke, fill: &Fill) -> u64 {
    let mut hasher = DefaultHasher::new();
    stroke.color.get().hash(&mut hasher);
    stroke.width.get().hash(&mut hasher);
    stroke.cap.get().hash(&mut hasher);
    fill.hash(&mut hasher);
    hasher.finish()
}
