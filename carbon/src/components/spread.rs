use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, CarbonEngine, Color, Component, decompose_render_node_ptr_list_into_vec, Evaluator, InjectionContext, MyManualMacroExpression, PropertiesCoproduct, Property, PropertyExpression, PropertyLiteral, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTree, RenderTreeContext, Repeat, RepeatItem, RepeatItemProperties, Scope, Size, StackFrame, Stroke, StrokeStyle, Transform, wrap_render_node_ptr_into_list};
use crate::primitives::frame::Frame;
use crate::primitives::placeholder::Placeholder;
use crate::rectangle::Rectangle;
use crate::rendering::{Size2D, Size2DFactory};

/*
TODO:
    [x] decide on API design, expected GUI experience
        - Direction (horiz/vert)
        - Gutter
        - Cell widths
    [x] expose a Spread element for consumption by engine
    [x] accept children, just like primitives e.g. `Group`
    [x] author an internal template, incl. `placeholder`ing children and `repeating` inputs
        <Frame repeat=self.children transform=get_transform(i)>
            <Placeholder index=i>
        </Frame>
        - need to be able to define/call methods on containing class (a la VB)
        - need to figure out polymorphism, Vec<T> (?) for repeat
        - need to figure out placeholder — special kind of rendernode?
    [x] Frame
        [x] Clipping
    [x] Placeholder
    [x] Repeat
        [x] "flattening yield" to support <Spread><Repeat n=5><Rect>...
        [x] scopes:
            [x] `i`, `datum`
            [x] braced templating {} ? or otherwise figure out `eval`
                - Code-gen?  piece together strings into a file and run rustc on it?
                * Can achieve this with Expressions for now
            [x] calling "class methods" from templates, e.g. <Repeat n=5><Rect color="get_color(i)"
                * Can achieve with expressions
    [ ] Scopes & DI
        [ ] Figure out dissonance between:  1. string based keys, 2. struct `Properties`
            and figure out how this plays out into the property DI mechanism.  Along the way,
            figure out how to inject complex objects (ideally with a path forward to a JS runtime.)
                - Option A:  write a macro that decorates expression definitions (or _is_ the exp. def.) and (if possible)
                             generates the necessary code to match param names to elsewhere-registered dependency streams (Angular-style)
                - Option A0: don't write the macro (above, A) yet, but write the expanded version of it by hand, incl. string deps (present-day approach)
                             ^ to achieve "DI," does this require a hand-rolled "monomorphization" of each expression invocation?
                             Or does a string dependency list suffice?  If the latter, we must solve a way to pass a data-type <D> through PolymorphicValue
                             Probably the answer is the _former_ — write the DI-binding logic manually alongside a string dep list (roughly how the macro unrolling will work)
        [ ] Support getting self (or Scope) for access to Repeat Data
            - use-case: translate each element within a `repeat` by `i * k`
        [ ] Quick pass on other relevant data to get from Scopes
    [ ] Layout
        [ ] Primary logic + repeat via expression
        [ ] Parameterize:
            - Gutter
            - Size specs
 */


//EXAMPLE SPREAD COMPONENT
// <Component>
//      <Metadata />
//      <Template>
//          <Repeat declarations={{(i, elem)}} iterable={{get_children()}}>
//              <Frame transform={{get_frame_transform(i)}} size={{get_frame_size(i)}}>
//                  <Placeholder index={{i}}>
//              </Frame>
//          <Repeat/>
//      </Template>
// </Component>



pub struct Spread {
    properties: Rc<RefCell<SpreadProperties>>,
    template: RenderNodePtrList,
}

impl Spread {
    pub fn new(properties: Rc<RefCell<SpreadProperties>>, adoptees: RenderNodePtrList) -> Self {
        //Component must be accessible so that we can unwrap its properties
        //Template is a "higher template" that belongs to Spread, not Component —
        //  this is the root of Spread's own rendering, and is what should be returned
        //  by get_children.  note that it includes a pointer (Rc) to `self.component` as well.


        let component: RenderNodePtr = Rc::new(RefCell::new(
            Component {
                template: init_and_retrieve_template(),
                adoptees,
                transform: Rc::new(RefCell::new(Default::default())),
                properties: Rc::new(RefCell::new(PropertiesCoproduct::Spread(Rc::clone(&properties)))),
            }
        ));
        let template: RenderNodePtrList = Rc::new(RefCell::new(vec![
            Rc::clone(&component)
        ]));

        Spread {
            template,
            properties,
        }
    }
}

