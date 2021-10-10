use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dynstack::DynStack;
use kurbo::{Affine, BezPath};
use piet::{Color, RenderContext, StrokeStyle};
use piet_web::WebRenderContext;

use crate::{PropertiesCoproduct, Property, PropertyLiteral, PropertyTreeContext, RenderTreeContext, Scope, StackFrame};
use crate::Size::Percent;

pub type RenderNodePtr = Rc<RefCell<dyn RenderNode>>;
pub type RenderNodePtrList = Rc<RefCell<Vec<RenderNodePtr>>>;

// Take a singleton node and wrap it into a Vec, e.g. to make a
// single node the entire `children` of another node
//TODO: handle this more elegantly, perhaps with some metaprogramming mojo?
pub fn wrap_render_node_ptr_into_list(rnp: RenderNodePtr) -> RenderNodePtrList {
    Rc::new(RefCell::new(vec![Rc::clone(&rnp)]))
}

pub fn decompose_render_node_ptr_list_into_vec(rnpl: RenderNodePtrList) -> Vec<RenderNodePtr> {
    let mut ret_vec = Vec::new();
    rnpl.borrow().iter().for_each(|rnp| {ret_vec.push(Rc::clone(&rnp))});
    ret_vec
}

pub struct RenderTree {
    pub root: RenderNodePtr //TODO:  maybe this should be more strictly a Rc<RefCell<Component>>, or a new type (alias) "ComponentPtr"
}

impl RenderTree {

}

/// `Runtime` is a container for data and logic needed by the `Engine`,
/// explicitly aside from rendering.  For example, logic for managing
/// scopes, stack frames, and properties should live here.
pub struct Runtime {
    stack: Vec<Rc<RefCell<StackFrame>>>,
    logger: fn(&str),
}

impl Runtime {
    pub fn new(logger: fn(&str)) -> Self {
        Runtime {
            stack: Vec::new(),
            logger,
        }
    }

    pub fn log(&self, message: &str) {
        (&self.logger)(message);
    }

    /// Return a pointer to the top StackFrame on the stack,
    /// without mutating the stack or consuming the value
    pub fn peek_stack_frame(&mut self) -> Option<Rc<RefCell<StackFrame>>> {
        if self.stack.len() > 0 {
            Some(Rc::clone(&self.stack[&self.stack.len() - 1]))
        }else{
            None
        }
    }

    /// Remove the top element from the stack.  Currently does
    /// nothing with the value of the popped StackFrame.
    pub fn pop_stack_frame(&mut self){
        self.stack.pop(); //TODO: handle value here if needed
    }

    /// Add a new frame to the stack, passing a list of adoptees
    /// that may be handled by `Placeholder` and a scope that includes
    pub fn push_stack_frame(&mut self, adoptees: RenderNodePtrList, scope: Box<Scope>) {


        //TODO:  for all children inside `adoptees`, check whether child `should_flatten`.
        //       If so, retrieve the `RenderNodePtrList` for its children and splice that list
        //       into a working full `RenderNodePtrList`.  This should be done recursively until
        //       there are no more descendents who are "contiguously flat".

        let parent = self.peek_stack_frame();

        self.stack.push(
            Rc::new(RefCell::new(
                StackFrame::new(adoptees, Rc::new(RefCell::new(*scope)), parent)
            ))
        );
    }

}

//TODO:  do we need to refactor primitive properties (like Rectangle::width)
//       into the same `Property` structure as Components?
//          e.g. a `get_properties()` method
//       this would be imporant for addressing properties e.g. through
//       the property tree

/*
 Node {
    id: String
    properties: vec![
        (String.from("size"), PropertyLiteral {value: 500.0})
    ]
 }
 */
//
// TODO:
//  - Rename ScopeFrame to RepeatFrame
//  - make RepeatFrame<D(atum)> a component definition instead of a primitive
//     - this gives us a scope, stack frame, & data model for free
//       (just need to pass `i, datum` into the component declaration via Repeat's "template")
//  - What else do we need to do for <D> and <T> as they relate to Component?
//    (e.g. Component<D>?  What are the implications?)
//


pub trait RenderNode
{

    fn eval_properties_in_place(&mut self, ctx: &PropertyTreeContext);

    /// Lifecycle event: fires after evaluating a node's properties in place and its descendents properties
    /// in place.  Useful for cleaning up after a node (e.g. popping from the runtime stack) because
    /// this is the last time this node will be visited within the property tree for this frame.
    /// (Empty) default implementation because this is a rarely needed hook
    fn post_eval_properties_in_place(&mut self, ctx: &PropertyTreeContext) {}

    fn get_children(&self, ) -> RenderNodePtrList;

    /// Returns the size of this node, or `None` if this node
    /// doesn't have a size (e.g. `Group`)
    fn get_size(&self) -> Option<Size2D>;


    /// Rarely needed:  Used for exotic tree traversals, e.g. for `Spread` > `Repeat` > `Rectangle`
    /// where the repeated `Rectangle`s need to be be considered direct children of `Spread`.
    /// `Repeat` overrides `should_flatten` to return true, which `Engine` interprets to mean "ignore this
    /// node and consume its children" during traversal
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

