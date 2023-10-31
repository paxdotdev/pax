use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Mul;
use std::rc::Rc;

use kurbo::{Affine, Point};
use pax_properties_coproduct::PropertiesCoproduct;
use pax_runtime_api::{Axis, CommonProperties, Transform2D};
use piet::{Color, StrokeStyle};
use piet_common::RenderContext;

use pax_runtime_api::{ArgsScroll, Layer, Size, PropertyInstance};

use crate::{HandlerRegistry, NodeRegistry, RenderTreeContext};
use crate::form_event::FormEvent;

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for rendernodes.
pub type InstanceNodePtr<R> = Rc<RefCell<dyn InstanceNode<R>>>;
pub type InstanceNodePtrList<R> = Rc<RefCell<Vec<InstanceNodePtr<R>>>>;

/// Given some InstanceNodePtrList, distill away all "slot-invisible" nodes (namely, `if` and `for`)
/// and return another InstanceNodePtrList with a flattened top-level list of nodes.
pub fn flatten_slot_invisible_nodes_recursive<R: 'static + RenderContext>(input_nodes: InstanceNodePtrList<R>) -> InstanceNodePtrList<R> {
    let mut output_nodes = Vec::new();

    for node in input_nodes.borrow().iter() {
        if node.borrow().is_invisible_to_slot() {
            let children = node.borrow().get_rendering_children();
            let flattened_children = flatten_slot_invisible_nodes_recursive(children);
            output_nodes.extend(flattened_children.borrow().iter().cloned());
        } else {
            output_nodes.push(node.clone());
        }
    }

    Rc::new(RefCell::new(output_nodes))
}

pub struct ScrollerArgs {
    pub size_inner_pane: [Box<dyn PropertyInstance<f64>>; 2],
    pub axes_enabled: [Box<dyn PropertyInstance<bool>>; 2],
}

pub struct InstantiationArgs<R: 'static + RenderContext> {
    pub common_properties: CommonProperties,
    pub properties: PropertiesCoproduct,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_registry: Rc<RefCell<NodeRegistry<R>>>,
    pub children: Option<InstanceNodePtrList<R>>,
    pub component_template: Option<InstanceNodePtrList<R>>,
    pub scroller_args: Option<ScrollerArgs>,



    ///used by Component instances, specifically to unwrap type-specific PropertiesCoproducts
    ///and recurse into descendant property computation
    pub compute_properties_fn:
        Option<Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>, &mut RenderTreeContext<R>)>>,
}

#[derive(Copy, Clone)]
pub struct Point2D {
    x: f64,
    y: f64,
}

impl Point2D {
    fn subtract(self, other: Point2D) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    fn dot(self, other: Point2D) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn normal(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    fn project_onto(self, axis: Point2D) -> f64 {
        let dot_product = self.dot(axis);
        dot_product / (axis.x.powi(2) + axis.y.powi(2))
    }
}

impl Mul<Point2D> for Affine {
    type Output = Point2D;

    #[inline]
    fn mul(self, other: Point2D) -> Point2D {
        let coeffs = self.as_coeffs();
        Point2D {
            x: coeffs[0] * other.x + coeffs[2] * other.y + coeffs[4],
            y: coeffs[1] * other.x + coeffs[3] * other.y + coeffs[5],
        }
    }
}

/// Stores the computed transform and the pre-transform bounding box (where the
/// other corner is the origin).  Useful for ray-casting, along with
#[derive(Clone)]
pub struct TransformAndBounds {
    pub transform: Affine,
    pub bounds: (f64, f64),
    pub clipping_bounds: Option<(f64, f64)>,
}

impl TransformAndBounds {
    pub fn corners(&self) -> [Point2D; 4] {
        let width = self.bounds.0;
        let height = self.bounds.1;

        let top_left = self.transform * Point2D { x: 0.0, y: 0.0 };
        let top_right = self.transform * Point2D { x: width, y: 0.0 };
        let bottom_left = self.transform * Point2D { x: 0.0, y: height };
        let bottom_right = self.transform
            * Point2D {
                x: width,
                y: height,
            };

        [top_left, top_right, bottom_right, bottom_left]
    }

