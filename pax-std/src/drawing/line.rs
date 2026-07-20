use kurbo::{Affine, BezPath, Shape};
use pax_engine::api::Size;
use pax_engine::*;
use pax_runtime::api::drawing::stroke_utils::{stroke_width_pixels, stroked_outline_path};
use pax_runtime::api::{use_RefCell, Layer, Material, RenderContext, Stroke};
use pax_runtime::BaseInstance;
use pax_runtime::{ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};

use crate::common::{begin_bounded_canvas_node, to_kurbo_point};

use_RefCell!();
use std::rc::Rc;

/// A 2D vector line segment.
///
/// `x1`/`y1` and `x2`/`y2` describe the segment endpoints in the primitive's
/// local coordinate space. The segment is rendered with `stroke`, whose cap
/// style controls how the two exposed endpoints terminate.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::line::LineInstance")]
pub struct Line {
    /// The x-coordinate of the start point.
    pub x1: Property<Size>,
    /// The y-coordinate of the start point.
    pub y1: Property<Size>,
    /// The x-coordinate of the end point.
    pub x2: Property<Size>,
    /// The y-coordinate of the end point.
    pub y2: Property<Size>,
    /// The stroke used to render the segment.
    pub stroke: Property<Stroke>,
    /// Light-reactive surface response.
    pub material: Property<Material>,
}

// Runtime instance backing `<Line>`.
pub struct LineInstance {
    base: BaseInstance,
}

impl InstanceNode for LineInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
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
        let (x1, y1, x2, y2, stroke, material) =
            expanded_node.with_properties_unwrapped(|properties: &mut Line| {
                (
                    properties.x1.clone(),
                    properties.y1.clone(),
                    properties.x2.clone(),
                    properties.y2.clone(),
                    properties.stroke.clone(),
                    properties.material.clone(),
                )
            });

        let deps = &[
            tab.untyped(),
            x1.untyped(),
            y1.untyped(),
            x2.untyped(),
            y2.untyped(),
            stroke.untyped(),
            material.untyped(),
            expanded_node.computed_opacity.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    cloned_context.mark_canvas_node_dirty(cloned_expanded_node.id);
                    cloned_context
                        .set_canvas_dirty(cloned_expanded_node.occlusion.get().render_layer_id)
                },
                deps,
            ));
    }

    fn ray_cast_test(
        &self,
        expanded_node: &ExpandedNode,
        ray: pax_runtime::api::math::Point2<pax_runtime::api::Window>,
    ) -> bool {
        self.resolve_coverage_path(expanded_node)
            .is_some_and(|path| {
                path_contains_with_tolerance(&path, kurbo::Point::new(ray.x, ray.y))
            })
    }

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<BezPath> {
        expanded_node.with_properties_unwrapped(|properties: &mut Line| {
            let tab = expanded_node.transform_and_bounds.get();
            let (start, end) = resolve_points(properties, tab.bounds);
            let stroke = properties.stroke.get();
            line_coverage_path(start, end, &stroke).map(|path| Affine::from(tab.transform) * path)
        })
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Line| {
            let stroke = properties.stroke.get();
            (stroke.color.get().alpha_0_1() * expanded_node.computed_opacity.get()).clamp(0.0, 1.0)
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

        expanded_node.with_properties_unwrapped(|properties: &mut Line| {
            let (start, end) = resolve_points(properties, scope.bounds);
            let stroke = properties.stroke.get();
            let material = properties.material.get();
            let stroke_width = stroke_width_pixels(&stroke);
            if stroke_width > f64::EPSILON {
                let bez_path = centerline_path(start, end);
                rc.save(scope.layer_id);
                rc.transform(scope.layer_id, scope.surface_transform);
                rc.stroke_with_material_and_opacity(
                    scope.layer_id,
                    bez_path,
                    &stroke,
                    &material,
                    expanded_node.computed_opacity.get(),
                );
                rc.restore(scope.layer_id);
            }
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
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|line: &mut Line| {
                f.debug_struct("Line")
                    .field("x1", &line.x1.get())
                    .field("y1", &line.y1.get())
                    .field("x2", &line.x2.get())
                    .field("y2", &line.y2.get())
                    .finish()
            }),
            None => f.debug_struct("Line").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

fn resolve_points(line: &Line, bounds: (f64, f64)) -> (kurbo::Point, kurbo::Point) {
    (
        to_kurbo_point(line.x1.get(), line.y1.get(), bounds),
        to_kurbo_point(line.x2.get(), line.y2.get(), bounds),
    )
}

fn centerline_path(start: kurbo::Point, end: kurbo::Point) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(start);
    path.line_to(end);
    path
}

fn line_coverage_path(start: kurbo::Point, end: kurbo::Point, stroke: &Stroke) -> Option<BezPath> {
    stroked_outline_path(&centerline_path(start, end), stroke)
}

fn path_contains_with_tolerance(path: &BezPath, point: kurbo::Point) -> bool {
    if path.contains(point) {
        return true;
    }

    const HIT_TEST_EPSILON: f64 = 1e-3;
    [
        (HIT_TEST_EPSILON, 0.0),
        (-HIT_TEST_EPSILON, 0.0),
        (0.0, HIT_TEST_EPSILON),
        (0.0, -HIT_TEST_EPSILON),
    ]
    .into_iter()
    .any(|(dx, dy)| path.contains(kurbo::Point::new(point.x + dx, point.y + dy)))
}

#[cfg(test)]
mod tests {
    use super::{line_coverage_path, path_contains_with_tolerance};
    use kurbo::{Point, Shape};
    use pax_runtime::api::{Property, Size, Stroke, StrokeCap, StrokeJoin};

    fn test_stroke(width: f64, cap: StrokeCap) -> Stroke {
        Stroke {
            color: Property::new(pax_runtime::api::Color::BLACK),
            width: Property::new(Size::Pixels(width.into())),
            cap: Property::new(cap),
            join: Property::new(StrokeJoin::default()),
        }
    }

    #[test]
    fn horizontal_line_coverage_has_area() {
        let path = line_coverage_path(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            &test_stroke(2.0, StrokeCap::Butt),
        )
        .unwrap();
        assert!(!path.bounding_box().is_zero_area());
    }

    #[test]
    fn zero_length_line_coverage_has_area() {
        let path = line_coverage_path(
            Point::new(4.0, 8.0),
            Point::new(4.0, 8.0),
            &test_stroke(6.0, StrokeCap::Round),
        )
        .unwrap();
        assert!(!path.bounding_box().is_zero_area());
    }

    #[test]
    fn line_coverage_contains_points_inside_stroke() {
        let path = line_coverage_path(
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            &test_stroke(6.0, StrokeCap::Butt),
        )
        .unwrap();
        assert!(path.contains(Point::new(10.0, 1.0)));
        assert!(!path.contains(Point::new(10.0, 4.0)));
    }

    #[test]
    fn round_caps_extend_beyond_segment_endpoints() {
        let path = line_coverage_path(
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            &test_stroke(6.0, StrokeCap::Round),
        )
        .unwrap();
        assert!(path_contains_with_tolerance(&path, Point::new(-2.0, 0.0)));
        assert!(path_contains_with_tolerance(&path, Point::new(22.0, 0.0)));
    }
}