    fn get_transform(&mut self) -> Rc<RefCell<Transform>>;
    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn render(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn post_render(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
}

pub struct Transform {
    pub translate: (Box<dyn Property<f64>>, Box<dyn Property<f64>>),
    pub scale: (Box<dyn Property<f64>>, Box<dyn Property<f64>>),
    pub rotate: Box<dyn Property<f64>>, //z-axis only for 2D rendering
    //TODO: add shear? needed at least to support ungrouping after scale+rotate
    pub origin: (Box<dyn Property<Size<f64>>>, Box<dyn Property<Size<f64>>>),
    pub align: (Box<dyn Property<f64>>, Box<dyn Property<f64>>),
    pub cached_computed_transform: Affine,
}

impl Default for Transform {
    fn default() -> Self {
        Transform{
            cached_computed_transform: Affine::default(),
            align: (Box::new(PropertyLiteral { value: 0.0 }), Box::new(PropertyLiteral { value: 0.0 })),
            origin: (Box::new(PropertyLiteral { value: Size::Pixel(0.0)}), Box::new(PropertyLiteral { value: Size::Pixel(0.0)})),
            translate: (Box::new(PropertyLiteral { value: 0.0}), Box::new(PropertyLiteral { value: 0.0})),
            scale: (Box::new(PropertyLiteral { value: 1.0}), Box::new(PropertyLiteral { value: 1.0})),
            rotate: Box::new(PropertyLiteral { value: 0.0 }),
        }
    }
}

impl Transform {

    pub fn eval_in_place(&mut self, ptc: &PropertyTreeContext) {
        &self.translate.0.eval_in_place(ptc);
        &self.translate.1.eval_in_place(ptc);
        &self.scale.0.eval_in_place(ptc);
        &self.scale.1.eval_in_place(ptc);
        &self.rotate.eval_in_place(ptc);
        &self.origin.0.eval_in_place(ptc);
        &self.origin.1.eval_in_place(ptc);
        &self.align.0.eval_in_place(ptc);
        &self.align.1.eval_in_place(ptc);

        //Note:  the final affine transform is NOT computed here in the Property Tree
        //       traversal, because it relies on rendering specific context, e.g. the
        //       node size and containing bounds. [Update: we are now passing bounds through
        //       the property tree context, so this may be worth revisiting.]
        //
        //       This is a somewhat awkward conceptual divide
        //       through the middle of `Transform`, especially since we ultimately
        //       want to cache the computed matrix for each node and only re-compute
        //       upon changes (making a call to calculate the transfrom from the `rendering` traversal
        //       look a LOT like a "special" eval_in_place kind-of call a la the `property` traversal.
        //
        //       If we revisit the design of Transform:
        //        - Consider that transform order would be nice to specify in userland, without needing a special magical API (ideally as expressive as Affine::translate() * Affine::scale())
        //        - See if we can better draw the boundaries between AUTHOR-TIME properties (translate/scale/rotate/origin/align)
        //          and RENDER-TIME properties (bounds, node size, computed matrix)
        //        - Take a closer look at the implementation of Affine in `kurbo`, e.g.
        //          the deref + multiplication behavior.  That approach, plus
        //          support for origin & align (and shear!) would be very nice.

    }

    //TODO:  if providing bounds is prohibitive or awkward for some use-case,
    //       we can make `bounds` and `align` BOTH TOGETHER optional — align requires `bounds`
    //       but it's the only thing that requires `bounds`

    //Distinction of note: scale, translate, rotate, origin, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    pub fn compute_transform_in_place(&mut self, node_size: (f64, f64), container_bounds: (f64, f64)) -> &Affine {
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


pub struct Component {
    pub template: Rc<RefCell<Vec<RenderNodePtr>>>,
    pub transform: Rc<RefCell<Transform>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
}

impl RenderNode for Component {
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //TODO: handle each of Component's `Expressable` properties
        //  - this includes any custom properties (inputs) passed into this component

        //TODO:  support adoptees here.  Currently hard-coding an empty vec
        ptc.runtime.borrow_mut().push_stack_frame(
            Rc::new(RefCell::new(vec![])),
              Box::new(Scope {
                  properties: Rc::clone(&self.properties)
              })
        );
    }

    fn post_eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //clean up the stack frame for the next component
        ptc.runtime.borrow_mut().pop_stack_frame();
    }

    fn get_children(&self) -> RenderNodePtrList {
        //Perhaps counter-intuitively, `Component`s return the root
        //of their template, rather than their `children`, for calls to get_children
        Rc::clone(&self.template)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
    fn pre_render(&mut self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
}

pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
}

#[derive(Copy, Clone)]
pub enum Size<T> {
    Pixel(T),
    Percent(T),
}

pub struct Rectangle {
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
    pub stroke: Stroke,
    pub fill: Box<dyn Property<Color>>,
}

impl RenderNode for Rectangle {
    fn get_children(&self) -> RenderNodePtrList {
        Rc::new(RefCell::new(vec![]))
    }
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        self.size.borrow_mut().0.eval_in_place(ptc);
        self.size.borrow_mut().1.eval_in_place(ptc);
        self.fill.eval_in_place(ptc);
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
    fn pre_render(&mut self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn render(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounding_dimens;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let fill: &Color = &self.fill.read();

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = *transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        rc.fill(transformed_bez_path, fill);
        rc.stroke(duplicate_transformed_bez_path, &self.stroke.color, self.stroke.width);
    }
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
}


pub struct If {

}

pub type Size2D = Rc<RefCell<(
    Box<dyn Property<Size<f64>>>,
    Box<dyn Property<Size<f64>>>,
)>>;

pub struct Size2DFactory {}

impl Size2DFactory {
    pub fn Literal(x: Size<f64>, y: Size<f64>) -> Size2D {
        Rc::new(RefCell::new(
            (
                Box::new(
                    PropertyLiteral { value: x }
                ),
                Box::new(
                    PropertyLiteral { value: y }
                )
            )
        ))
    }
    pub fn default() -> Size2D {
       Rc::new(RefCell::new(
            (
                Box::new(
                    PropertyLiteral { value: Size::Percent(100.0) }
                ),
                Box::new(
                    PropertyLiteral { value: Size::Percent(100.0) }
                )
            )
        ))
    }
}