    //Applies the separating axis theorem to determine whether two `TransformAndBounds` intersect.
    pub fn intersects(&self, other: &Self) -> bool {
        let corners_self = self.corners();
        let corners_other = other.corners();

        for i in 0..2 {
            let axis = corners_self[i].subtract(corners_self[(i + 1) % 4]).normal();

            let self_projections: Vec<_> =
                corners_self.iter().map(|&p| p.project_onto(axis)).collect();
            let other_projections: Vec<_> = corners_other
                .iter()
                .map(|&p| p.project_onto(axis))
                .collect();

            let (min_self, max_self) = min_max_projections(&self_projections);
            let (min_other, max_other) = min_max_projections(&other_projections);

            // Check for non-overlapping projections
            if max_self < min_other || max_other < min_self {
                // By the separating axis theorem, non-overlap of projections on _any one_ of the axis-normals proves that these polygons do not intersect.
                return false;
            }
        }
        true
    }
}

fn min_max_projections(projections: &[f64]) -> (f64, f64) {
    let min_projection = *projections
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_projection = *projections
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    (min_projection, max_projection)
}


pub enum NodeType {
    Component,
    Primitive,
}

/// The base trait for a RenderNode, representing any node that can
/// be rendered by the engine.
/// T: a member of PropertiesCoproduct, representing the type of the set of properites
/// associated with this node.
pub trait InstanceNode<R: 'static + RenderContext> {
    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized;

    /// Return the list of nodes that are children of this node at render-time.
    fn get_rendering_children(&self) -> InstanceNodePtrList<R>;

    /// For Components only, return the slot children passed into that Component.  For example, for `<Stacker><Group /></Stacker>`,
    /// Stacker#get_slot_children would return the `<Group />` that was passed in by the component that authored both the `<Stacker>` and the `<Group>` in its template.
    /// Note that `get_rendering_children`, in contrast, would return the root of Stacker's own template, not the `<Group />`.
    /// This is used when computing properties, in order to compute, for example, both Stacker and Group in the context of the same parent
    /// component and its runtime stack, instead of evaluating Group in the context of Stacker's internal template + runtime stack.
    fn get_slot_children(&self) -> Option<InstanceNodePtrList<R>> {
        None
    }

    /// Describes the type of this node; Primitive by default, overridden by Component
    fn get_node_type(&self) -> NodeType {
        NodeType::Primitive
    }

    /// Consumes the children of this node at render-time that should be removed.
    /// This occurs when they were mounted in some previous frame but now need to be removed after a property change
    /// This function resets this list once returned
    fn pop_cleanup_children(&mut self) -> InstanceNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn get_properties(&self) -> Rc<RefCell<PropertiesCoproduct>> {
        //need to refactor signature and pass in id_chain + either rtc + registry or just registry
        todo!("Look up ExpandedNode via id_chain + registry; return clone of stored PropertiesCoproduct")
    }

    fn get_common_properties(&self) -> Rc<RefCell<CommonProperties>> {
        todo!("Look up ExpandedNode via id_chain + registry; return clone of stored CommonProperties")
    }

