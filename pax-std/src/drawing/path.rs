use kurbo::{Affine, BezPath};

use pax_engine::api::{Fill, PathElement};
use pax_runtime::api::drawing::stroke_utils::{stroke_width_pixels, stroked_outline_path};
use pax_runtime::api::{borrow, borrow_mut, use_RefCell};
use pax_runtime::api::{Layer, RenderContext, Stroke};
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
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::path::PathInstance")]
pub struct Path {
    /// The path commands and control points, expressed in local coordinates.
    pub elements: Property<Vec<PathElement>>,
    /// The stroke applied along the path centerline.
    pub stroke: Property<Stroke>,
    /// The fill applied to the interior of closed contours.
    pub fill: Property<Fill>,
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
        let (elements, stroke, fill) =
            expanded_node.with_properties_unwrapped(|properties: &mut Path| {
                (
                    properties.elements.clone(),
                    properties.stroke.clone(),
                    properties.fill.clone(),
                )
            });

        let deps = &[
            tab.untyped(),
            elements.untyped(),
            stroke.untyped(),
            fill.untyped(),
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
            let mut coverage = BezPath::new();
            if fill.coverage_alpha_0_1() > f64::EPSILON {
                coverage.extend(local_path.elements().iter().copied());
            }
            if stroke.color.get().alpha_0_1() > f64::EPSILON {
                if let Some(stroke_outline) = stroked_outline_path(&local_path, &stroke) {
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
            rc.save(scope.layer_id);
            rc.transform(scope.layer_id, scope.surface_transform);
            rc.clip(scope.layer_id, clip_path.clone());
            rc.fill_with_opacity(scope.layer_id, bez_path.clone(), &fill, opacity);
            if stroke_width_pixels(&stroke) > f64::EPSILON {
                rc.stroke_with_opacity(scope.layer_id, bez_path, &stroke, opacity);
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
