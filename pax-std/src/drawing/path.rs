use kurbo::{Affine, BezPath, CubicBez, Line, ParamCurve, ParamCurveArclen, PathSeg, QuadBez};

use pax_engine::api::{Fill, PathElement};
use pax_runtime::api::drawing::stroke_utils::{stroke_width_pixels, stroked_outline_path};
use pax_runtime::api::{borrow, borrow_mut, use_RefCell};
use pax_runtime::api::{Layer, Material, Numeric, RenderContext, Stroke, UnitValue};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use crate::common::{begin_bounded_canvas_node, to_kurbo_point};
use pax_engine::*;

use_RefCell!();
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

/// A 2D vector path for arbitrary Bézier and line-segment chains.
///
/// `elements` describes the path in local coordinates. `fill` paints the
/// interior of closed contours, while `stroke` paints the path itself; for
/// open subpaths, the stroke cap controls the exposed endpoints.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::path::PathInstance")]
pub struct Path {
    /// The path commands and control points, expressed in local coordinates.
    pub elements: Property<Vec<PathElement>>,
    /// The stroke applied along the path centerline.
    pub stroke: Property<Stroke>,
    /// The fill applied to the interior of closed contours.
    pub fill: Property<Fill>,
    /// Light-reactive surface response.
    pub material: Property<Material>,
    /// Start position of the visible stroke range over the path's total length.
    pub draw_start: Property<UnitValue>,
    /// End position of the visible stroke range over the path's total length.
    pub draw_end: Property<UnitValue>,
}

impl Default for Path {
    fn default() -> Self {
        Self {
            elements: Default::default(),
            stroke: Default::default(),
            fill: Default::default(),
            material: Default::default(),
            draw_start: Property::new(UnitValue::Unitless(Numeric::F64(0.0))),
            draw_end: Property::new(UnitValue::Unitless(Numeric::F64(1.0))),
        }
    }
}

impl Path {
    /// Starts a new path at the provided point.
    pub fn start(x: Size, y: Size) -> Vec<PathElement> {
        let mut start: Vec<PathElement> = Vec::new();
        start.push(PathElement::Point(x, y));
        start
    }

    /// Appends a straight line segment to the provided point.
    pub fn line_to(mut path: Vec<PathElement>, x: Size, y: Size) -> Vec<PathElement> {
        path.push(PathElement::Line);
        path.push(PathElement::Point(x, y));
        path
    }

    /// Appends a quadratic Bézier curve with one control point.
    pub fn curve_to(
        mut path: Vec<PathElement>,
        h_x: Size,
        h_y: Size,
        x: Size,
        y: Size,
    ) -> Vec<PathElement> {
        path.push(PathElement::Quadratic(h_x, h_y));
        path.push(PathElement::Point(x, y));
        path
    }
}

// Runtime instance backing `<Path>`.
pub struct PathInstance {
    base: BaseInstance,
}

