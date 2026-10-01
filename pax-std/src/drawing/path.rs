use kurbo::{Affine, BezPath, Rect, Shape};
use pax_engine::api::LocalProperty;

use pax_engine::api::PathElement;
use pax_runtime::api as pax_runtime_api;
use pax_runtime::api::drawing::path_smoothing::smooth_bez_path;
use pax_runtime::api::{borrow, borrow_mut, use_RefCell};
use pax_runtime::api::{Layer, Numeric, PathSmoothing, RenderContext, Stroke, UnitValue};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use crate::common::{begin_bounded_canvas_node, to_kurbo_point};
use pax_engine::*;

use_RefCell!();
use std::iter;
use std::rc::Rc;

#[cfg(test)]
use pax_runtime::api::drawing::path_trim::trim_bez_path as trim_bez_path_to_unit_range;

/// A 2D vector path for arbitrary Bézier and line-segment chains.
///
/// `elements` describes the path in local coordinates. `fill` paints the
/// interior of closed contours, while `stroke` paints the path itself; for
/// open subpaths, the stroke cap controls the exposed endpoints. Path geometry
/// may draw outside the element's layout bounds; use a `Frame` or `Mask` when
/// that overflow should be clipped.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::path::PathInstance")]
pub struct Path {
    /// The path commands and control points, expressed in local coordinates.
    pub elements: Property<Vec<PathElement>>,
    /// The stroke applied along the path centerline.
    pub stroke: Property<Vec<Stroke>>,
    /// The fill applied to the interior of closed contours.
    pub fill: Property<Vec<pax_runtime_api::Fill>>,
    /// Optional curve smoothing applied before rendering path geometry.
    pub smoothing: Property<PathSmoothing>,
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
            fill: Property::new(vec![pax_runtime_api::Fill::default()]),
            smoothing: Default::default(),
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
    fn supports_alpha_source_render(&self) -> bool {
        true
    }

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
        let env = Rc::clone(&expanded_node.stack);
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            expanded_node
                .get_node_context(context)
                .provide_store(PathContext {
                    elements: properties.elements.clone(),
                })
                .expect("Path is mounting");
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
        let (elements, stroke, fill, smoothing, draw_start, draw_end) = expanded_node
            .with_properties_unwrapped(|properties: &mut Path| {
                (
                    properties.elements.local(),
                    properties.stroke.local(),
                    properties.fill.local(),
                    properties.smoothing.local(),
                    properties.draw_start.local(),
                    properties.draw_end.local(),
                )
            });

