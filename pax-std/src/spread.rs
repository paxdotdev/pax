use std::rc::Rc;
use pax::*;
use pax::api::{Size2D, Size, ArgsRender, Property};
use crate::primitives::Frame;
use crate::types::{SpreadDirection, SpreadCellProperties};

/// A layout component which renders a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  Spreads can be stacked inside of each other, horizontally
/// and vertically, alongside `Transform.align` and `Transform.origin` to achieve any 2D layout.
#[pax(
    @template {
        @for (elem, i) in self.computed_layout_spec {
            <Frame transform=@{translate(elem.x_px, elem.y_px)} size=@{[elem.width_px, elem.height_px]}>
                <Placeholder index=@i>
            </Frame>
        }
    }
)]
pub struct Spread {
    pub computed_layout_spec: Property<Vec<Rc<SpreadCellProperties>>>,
    pub direction: Property<SpreadDirection>,
    pub cell_count: Property<usize>,
    pub gutter_width: Property<Size>,

    //These two data structures act as "sparse maps," where
    //the first element in the tuple is the index of the cell/gutter to
    //override and the second is the override value.  In the absence
    //of overrides (`vec![]`), cells and gutters will divide space evenly.
    pub overrides_cell_size: Property<Vec<(usize, Size)>>,
    pub overrides_gutter_size: Property<Vec<(usize, Size)>>,
}


impl Spread {

