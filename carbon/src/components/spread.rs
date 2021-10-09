use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, PolymorphicType, PolymorphicValue, Property, PropertyExpression, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTree, RenderTreeContext, Size, Variable, wrap_render_node_ptr_into_list, PropertyLiteral, Scope, Repeat, Rectangle, Color, Stroke, StrokeStyle, Evaluator, StackFrame, InjectionContext, decompose_render_node_ptr_list_into_vec, Transform, RepeatProperties, StackUnion};
use crate::primitives::placeholder::Placeholder;
use crate::primitives::frame::Frame;
use std::any::Any;
use std::mem::ManuallyDrop;


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


pub struct Spread<D> {
    pub children: RenderNodePtrList,
    pub id: String,
    pub size: (
        Box<dyn Property<Size<f64>>>,
        Box<dyn Property<Size<f64>>>,
    ),
    pub transform: Transform,
    pub properties: Rc<StackUnion<D>>,

    template: RenderNodePtrList,
}

/* FUTURE CODEGEN VIA MACRO */
struct RepeatInjectorMacroExpression<T> {
    pub variadic_evaluator: fn(scope: Rc<RepeatProperties<SpreadCellProperties>>) -> T,
}

impl<T> RepeatInjectorMacroExpression<T> {}

impl<T, D> Evaluator<T, D> for RepeatInjectorMacroExpression<T> {
    fn inject_and_evaluate(&self, ic: &InjectionContext<D>) -> T {
        //TODO:CODEGEN


        //Perhaps there's a single stack frame with all `n` of the RepeatProperties<D> for
        //this data set.  To get the right `properties_borrowed`, we then need to peek the stack
        //frame and look up the all_the_properties[i] -- but how do we know `i` here?
        //Alternatively, `Repeat` can push a stack frame on each iteration? and pop it before the next



        let stack_frame = &ic.stack_frame;
        let stack_frame_borrowed = stack_frame.borrow();
        let scope = &stack_frame_borrowed.get_scope();
        let scope_borrowed = scope.borrow();
        // let properties_as_any = &scope_borrowed.properties as &dyn Any;
        let repeat_properties = Rc::clone( unsafe { &scope_borrowed.properties.repeat_spread_cell });

        // ic.engine.runtime.borrow()
        //     .log(
        //         &format!("Any type {}", type_name_of_val( properties_as_any))
        //     );
        /// DEBUG:
        //print the `type ID` of our received Any type
        //along with the `type ID`s of suspected stand-ins, incl. their references.
        //
        // properties_as_any.type_id().
        // let suspect_a : &Rc<RefCell<


        /// END DEBUG
        ///
        /// Conclusion of the `Any` foray:  we won't be able
        /// to downcast into an Rc, making it a non-viable path
        ///
        /// Another possible direction:
        /// invert the roles of data and evaluation —
        /// turn `evaluate_property(&str)` into a trait method
        /// on stack frames...
        /// A token can be
        ///  - a property name (like an implicit &self.name for the token `name`)
        ///  - a child node id (like an implicit &self.children.get_by_id(id) for the token `id`)
        ///  - a nested descendent (child.child) or descendent-property (child.property)

        /// We can create a higher-order data structure for a Property
        /// Will we be able to store polymorphic Properties elegantly alongside each other?
        /// Are we back to union-typed
        ///
        /// Yet another possible direction:  make each stack frame a big fat union type, that is:
        /// for all polymorphic "scope shapes" in an application (e.g. RepeatProperties, SpreadProperties)
        /// build a single mega-structure that contains memory (a field) for every one of those scopes
        /// and attach an instance of that mega-structure to each stack frame.
        /// Then:  each component knows how to pack and unpack its specific "scope shape"
        ///        into/out-of that mega structure.
        ///        That mega-structure declaration & maintenance can ultimately be managed with macros.
        /// Question: will this make for an unmanageable memory footprint?
        ///           will the bloated memory footprint become a bottleneck at any point for iteration, or e.g. stack traversal?
        ///           will there be a realistic real-world use-case with so many distinct scopes * so deep a call stack that we exceed
        ///           Napkin math:  imagine a component tree 500 elements deep (huge, but maybe possible e.g. in a heavily componentized app)
        ///                         then imagine that each distinct "scope shape" (property set) contains 100kb of data
        ///                         mega_structure_size = (500 * 100) => 50.000 MB
        ///                         total_runtime_memory_footpring = 500 * 50.0 MB => 25,000.00 MB
        ///           Memory would grow quadratically with the depth of component tree
        ///           This might be viable for the short term, AND there's likely a path forward to a smaller footprint
        ///           using a combo of macros and `unsafe` (each stack frame becomes the size of the "single largest stack frame," then perform an unsafe cast on each blob of memory)
        ///           Perhaps a simple version of this can be achieved with Rust's union types
        ///
        /// What if we limit the possible types for Properties?
        /// For example, if Properties can be only one of {String, Number, Boolean}, arrays, and MAYBE JS-hash-like nested {string => {String, Number, Boolean}}
        /// This would enable us to decorate plain-ol Rust structs with a macro
        /// that introspects the definition and builds a string-keyed lookup table


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


        (self.variadic_evaluator)(repeat_properties)
    }
}

/* END FUTURE CODEGEN VIA MACRO */