pub struct SpreadProperties {
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
    pub cell_count: Box<dyn Property<usize>>,
    pub gutter_width: Box<dyn Property<Size<f64>>>,

    //These two data structures act as "sparse maps," where
    //the first element in the tuple is the index of the cell/gutter to
    //override and the second is the override value.  In the absence
    //of overrides (`vec![]`), cells and gutters will divide space
    //evenly.
    //TODO: these should probably be Expressable
    pub overrides_cell_size: Vec<(usize, Size<f64>)>,
    pub overrides_gutter_size: Vec<(usize, Size<f64>)>,

    //storage for memoized layout calc
    //TODO: any way to make this legit private while supporting `..Default::default()` ergonomics?
    pub _cached_computed_layout_spec: Vec<Rc<SpreadCellProperties>>,
}

impl Default for SpreadProperties {
    fn default() -> Self {
        SpreadProperties {
            size: Size2DFactory::default(),
            transform: Default::default(),
            cell_count: Box::new(PropertyLiteral{value: 0}),
            gutter_width: Box::new(PropertyLiteral{value: Size::Pixel(0.0)}),
            _cached_computed_layout_spec: vec![],
            overrides_cell_size: vec![],
            overrides_gutter_size: vec![],
        }
    }
}

impl SpreadProperties {
    pub fn eval_in_place(&mut self, rtc: &RenderTreeContext) {
        &self.size.borrow_mut().0.eval_in_place(rtc);
        &self.size.borrow_mut().1.eval_in_place(rtc);
        &self.cell_count.eval_in_place(rtc);
        &self.gutter_width.eval_in_place(rtc);
        &self.transform.borrow_mut().eval_in_place(rtc);
        &self.calc_layout_spec_in_place(rtc);
    }

    pub fn calc_layout_spec_in_place(&mut self, rtc: &RenderTreeContext) {
        let bounds = rtc.bounds;
        let gutter_calc = match *self.gutter_width.read() {
            Size::Pixel(px) => px,
            Size::Percent(pct) => bounds.0 * (pct / 100.0),
        };
        let cell_count = *self.cell_count.read() as f64;

        //hard-coding horizontal to start
        let usable_interior_space = bounds.0 - (cell_count + 1.0) * gutter_calc;
        let per_cell_space = usable_interior_space / cell_count;

        //TODO: account for overrides
        self._cached_computed_layout_spec = (0..(cell_count as usize)).into_iter().map(|i| {

            // rtc.runtime.borrow_mut().log(&format!("Caching computed cells: {}", ((i + 1) as f64) * (gutter_calc) + (i as f64) * per_cell_space));
            Rc::new(SpreadCellProperties {
                height_px: bounds.1,
                width_px: per_cell_space,
                x_px: ((i + 1) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                y_px: 0.0,
            })
        }).collect();

    }
}


impl RenderNode for Spread {