impl InstanceNode for PathInstance {
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
        // create a new stack to be able to insert a local store specific for this node and the
        // ones bellow. If not done, things above this node could potentially access it
        let env = expanded_node.stack.push(HashMap::new());
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            env.insert_stack_local_store(PathContext {
                elements: properties.elements.clone(),
            });
            let children = borrow!(self.base().get_instance_children());
            if !children.is_empty() {
                let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
                let new_children = expanded_node.generate_children(
                    children_with_envs,
                    context,
                    &expanded_node.parent_frame,
                    true,
                );
                // Path child nodes write PathElement values into the local PathContext store.
                *borrow_mut!(expanded_node.expanded_projected_children) =
                    Some(new_children.clone());
                expanded_node.children.set(new_children);
            }
        });

        let tab = expanded_node.transform_and_bounds.clone();
        let (elements, stroke, fill, material, draw_start, draw_end) = expanded_node
            .with_properties_unwrapped(|properties: &mut Path| {
                (
                    properties.elements.clone(),
                    properties.stroke.clone(),
                    properties.fill.clone(),
                    properties.material.clone(),
                    properties.draw_start.clone(),
                    properties.draw_end.clone(),
                )
            });

        let deps = &[
            tab.untyped(),
            elements.untyped(),
            stroke.untyped(),
            fill.untyped(),
            material.untyped(),
            draw_start.untyped(),
            draw_end.untyped(),
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

    fn requires_non_reactive_update(&self, expanded_node: &ExpandedNode) -> bool {
        borrow!(expanded_node.expanded_projected_children).is_some()
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        if borrow!(expanded_node.expanded_projected_children).is_some() {
            // Path child nodes can change the path command list, but plain property-backed paths
            // have no projected child structure to recompute on each tick.
            expanded_node.compute_flattened_projected_children();
        }
    }

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let bounds = expanded_node.transform_and_bounds.get().bounds;
            let elements = properties.elements.get();
            let local_path = build_local_bez_path(&elements, bounds)?;
            let fill = properties.fill.get();
            let stroke = properties.stroke.get();
            let draw_start = properties.draw_start.get();
            let draw_end = properties.draw_end.get();
            let mut coverage = BezPath::new();
            if fill.coverage_alpha_0_1() > f64::EPSILON {
                coverage.extend(local_path.elements().iter().copied());
            }
            if stroke.color.get().alpha_0_1() > f64::EPSILON {
                let trimmed_path = trim_bez_path(&local_path, draw_start, draw_end);
                if let Some(stroke_outline) = stroked_outline_path(&trimmed_path, &stroke) {
                    coverage.extend(stroke_outline.elements().iter().copied());
                }
            }

            if coverage.elements().is_empty() {
                return None;
            }
            let tab = expanded_node.transform_and_bounds.get();
            Some(Affine::from(tab.transform) * coverage)
        })
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let fill_alpha = properties.fill.get().coverage_alpha_0_1();
            let stroke_alpha = properties.stroke.get().color.get().alpha_0_1();
            (fill_alpha.max(stroke_alpha) * expanded_node.computed_opacity.get()).clamp(0.0, 1.0)
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

        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let bounds = scope.bounds;
            let elements = properties.elements.get();
            let Some(bez_path) = build_local_bez_path(&elements, bounds) else {
                return;
            };

            let mut clip_path = BezPath::new();
            let (width, height) = scope.bounds;
            clip_path.move_to((0.0, 0.0));
            clip_path.line_to((width, 0.0));
            clip_path.line_to((width, height));
            clip_path.line_to((0.0, height));
            clip_path.line_to((0.0, 0.0));
            clip_path.close_path();
            //our "save point" before clipping — restored to in the post_render
            let opacity = expanded_node.computed_opacity.get();
            let fill = properties.fill.get();
            let stroke = properties.stroke.get();
            let material = properties.material.get();
            let draw_start = properties.draw_start.get();
            let draw_end = properties.draw_end.get();
            rc.save(scope.layer_id);
            rc.transform(scope.layer_id, scope.surface_transform);
            rc.clip(scope.layer_id, clip_path.clone());
            rc.fill_with_material_and_opacity(
                scope.layer_id,
                bez_path.clone(),
                &fill,
                &material,
                opacity,
            );
            if stroke_width_pixels(&stroke) > f64::EPSILON {
                let trimmed_path = trim_bez_path(&bez_path, draw_start, draw_end);
                rc.stroke_with_material_and_opacity(
                    scope.layer_id,
                    trimmed_path,
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

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Path").finish()
    }
}

