use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Mul;
use std::rc::Rc;

use kurbo::{Affine, Point};
use pax_properties_coproduct::PropertiesCoproduct;
use pax_runtime_api::Transform2D;
use piet::{Color, StrokeStyle};
use piet_common::RenderContext;

use pax_runtime_api::{ArgsScroll, Layer, Size, Rotation};

use crate::{HandlerRegistry, InstanceRegistry, RenderTreeContext};

use pax_runtime_api::PropertyInstance;

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for rendernodes.
pub type RenderNodePtr<R> = Rc<RefCell<dyn RenderNode<R>>>;
pub type RenderNodePtrList<R> = Rc<RefCell<Vec<RenderNodePtr<R>>>>;

pub struct ScrollerArgs {
    pub size_inner_pane: [Box<dyn PropertyInstance<f64>>; 2],
    pub axes_enabled: [Box<dyn PropertyInstance<bool>>; 2],
}

pub struct InstantiationArgs<R: 'static + RenderContext> {
    pub common_properties: CommonProperties,
    pub properties: PropertiesCoproduct,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_registry: Rc<RefCell<InstanceRegistry<R>>>,
    pub children: Option<RenderNodePtrList<R>>,
    pub component_template: Option<RenderNodePtrList<R>>,
    pub scroller_args: Option<ScrollerArgs>,
    /// used by Slot
    pub slot_index: Option<Box<dyn PropertyInstance<pax_runtime_api::Numeric>>>,

    ///used by Repeat — the _vec and _range variants are modal, describing whether the source
    ///is encoded as a Vec<T> or as a Range<...>
    pub repeat_source_expression_vec:
        Option<Box<dyn PropertyInstance<Vec<Rc<PropertiesCoproduct>>>>>,
    pub repeat_source_expression_range: Option<Box<dyn PropertyInstance<std::ops::Range<isize>>>>,

    ///used by Conditional
    pub conditional_boolean_expression: Option<Box<dyn PropertyInstance<bool>>>,

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

            if self_projections
                .iter()
                .cloned()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
                < other_projections
                    .iter()
                    .cloned()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap()
                || other_projections
                    .iter()
                    .cloned()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap()
                    < self_projections
                        .iter()
                        .cloned()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap()
            {
                // By the separating axis theorem, non-overlap of projections on _any one_ of the axis-normals proves that these polygons do not intersect.
                return false;
            }
        }

        true
    }
}

/// Struct containing fields shared by all RenderNodes.
/// Each property here is special-cased by the compiler when parsing element properties (e.g. `<SomeElement width={...} />`)
/// Retrieved via <dyn RenderNode>#get_common_properties
pub struct CommonProperties {
    pub x: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub y: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub width: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub height: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub scale_x: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub scale_y: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub shear_x: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub shear_y: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub rotate: Option<Rc<RefCell<dyn PropertyInstance<Rotation>>>>,
    pub anchor_x: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub anchor_y: Option<Rc<RefCell<dyn PropertyInstance<Size>>>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<pax_runtime_api::Transform2D>>>,
}

impl Default for CommonProperties {
    fn default() -> Self {
        Self {
            x: Default::default(),
            y: Default::default(),
            width: Default::default(),
            height: Default::default(),
            scale_x: Default::default(),
            scale_y: Default::default(),
            shear_x: Default::default(),
            shear_y: Default::default(),
            rotate: Default::default(),
            anchor_x: Default::default(),
            anchor_y: Default::default(),
            transform: Transform2D::default_wrapped(),
        }
    }
}