    fn get_rendering_children(&self) -> RenderNodePtrList {

        // return the root of the internal template here — as long
        // as we capture refs to (c) and (d) below during Spread's `render` or `pre_render` fn,
        // we can happily let rendering just take its course,
        // recursing through the subtree starting with (e).
        //
        // example application render tree
        //          a( root )
        //              |
        //          b( Spread )
        //         /          \
        //    c( Rect )      d( Rect )
        //
        // example Spread (component) template render tree
        //          e( root )
        //              |         //  expanded:
        //          f( Repeat  .. //  n=2 )
        //           /            //      \
        //      g( Frame )        //     i( Frame )
        //          |             //        |
        //      h( Placeholder )  //     j( Placeholder )
        //
        // traversal order:
        // [a b e f g h c i j d]
        //
        // a: load the application root Group
        // b: found a Spread, start rendering it
        //    get its children from the Engine (get_children)
        //    — these are the `adoptees` that will be passed to `Placeholder`
        //    and they need to be tracked.
        //    We can do this with a RenderNodeContext that we pass between recursive calls
        //    when rendering.  We can keep a stack of prefab "scopes," allowing `placeholder`'s render
        //    function to handily grab a reference to `adoptees[i]` when needed.  The stack
        //    enables components to nest among themselves
        // e: is Spread::render()
        // f: first child of Spread — it's a Repeat;
        //    loop twice, first passing rendering onto a Frame (g), waiting for it to return,
        //    then passing onto the next Frame (i)
        // g: render the containing frame in the correct position,
        //    (plus clipping, maybe)
        // h: needs to "evaluate" into the rectangle itself — directs the
        //    flow of the render tree to (c) via the Context described in (b)
        // c: finally render the rectangle itself; return & allow recursion to keep whirring
        // i,j,d: repeat g,h,c

        //return root of internal template here, instead of `self.children`
        //(which are the adoptees provided by instantiator)


        //THIS!!!!!!
        //We're retuning a NEW INSTANCE of a component on every frame.
        //Thus the fresh new instance is stuck on the cached value (maybe?)
        //To fix, try: returning a ref to an existing, non-volatile Component and template
        //(like the old approach that attached template to the local struct)


        Rc::clone(&self.template)
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.properties.borrow().size)) }

    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.properties.borrow().transform) }

    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        self.properties.borrow_mut().eval_in_place(rtc);
    }
}