pub struct SpreadProperties {
    pub gutter: Size<f64>,
    pub cell_size_spec: Option<Vec<Size<f64>>>,
}

pub(crate) struct SpreadCellProperties {
    //TODO: map correct types to our relevant transforms
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}


impl Spread {
    pub fn new(
        children: RenderNodePtrList,
        id: String,
        size: (Box<dyn Property<Size<f64>>>, Box<dyn Property<Size<f64>>>),
        transform: Transform,
        properties: Rc<StackUnion>,
    ) -> Self {
        let child_data_list : Vec<Rc<SpreadCellProperties>> =
            children.borrow()
            .iter()
            .enumerate()
            .map(|(i, _rnp)| { Rc::new(SpreadCellProperties {
                height: 100,
                width: 100,
                x: 100,
                y: 100,
             }) })
            .collect();

        Spread {
            children,
            id,
            size,
            transform,
            properties,
            //private "component declaration" here, for template & variables

            template: Rc::new(RefCell::new(
                vec![
                    Rc::new(RefCell::new(
                        Repeat::new(
                            child_data_list ,
                            Rc::new(RefCell::new(vec![
                                Rc::new(RefCell::new(
                                    Frame {
                                        id: "spread_frame".to_string(),
                                        children: Rc::new(RefCell::new(vec![Rc::new(RefCell::new(
                                            Placeholder::new(
                                                "spread_frame_placeholder".to_string(),
                                                Transform::default(),
                                                Box::new(PropertyExpression {
                                                    cached_value: 0,
                                                    dependencies: vec!["engine".to_string()],
                                                    evaluator: RepeatInjectorMacroExpression {variadic_evaluator: |scope: Rc<RepeatProperties<SpreadCellProperties>>| -> usize {
                                                        scope.i
                                                    }}
                                                })
                                            )
                                        ))])),
                                        size: (
                                            Box::new(PropertyLiteral {value: Size::Pixel(50.0)}),
                                            Box::new(PropertyLiteral {value: Size::Pixel(50.0)}),
                                        ),
                                        transform: Transform::default(),
                                    }

                                ))
                            ])),
                            "id".to_string(),
                            Transform::default()
                        )
                    ))
                ]
            )),

            /*
            Rc::new(RefCell::new(
                Frame {
                    id: "cell_frame_left".to_string(),
                    align: (0.0, 0.0),
                    origin: (Size::Pixel(0.0), Size::Pixel(0.0), ),
                    size: (
                        (
                            Box::new(PropertyLiteral { value: Size::Percent(50.0) }),
                            Box::new(PropertyLiteral { value: Size::Percent(100.0) }),
                        )
                    ),
                    transform: Affine::default(),

                    children: Rc::new(RefCell::new(vec![
                        Rc::new(RefCell::new(
                            Placeholder::new("spread_frame_placeholder_left".to_string(), Affine::default(), 0)
                        )),
                    ])),
                }
            )),
            Rc::new(RefCell::new(
                Frame {
                    id: "cell_frame_right".to_string(),
                    align: (1.0, 0.0),
                    origin: (Size::Percent(100.0), Size::Pixel(0.0), ),
                    size: (
                        (
                            Box::new(PropertyLiteral { value: Size::Percent(50.0) }),
                            Box::new(PropertyLiteral { value: Size::Percent(100.0) }),
                        )
                    ),
                    transform: Affine::default(),

                    children: Rc::new(RefCell::new(
                        vec![
                            Rc::new(RefCell::new(
                                Placeholder::new("spread_frame_placeholder_right".to_string(), Affine::default(), 1)
                            )),
                        ])
                    ),
                }
            )),
             */
        }
    }
}



impl<D> RenderNode for Spread<D> {

    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext<D>) {
        //TODO: handle each of Spread's `Expressable` properties

        //TODO:  handle caching children/adoptees

        ptc.runtime.borrow_mut().push_stack_frame(
            Rc::clone(&self.children),
              Box::new(Scope {
                  properties: Rc::clone(&self.properties)
              })
        );
    }

    fn post_eval_properties_in_place(&mut self, ptc: &PropertyTreeContext<D>) {
        //clean up the stack frame for the next component
        ptc.runtime.borrow_mut().pop_stack_frame();
    }

    fn get_children(&self) -> RenderNodePtrList {

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
        // example Spread (prefab) render tree
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
        //(which are the adoptees)
        Rc::clone(&self.template)
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { Some((*self.size.0.read(), *self.size.1.read())) }

    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_transform_computed(&self) -> &Affine {
        &self.transform.cached_computed_transform
    }

    fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //TODO:  calc & memoize the layout/transform for each cell of the Sprad
        //       probably need to do the memoization via a RefCell for mutability concerns,
        //       since pre_render happens during immutable render tree recursion

        //Algo:
        // 1. determine number of adoptees (&self.children), `n`
        // 2. determine gutter, `g`
        // 3. determine bounding size, `(x,y)`

        match &self.properties.spread.cell_size_spec {
            //If a cell_size_spec is provided, use it.
            Some(cell_size_spec) => (),
            //Otherwise, calculate one
            None => {

            }
        }


    }

    fn render(&self, _sc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {
        //TODO:  render cell borders if appropriate
    }

    fn post_render(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {

    }

}

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