fn build_local_bez_path(elements: &[PathElement], bounds: (f64, f64)) -> Option<BezPath> {
    let mut bez_path = BezPath::new();
    let mut itr_elems = elements.iter();

    if let Some(elem) = itr_elems.next() {
        if let &PathElement::Point(x, y) = elem {
            bez_path.move_to(to_kurbo_point(x, y, bounds));
        } else {
            log::warn!("path must start with point");
            return None;
        }
    }

    while let Some(elem) = itr_elems.next() {
        match elem {
            &PathElement::Point(x, y) => {
                bez_path.move_to(to_kurbo_point(x, y, bounds));
            }
            &PathElement::Line => {
                let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                    log::warn!("line expects to be followed by a point");
                    return None;
                };
                bez_path.line_to(to_kurbo_point(x, y, bounds));
            }
            &PathElement::Quadratic(h_x, h_y) => {
                let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                    log::warn!("curve expects to be followed by a point");
                    return None;
                };
                bez_path.quad_to(
                    to_kurbo_point(h_x, h_y, bounds),
                    to_kurbo_point(x, y, bounds),
                );
            }
            &PathElement::Cubic(v1, v2, v3, v4) => {
                let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                    log::warn!("curve expects to be followed by a point");
                    return None;
                };
                bez_path.curve_to(
                    to_kurbo_point(v1, v2, bounds),
                    to_kurbo_point(v3, v4, bounds),
                    to_kurbo_point(x, y, bounds),
                );
            }
            &PathElement::Close => {
                bez_path.close_path();
            }
            PathElement::Empty => (),
        }
    }

    Some(bez_path)
}

const PATH_TRIM_ACCURACY: f64 = 0.1;

#[derive(Clone, Copy)]
struct TrimSegment {
    segment: PathSeg,
    contour_index: usize,
    closes_contour: bool,
}

#[derive(Clone, Copy)]
struct TrimContour {
    first_segment_index: Option<usize>,
    last_segment_index: Option<usize>,
    is_closed: bool,
}

fn trim_bez_path(path: &BezPath, draw_start: UnitValue, draw_end: UnitValue) -> BezPath {
    let start = draw_start.to_clamped_unit_float();
    let end = draw_end.to_clamped_unit_float();
    if start <= f64::EPSILON && end >= 1.0 - f64::EPSILON {
        return path.clone();
    }
    if start >= end {
        return BezPath::new();
    }

    let (segments, contours) = collect_trim_segments(path);
    let lengths = segments
        .iter()
        .map(|record| record.segment.arclen(PATH_TRIM_ACCURACY).max(0.0))
        .collect::<Vec<_>>();
    let total_length = lengths.iter().sum::<f64>();
    if total_length <= f64::EPSILON {
        return BezPath::new();
    }

    let trim_start = start * total_length;
    let trim_end = end * total_length;
    let mut trimmed = BezPath::new();
    let mut current_point = None;
    let mut cursor = 0.0;
    let mut segment_starts = Vec::with_capacity(lengths.len());
    let mut segment_ends = Vec::with_capacity(lengths.len());

    for segment_length in &lengths {
        segment_starts.push(cursor);
        cursor += segment_length;
        segment_ends.push(cursor);
    }

    let mut fully_visible_closed_contours = vec![false; contours.len()];
    for (contour_index, contour) in contours.iter().enumerate() {
        if !contour.is_closed {
            continue;
        }
        let (Some(first_segment_index), Some(last_segment_index)) =
            (contour.first_segment_index, contour.last_segment_index)
        else {
            continue;
        };
        let contour_start = segment_starts[first_segment_index];
        let contour_end = segment_ends[last_segment_index];
        fully_visible_closed_contours[contour_index] =
            trim_start <= contour_start + f64::EPSILON && trim_end >= contour_end - f64::EPSILON;
    }

    for (index, (record, segment_length)) in segments.into_iter().zip(lengths).enumerate() {
        let segment_start = segment_starts[index];
        let segment_end = segment_ends[index];
        if segment_length <= f64::EPSILON || segment_end <= trim_start + f64::EPSILON {
            continue;
        }
        if segment_start >= trim_end - f64::EPSILON {
            break;
        }

        let local_start = (trim_start - segment_start).clamp(0.0, segment_length);
        let local_end = (trim_end - segment_start).clamp(0.0, segment_length);
        if local_start >= local_end {
            continue;
        }

        let whole_segment_visible =
            local_start <= f64::EPSILON && local_end >= segment_length - f64::EPSILON;
        let visible_segment = if whole_segment_visible {
            record.segment
        } else {
            let t0 = if local_start <= f64::EPSILON {
                0.0
            } else {
                record.segment.inv_arclen(local_start, PATH_TRIM_ACCURACY)
            };
            let t1 = if local_end >= segment_length - f64::EPSILON {
                1.0
            } else {
                record.segment.inv_arclen(local_end, PATH_TRIM_ACCURACY)
            };
            if t0 >= t1 {
                continue;
            }
            record.segment.subsegment(t0..t1)
        };

        append_segment(&mut trimmed, &mut current_point, visible_segment);
        if whole_segment_visible
            && record.closes_contour
            && fully_visible_closed_contours[record.contour_index]
        {
            trimmed.close_path();
            current_point = None;
        }
    }

    trimmed
}