pub struct SpreadCellProperties {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

impl Default for SpreadCellProperties{
    fn default() -> Self {
        SpreadCellProperties {
            x_px: 0.0,
            y_px: 0.0,
            width_px: 0.0,
            height_px: 0.0,
        }
    }
}




/*
TODO:  figure out why expressions aren't evaluating beyond the root

Findings so far:
     - Property tree traversal is working, as evidenced by "accumulated bounds" logging
     - When we set up a simple expression for bg color of a Rectangle inside Spread,
       we observe that the color doesn't update beyond the cached value.  Th



 */



fn init_and_retrieve_template() -> RenderNodePtrList {
    Rc::new(RefCell::new(
        vec![
            // Rc::new(RefCell::new(
            // Rectangle {
            //         transform: Rc::new(RefCell::new(Transform::default())),
            //         fill: Box::new(
            //             // PropertyLiteral {value: Color::rgba(1.0, 0.0, 0.0, 1.0)}
            //             PropertyExpression {
            //                 cached_value: Color::hlc(1.0,75.0,75.0),
            //                 evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
            //                     Color::hlc((engine.frames_elapsed % 360) as f64, 75.0, 75.0)
            //                 }}
            //             }
            //         ),
            //         stroke: Stroke {
            //             width: 4.0,
            //             style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
            //             color: Color::rgba(0.0, 0.5, 0.5, 1.0)
            //         },
            //         size: Size2DFactory::Literal(Size::Percent(100.0), Size::Percent(100.0)),
            //     }
            // )),

            Rc::new(RefCell::new(
                Repeat {
                    // data_list: Box::new(PropertyLiteral {value: vec![
                    //     Rc::new(PropertiesCoproduct::SpreadCell(Rc::new(SpreadCellProperties{height_px: 100.0, width_px: 100.0, x_px: 0.0, y_px: 0.0}))),
                    //     Rc::new(PropertiesCoproduct::SpreadCell(Rc::new(SpreadCellProperties{height_px: 140.0, width_px: 200.0, x_px: 150.0, y_px: 150.0}))),
                    // ]}),
                    data_list: Box::new(PropertyExpression {
                        cached_value: vec![],
                        evaluator: SpreadPropertiesInjector {variadic_evaluator: |scope: Rc<RefCell<SpreadProperties>>| -> Vec<Rc<PropertiesCoproduct>> {
                            scope.borrow()._cached_computed_layout_spec.iter()
                                .map(|scp|{Rc::new(PropertiesCoproduct::SpreadCell(Rc::clone(scp)))}).collect()
                        }}
                    }),
                    template: Rc::new(RefCell::new(vec![
                        Rc::new(RefCell::new(
                            Frame {
                                size: Rc::new(RefCell::new((
                                    Box::new(PropertyExpression {
                                        cached_value: Size::Pixel(100.0),
                                        evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> Size<f64> {
                                            //TODO:  unwrap SpreadCell from the repeat-item.
                                            //       make this part of the expression! macro
                                            match &*scope.borrow().datum {
                                                PropertiesCoproduct::SpreadCell(sc) => {
                                                    Size::Pixel(sc.width_px)
                                                },
                                                _ => panic!("Unknown property coproduct")
                                            }
                                        }}
                                    }),
                                    Box::new(PropertyExpression {
                                        cached_value: Size::Pixel(100.0),
                                        evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> Size<f64> {
                                            //TODO:  unwrap SpreadCell from the repeat-item.
                                            //       make this part of the expression! macro
                                            match &*scope.borrow().datum {
                                                PropertiesCoproduct::SpreadCell(sc) => {
                                                    Size::Pixel(sc.height_px)
                                                },
                                                _ => panic!("Unknown property coproduct")
                                            }
                                        }}
                                    }),
                                ))),
                                transform: Rc::new(RefCell::new(
                                    Transform {
                                            translate: (
                                                Box::new(PropertyExpression {
                                                    cached_value: 0.0,
                                                    evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> f64 {
                                                        //TODO:  unwrap SpreadCell from the repeat-item.
                                                        //       make this part of the expression! macro
                                                        match &*scope.borrow().datum {
                                                            PropertiesCoproduct::SpreadCell(sc) => {
                                                                sc.x_px
                                                            },
                                                            _ => panic!("Unknown property coproduct")
                                                        }
                                                    }}
                                                }),
                                                Box::new(PropertyExpression {
                                                    cached_value: 0.0,
                                                    evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> f64 {
                                                        //TODO:  make this part of the expression! macro
                                                        match &*scope.borrow().datum {
                                                            PropertiesCoproduct::SpreadCell(sc) => {
                                                                sc.y_px
                                                            },
                                                            _ => panic!("Unknown property coproduct")
                                                        }
                                                    }}
                                                })
                                            ),
                                            ..Default::default()
                                        },
                                )),
                                children: Rc::new(RefCell::new(vec![
                                    // Rc::new(RefCell::new(
                                    //                 Rectangle {
                                    //                     transform: Rc::new(RefCell::new(Transform::default())),
                                    //                     fill: Box::new(
                                    //                         PropertyExpression {
                                    //                             cached_value: Color::hlc(0.0,0.0,0.0),
                                    //                             // expression!(|engine: &CarbonEngine| ->
                                    //                             evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
                                    //                                 Color::hlc((engine.frames_elapsed % 360) as f64, 75.0, 75.0)
                                    //                             }}
                                    //                         }
                                    //                     ),
                                    //                     stroke: Stroke {
                                    //                         width: 4.0,
                                    //                         style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    //                         color: Color::rgba(0.0, 0.0, 1.0, 1.0)
                                    //                     },
                                    //                     size: Size2DFactory::Literal(Size::Percent(100.0), Size::Percent(100.0)),
                                    //                 }
                                    //             )),
                                    Rc::new(RefCell::new(
                                    Rectangle {
                                            transform: Rc::new(RefCell::new(Transform::default())),
                                            fill: Box::new(
                                                // PropertyLiteral {value: Color::rgba(1.0, 0.0, 0.0, 1.0)}
                                                PropertyExpression {
                                                    cached_value: Color::hlc(1.0,75.0,75.0),
                                                    evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
                                                        Color::hlc((engine.frames_elapsed % 360) as f64, 75.0, 75.0)
                                                    }}
                                                }
                                            ),
                                            stroke: Stroke {
                                                width: 4.0,
                                                style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                color: Color::rgba(0.0, 0.5, 0.5, 1.0)
                                            },
                                            size: Size2DFactory::Literal(Size::Percent(100.0), Size::Percent(100.0)),
                                        }
                                    )),
                        //             Rc::new(RefCell::new(
                        //                 Placeholder::new(
                        //                     Transform::default(),
                        //                     Box::new(PropertyExpression {
                        //                         cached_value: 0,
                        //                         evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> usize {
                        //                             //TODO:  unwrap SpreadCell from the repeat-item.
                        //                             //       make this part of the expression! macro
                        //                             scope.borrow().i
                        //                         }}
                        //                     })
                        //                 )
                        //             ))

                                ])),

                            }
                        ))
                    ])),
                    ..Default::default()
                }
            ))
        ]
    ))
}




