use std::cell::RefCell;
use std::rc::Rc;

use kurbo::{Affine};
use piet::{Color, StrokeStyle};
use piet_web::WebRenderContext;

use crate::{PropertyValue, PropertyValueLiteral, RenderTreeContext, HostPlatformContext};

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for rendernodes.
pub type RenderNodePtr = Rc<RefCell<dyn RenderNode>>;
pub type RenderNodePtrList = Rc<RefCell<Vec<RenderNodePtr>>>;

/// Very thin structure representing the root of a `RenderTree`. The engine
/// receives one of these for rendering.  We may want to make use of this inside Components
/// as well, i.e. to represent a template.
pub struct RenderTree {
    pub root: RenderNodePtr //TODO:  maybe this should be more strictly a Rc<RefCell<Component>>, or a new type (alias) "ComponentPtr"
}

impl RenderTree {}

/// The base trait for a RenderNode, representing any node that can
/// be rendered by the engine.
pub trait RenderNode
{
    /// Return the list of nodes that are children of this node at render-time.
    /// Note that "children" is somewhat overloaded, hence "rendering_children" here.
    /// "Children" may indicate a.) a template root, b.) adoptees, c.) primitive children
    /// Each RenderNode is responsible for determining at render-time which of these concepts
    /// to pass to the engine for rendering, and that distinction occurs inside `get_rendering_children`
    fn get_rendering_children(&self) -> RenderNodePtrList;

    /// Returns the size of this node, or `None` if this node
    /// doesn't have a size (e.g. `Group`)
    fn get_size(&self) -> Option<Size2D>;


    /// TODO:  do we want to track timelines at the RenderNode level
    ///        or at the StackFrame level?
    ///
    ///        for example, when evaluating compute_in_place for a ProeprtyValueTimeline,
    ///        does the rtc.timeline_playhead_position get populated by
    ///        recursing through RenderNodes, or by traversing StackFrames?
    ///
    ///        instinctively, the latter — most RenderNodes don't mess with timelines,
    ///        and currently `having a timeline` == `having a stackframe`

    /// Returns a Timeline if this render node specifies one,
    // fn get_timeline(&self) -> Option<Timeline> {None}

    /// Rarely needed:  Used for exotic tree traversals, e.g. for `Spread` > `Repeat` > `Rectangle`
    /// where the repeated `Rectangle`s need to be be considered direct children of `Spread`.
    /// `Repeat` overrides `should_flatten` to return true, which `Engine` interprets to mean "ignore this
    /// node and consume its children" during traversal.
    ///
    /// This may also be useful as a check during placeholder -> adoptee
    /// searching via stackframes — currently placeholders will recurse
    /// up the stackframe looking for adoptees, but it may be the case that
    /// checking should_flatten and NOT recursing is better behavior.  TBD
    /// as more use-cases are vetted.
    fn should_flatten(&self) -> bool {
        false
    }

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_size() {
            None => bounds,
            Some(size_raw) => {
                (
                    match size_raw.borrow().0.read() {
                        Size::Pixel(width) => {
                            *width
                        },
                        Size::Percent(width) => {
                            bounds.0 * (*width / 100.0)
                        }
                    },
                    match size_raw.borrow().1.read() {
                        Size::Pixel(height) => {
                            *height
                        },
                        Size::Percent(height) => {
                            bounds.1 * (*height / 100.0)
                        }
                    }
                )
            }
        }
    }

    /// Return the "sugary" transform object for this node, which
    /// is likely buried inside a polymorphic PropertiesCoproduct object
    fn get_transform(&mut self) -> Rc<RefCell<Transform>>;

    /// Very first lifecycle method during each render loop, used to compute
    /// properties in advance of rendering.
    /// Occurs in a pre-order traversal of the render tree.
    fn compute_properties(&mut self, _rtc: &mut RenderTreeContext) {
        //no-op default implementation
    }

    /// Second lifecycle method during each render loop, occurs AFTER
    /// properties have been computed, but BEFORE rendering or traversing descendents.
    /// Example use-case: perform side-effects to the drawing context.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    fn pre_render(&mut self, _rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree.
    fn render(&self, _rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
        //no-op default implementation
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing context
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    fn post_render(&mut self, _rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
        //no-op default implementation
    }
}

/// A sugary representation of an Affine transform+, including
/// `origin` and `align` as layout-computed properties.
///
/// `translate` represents an (x,y) affine translation
/// `scale`     represents an (x,y) non-uniform affine scale
/// `rotate`    represents a (z) affine rotation (intuitive 2D rotation)
/// `origin`    represents the "(0,0)" point of the render node as it relates to its own bounding box.
///             By default that's the top-left of the element, but `origin` allows that
///             to be offset either by a pixel or percentage-of-element-size
///             for each of (x,y)
/// `align`     the offset of this element's `origin` as it relates to the element's parent.
///             By default this is the top-left corner of the parent container,
///             but can be set to be any value [0,1] for each of (x,y), representing
///             the percentage (between 0.0 and 1.0) multiplied by the parent container size.
///             For example, an align of (0.5, 0.5) will center an element's `origin` point both vertically
///             and horizontally within the parent container.  Combined with an origin of (Size::Percent(50.0), Size::Percent(50.0)),
///             an element will appear fully centered within its parent.
///
/// Note that transform order is currently hard-coded.  This could be amended
/// upon deriving a suitable API — this may look like passing a manual `Affine` object
pub struct Transform {
    pub translate: (Box<dyn PropertyValue<f64>>, Box<dyn PropertyValue<f64>>),
    pub scale: (Box<dyn PropertyValue<f64>>, Box<dyn PropertyValue<f64>>),
    pub rotate: Box<dyn PropertyValue<f64>>, //z-axis only for 2D rendering
    //TODO: add shear? needed at least to support ungrouping after scale+rotate
    pub origin: (Box<dyn PropertyValue<Size>>, Box<dyn PropertyValue<Size>>),
    pub align: (Box<dyn PropertyValue<f64>>, Box<dyn PropertyValue<f64>>),
    pub cached_computed_transform: Affine,
}

