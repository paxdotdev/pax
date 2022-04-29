use std::cell::RefCell;
use std::rc::Rc;

use kurbo::{Affine};
use piet::{Color, StrokeStyle};
use piet_common::RenderContext;
use pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{ArgsCoproduct, Size, Size2D};

use crate::{RenderTreeContext, HandlerRegistry, InstanceMap};

use pax_runtime_api::{PropertyInstance, PropertyLiteral};

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for rendernodes.
pub type RenderNodePtr<R> = Rc<RefCell<dyn RenderNode<R>>>;
pub type RenderNodePtrList<R> = Rc<RefCell<Vec<RenderNodePtr<R>>>>;



pub struct InstantiationArgs<R: 'static + RenderContext> {
    pub properties: PropertiesCoproduct,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_map: Rc<RefCell<InstanceMap<R>>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub size: Option<[Box<dyn PropertyInstance<Size>>;2]>,
    pub children: Option<RenderNodePtrList<R>>,
    pub component_template: Option<RenderNodePtrList<R>>,

    //used in special cases where certain Component instances should not
    //interfere with Slot <> Adoptee linking, e.g. for the
    //internals of Repeat
    pub should_skip_adoption: bool,

    //used by Slot
    pub slot_index: Option<Box<dyn PropertyInstance<usize>>>,

    ///used by Repeat
    pub repeat_data_list: Option<Box<dyn PropertyInstance<Vec<Rc<PropertiesCoproduct>>>>>,

    ///used by Conditional
    pub conditional_boolean_expression: Option<Box<dyn PropertyInstance<bool>>>,

    ///used by Component instances, specifically to unwrap type-specific PropertiesCoproducts
    ///and recurse into descendant property computation
    pub compute_properties_fn: Option<Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>,&mut RenderTreeContext<R>)>>,
}

/// The base trait for a RenderNode, representing any node that can
/// be rendered by the engine.
/// T: a member of PropertiesCoproduct, representing the type of the set of properites
/// associated with this node.
pub trait RenderNode<R: 'static + RenderContext>
{

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized;

    /// Return the list of nodes that are children of this node at render-time.
    /// Note that "children" is somewhat overloaded, hence "rendering_children" here.
    /// "Children" may indicate a.) a template root, b.) adoptees, c.) primitive children
    /// Each RenderNode is responsible for determining at render-time which of these concepts
    /// to pass to the engine for rendering, and that distinction occurs inside `get_rendering_children`
    fn get_rendering_children(&self) -> RenderNodePtrList<R>;


    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        None //default no-op
    }

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
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_size() {
            None => bounds,
            Some(size_raw) => {
                (
                    match size_raw.borrow()[0].get() {
                        Size::Pixel(width) => {
                            *width
                        },
                        Size::Percent(width) => {
                            bounds.0 * (*width / 100.0)
                        }
                    },
                    match size_raw.borrow()[1].get() {
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

    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>>;

    /// First lifecycle method during each render loop, used to compute
    /// properties in advance of rendering.
    /// Occurs in a pre-order traversal of the render tree.
    fn compute_properties(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// Second lifecycle method during each render loop, occurs AFTER
    /// properties have been computed, but BEFORE rendering
    /// Example use-case: perform side-effects to the drawing context.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    fn handle_pre_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree. Most primitives
    /// are expected to draw their contents to the rendering context during this event.
    fn handle_render(&self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing context
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    fn handle_post_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }


    /// Fires during the tick when a node is first attached to the render tree.  For example,
    /// this event fires by all nodes on the global first tick, and by all nodes in a subtree
    /// when a `Conditional` subsequently turns on a subtree (i.e. when the `Conditional`s criterion becomes `true` after being `false` through the end of at least 1 frame.)
    /// A use-case: send a message to native renderers that a `Text` element should be rendered and tracked
    fn handle_post_mount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// Fires during element dismount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    fn handle_pre_dismount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }


}

pub trait LifecycleNode {


}

use pax_runtime_api::Transform2D;



pub trait ComputableTransform {
    fn compute_transform_matrix(&self, node_size: (f64, f64), container_bounds: (f64, f64)) -> (Affine,Affine);
}

impl ComputableTransform for Transform2D {
    //Distinction of note: scale, translate, rotate, anchor, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    //Returns (Base affine transform, align component)
    fn compute_transform_matrix(&self, node_size: (f64, f64), container_bounds: (f64, f64)) -> (Affine,Affine)  {
        let anchor_transform = match &self.anchor {
            Some(anchor) => {
                Affine::translate(
                    (
                        match anchor[0] {
                            Size::Pixel(x) => {
                                -x
                            },
                            Size::Percent(x) => {
                                -node_size.0 * (x / 100.0)
                            },
                        },
                        match anchor[1] {
                            Size::Pixel(y) => {
                                -y
                            },
                            Size::Percent(y) => {
                                -node_size.1 * (y / 100.0)
                            },
                        }
                    )
                )
            },
            //No anchor applied: treat as 0,0; identity matrix
            None => {Affine::default()}
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
            Some(previous) => {(*previous).compute_transform_matrix(node_size, container_bounds)},
            None => {(Affine::default(), Affine::default())},
        };

        let align_component = match &self.align {
            Some(align) => {
                let x_percent = if let Size::Percent(x) = align[0] {x/100.0} else {panic!("Align requires a Size::Percent value")};
                let y_percent = if let Size::Percent(y) = align[1] {y/100.0} else {panic!("Align requires a Size::Percent value")};
                Affine::translate((x_percent * container_bounds.0, y_percent * container_bounds.1))},
            None => {
                previous_align_component //which defaults to identity
            }
        };

        //align component is passed separately because it is global for a given sequence of Transform operations
        (anchor_transform * transform * previous_transform, align_component)
    }

}

/// Represents the outer stroke of a drawable element
pub struct StrokeInstance {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
    //TODO: stroke alignment, inner/outer/center?
}