/* FUTURE CODEGEN VIA MACRO [MAYBE?] */
pub(crate) struct RepeatInjector<T> {
    pub variadic_evaluator: fn(scope: Rc<RefCell<RepeatItem>>) -> T,
}

impl<T> RepeatInjector<T> {}

impl<T> Evaluator<T> for RepeatInjector<T> {
    fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
        //TODO:CODEGEN

        let stack_frame = &ic.stack_frame;
        let stack_frame_borrowed = stack_frame.borrow();
        let scope = &stack_frame_borrowed.get_scope();
        let scope_borrowed = scope.borrow();
        let repeat_properties = Rc::clone(  &scope_borrowed.properties );

        let unwrapped_repeat_properties = match &*repeat_properties.borrow() {
            PropertiesCoproduct::RepeatItem(rs) => {
                Rc::clone(rs)
            },
            _ => {
                panic!("Unexpected type.");
            }
        };

        (self.variadic_evaluator)(unwrapped_repeat_properties)
    }
}
/* END FUTURE CODEGEN VIA MACRO */
/* MORE CODEGEN? */

struct SpreadPropertiesInjector<T> {
    pub variadic_evaluator: fn(scope: Rc<RefCell<SpreadProperties>>) -> T,
}

impl<T> SpreadPropertiesInjector<T> {}

impl<T> Evaluator<T> for SpreadPropertiesInjector<T> {
    fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
        //TODO:CODEGEN

        let stack_frame = &ic.stack_frame;
        let stack_frame_borrowed = stack_frame.borrow();
        let scope = &stack_frame_borrowed.get_scope();
        let scope_borrowed = scope.borrow();
        let properties = Rc::clone(  &scope_borrowed.properties );

        let unwrapped_properties = match &*properties.borrow() {
            PropertiesCoproduct::Spread(rs) => {
                Rc::clone(rs)
            },
            _ => {
                panic!("Unexpected type.");
            }
        };

        (self.variadic_evaluator)(unwrapped_properties)
    }
}

/* END MORE CODEGEN */
/* Things that a component needs to do, beyond being just a rendernode
    - declare a list of Properties [maybe this should be the same as RenderNodes?? i.e. change RenderNode to be more like this]
    - push a stackframe during pre_render and pop it during post_render
    - declare a `template` that's separate from its `children`
*/

/* Thinking ahead briefly to the userland component authoring experience:
    - Importantly, every application's `root` is a _component definition_,
      including custom properties (component inputs), timelines, event handlers, and a template
    - What does that declaration look like?

    // No "children;" just template. Children are passed by consumer, e.g. web or ios codebase
    // EXAMPLE ROOT COMPONENT (application definition, "bin crate")
    <Component>
        <Metadata />
        <Template>
            ``` special {{}} support in here?
            <Group>
                <SomeOtherCustomComponent>
                    <Rect />
                    <Rect />
                    <Rect />
                </SomeOtherCustomComponent>
                <Rect>
                ...
            </Group>
            ```
        </Template>
        <Properties>
            //int numClicks = 0;
            //support piecewise property definitions here for timelines?
            // or are timelines a separate concept?
            // ** perhaps `defaults` are specified here, and `sparse overrides`
            // are implemented by timelines **
        </Properties>
        <Timeline>
            //default timeline is implicit — multiple (modal) timelines could be supported
            //units are frames, NOT ms
            0: //collection of (PropertyKeyframe | EventHandler)
            15:
        </Timeline>
        <EventHandlers>
        </EventHandlers>
    </Component>

    //EXAMPLE SPREAD COMPONENT
    <Component>
        <Metadata />
        <Template>
            <Repeat declarations={{(i, elem)}} iterable={{get_children()}}>
                <Frame transform={{get_frame_transform(i)}} size={{get_frame_size(i)}}>
                    <Placeholder index={{i}}>
                </Frame>
            <Repeat/>
        </Template>
    </Component>


// for={for (i, elem) in (&self) -> { &self.get_children()}.iter().enumerate()}
 */





        //
        // let repeat_properties = Rc::clone(
        //     properties_as_any
        //     .downcast_ref::<Rc<RepeatProperties<SpreadCellProperties>>>()
        //     .unwrap()
        // );



        /* TODO:  evaluate if this Any/downcasting approach could work for us:
            fn main() {
                let a: Box<dyn Any> = Box::new(B);
                let _: &B = match a.downcast_ref::<B>() {
                    Some(b) => b,
                    None => panic!("&a isn't a B!")
                };
            }


            OR


            use std::any::Any;

            trait A {
                fn as_any(&self) -> &dyn Any;
            }

            struct B;

            impl A for B {
                fn as_any(&self) -> &dyn Any {
                    self
                }
            }

            fn main() {
                let a: Box<dyn A> = Box::new(B);
                // The indirection through `as_any` is because using `downcast_ref`
                // on `Box<A>` *directly* only lets us downcast back to `&A` again.
                // The method ensures we get an `Any` vtable that lets us downcast
                // back to the original, concrete type.
                let b: &B = match a.as_any().downcast_ref::<B>() {
                    Some(b) => b,
                    None => panic!("&a isn't a B!"),
                };
            }
        */


        /* Other options:
            - Eliminate variadic expressions; instead just always pass the same fat args (e.g. Engine, Scope)
            - do some unsafe casting, enforced "safe" by our own runtime stack, a makeshift polymorphic smart pointer mechanism
         */