fn collect_trim_segments(path: &BezPath) -> (Vec<TrimSegment>, Vec<TrimContour>) {
    let mut segments = Vec::new();
    let mut contours = Vec::new();
    let mut current_contour = None;
    let mut contour_start = None;
    let mut current_point = None;

    for element in path.iter() {
        match element {
            kurbo::PathEl::MoveTo(point) => {
                current_contour = Some(contours.len());
                contours.push(TrimContour {
                    first_segment_index: None,
                    last_segment_index: None,
                    is_closed: false,
                });
                contour_start = Some(point);
                current_point = Some(point);
            }
            kurbo::PathEl::LineTo(point) => {
                let (Some(from), Some(contour_index)) = (current_point, current_contour) else {
                    continue;
                };
                push_trim_segment(
                    &mut segments,
                    &mut contours,
                    contour_index,
                    PathSeg::Line(Line::new(from, point)),
                    false,
                );
                current_point = Some(point);
            }
            kurbo::PathEl::QuadTo(control, point) => {
                let (Some(from), Some(contour_index)) = (current_point, current_contour) else {
                    continue;
                };
                push_trim_segment(
                    &mut segments,
                    &mut contours,
                    contour_index,
                    PathSeg::Quad(QuadBez::new(from, control, point)),
                    false,
                );
                current_point = Some(point);
            }
            kurbo::PathEl::CurveTo(control_1, control_2, point) => {
                let (Some(from), Some(contour_index)) = (current_point, current_contour) else {
                    continue;
                };
                push_trim_segment(
                    &mut segments,
                    &mut contours,
                    contour_index,
                    PathSeg::Cubic(CubicBez::new(from, control_1, control_2, point)),
                    false,
                );
                current_point = Some(point);
            }
            kurbo::PathEl::ClosePath => {
                let (Some(from), Some(start), Some(contour_index)) =
                    (current_point, contour_start, current_contour)
                else {
                    continue;
                };
                contours[contour_index].is_closed = true;
                if same_kurbo_point(from, start) {
                    if let Some(last_segment_index) = contours[contour_index].last_segment_index {
                        segments[last_segment_index].closes_contour = true;
                    }
                } else {
                    push_trim_segment(
                        &mut segments,
                        &mut contours,
                        contour_index,
                        PathSeg::Line(Line::new(from, start)),
                        true,
                    );
                    current_point = Some(start);
                }
            }
        }
    }

    (segments, contours)
}

fn push_trim_segment(
    segments: &mut Vec<TrimSegment>,
    contours: &mut [TrimContour],
    contour_index: usize,
    segment: PathSeg,
    closes_contour: bool,
) {
    let segment_index = segments.len();
    let contour = &mut contours[contour_index];
    if contour.first_segment_index.is_none() {
        contour.first_segment_index = Some(segment_index);
    }
    contour.last_segment_index = Some(segment_index);
    segments.push(TrimSegment {
        segment,
        contour_index,
        closes_contour,
    });
}

fn append_segment(path: &mut BezPath, current_point: &mut Option<kurbo::Point>, segment: PathSeg) {
    let start = segment.start();
    if current_point
        .map(|point| !same_kurbo_point(point, start))
        .unwrap_or(true)
    {
        path.move_to(start);
    }
    path.push(segment.as_path_el());
    *current_point = Some(segment.end());
}

