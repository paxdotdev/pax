use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, Component, Evaluator, InjectionContext, PropertiesCoproduct, Property, PropertyLiteral, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, RepeatItem, Scope, Size, StackFrame, Transform, wrap_render_node_ptr_into_list, Stroke, StrokeStyle, Rectangle, Color, Size2DFactory, PropertyExpression, RepeatInjector, Placeholder, Frame};
use crate::rendering::Size2D;

pub struct Repeat {
    pub template: RenderNodePtrList,
    pub data_list: Box<Property<Vec<Rc<PropertiesCoproduct>>>>,
    pub transform: Rc<RefCell<Transform>>,

    //TODO: any way to make this legit-private along with the ..Default::default() syntax?
    pub _virtual_children: RenderNodePtrList,
}

pub struct RepeatProperties {

}

/// Data structure for the virtually duplicated container that surrounds repeated nodes.
/// This is attached to a Component<RepeatFrame> that `Repeat` adds to its children dynamically
/// during property-tree traversal
pub struct RepeatItemProperties {
    pub i: usize,
    pub datum: Rc<PropertiesCoproduct>,
    pub id: String,
}

impl Repeat {
}

impl Default for Repeat {
    fn default() -> Self {
        Repeat {
            template: Rc::new(RefCell::new(vec![])),
            data_list: Box::new(PropertyLiteral {value: vec![]}),
            transform: Default::default(),
            _virtual_children: Rc::new(RefCell::new(vec![]))
        }
    }
}

impl RenderNode for Repeat {
    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //TODO: handle each of Repeat's `Expressable` properties

        self.data_list.eval_in_place(rtc);
        self.transform.borrow_mut().eval_in_place(rtc);

        //reset children:
        //wrap data_list into repeat_items and attach "puppeteer" components that attach
        //the necessary data as stack frame context
        self._virtual_children = Rc::new(RefCell::new(
            self.data_list.read().iter().enumerate().map(|(i, datum)| {
                let properties = Rc::new(RefCell::new(
                    RepeatItem { i, datum: Rc::clone(datum)}
                ));

                let render_node : RenderNodePtr = Rc::new(RefCell::new(
                    Component {
                        adoptees: Rc::new(RefCell::new(vec![])),

                        //THIS IS THE PROBLEM!  Naive Rc::clone()ing causes
                        //  property evaluation caches to clobber each other.
                        //  The last one calculated "wins"
                        template: Rc::clone(&self.template),
                        transform: Rc::new(RefCell::new(Transform::default())),
                        properties: Rc::new(RefCell::new(PropertiesCoproduct::RepeatItem(properties))),
                    }
                ));

                //
                // let derefed = Rc::clone(datum);
                // let spread_cell_properties = match &*derefed {
                //     PropertiesCoproduct::SpreadCell(sc) => {
                //         sc
                //     }
                //     &_ => {panic!("ain't a spreadcell, ain't it?")}
                // };
                //
                // // let render_node : RenderNodePtr = Rc::new(RefCell::new(
                // //                     Rectangle {
                // //                             fill: Box::new(
                // //                                 // PropertyLiteral {value: Color::rgba(1.0, 0.0, 0.0, 1.0)}
                // //                                 PropertyLiteral{value: Color::rgba(1.0,0.0,0.0,1.0)}
                // //                             ),
                // //                             stroke: Stroke {
                // //                                 width: 4.0,
                // //                                 style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                // //                                 color: Color::rgba(0.0, 0.5, 0.5, 1.0)
                // //                             },
                // //                             size: Rc::new(RefCell::new((
                // //                                 Box::new(PropertyLiteral {value: Size::Pixel(spread_cell_properties.width_px)}),
                // //                                 Box::new(PropertyLiteral {value: Size::Pixel(spread_cell_properties.height_px)}),
                // //                             ))),
                // //                 transform: Rc::new(RefCell::new(
                // //                     Transform {
                // //                             translate: (
                // //                                 Box::new(PropertyLiteral {value: spread_cell_properties.x_px}),
                // //                                 Box::new(PropertyLiteral {value: spread_cell_properties.y_px}),
                // //                             ),
                // //                             ..Default::default()
                // //                         },
                // //                 )),
                // //                         }
                // //                     ));
                //
                // //Because the following works (!) it seems the culprit
                // //for this bug lies somewhere inside Component
                // let render_node : RenderNodePtr = Rc::new(RefCell::new(
                //     Frame {
                //         size: Rc::new(RefCell::new((
                //             Box::new(PropertyLiteral {value: Size::Pixel(spread_cell_properties.width_px)}),
                //             Box::new(PropertyLiteral {value: Size::Pixel(spread_cell_properties.height_px)}),
                //         ))),
                //         transform: Rc::new(RefCell::new(
                //             Transform {
                //                     translate: (
                //                         Box::new(PropertyLiteral {value: spread_cell_properties.x_px}),
                //                         Box::new(PropertyLiteral {value: spread_cell_properties.y_px}),
                //                     ),
                //                     ..Default::default()
                //                 },
                //         )),
                //         children: Rc::new(RefCell::new(vec![Rc::new(RefCell::new(
                //             Placeholder::new(
                //                 Transform::default(),
                //                 Box::new(PropertyLiteral {value: i})
                //             )
                //         ))]))
                //     }
                // ));


                // let render_node: RenderNodePtr = ;

                render_node
            }).collect()
        ));

