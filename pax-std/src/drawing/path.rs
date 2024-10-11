use kurbo::BezPath;

use pax_engine::api::{Fill, PathElement};
use pax_runtime::api::{borrow, borrow_mut, use_RefCell};
use pax_runtime::api::{Color, Layer, RenderContext, Stroke};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use piet::{LinearGradient, RadialGradient};

use crate::common::Point;
use pax_engine::*;

use_RefCell!();
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

/// A basic 2D vector path for arbitrary Bézier / line-segment chains
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::path::PathInstance")]
pub struct Path {
    pub elements: Property<Vec<PathElement>>,
    pub stroke: Property<Stroke>,
    pub fill: Property<Fill>,
}

impl Path {
    pub fn start(x: Size, y: Size) -> Vec<PathElement> {
        let mut start: Vec<PathElement> = Vec::new();
        start.push(PathElement::Point(x, y));
        start
    }
    pub fn line_to(mut path: Vec<PathElement>, x: Size, y: Size) -> Vec<PathElement> {
        path.push(PathElement::Line);
        path.push(PathElement::Point(x, y));
        path
    }

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

pub struct PathInstance {
    base: BaseInstance,
    // This property is used to dirty the canvas when the path changes
    changed: Property<()>,
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
                    invisible_to_raycasting: true,
                    layer: Layer::Canvas,
                    is_component: false,
                    is_slot: false,
                },
            ),
            changed: Property::new(()),
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
            let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
            let new_children = expanded_node.generate_children(
                children_with_envs,
                context,
                &expanded_node.parent_frame,
                true,
            );
            // set slot children to all to make children compute and update their slot index
            // (see expanded_node compute_expanded and flattened children)
            *borrow_mut!(expanded_node.expanded_slot_children) = Some(new_children.clone());
            expanded_node.children.set(new_children);
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
        ];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();

        self.changed.replace_with(Property::computed(
            move || {
                cloned_context
                    .set_canvas_dirty(cloned_expanded_node.occlusion.get().occlusion_layer_id);
                ()
            },
            deps,
        ));
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        // NOTE: do not update children here,
        // we know that all of the expanded and flattened children
        // are the same as the once being rendered
        expanded_node.compute_flattened_slot_children();
        self.changed.get();
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        let layer_id = expanded_node.occlusion.get().occlusion_layer_id;

        if !rtc.is_canvas_dirty(&layer_id) {
            return;
        }

        expanded_node.with_properties_unwrapped(|properties: &mut Path| {
            let bounds = expanded_node.transform_and_bounds.get().bounds;

            // TODO make this only recompute if path changed since last frame
            let mut bez_path = BezPath::new();
            properties.elements.read(|elems| {
                let mut itr_elems = elems.iter();

                if let Some(elem) = itr_elems.next() {
                    if let &PathElement::Point(x, y) = elem {
                        bez_path.move_to(Point { x, y }.to_kurbo_point(bounds));
                    } else {
                        log::warn!("path must start with point");
                        return;
                    }
                }

                while let Some(elem) = itr_elems.next() {
                    match elem {
                        &PathElement::Point(x, y) => {
                            bez_path.move_to(Point { x, y }.to_kurbo_point(bounds));
                        }
                        &PathElement::Line => {
                            let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                                log::warn!("line expects to be followed by a point");
                                return;
                            };
                            bez_path.line_to(Point { x, y }.to_kurbo_point(bounds));
                        }
                        &PathElement::Quadratic(h_x, h_y) => {
                            let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                                log::warn!("curve expects to be followed by a point");
                                return;
                            };
                            bez_path.quad_to(
                                Point { x: h_x, y: h_y }.to_kurbo_point(bounds),
                                Point { x, y }.to_kurbo_point(bounds),
                            );
                        }
                        PathElement::Cubic(vals) => {
                            let Some(&PathElement::Point(x, y)) = itr_elems.next() else {
                                log::warn!("curve expects to be followed by a point");
                                return;
                            };
                            bez_path.curve_to(
                                Point {
                                    x: vals.0,
                                    y: vals.1,
                                }
                                .to_kurbo_point(bounds),
                                Point {
                                    x: vals.2,
                                    y: vals.3,
                                }
                                .to_kurbo_point(bounds),
                                Point { x, y }.to_kurbo_point(bounds),
                            );
                        }
                        &PathElement::Close => {
                            bez_path.close_path();
                        }
                        PathElement::Empty => (), //no-op
                    }
                }
            });

            let tab = expanded_node.transform_and_bounds.get();
            let transform = Into::<kurbo::Affine>::into(tab.transform);
            let mut clip_path = BezPath::new();
            let (width, height) = tab.bounds;
            clip_path.move_to((0.0, 0.0));
            clip_path.line_to((width, 0.0));
            clip_path.line_to((width, height));
            clip_path.line_to((0.0, height));
            clip_path.line_to((0.0, 0.0));
            clip_path.close_path();
            let transformed_clip_path = transform * clip_path;
            let transformed_bez_path = transform * bez_path;
            let duplicate_transformed_bez_path = transformed_bez_path.clone();
            //our "save point" before clipping — restored to in the post_render

            rc.save(layer_id);
            match properties.fill.get() {
                Fill::Solid(color) => {
                    rc.fill(
                        layer_id,
                        transformed_bez_path,
                        &color.to_piet_color().into(),
                    );
                }
                Fill::LinearGradient(linear) => {
                    let linear_gradient = LinearGradient::new(
                        Fill::to_unit_point(linear.start, (width, height)),
                        Fill::to_unit_point(linear.end, (width, height)),
                        Fill::to_piet_gradient_stops(linear.stops.clone()),
                    );
                    rc.fill(layer_id, transformed_bez_path, &linear_gradient.into())
                }
                Fill::RadialGradient(radial) => {
                    let origin = Fill::to_unit_point(radial.start, (width, height));
                    let center = Fill::to_unit_point(radial.end, (width, height));
                    let gradient_stops = Fill::to_piet_gradient_stops(radial.stops.clone());
                    let radial_gradient = RadialGradient::new(radial.radius, gradient_stops)
                        .with_center(center)
                        .with_origin(origin);
                    rc.fill(layer_id, transformed_bez_path, &radial_gradient.into());
                }
            }
            rc.clip(layer_id, transformed_clip_path.clone());
            if properties
                .stroke
                .get()
                .width
                .get()
                .expect_pixels()
                .to_float()
                > f64::EPSILON
            {
                rc.stroke(
                    layer_id,
                    duplicate_transformed_bez_path,
                    &properties.stroke.get().color.get().to_piet_color().into(),
                    properties
                        .stroke
                        .get()
                        .width
                        .get()
                        .expect_pixels()
                        .to_float(),
                );
            }
            rc.restore(layer_id);
        });
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

use pax_engine::{
    api::{NodeContext, Size, Store},
    pax, Property,
};

pub struct PathContext {
    pub elements: Property<Vec<PathElement>>,
}

impl Store for PathContext {}

#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathPoint {
    pub x: Property<Size>,
    pub y: Property<Size>,
    pub on_change: Property<bool>,
}

impl PathPoint {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.clone();
        let y = self.y.clone();
        let id = ctx.slot_index.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self.on_change.replace_with(Property::computed(
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

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathLine {
    pub on_change: Property<bool>,
}

impl PathLine {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.untyped()];
        self.on_change.replace_with(Property::computed(
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
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathClose {
    pub on_change: Property<bool>,
}

impl PathClose {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.untyped()];
        self.on_change.replace_with(Property::computed(
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

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathCurve {
    pub x: Property<Size>,
    pub y: Property<Size>,
    pub on_change: Property<bool>,
}

impl PathCurve {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.clone();
        let y = self.y.clone();
        let id = ctx.slot_index.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self.on_change.replace_with(Property::computed(
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

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}
