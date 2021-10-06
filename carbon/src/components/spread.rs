use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, PolymorphicType, PolymorphicValue, Property, PropertyExpression, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTree, RenderTreeContext, Size, Variable, wrap_render_node_ptr_into_list, PropertyLiteral};
use crate::primitives::placeholder::Placeholder;
use crate::primitives::frame::Frame;






/*
TODO:
    [x] decide on API design, expected GUI experience
        - Direction (horiz/vert)
        - Gutter
        - Cell widths
    [x] expose a Spread element for consumption by engine
    [x] accept children, just like primitives e.g. `Group`
    [ ] author an internal template, incl. `placeholder`ing children and `repeating` inputs
        <Frame repeat=self.children transform=get_transform(i)>
            <Placeholder index=i>
        </Frame>
        - need to be able to define/call methods on containing class (a la VB)
        - need to figure out polymorphism, Vec<T> (?) for repeat
        - need to figure out placeholder — special kind of rendernode?
    [x] Frame
        [x] Clipping
    [x] Placeholder
        - might be done but can't be tested until we have a proper Component
          subtree ("prefab render tree") to work with
    [ ] Repeat
        [ ] "flattening placeholder" to support <Spread><Repeat n=5><Rect>...
        [ ] scopes:
            [ ] `i`
            [ ] braced templating {} ? or otherwise figure out `eval`
                - Code-gen?  piece together strings into a file and run rustc on it?
            [ ] calling "class methods" from templates, e.g. <Repeat n=5><Rect color="get_color(i)"
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
    pub children: RenderNodePtrList,
    pub id: String,
    pub align: (f64, f64),
    pub origin: (Size<f64>, Size<f64>),
    pub size: (
        Box<dyn Property<Size<f64>>>,
        Box<dyn Property<Size<f64>>>,
    ),
    pub transform: Affine,

    pub gutter: Size<f64>,
    pub cell_size_spec: Option<Vec<Size<f64>>>,

    template: RenderNodePtrList,
    variables: Vec<Variable>,
}

impl Spread {
    pub fn new(
    //TODO:  parameterize
        children: RenderNodePtrList,
        id: String,
        align: (f64, f64),
        origin: (Size<f64>, Size<f64>),
        size: (Box<dyn Property<Size<f64>>>, Box<dyn Property<Size<f64>>>),
        transform: Affine,
        gutter: Size<f64>,
        cell_size_spec: Option<Vec<Size<f64>>>,
    ) -> Self {
        Spread {
            children,
            id,
            align,
            origin,
            size,
            transform,
            gutter,
            cell_size_spec,
            //private "component declaration" here, for template & variables
            template: Rc::new(RefCell::new(
                vec![
                    Rc::new(RefCell::new(
                    //TODO:wrap in repeat
                    Frame {
                        id: "cell_frame_left".to_string(),
                        align: (0.0, 0.0),
                        origin: (Size::Pixel(0.0), Size::Pixel(0.0),),
                        size:(
                            (
                                Box::new(PropertyLiteral { value: Size::Percent(50.0) } ),
                                Box::new(PropertyLiteral { value: Size::Percent(100.0) } ),
                            )
                        ),
                        transform: Affine::default(),

                        children:  Rc::new(RefCell::new(vec![
                            Rc::new(RefCell::new(
                                Placeholder::new( "spread_frame_placeholder_left".to_string(), Affine::default())
                            )),
                        ])),
                    })),
                    Rc::new(RefCell::new(
                        //TODO:wrap in repeat
                        Frame {
                            id: "cell_frame_right".to_string(),
                            align: (1.0, 0.0),
                            origin: (Size::Percent(100.0), Size::Pixel(0.0),),
                            size:(
                                (
                                    Box::new(PropertyLiteral { value: Size::Percent(50.0) } ),
                                    Box::new(PropertyLiteral { value: Size::Percent(100.0) } ),
                                )
                            ),
                            transform: Affine::default(),

                            children:  Rc::new(RefCell::new(vec![
                                Rc::new(RefCell::new(
                                    Placeholder::new( "spread_frame_placeholder_right".to_string(), Affine::default())
                                )),
                            ])
                        ),
                        }
                    )),
                ])
            ),
            variables: vec![]
        }
    }
}



impl RenderNode for Spread {

    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Spread's `Expressable` properties
    }

    fn get_align(&self) -> (f64, f64) { self.align }
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
        //              |      //  expanded:
        //          f( Repeat  //  n=2 )
        //           /         //    \
        //      g( Frame )     //   i( Frame )
        //          |          //      |
        //      h( Placeholder )     //   j( Placeholder )
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
    fn get_origin(&self) -> (Size<f64>, Size<f64>) { self.origin }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //TODO:  calc & memoize the layout/transform for each cell of the Sprad
        //       probably need to do the memoization via a RefCell for mutability concerns,
        //       since pre_render happens during immutable render tree recursion

        //Algo:
        // 1. determine number of adoptees (&self.children), `n`
        // 2. determine gutter, `g`
        // 3. determine bounding size, `(x,y)`

        match &self.cell_size_spec {
            //If a cell_size_spec is provided, use it.
            Some(cell_size_spec) => (),
            //Otherwise, calculate one
            None => {

            }
        }


        rtc.runtime.borrow_mut().push_stack_frame(
            Rc::clone(&self.children)
        );
    }

    fn render(&self, _sc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {
        //TODO:  render cell borders if appropriate
    }

    fn post_render(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //clean up the stack frame for the next component
        rtc.runtime.borrow_mut().pop_stack_frame();
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