fn same_kurbo_point(a: kurbo::Point, b: kurbo::Point) -> bool {
    (a.x - b.x).abs() <= f64::EPSILON && (a.y - b.y).abs() <= f64::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::PathEl;

    fn unit(value: f64) -> UnitValue {
        UnitValue::Unitless(Numeric::F64(value))
    }

    fn point_from_move(element: PathEl) -> kurbo::Point {
        match element {
            PathEl::MoveTo(point) => point,
            _ => panic!("expected MoveTo, got {element:?}"),
        }
    }

    fn point_from_line(element: PathEl) -> kurbo::Point {
        match element {
            PathEl::LineTo(point) => point,
            _ => panic!("expected LineTo, got {element:?}"),
        }
    }

    fn assert_point_near(actual: kurbo::Point, expected: kurbo::Point) {
        assert!(
            (actual.x - expected.x).abs() < 1e-6 && (actual.y - expected.y).abs() < 1e-6,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn trim_empty_path_returns_empty_path() {
        let path = BezPath::new();
        let trimmed = trim_bez_path(&path, unit(0.0), unit(1.0));

        assert!(trimmed.elements().is_empty());
    }

    #[test]
    fn trim_line_partial_range() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((100.0, 0.0));

        let trimmed = trim_bez_path(&path, unit(0.25), unit(0.75));
        let elements = trimmed.elements();

        assert_eq!(elements.len(), 2);
        assert_point_near(point_from_move(elements[0]), kurbo::Point::new(25.0, 0.0));
        assert_point_near(point_from_line(elements[1]), kurbo::Point::new(75.0, 0.0));
    }

    #[test]
    fn trim_crosses_line_boundaries_without_internal_moves() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((50.0, 0.0));
        path.line_to((100.0, 0.0));

        let trimmed = trim_bez_path(&path, unit(0.25), unit(0.75));
        let elements = trimmed.elements();

        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0], PathEl::MoveTo(_)));
        assert!(matches!(elements[1], PathEl::LineTo(_)));
        assert!(matches!(elements[2], PathEl::LineTo(_)));
        assert_point_near(point_from_move(elements[0]), kurbo::Point::new(25.0, 0.0));
        assert_point_near(point_from_line(elements[1]), kurbo::Point::new(50.0, 0.0));
        assert_point_near(point_from_line(elements[2]), kurbo::Point::new(75.0, 0.0));
    }

    #[test]
    fn trim_quadratic_and_cubic_segments() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.quad_to((50.0, 100.0), (100.0, 0.0));
        path.curve_to((125.0, -50.0), (175.0, 50.0), (200.0, 0.0));

        let trimmed = trim_bez_path(&path, unit(0.1), unit(0.9));

        assert!(trimmed.elements().len() >= 3);
        assert!(matches!(trimmed.elements()[0], PathEl::MoveTo(_)));
        assert!(
            trimmed
                .segments()
                .any(|segment| matches!(segment, PathSeg::Quad(_) | PathSeg::Cubic(_))),
            "expected at least one curved segment after trimming"
        );
    }

    #[test]
    fn close_path_contributes_to_trimmed_length() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((100.0, 0.0));
        path.close_path();

        let trimmed = trim_bez_path(&path, unit(0.75), unit(1.0));
        let elements = trimmed.elements();

        assert_eq!(elements.len(), 2);
        assert_point_near(point_from_move(elements[0]), kurbo::Point::new(50.0, 0.0));
        assert_point_near(point_from_line(elements[1]), kurbo::Point::new(0.0, 0.0));
    }

    #[test]
    fn trim_closes_completed_contours_before_path_end() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((10.0, 0.0));
        path.line_to((10.0, 10.0));
        path.line_to((0.0, 10.0));
        path.close_path();
        path.move_to((100.0, 0.0));
        path.line_to((200.0, 0.0));

        let trimmed = trim_bez_path(&path, unit(0.0), unit(0.5));
        let elements = trimmed.elements();
        let close_index = elements
            .iter()
            .position(|element| matches!(element, PathEl::ClosePath))
            .expect("expected completed first contour to be closed");

        assert!(
            elements
                .iter()
                .skip(close_index + 1)
                .any(|element| matches!(element, PathEl::MoveTo(_))),
            "expected later partial contour after completed closed contour"
        );
    }

    #[test]
    fn trim_closes_completed_contours_with_explicit_return_to_start() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((10.0, 0.0));
        path.line_to((0.0, 0.0));
        path.close_path();
        path.move_to((100.0, 0.0));
        path.line_to((200.0, 0.0));

        let trimmed = trim_bez_path(&path, unit(0.0), unit(0.3));
        let elements = trimmed.elements();

        assert!(
            elements
                .iter()
                .any(|element| matches!(element, PathEl::ClosePath)),
            "expected zero-length ClosePath to be preserved for the completed contour"
        );
    }

    #[test]
    fn trim_clamps_and_empty_when_start_reaches_end() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((100.0, 0.0));

        assert_eq!(
            trim_bez_path(&path, unit(-1.0), unit(2.0)).elements(),
            path.elements()
        );
        assert!(trim_bez_path(&path, unit(0.5), unit(0.5))
            .elements()
            .is_empty());
        assert!(trim_bez_path(&path, unit(0.75), unit(0.25))
            .elements()
            .is_empty());
    }
}

