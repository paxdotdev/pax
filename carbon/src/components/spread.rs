use std::cell::{RefCell};
use piet_web::{WebRenderContext};
use crate::{Variable, Property, Affine, PropertyTreeContext, RenderNode, Size, RenderTreeContext, RenderTree, RenderNodePtrList, wrap_render_node_ptr_into_list, RenderNodePtr, Yield, PropertyExpression, PolymorphicValue, PolymorphicType};
use std::rc::Rc;
use std::collections::HashMap;
use kurbo::BezPath;
use piet::RenderContext;

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

    template: RenderNodePtrList,
    variables: Vec<Variable>,
}

//NEXT:  implement Frame, so that we can fill in the template
//       of Spread
//THEN:  implement Repeat, so that we can repeat our Frames for real
pub struct Frame {
    pub id: String,
    pub children: RenderNodePtrList,
    pub align: (f64, f64),
    pub origin: (Size<f64>, Size<f64>),
    pub size: (
        Box<dyn Property<Size<f64>>>,
        Box<dyn Property<Size<f64>>>,
    ),
    pub transform: Affine,

    //Is Frame a Component, or just a special-rendering primitive?
    //   Seems like the latter - no need for a fancy template; just
    //   take children + a size/transform spec, then start kicking some
    //   clipping masks through the render-recursion logic

    //TODO:
    // Understand clipping + Piet
    //   easy:  rc.clip(T: Shape), draw, then rc.restore()
    // Revisit mixed mode clipping, at least for Web (ensure viable path)
    //   seems like we can implement a similar API:  wrc.clip(path), draw (scroll bars, text, form controls), then wrc.restore()
    // Add clipping (path?) data to the RenderTreeContext here in Frame's logic
    //   or.... maybe we can just use rc.clip() directly from pre_render (and rc.restore() from post_render)
}

impl RenderNode for Frame {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Frame's `Expressable` properties
    }
    fn get_align(&self) -> (f64, f64) { self.align }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
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

        // construct a BezPath of this frame's bounds * its transform,
        // then pass that BezPath into rc.clip() [which pushes a clipping context to a piet-internal stack]
        //TODO:  if clipping is TURNED OFF for this Frame, don't do any of this
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounding_dimens;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        // rtc.runtime.borrow().log(&format!("Clipping: {} x {}", width ,height));

        let transformed_bez_path = *transform * bez_path;
        rc.save(); //our "save point" before clipping — restored to in the post_render
        rc.clip(transformed_bez_path);
    }
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //pop the clipping context from the stack
        rc.restore();
    }
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
    ) -> Self {
        Spread {
            children,
            id,
            align,
            origin,
            size,
            transform,
            //private "component declaration" here
            template: Rc::new(RefCell::new(
                vec![ Rc::new(RefCell::new(Frame {
                    id: "cell_frame".to_string(),
                    align: (0.0, 0.0),
                    origin: (Size::Pixel(0.0), Size::Pixel(0.0),),
                    size:(
                        Box::new(PropertyExpression {
                            last_value: Size::Pixel(100.0),
                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                unsafe {
                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                    return Size::Pixel((frames_elapsed / 100.).sin() * 500.)
                                }
                            })
                        }),
                        Box::new(PropertyExpression {
                            last_value: Size::Pixel(500.0),
                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| {
                                unsafe {
                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                    return Size::Pixel((frames_elapsed / 100.).sin() * 500.)
                                }
                            })
                        })
                    ),
                    transform: Affine::default(),

                    children:  Rc::new(RefCell::new(vec![
                        //TODO:wrap in repeat
                        Rc::new(RefCell::new(
                            Yield::new( "spread_frame_yield".to_string(), Affine::default())
                        ))
                    ])),
                }))])
            ),
            variables: vec![]
        }
    }
}

//EXAMPLE SPREAD COMPONENT
// <Component>
    // <Metadata />
    // <Template>
        // <Repeat declarations={{(i, elem)}} iterable={{get_children()}}>
            // <Frame transform={{get_frame_transform(i)}} size={{get_frame_size(i)}}>
                // <Yield index={{i}}>
            // </Frame>
        // <Repeat/>
    // </Template>
// </Component>


/*
TODO:
    [x] decide on API design, expected GUI experience
        - Direction (horiz/vert)
        - Gutter
        - Cell widths
    [x] expose a Spread element for consumption by engine
    [x] accept children, just like primitives e.g. `Group`
    [ ] author an internal template, incl. `yield`ing children and `repeating` inputs
        <Frame repeat=self.children transform=get_transform(i)>
            <Yield index=i>
        </Frame>
        - need to be able to define/call methods on containing class (a la VB)
        - need to figure out polymorphism, Vec<T> (?) for repeat
        - need to figure out yield — special kind of rendernode?
    [ ] Frame
        [ ] Clipping
    [x] Yield
        - might be done but can't be tested until we have a proper Component
          subtree ("prefab render tree") to work with
    [ ] Repeat
        [ ] "flattening yield" to support <Spread><Repeat n=5><Rect>...
        [ ] scopes:
            [ ] `i`
            [ ] braced templating {} ? or otherwise figure out `eval`
                - Code-gen?  piece together strings into a file and run rustc on it?
            [ ] calling "class methods" from templates, e.g. <Repeat n=5><Rect color="get_color(i)"
 */

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
        //      h( Yield )     //   j( Yield )
        //
        // traversal order:
        // [a b e f g h c i j d]
        //
        // a: load the application root Group
        // b: found a Spread, start rendering it
        //    get its children from the Engine (get_children)
        //    — these are the `adoptees` that will be passed to `Yield`
        //    and they need to be tracked.
        //    We can do this with a RenderNodeContext that we pass between recursive calls
        //    when rendering.  We can keep a stack of prefab "scopes," allowing `yield`'s render
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

        //TODO:  return root of internal template here, instead of `self.children`
        //       (which are the adoptees)
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

        rtc.runtime.borrow_mut().push_stack_frame(
            Rc::clone(&self.get_children())
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
                    <Yield index={{i}}>
                </Frame>
            <Repeat/>
        </Template>
    </Component>


// for={for (i, elem) in (&self) -> { &self.get_children()}.iter().enumerate()}
 */