    /// Determines whether the provided ray, orthogonal to the view plane,
    /// intersects this rendernode. `tab` must also be passed because these are specific
    /// to a ExpandedNode
    fn ray_cast_test(&self, ray: &(f64, f64), tab: &TransformAndBounds) -> bool {
        //short-circuit fail for Group and other size-None elements.
        //This doesn't preclude event handlers on Groups and size-None elements --
        //it just requires the event to "bubble".  otherwise, `Component A > Component B` will
        //never allow events to be bound to `B` — they will be vacuously intercepted by `A`
        if let None = self.get_size() {
            return false;
        }

        let inverted_transform = tab.transform.inverse();
        let transformed_ray = inverted_transform * Point { x: ray.0, y: ray.1 };

        let relevant_bounds = match tab.clipping_bounds {
            None => tab.bounds,
            Some(cp) => cp,
        };

        //Default implementation: rectilinear bounding hull
        transformed_ray.x > 0.0
            && transformed_ray.y > 0.0
            && transformed_ray.x < relevant_bounds.0
            && transformed_ray.y < relevant_bounds.1
    }



    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        None //default no-op
    }

    /// Used at least by ray-casting; only nodes that clip content (and thus should
    /// not allow outside content to respond to ray-casting) should return true
    fn get_clipping_bounds(&self) -> Option<(Size, Size)> {
        None
    }

    /// Returns the size of this node, or `None` if this node
    /// doesn't have a size (e.g. `Group`)
    fn get_size(&self) -> Option<(Size, Size)> {
        Some((
            *self.get_common_properties().width.as_ref().borrow().get(),
            *self.get_common_properties().height.as_ref().borrow().get(),
        ))
    }

    /// Returns unique integer ID of this RenderNode instance.  Note that
    /// individual rendered elements may share an instance_id, for example
    /// inside of `Repeat`.  See also `ExpandedNode` and `RenderTreeContext::get_id_chain`, which enables globally
    /// unique node addressing in the context of an in-progress render tree traversal.
    fn get_instance_id(&self) -> u32;

    /// Used for exotic tree traversals for `Slot`, e.g. for `Stacker` > `Repeat` > `Rectangle`
    /// where the repeated `Rectangle`s need to be be considered direct children of `Stacker`.
    /// `Repeat` and `Conditional` override `is_invisible_to_slot` to return true
    fn is_invisible_to_slot(&self) -> bool {
        false
    }

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_size() {
            None => bounds,
            Some(size_raw) => (
                size_raw.0.evaluate(bounds, Axis::X),
                size_raw.1.evaluate(bounds, Axis::Y),
            ),
        }
    }

    /// Returns the clipping bounds of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn compute_clipping_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_clipping_bounds() {
            None => bounds,
            Some(size_raw) => (
                size_raw.0.evaluate(bounds, Axis::X),
                size_raw.1.evaluate(bounds, Axis::Y),
            ),
        }
    }

    /// Lifecycle method used by at least Component to introduce a properties stack frame
    fn handle_pre_compute_properties(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// Lifecycle method used by at least Component to pop a properties stack frame
    fn handle_post_compute_properties(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// First lifecycle method during each render loop, used to compute
    /// properties in advance of rendering.  Returns an ExpandedNode for the
    fn handle_compute_properties(&mut self, _rtc: &mut RenderTreeContext<R>) -> Rc<RefCell<crate::ExpandedNode<R>>>;

    /// Used by elements that need to communicate across native rendering bridge (for example: Text, Clipping masks, scroll containers)
    /// Called by engine after `compute_properties`, passed calculated size and transform matrix coefficients for convenience
    /// Expected to induce side-effects (if appropriate) via enqueueing messages to the native message queue
    ///
    /// An implementor of `compute_native_patches` is responsible for determining which properties if any have changed
    /// (e.g. by keeping a local patch object as a cache of last known values.)
    fn compute_native_patches(
        &mut self,
        _rtc: &mut RenderTreeContext<R>,
        _computed_size: (f64, f64),
        _transform_coeffs: Vec<f64>,
        _z_index: u32,
        _subtree_depth: u32,
    ) {
        //no-op default implementation
    }

    /// Second lifecycle method during each render loop, occurs after
    /// properties have been computed, but before rendering
    /// Example use-case: perform side-effects to the drawing contexts.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    fn handle_pre_render(
        &mut self,
        _rtc: &mut RenderTreeContext<R>,
        _rcs: &mut HashMap<String, R>,
    ) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// after all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree. Most primitives
    /// are expected to draw their contents to the rendering context during this event.
    fn handle_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// after all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing contexts
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    fn handle_post_render(
        &mut self,
        _rtc: &mut RenderTreeContext<R>,
        _rcs: &mut HashMap<String, R>,
    ) {
        //no-op default implementation
    }

    /// Fires during the tick when a node is first attached to the render tree.  For example,
    /// this event fires by all nodes on the global first tick, and by all nodes in a subtree
    /// when a `Conditional` subsequently turns on a subtree (i.e. when the `Conditional`s criterion becomes `true` after being `false` through the end of at least 1 frame.)
    /// A use-case: send a message to native renderers that a `Text` element should be rendered and tracked
    fn handle_mount(&mut self, _rtc: &mut RenderTreeContext<R>, _z_index: u32) {
        //no-op default implementation
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    fn handle_unmount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// Returns the layer type (`Layer::Native` or `Layer::Canvas`) for this RenderNode.
    /// Default is `Layer::Canvas`, and must be overwritten for native rendering
    fn get_layer_type(&mut self) -> Layer {
        Layer::Canvas
    }

    /// Invoked by event interrupts to pass scroll information to render node
    fn handle_scroll(&mut self, _args_scroll: ArgsScroll) {
        //no-op default implementation
    }

    /// Returns the scroll offset from a Scroller component
    /// Used by the engine to transform its children
    fn get_scroll_offset(&mut self) -> (f64, f64) {
        (0.0, 0.0)
    }

    fn handle_form_event(&mut self, event: FormEvent) {
        panic!("form event sent to non-compatible component: {:?}", event)
    }
}

pub trait LifecycleNode {}

pub trait ComputableTransform {
    fn compute_transform2d_matrix(
        &self,
        node_size: (f64, f64),
        container_bounds: (f64, f64),
    ) -> Affine;
}

impl ComputableTransform for Transform2D {
    //Distinction of note: scale, translate, rotate, anchor, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    //Returns (Base affine transform, align component)
    fn compute_transform2d_matrix(
        &self,
        node_size: (f64, f64),
        container_bounds: (f64, f64),
    ) -> Affine {
        //Three broad strokes:
        // a.) compute anchor
        // b.) decompose "vanilla" affine matrix
        // c.) combine with previous transform chain (assembled via multiplication of two Transform2Ds, e.g. in PAXEL)

        // Compute anchor
        let anchor_transform = match &self.anchor {
            Some(anchor) => Affine::translate((
                match anchor[0] {
                    Size::Pixels(pix) => -pix.get_as_float(),
                    Size::Percent(per) => -node_size.0 * (per / 100.0),
                    Size::Combined(pix, per) => {
                        -pix.get_as_float() + (-node_size.0 * (per / 100.0))
                    }
                },
                match anchor[1] {
                    Size::Pixels(pix) => -pix.get_as_float(),
                    Size::Percent(per) => -node_size.1 * (per / 100.0),
                    Size::Combined(pix, per) => {
                        -pix.get_as_float() + (-node_size.0 * (per / 100.0))
                    }
                },
            )),
            //No anchor applied: treat as 0,0; identity matrix
            None => Affine::default(),
        };

        //decompose vanilla affine matrix and pack into `Affine`
        let (scale_x, scale_y) = if let Some(scale) = self.scale {
            (scale[0].expect_percent(), scale[1].expect_percent())
        } else {
            (1.0, 1.0)
        };

        let (skew_x, skew_y) = if let Some(skew) = self.skew {
            (skew[0], skew[1])
        } else {
            (0.0, 0.0)
        };

        let (translate_x, translate_y) = if let Some(translate) = &self.translate {
            (
                translate[0].evaluate(container_bounds, Axis::X),
                translate[1].evaluate(container_bounds, Axis::Y),
            )
        } else {
            (0.0, 0.0)
        };

        let rotate_rads = if let Some(rotate) = &self.rotate {
            rotate.get_as_radians()
        } else {
            0.0
        };

        let cos_theta = rotate_rads.cos();
        let sin_theta = rotate_rads.sin();

        // Elements for a combined scale and rotation
        let a = scale_x * cos_theta - scale_y * skew_x * sin_theta;
        let b = scale_x * sin_theta + scale_y * skew_x * cos_theta;
        let c = -scale_y * sin_theta + scale_x * skew_y * cos_theta;
        let d = scale_y * cos_theta + scale_x * skew_y * sin_theta;

        // Translation
        let e = translate_x;
        let f = translate_y;

        let coeffs = [a, b, c, d, e, f];
        let transform = Affine::new(coeffs);

        // Compute and combine previous_transform
        let previous_transform = match &self.previous {
            Some(previous) => (*previous).compute_transform2d_matrix(node_size, container_bounds),
            None => Affine::default(),
        };

        transform * anchor_transform * previous_transform
    }
}

/// Represents the outer stroke of a drawable element
pub struct StrokeInstance {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
    //FUTURE: stroke alignment, inner/outer/center?
}