        let appearance =
            crate::common::watch_appearance(expanded_node, context, fill.clone(), stroke.clone());
        let deps = &[
            appearance.untyped(),
            tab.untyped(),
            elements.untyped(),
            stroke.untyped(),
            fill.untyped(),
            smoothing.untyped(),
            draw_start.untyped(),
            draw_end.untyped(),
            expanded_node.computed_opacity.untyped(),
            expanded_node.computed_opacity_scopes.untyped(),
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        expanded_node
            .changed_listener
            .replace_with(LocalProperty::computed(
                move || {
                    appearance.get();
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
            let elements = properties.elements.local().get();
            let local_path = smooth_bez_path(
                &build_local_bez_path(&elements, bounds)?,
                properties.smoothing.local().get(),
            );
            let strokes = if properties.draw_start.local().get().to_clamped_unit_float()
                < properties.draw_end.local().get().to_clamped_unit_float()
            {
                properties.stroke.local().get()
            } else {
                Vec::new()
            };
            let coverage = crate::common::appearance_coverage_path(
                &local_path,
                &properties.fill.local().get(),
                &strokes,
            );

            if coverage.elements().is_empty() {
                return None;
            }
            let tab = expanded_node.transform_and_bounds.get();
            Some(Affine::from(tab.transform) * coverage)
        })
    }

    fn resolve_alpha_mask_paints(
        &self,
        node: &ExpandedNode,
    ) -> Vec<pax_runtime_api::AlphaMaskPaint> {
        node.with_properties_unwrapped(|p: &mut Path| {
            let Some(path) = build_local_bez_path(
                &p.elements.local().get(),
                node.transform_and_bounds.get().bounds,
            ) else {
                return Vec::new();
            };
            let path = smooth_bez_path(&path, p.smoothing.local().get());
            crate::common::alpha_mask_paints_with_range(
                node,
                path,
                p.fill.local().get(),
                p.stroke.local().get(),
                p.draw_start.local().get().to_clamped_unit_float(),
                p.draw_end.local().get().to_clamped_unit_float(),
            )
        })
    }

    fn resolve_occlusion_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let bounds = expanded_node.transform_and_bounds.get().bounds;
            let elements = properties.elements.local().get();
            let local_path = smooth_bez_path(
                &build_local_bez_path(&elements, bounds)?,
                properties.smoothing.local().get(),
            );
            let coverage = crate::common::appearance_coverage_path(
                &local_path,
                &properties.fill.local().get(),
                &properties.stroke.local().get(),
            );

            if coverage.elements().is_empty() {
                return None;
            }
            let tab = expanded_node.transform_and_bounds.get();
            Some(Affine::from(tab.transform) * coverage)
        })
    }

    fn resolve_coverage_opacity(&self, expanded_node: &ExpandedNode) -> f64 {
        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            (crate::common::appearance_coverage_alpha(
                &properties.fill.local().get(),
                &properties.stroke.local().get(),
            ) * expanded_node.computed_opacity.get())
            .clamp(0.0, 1.0)
        })
    }

    fn property_requires_occlusion_recompute(&self, property_name: &str) -> bool {
        !matches!(property_name, "draw_start" | "draw_end" | "material")
    }

    fn prepare_canvas_geometry(
        &self,
        expanded_node: &ExpandedNode,
    ) -> pax_runtime::scene_geometry::CanvasGeometry {
        let bounds = expanded_node.transform_and_bounds.get().bounds;
        let layout_bounds = Rect::new(0.0, 0.0, bounds.0, bounds.1);
        let (bez_path, local_coverage_bounds) =
            expanded_node.with_properties_unwrapped(|properties: &mut Path| {
                let elements = properties.elements.local().get();
                let bez_path = build_local_bez_path(&elements, bounds);
                let local_coverage_bounds = bez_path
                    .as_ref()
                    .filter(|path| !path.elements().is_empty())
                    .map(|path| {
                        path_local_coverage_bounds(
                            path,
                            properties.smoothing.local().get(),
                            properties
                                .stroke
                                .get()
                                .iter()
                                .map(Stroke::width_pixels)
                                .fold(0.0, f64::max),
                        )
                    })
                    .unwrap_or(layout_bounds);
                (bez_path, local_coverage_bounds)
            });

        pax_runtime::scene_geometry::CanvasGeometry {
            local_bounds: Some(local_coverage_bounds),
            path: bez_path.map(Rc::new),
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

        let geometry = rtc.canvas_geometry_for_node(expanded_node);
        let bez_path = geometry.local.path.as_deref();
        let Some(scope) = begin_bounded_canvas_node(rc, expanded_node, rtc) else {
            return;
        };

        if let Some(bez_path) = bez_path {
            expanded_node.with_properties_unwrapped(|properties: &mut Path| {
                rc.save(scope.layer_id);
                rc.transform(scope.layer_id, scope.surface_transform);
                crate::common::draw_appearance(
                    rc,
                    scope.layer_id,
                    bez_path.clone(),
                    &properties.fill.local().get(),
                    &properties.stroke.local().get(),
                    scope.paint_opacity,
                    properties.smoothing.local().get(),
                    properties.draw_start.local().get().to_clamped_unit_float(),
                    properties.draw_end.local().get().to_clamped_unit_float(),
                );
                rc.restore(scope.layer_id);
            });
        }
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

fn path_local_coverage_bounds(path: &BezPath, smoothing: PathSmoothing, stroke_width: f64) -> Rect {
    let path = smooth_bez_path(path, smoothing);
    let stroke_padding = stroke_width.max(0.0);
    path.bounding_box().inflate(stroke_padding, stroke_padding)
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

#[cfg(test)]
fn trim_bez_path(path: &BezPath, draw_start: UnitValue, draw_end: UnitValue) -> BezPath {
    trim_bez_path_to_unit_range(
        path,
        draw_start.to_clamped_unit_float(),
        draw_end.to_clamped_unit_float(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{PathEl, PathSeg};

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
    fn local_coverage_bounds_include_geometry_outside_layout_bounds() {
        let mut path = BezPath::new();
        path.move_to((-25.0, -10.0));
        path.line_to((125.0, 110.0));

        let bounds = path_local_coverage_bounds(&path, PathSmoothing::None, 4.0);

        assert_eq!(bounds, Rect::new(-29.0, -14.0, 129.0, 114.0));
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.local();
        let y = self.y.local();
        let id = ctx.slot_index.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self._on_change.replace_with(LocalProperty::computed(
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.untyped()];
        self._on_change.replace_with(LocalProperty::computed(
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.untyped()];
        self._on_change.replace_with(LocalProperty::computed(
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.local();
        let y = self.y.local();
        let id = ctx.slot_index.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self._on_change.replace_with(LocalProperty::computed(
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
            .with_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
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
