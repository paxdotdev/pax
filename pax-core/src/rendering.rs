use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Mul;
use std::rc::Rc;

use kurbo::{Affine, Point};
use pax_runtime_api::{Axis, CommonProperties, Transform2D};
use piet::{Color, StrokeStyle};
use piet_common::RenderContext;

use pax_runtime_api::{ArgsScroll, Layer, PropertyInstance, Size};

use crate::form_event::FormEvent;
use crate::{
    ExpandedNode, HandlerRegistry, NodeRegistry, PropertiesTreeContext, RenderTreeContext,
};

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for instance nodes.
pub type InstanceNodePtr<R> = Rc<RefCell<dyn InstanceNode<R>>>;
pub type InstanceNodePtrList<R> = Rc<RefCell<Vec<InstanceNodePtr<R>>>>;

/// Given some InstanceNodePtrList, distill away all "slot-invisible" nodes (namely, `if` and `for`)
/// and return another InstanceNodePtrList with a flattened top-level list of nodes.
pub fn flatten_slot_invisible_nodes_recursive<R: 'static + RenderContext>(
    input_nodes: InstanceNodePtrList<R>,
) -> InstanceNodePtrList<R> {
    let mut output_nodes = Vec::new();

    for node in input_nodes.borrow().iter() {
        if node.borrow().is_invisible_to_slot() {
            let children = node.borrow().get_instance_children();
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
    pub common_properties: Rc<RefCell<CommonProperties>>,
    pub properties: Rc<RefCell<dyn Any>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub node_registry: Rc<RefCell<NodeRegistry<R>>>,
    pub children: Option<InstanceNodePtrList<R>>,
    pub component_template: Option<InstanceNodePtrList<R>>,
    pub scroller_args: Option<ScrollerArgs>,

    ///used by Component instances, specifically to unwrap dyn Any properties
    ///and recurse into descendant property computation
    pub compute_properties_fn:
        Option<Box<dyn FnMut(Rc<RefCell<dyn Any>>, &mut RenderTreeContext<R>)>>,
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
    // pub clipping_bounds: Option<(f64, f64)>,
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

#[derive(Clone)]
pub enum NodeType {
    Component,
    Primitive,
}

/// Central runtime representation of a properties-computable and renderable node.
/// `InstanceNode`s are conceptually stateless, and rely on [`ExpandedNode`]s for stateful representations.
///
/// An `InstanceNode` sits in between a [`pax_compiler::TemplateNodeDefinition`], the
/// compile-time `definition` analogue to this `instance`, and [`ExpandedNode`].
///
/// There is a 1:1 relationship between [`pax_compiler::TemplateNodeDefinition`]s and `InstanceNode`s.
/// There is a one-to-many relationship between one `InstanceNode` and possibly many variant [`ExpandedNode`]s,
/// due to duplication via `for`.
///
/// `InstanceNode`s are architecturally "type-aware" — they can perform type-specific operations e.g. on the state stored in [`ExpandedNode`], while
/// [`ExpandedNode`]s are "type-blind".  The latter store polymorphic data but cannot operate on it without the type-aware assistance of their linked `InstanceNode`.
///
/// (See [`RepeatInstance#expand_node`] where we visit a singular `InstanceNode` several times, producing multiple [`ExpandedNode`]s.)
pub trait InstanceNode<R: 'static + RenderContext> {
    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized;

    /// Return the list of instance nodes that are children of this one.  Intuitively, this will return
    /// instance nodes mapping exactly to the template node definitions.
    /// For `Component`s, `get_instance_children` returns the root(s) of its template, not its `slot_children`.
    /// (see [`get_slot_children`] for the way to retrieve the latter.)
    fn get_instance_children(&self) -> InstanceNodePtrList<R>;

    /// For Components only, return the slot children passed into that Component.  For example, for `<Stacker><Group /></Stacker>`,
    /// Stacker#get_slot_children would return the `<Group />` that was passed in by the component-template where both the `<Stacker>` and the `<Group>` were defined.
    /// Note that `get_instance_children`, in contrast, would return the root of Stacker's own template, not the `<Group />`.
    /// This is used when computing properties, in order to compute, for example, both Stacker and Group in the context of the same parent
    /// component and its runtime stack, instead of evaluating Group in the context of Stacker's internal template + runtime stack.
    fn get_slot_children(&self) -> Option<InstanceNodePtrList<R>> {
        None
    }

    /// Describes the type of this node; Primitive by default, overridden by Component
    fn get_node_type(&self) -> NodeType {
        NodeType::Primitive
    }

    /// Returns a handle to a node-managed HandlerRegistry, a mapping between event types and handlers.
    /// Each node that can handle events is responsible for implementing this; Component instances generate
    /// the necessary code to wire up userland events like `<SomeNode @click=self.handler>`. Primitives must handle
    /// this explicitly, see e.g. `[pax_std_primitives::RectangleInstance#get_handler_registry]`.
    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        None //default no-op
    }

    /// Returns the bounds of an InstanceNode.  This computation requires a stateful [`ExpandedNode`], yet requires
    /// customization at the trait-implementor level (dyn InstanceNode), thus this method accepts an expanded_node
    /// parameter.
    /// The default implementation retrieves the expanded_node's [`pax_runtime_api::CommonProperties#width`] and [`pax_runtime_api::CommonProperties#height`]
    fn get_size(&self, expanded_node: &ExpandedNode<R>) -> (Size, Size) {
        let common_properties = expanded_node.get_common_properties();
        let common_properties_borrowed = common_properties.borrow();
        (
            common_properties_borrowed.width.get().clone(),
            common_properties_borrowed.height.get().clone(),
        )
    }

    #[allow(unused_variables)]
    fn get_clipping_bounds(&self, expanded_node: &ExpandedNode<R>) -> Option<(Size, Size)> {
        None
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

    /// Allows an `InstanceNode` to specify that the properties computation engine should not expand nodes
    /// for its subtree (to stop recursing externally,) because this node will manage its own recursion for expanding its subtree.
    /// It's expected that node that returns `true` will call `recurse_expand_nodes` on instance nodes in its subtree.
    /// Use-cases include Repeat, Conditional, and Component, which, for various reasons, must custom-manage how their properties subtree is calculated.
    fn manages_own_subtree_for_expansion(&self) -> bool {
        false
    }

    /// Expands the current `InstanceNode` into a stateful `ExpandedNode`, with its own instances of properties & common properties, in the context of the
    /// provided `PropertiesTreeContext`.  Node expansion takes into account the "parallel selves" that an `InstanceNode` may have through the
    /// lens of declarative control flow, [`ConditionalInstance`] and [`RepeatInstance`].
    #[allow(unused_variables)]
    fn expand_node_and_compute_properties(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<crate::ExpandedNode<R>>>;

    /// Used by elements that need to communicate across native rendering bridge (for example: Text, Clipping masks, scroll containers)
    /// Called by engine after [`expand_node`], passed calculated size and transform matrix coefficients for convenience
    /// Expected to induce side-effects (if appropriate) via enqueueing messages to the native message queue
    ///
    /// An implementor of `handle_native_patches` is responsible for determining which properties if any have changed
    /// (e.g. by keeping a local patch object as a cache of last known values.)
    #[allow(unused_variables)]
    fn handle_native_patches(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
        computed_size: (f64, f64),
        transform_coeffs: Vec<f64>,
        z_index: u32,
        subtree_depth: u32,
    ) {
        //no-op default implementation
    }

    /// Second lifecycle method during each render loop, occurs after
    /// properties have been computed, but before rendering
    /// Example use-case: perform side-effects to the drawing contexts.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    #[allow(unused_variables)]
    fn handle_pre_render(&mut self, rtc: &mut RenderTreeContext<R>, rcs: &mut HashMap<String, R>) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// after all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree. Most primitives
    /// are expected to draw their contents to the rendering context during this event.
    #[allow(unused_variables)]
    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        //no-op default implementation
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// after all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing contexts
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    #[allow(unused_variables)]
    fn handle_post_render(&mut self, rtc: &mut RenderTreeContext<R>, rcs: &mut HashMap<String, R>) {
        //no-op default implementation
    }

    /// Fires during the tick when a node is first attached to the render tree.  For example,
    /// this event fires by all nodes on the global first tick, and by all nodes in a subtree
    /// when a `Conditional` subsequently turns on a subtree (i.e. when the `Conditional`s criterion becomes `true` after being `false` through the end of at least 1 frame.)
    /// A use-case: send a message to native renderers that a `Text` element should be rendered and tracked
    #[allow(unused_variables)]
    fn handle_mount(&mut self, ptc: &mut PropertiesTreeContext<R>) {
        //no-op default implementation
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    #[allow(unused_variables)]
    fn handle_unmount(&mut self, ptc: &mut PropertiesTreeContext<R>) {
        //no-op default implementation
    }

    /// Returns the layer type (`Layer::Native` or `Layer::Canvas`) for this RenderNode.
    /// Default is `Layer::Canvas`, and must be overwritten for `InstanceNode`s that manage native
    /// content.
    fn get_layer_type(&mut self) -> Layer {
        Layer::Canvas
    }

    /// Invoked by event interrupts to pass scroll information to render node
    #[allow(unused_variables)]
    fn handle_scroll(&mut self, args_scroll: ArgsScroll) {
        //no-op default implementation
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