/// The base trait for a RenderNode, representing any node that can
/// be rendered by the engine.
/// T: a member of PropertiesCoproduct, representing the type of the set of properites
/// associated with this node.
pub trait RenderNode<R: 'static + RenderContext> {
    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized;

    /// Return the list of nodes that are children of this node at render-time.
    /// Note that "children" is somewhat overloaded, hence "rendering_children" here.
    /// "Children" may indicate a.) a template root, b.) adoptees, c.) primitive children
    /// Each RenderNode is responsible for determining at render-time which of these concepts
    /// to pass to the engine for rendering, and that distinction occurs inside `get_rendering_children`
    fn get_rendering_children(&self) -> RenderNodePtrList<R>;

    /// Consumes the children of this node at render-time that should be removed.
    /// This occurs when they were mounted in some previous frame but now need to be removed after a property change
    /// This function resets this list once returned
    fn pop_cleanup_children(&mut self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    ///Determines whether the provided ray, orthogonal to the view plane,
    ///intersects this rendernode. `tab` must also be passed because these are specific
    ///to a RepeatExpandedNode
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

    fn get_common_properties(&self) -> &CommonProperties;

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
        Some((self.get_common_properties().width.as_ref().unwrap().borrow().get().clone(),self.get_common_properties().height.as_ref().unwrap().borrow().get().clone()))
    }

    /// Returns unique integer ID of this RenderNode instance.  Note that
    /// individual rendered elements may share an instance_id, for example
    /// inside of `Repeat`.  See also `RenderTreeContext::get_id_chain`, which enables globally
    /// unique node addressing in the context of an in-progress render tree traversal.
    fn get_instance_id(&self) -> u32;

    /// Used for exotic tree traversals, e.g. for `Stacker` > `Repeat` > `Rectangle`
    /// where the repeated `Rectangle`s need to be be considered direct children of `Stacker`.
    /// `Repeat` overrides `should_flatten` to return true, which `Engine` interprets to mean "ignore this
    /// node and consume its children" during traversal.
    ///
    /// This may also be useful as a check during slot -> adoptee
    /// searching via stackframes — currently slots will recurse
    /// up the stackframe looking for adoptees, but it may be the case that
    /// checking should_flatten and NOT recursing is better behavior.  TBD
    /// as more use-cases are vetted.
    fn should_flatten(&self) -> bool {
        false
    }

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_size() {
            None => bounds,
            Some(size_raw) => (
                size_raw.0.evaluate(bounds),
                size_raw.1.evaluate(bounds),
            ),
        }
    }

    /// Returns the clipping bounds of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn compute_clipping_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_clipping_bounds() {
            None => bounds,
            Some(size_raw) => (
                size_raw.0.evaluate(bounds),
                size_raw.1.evaluate(bounds),
            ),
        }
    }
    /// First lifecycle method during each render loop, used to compute
    /// properties in advance of rendering.
    /// Occurs in a pre-order traversal of the render tree.
    fn compute_properties(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

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

    /// Second lifecycle method during each render loop, occurs AFTER
    /// properties have been computed, but BEFORE rendering
    /// Example use-case: perform side-effects to the drawing contexts.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    fn handle_will_render(
        &mut self,
        _rtc: &mut RenderTreeContext<R>,
        _rcs: &mut HashMap<String, R>,
    ) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree. Most primitives
    /// are expected to draw their contents to the rendering context during this event.
    fn handle_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing contexts
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    fn handle_did_render(
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
    fn handle_did_mount(&mut self, _rtc: &mut RenderTreeContext<R>, _z_index: u32) {
        //no-op default implementation
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    fn handle_will_unmount(&mut self, _rtc: &mut RenderTreeContext<R>) {
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
}

pub trait LifecycleNode {}


pub trait ComputableTransform {
    fn compute_transform_matrix(
        &self,
        node_size: (f64, f64),
        container_bounds: (f64, f64),
    ) -> (Affine, Affine);
}

impl ComputableTransform for Transform2D {
    //Distinction of note: scale, translate, rotate, anchor, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    //Returns (Base affine transform, align component)
    fn compute_transform_matrix(
        &self,
        node_size: (f64, f64),
        container_bounds: (f64, f64),
    ) -> (Affine, Affine) {
        let anchor_transform = match &self.anchor {
            Some(anchor) => Affine::translate((
                match anchor[0] {
                    Size::Pixels(x) => -x.get_as_float(),
                    Size::Percent(x) => -node_size.0 * (x / 100.0),
                },
                match anchor[1] {
                    Size::Pixels(y) => -y.get_as_float(),
                    Size::Percent(y) => -node_size.1 * (y / 100.0),
                },
            )),
            //No anchor applied: treat as 0,0; identity matrix
            None => Affine::default(),
        };

        let mut transform = Affine::default();
        if let Some(rotate) = &self.rotate {
            transform = transform * Affine::rotate(*rotate);
        }
        if let Some(scale) = &self.scale {
            transform = transform * Affine::scale_non_uniform(scale[0], scale[1]);
        }
        if let Some(translate) = &self.translate {
            transform = transform * Affine::translate((translate[0], translate[1]));
        }

        //if this has an align component, return it.else {if previous has an align component, return it }

        let (previous_transform, previous_align_component) = match &self.previous {
            Some(previous) => (*previous).compute_transform_matrix(node_size, container_bounds),
            None => (Affine::default(), Affine::default()),
        };

        let align_component = match &self.align {
            Some(align) => {
                let x_percent = if let Size::Percent(x) = align[0] {
                    x / 100.0
                } else {
                    panic!("Align requires a Size::Percent value")
                };
                let y_percent = if let Size::Percent(y) = align[1] {
                    y / 100.0
                } else {
                    panic!("Align requires a Size::Percent value")
                };
                Affine::translate((
                    x_percent * container_bounds.0,
                    y_percent * container_bounds.1,
                ))
            }
            None => {
                previous_align_component //which defaults to identity
            }
        };

        //align component is passed separately because it is global for a given sequence of Transform operations
        (
            anchor_transform * transform * previous_transform,
            align_component,
        )
    }
}

/// Represents the outer stroke of a drawable element
pub struct StrokeInstance {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
    //FUTURE: stroke alignment, inner/outer/center?
}