use pax_engine::{
    api::{NodeContext, Size, Store},
    pax, Property,
};

// Local store shared by `Path` and its path-builder children.
pub struct PathContext {
    // Shared path element list mutated by path-builder child components.
    pub elements: Property<Vec<PathElement>>,
}

impl Store for PathContext {}

/// Path child component that inserts a `PathElement::Point`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathPoint {
    /// Point x-coordinate.
    pub x: Property<Size>,
    /// Point y-coordinate.
    pub y: Property<Size>,
    // Private reactive hook that writes this child into the parent path.
    pub _on_change: Property<bool>,
}

impl PathPoint {
    // Registers this child as a point command in the parent `Path`.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.clone();
        let y = self.y.clone();
        let id = ctx.slot_index.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self._on_change.replace_with(Property::computed(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::Point(x.get(), y.get());
                });
                false
            },
            &deps,
        ));
    }

    // Removes this child command from the parent `Path`.
    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.get().unwrap();
        path_elems.update(|elems| {
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }

    // Forces the path-command computed property to run.
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self._on_change.get();
    }
}

/// Path child component that inserts a straight line segment.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathLine {
    // Private reactive hook that writes this child into the parent path.
    pub _on_change: Property<bool>,
}

impl PathLine {
    // Registers this child as a line command in the parent `Path`.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.untyped()];
        self._on_change.replace_with(Property::computed(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::Line;
                });
                false
            },
            &deps,
        ));
    }

    // Removes this child command from the parent `Path`.
    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.get().unwrap();
        path_elems.update(|elems| {
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }
    // Forces the path-command computed property to run.
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self._on_change.get();
    }
}

/// Path child component that closes the current contour.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathClose {
    // Private reactive hook that writes this child into the parent path.
    pub _on_change: Property<bool>,
}

impl PathClose {
    // Registers this child as a close-path command in the parent `Path`.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.untyped()];
        self._on_change.replace_with(Property::computed(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::Close;
                });
                false
            },
            &deps,
        ));
    }
    // Removes this child command from the parent `Path`.
    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.clone();
        path_elems.update(|elems| {
            let id = id.get().unwrap();
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }

    // Forces the path-command computed property to run.
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self._on_change.get();
    }
}

/// Path child component that inserts a quadratic curve control point.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathCurve {
    /// Control point x-coordinate.
    pub x: Property<Size>,
    /// Control point y-coordinate.
    pub y: Property<Size>,
    // Private reactive hook that writes this child into the parent path.
    pub _on_change: Property<bool>,
}

impl PathCurve {
    // Registers this child as a quadratic curve command in the parent `Path`.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.clone();
        let y = self.y.clone();
        let id = ctx.slot_index.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self._on_change.replace_with(Property::computed(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::Quadratic(x.get(), y.get());
                });
                false
            },
            &deps,
        ));
    }

    // Removes this child command from the parent `Path`.
    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.get().unwrap();
        path_elems.update(|elems| {
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }

    // Forces the path-command computed property to run.
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self._on_change.get();
    }
}