        // rtc.runtime.borrow_mut().log(&format!("Computed virtual children, length{}", self._virtual_children.borrow().len()));

    }


    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self._virtual_children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
}


/*
lab journal, zb
---------------

To support polymorphic data <T> inside stack frames,
we need a `dyn SomeTrait` contract that stack frame data
can adhere to (instead of arbitrary `T`)

ex. `repeat` element stackframe data:
{
    index: usize,
    datum: T
}

We could have any stack frame abide by this contract:

StackFrameData<T> {
    get_index() -> usize;
    get_datum() -> Box<dyn T>;
}
...but how does the consumer know it's dealing with `T`?  Where does `T` come from?

Ultimately, it's userland.  E.g. consider the user-provided data:
cats = [{fur_color: Colors.WHITE, eye_color: Colors.BLUE}, {fur_color: Colors.BROWN, eye_color: Colors.GREEN}]
describes a schema and thus `T` of {fur_color: Color, eye_color: Color}

Perhaps this gets easier if we introduce our `scope` object here, and deal with a string:value (dynamic) lookup?

This turns our StackFrameData approach into:

StackFrame {
    get_scope() -> Scope;
}

along with

Scope {
    get_type(key: &str) -> PolymorphicType // used for unsafe unboxing of value
    get_value(key: &str) -> PolymorphicValue
}

When working with a Scope inside a `repeat`, the user looks up values & types by (string) key.

Seems like a suitable solution.

 */







        //Can we operate on a guarantee that for `n` elements in a repeat, the consumer (expression)
        //will be invoked exactly `n` times?  If so, we could push a stackframe for each datum (in reverse)
        //so that each invocation consumes a new stack frame, in order.  The tricky piece of this is
        //a need to introduce stack frame `pop`s somewhere before the post_eval_properties_in_place lifecycle
        //method, in a way that's unique to `repeat`.

        //An alternative approach to this problem, which operates with the grain of "one stack frame
        //per component instance," is to add an iterator to a new RepeatPropertiesContainer, which
        //yields the next `RepeatProperties` on each invocation.  This may require simply modifying
        //the inject_and_evaluate logic.  Perhaps we can introduce a `.next` method on Evaluator, with
        //a default implementation that's a no-op, but which Repeat can override to step through
        //an iterator.

        // rtc.runtime.borrow_mut().push_stack_frame(
        //     Rc::clone(&self.children),
        //       Box::new(Scope {
        //           properties: Rc::clone(&self.properties) as Rc<dyn Any>
        //       })
        // );