    #[pax_on(pre_render)]
    pub fn handle_pre_render(&mut self, args: ArgsRender) {
        // pax::log("handling pre-render from Spread");
        //TODO: dirty check
        let bounds = args.bounds;

        let active_bound = match *self.direction.get() {
            SpreadDirection::Horizontal => bounds.0,
            SpreadDirection::Vertical => bounds.1
        };

        let gutter_calc = match *self.gutter_width.get() {
            Size::Pixel(px) => px,
            Size::Percent(pct) => active_bound * (pct / 100.0),
        };

        let cell_count = *self.cell_count.get() as f64;

        let usable_interior_space = active_bound - (cell_count + 1.0) * gutter_calc;
        let per_cell_space = usable_interior_space / cell_count;

        //TODO: account for overrides
        self.computed_layout_spec.set((0..(cell_count as usize)).into_iter().map(|i| {
            match self.direction.get() {
                SpreadDirection::Horizontal =>
                    Rc::new(SpreadCellProperties {
                        height_px: bounds.1,
                        width_px: per_cell_space,
                        x_px: ((i + 1) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                        y_px: 0.0,
                    }),
                SpreadDirection::Vertical =>
                    Rc::new(SpreadCellProperties {
                        height_px: per_cell_space,
                        width_px: bounds.0,
                        x_px: 0.0,
                        y_px: ((i + 1) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                    }),
            }
        }).collect());

    }


}

//
// impl RenderNode for SpreadInstance {

// pub fn instantiate(args: InstantiationArgs) -> Self {
//     //Component must be accessible so that we can unwrap its properties
//     //Template is a "higher template" that belongs to Spread, not Component —
//     //  this is the root of Spread's own rendering, and is what should be returned
//     //  by get_children.
//
//     // let properties = if let PropertiesCoproduct::Spread(p) = args.properties { p } else {unreachable!("wrong type")};
//
//     // let wrapped_properties = Rc::new(RefCell::new(args.properties));
//     let component: RenderNodePtr = Rc::new(RefCell::new(
//         ComponentInstance {
//             template: init_and_retrieve_template(Rc::clone(&args.instance_map)),
//             adoptees: args.component_adoptees.expect("adoptees expected for Spread, even if empty vec"),
//             handler_registry: None,
//             transform: Transform::default_wrapped(),
//             properties: Rc::new(RefCell::new(PropertiesCoproduct::Empty)),
//             timeline: None,
//             compute_properties_fn: Box::new(|_,_|{})
//         }
//     ));
//     let template: RenderNodePtrList = Rc::new(RefCell::new(vec![
//         Rc::clone(&component)
//     ]));
//
//     let properties = if let PropertiesCoproduct::Spread(p) = args.properties { p } else {unreachable!("wrong type")};
//
//     SpreadInstance {
//         inner_component: template,
//         size: Rc::new(RefCell::new(args.size.expect("Spread requires size"))),
//         transform: args.transform,
//         properties: Rc::new(RefCell::new(properties)),
//         _cached_computed_layout_spec: vec![]
//     }
// }
//
//     fn get_rendering_children(&self) -> RenderNodePtrList {
//         // return the root of the internal template here — as long
//         // as we capture refs to (c) and (d) below during Spread's `render` or `pre_render` fn,
//         // we can happily let rendering just take its course,
//         // recursing through the subtree starting with (e).
//         //
//         // example application render tree
//         //          a( root )
//         //              |
//         //          b( Spread )
//         //         /          \
//         //    c( Rect )      d( Rect )
//         //
//         // example Spread (component) template render tree
//         //          e( root )
//         //              |         //  expanded:
//         //          f( Repeat  .. //  n=2 )
//         //           /            //      \
//         //      g( Frame )        //     i( Frame )
//         //          |             //        |
//         //      h( Placeholder )  //     j( Placeholder )
//         //
//         // traversal order:
//         // [a b e f g h c i j d]
//         //
//         // a: load the application root Group
//         // b: found a Spread, start rendering it
//         //    get its children from the Engine (get_children)
//         //    — these are the `adoptees` that will be passed to `Placeholder`
//         //    and they need to be tracked.
//         //    We can do this with a RenderNodeContext that we pass between recursive calls
//         //    when rendering.  We can keep a stack of prefab "scopes," allowing `placeholder`'s render
//         //    function to handily grab a reference to `adoptees[i]` when needed.  The stack
//         //    enables components to nest among themselves
//         // e: is Spread::render()
//         // f: first child of Spread — it's a Repeat;
//         //    loop twice, first passing rendering onto a Frame (g), waiting for it to return,
//         //    then passing onto the next Frame (i)
//         // g: render the containing frame in the correct position,
//         //    (plus clipping, maybe)
//         // h: needs to "evaluate" into the rectangle itself — directs the
//         //    flow of the render tree to (c) via the Context described in (b)
//         // c: finally render the rectangle itself; return & allow recursion to keep whirring
//         // i,j,d: repeat g,h,c
//
//         Rc::clone(&self.inner_component)
//     }
//
//     fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> where Self: Sized {
//         todo!()
//     }
//
//     fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
//
//     fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }
//
//     fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
//         self.compute_in_place(rtc);
//     }
// }

// fn init_and_retrieve_template(instance_map: Rc<RefCell<InstanceMap>>) -> RenderNodePtrList {
//     Rc::new(RefCell::new(
//         vec![
//             Rc::new(RefCell::new(
//                 RepeatInstance::instantiate(InstantiationArgs{
//                     properties: PropertiesCoproduct::Empty,
//                     handler_registry: None,
//                     instance_map: Rc::clone(&instance_map),
//                     transform: Transform::default_wrapped(),
//                     size: None,
//                     component_template: None,
//                     primitive_children: Some(Rc::new(RefCell::new(vec![
//                         Frame::instantiate(InstantiationArgs {
//                             properties: PropertiesCoproduct::Empty,
//                             handler_registry: None,
//                             instance_map: Rc::clone(&instance_map),
//                             transform: Rc::new(RefCell::new(
//                                 PropertyExpression {
//                                     id: "7870505b-27b4-4871-a239-f734db0e5709".to_string(),
//                                     cached_value: Transform::default(),
//                                 }
//                             )),
//                             size: None,
//                             primitive_children: Some(Rc::new(RefCell::new(
//                                 vec![]
//                             ))),
//                             component_template: None,
//                             component_adoptees: None,
//                             placeholder_index: None,
//                             data_list: None,
//                             compute_properties_fn: None
//                         })
//                     ]))),
//                     component_adoptees: None,
//                     placeholder_index: None,
//                     data_list: Some(Box::new(
//                         PropertyExpression {
//                             id:  "8d81fc58-3d2e-4e9d-b7c0-e219819b9684".to_string(),
//                             cached_value: vec![]
//                     })),
//                     compute_properties_fn: None
//                 })
//             ))
//         ]
//     ))
//     //             Repeat {
//     //                 data_list: Box::new(PropertyExpression {
//     //                     cached_value: vec![],
//     //                     evaluator: SpreadPropertiesInjector {variadic_evaluator: |properties: Rc<RefCell<SpreadProperties>>| -> Vec<Rc<PropertiesCoproduct>> {
//     //                         properties.borrow()._cached_computed_layout_spec.iter()
//     //                             .map(|scp|{Rc::new(PropertiesCoproduct::SpreadCell(Rc::clone(scp)))}).collect()
//     //                     }}
//     //                 }),
//     //                 template: Rc::new(RefCell::new(vec![
//     //                     Rc::new(RefCell::new(
//     //                         Frame {
//     //                             size: Rc::new(RefCell::new((
//     //                                 Box::new(PropertyExpression {
//     //                                     cached_value: Size::Pixel(100.0),
//     //                                     evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> Size {
//     //                                         match &*scope.borrow().datum {
//     //                                             PropertiesCoproduct::SpreadCell(sc) => {
//     //                                                 Size::Pixel(sc.width_px)
//     //                                             },
//     //                                             _ => panic!("Unknown property coproduct")
//     //                                         }
//     //                                     }}
//     //                                 }),
//     //                                 Box::new(PropertyExpression {
//     //                                     cached_value: Size::Pixel(100.0),
//     //                                     evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> Size {
//     //                                         match &*scope.borrow().datum {
//     //                                             PropertiesCoproduct::SpreadCell(sc) => {
//     //                                                 Size::Pixel(sc.height_px)
//     //                                             },
//     //                                             _ => panic!("Unknown property coproduct")
//     //                                         }
//     //                                     }}
//     //                                 }),
//     //                             ))),
//     //                             transform: Rc::new(RefCell::new(
//     //                                 Transform {
//     //                                         translate: (
//     //                                             Box::new(PropertyExpression {
//     //                                                 cached_value: 0.0,
//     //                                                 evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> f64 {
//     //                                                     match &*scope.borrow().datum {
//     //                                                         PropertiesCoproduct::SpreadCell(sc) => {
//     //                                                             sc.x_px
//     //                                                         },
//     //                                                         _ => panic!("Unknown property coproduct")
//     //                                                     }
//     //                                                 }}
//     //                                             }),
//     //                                             Box::new(PropertyExpression {
//     //                                                 cached_value: 0.0,
//     //                                                 evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> f64 {
//     //                                                     match &*scope.borrow().datum {
//     //                                                         PropertiesCoproduct::SpreadCell(sc) => {
//     //                                                             sc.y_px
//     //                                                         },
//     //                                                         _ => panic!("Unknown property coproduct")
//     //                                                     }
//     //                                                 }}
//     //                                             })
//     //                                         ),
//     //                                         ..Default::default()
//     //                                     },
//     //                             )),
//     //                             children: Rc::new(RefCell::new(vec![
//     //                                 Rc::new(RefCell::new(
//     //                                     Placeholder::new(
//     //                                         Transform::default(),
//     //                                         Box::new(PropertyExpression {
//     //                                             cached_value: 0,
//     //                                             evaluator: RepeatInjector {variadic_evaluator: |scope: Rc<RefCell<RepeatItem>>| -> usize {
//     //                                                 scope.borrow().i
//     //                                             }}
//     //                                         })
//     //                                     )
//     //                                 ))
//     //                             ])),
//     //                         }
//     //                     ))
//     //                 ])),
//     //                 ..Default::default()
//     //             }
//     //         ))
//     //     ]
//     // ))
// }


//
// /* FUTURE CODEGEN VIA MACRO [MAYBE?] */
// pub(crate) struct RepeatInjector<T> {
//     pub variadic_evaluator: fn(scope: Rc<RefCell<RepeatItem>>) -> T,
// }
//
// impl<T> RepeatInjector<T> {}
//
// impl<T> Evaluator<T> for RepeatInjector<T> {
//     fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
//         //TODO:CODEGEN
//
//         let stack_frame = &ic.stack_frame;
//         let stack_frame_borrowed = stack_frame.borrow();
//         let scope = &stack_frame_borrowed.get_scope();
//         let scope_borrowed = scope.borrow();
//         let repeat_properties = Rc::clone(  &scope_borrowed.properties );
//
//         let unwrapped_repeat_properties = match &*repeat_properties.borrow() {
//             PropertiesCoproduct::RepeatItem(rs) => {
//                 Rc::clone(rs)
//             },
//             _ => {
//                 panic!("Unexpected type.");
//             }
//         };
//
//         (self.variadic_evaluator)(unwrapped_repeat_properties)
//     }
// }
/* END FUTURE CODEGEN VIA MACRO */
/* MORE CODEGEN? */

// struct SpreadPropertiesInjector<T> {
//     pub variadic_evaluator: fn(scope: Rc<RefCell<SpreadProperties>>) -> T,
// }
//
// impl<T> SpreadPropertiesInjector<T> {}
//
// impl<T> Evaluator<T> for SpreadPropertiesInjector<T> {
//     fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
//         //TODO:CODEGEN
//
//         let stack_frame = &ic.stack_frame;
//         let stack_frame_borrowed = stack_frame.borrow();
//         let scope = &stack_frame_borrowed.get_scope();
//         let scope_borrowed = scope.borrow();
//         let properties = Rc::clone(  &scope_borrowed.properties );
//
//         let unwrapped_properties = match &*properties.borrow() {
//             PropertiesCoproduct::Spread(rs) => {
//                 Rc::clone(rs)
//             },
//             _ => {
//                 panic!("Unexpected type.");
//             }
//         };
//
//         (self.variadic_evaluator)(unwrapped_properties)
//     }
// }

/* END MORE CODEGEN */



// zb lab journal

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