impl Default for Transform {
    fn default() -> Self {
        Transform{
            cached_computed_transform: Affine::default(),
            align: (Box::new(PropertyValueLiteral { value: 0.0 }), Box::new(PropertyValueLiteral { value: 0.0 })),
            origin: (Box::new(PropertyValueLiteral { value: Size::Pixel(0.0)}), Box::new(PropertyValueLiteral { value: Size::Pixel(0.0)})),
            translate: (Box::new(PropertyValueLiteral { value: 0.0}), Box::new(PropertyValueLiteral { value: 0.0})),
            scale: (Box::new(PropertyValueLiteral { value: 1.0}), Box::new(PropertyValueLiteral { value: 1.0})),
            rotate: Box::new(PropertyValueLiteral { value: 0.0 }),
        }
    }
}

impl Transform {

    pub fn compute_in_place(&mut self, rtc: &RenderTreeContext) {
        &self.translate.0.compute_in_place(rtc);
        &self.translate.1.compute_in_place(rtc);
        &self.scale.0.compute_in_place(rtc);
        &self.scale.1.compute_in_place(rtc);
        &self.rotate.compute_in_place(rtc);
        &self.origin.0.compute_in_place(rtc);
        &self.origin.1.compute_in_place(rtc);
        &self.align.0.compute_in_place(rtc);
        &self.align.1.compute_in_place(rtc);
    }

    //TODO:  if providing bounds is prohibitive or awkward for some use-case,
    //       we can make `bounds` and `align` BOTH TOGETHER optional — align requires `bounds`
    //       but it's the only thing that requires `bounds`

    //Distinction of note: scale, translate, rotate, origin, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    pub fn compute_matrix_in_place(&mut self, node_size: (f64, f64), container_bounds: (f64, f64)) -> &Affine {
        let origin_transform = Affine::translate(
        (
                match self.origin.0.read() {
                    Size::Pixel(x) => { -x },
                    Size::Percent(x) => {
                        -node_size.0 * (x / 100.0)
                    },
                },
                match self.origin.1.read() {
                    Size::Pixel(y) => { -y },
                    Size::Percent(y) => {
                        -node_size.1 * (y / 100.0)
                    },
                }
            )
        );

        //TODO: support custom user-specified transform order?
        // Is the only use-case for this or is grouping sufficient to achieve "rotation about an axis"?
        // If so, grouping/framing is likely sufficient
        let base_transform =
            Affine::rotate(*self.rotate.read()) *
            Affine::scale_non_uniform(*self.scale.0.read(), *self.scale.1.read()) *
            Affine::translate((*self.translate.0.read(), *self.translate.1.read()));

        let align_transform = Affine::translate((self.align.0.read() * container_bounds.0, self.align.1.read() * container_bounds.1));
        self.cached_computed_transform = align_transform * origin_transform * base_transform;
        &self.cached_computed_transform
    }

    pub fn get_cached_computed_value(&self) -> &Affine {
        &self.cached_computed_transform
    }

}

/// Represents the outer stroke of a drawable element
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
    //TODO: stroke alignment, inner/outer/center?
}

/// A size value that can be either a concrete pixel value
/// or a percent of parent bounds
#[derive(Copy, Clone)]
pub enum Size {
    Pixel(f64),
    Percent(f64),
}

// More than just a tuble of (Size, Size),
// Size2D wraps up Properties as well to make it easy
// to declare expressable Size properties
pub type Size2D = Rc<RefCell<(
    Box<dyn PropertyValue<Size>>,
    Box<dyn PropertyValue<Size>>,
)>>;


/// Used as an ergonomic aid for instantiating Size2Ds
pub struct Size2DFactory {}

impl Size2DFactory {
    pub fn literal(x: Size, y: Size) -> Size2D {
        Rc::new(RefCell::new(
            (
                Box::new(
                    PropertyValueLiteral { value: x }
                ),
                Box::new(
                    PropertyValueLiteral { value: y }
                )
            )
        ))
    }
    pub fn default() -> Size2D {
       Rc::new(RefCell::new(
            (
                Box::new(
                    PropertyValueLiteral { value: Size::Percent(100.0) }
                ),
                Box::new(
                    PropertyValueLiteral { value: Size::Percent(100.0) }
                )
            )
        ))
    }
}