// repeat_properties
        // ic.engine.runtime.borrow()
        //     .log(
        //         &format!("Any type {}", type_name_of_val( properties_as_any))
        //     );

        //print the `type ID` of our received Any type
        //along with the `type ID`s of suspected stand-ins, incl. their references.
        //
        // properties_as_any.type_id().
        // let suspect_a : &Rc<RefCell<


        // END DEBUG
        //
        // Conclusion of the `Any` foray:  we won't be able
        // to downcast into an Rc, making it a non-viable path
        //
        // Another possible direction:
        // invert the roles of data and evaluation —
        // turn `evaluate_property(&str)` into a trait method
        // on stack frames...
        // A token can be
        //  - a property name (like an implicit &self.name for the token `name`)
        //  - a child node id (like an implicit &self.children.get_by_id(id) for the token `id`)
        //  - a nested descendent (child.child) or descendent-property (child.property)
        //
        // We can create a higher-order data structure for a Property
        // Will we be able to store polymorphic Properties elegantly alongside each other?
        // Are we back to union-typed
        //
        // Yet another possible direction:  make each stack frame a big fat union type, that is:
        // for all polymorphic "scope shapes" in an application (e.g. RepeatProperties, SpreadProperties)
        // build a single mega-structure that contains memory (a field) for every one of those scopes
        // and attach an instance of that mega-structure to each stack frame.
        // Then:  each component knows how to pack and unpack its specific "scope shape"
        //        into/out-of that mega structure.
        //        That mega-structure declaration & maintenance can ultimately be managed with macros.
        // Question: will this make for an unmanageable memory footprint?
        //           will the bloated memory footprint become a bottleneck at any point for iteration, or e.g. stack traversal?
        //           will there be a realistic real-world use-case with so many distinct scopes * so deep a call stack that we exceed
        //           Napkin math:  imagine a component tree 500 elements deep (huge, but maybe possible e.g. in a heavily componentized app)
        //                         then imagine that each distinct "scope shape" (property set) contains 100kb of data
        //                         mega_structure_size = (500 * 100) => 50.000 MB
        //                         total_runtime_memory_footpring = 500 * 50.0 MB => 25,000.00 MB
        //           Memory would grow quadratically with the depth of component tree
        //           This might be viable for the short term, AND there's likely a path forward to a smaller footprint
        //           using a combo of macros and `unsafe` (each stack frame becomes the size of the "single largest stack frame," then perform an unsafe cast on each blob of memory)
        //           Perhaps a simple version of this can be achieved with Rust's union types
        //
        // What if we limit the possible types for Properties?
        // For example, if Properties can be only one of {String, Number, Boolean}, arrays, and MAYBE JS-hash-like nested {string => {String, Number, Boolean}}
        // This would enable us to decorate plain-ol Rust structs with a macro
        // that introspects the definition and builds a string-keyed lookup table
