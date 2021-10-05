use std::cell::{RefCell};
use piet_web::{WebRenderContext};
use crate::{Variable, Property, Affine, PropertyTreeContext, RenderNode, Size, RenderTreeContext, RenderTree, RenderNodePtrList, wrap_render_node_ptr_into_list, RenderNodePtr, Yield};
use std::rc::Rc;

pub struct Stack {
    pub children: RenderNodePtrList,

    template: Rc<RefCell<RenderTree>>,
    pub id: String,
    pub align: (f64, f64),
    pub origin: (Size<f64>, Size<f64>),
    pub size: (
        Box<dyn Property<Size<f64>>>,
        Box<dyn Property<Size<f64>>>,
    ),
    pub transform: Affine,
    pub variables: Vec<Variable>,
}



pub trait Component {
    fn get_template_root() -> RenderNodePtr;
}

impl Component for Stack {
    fn get_template_root() -> RenderNodePtr {
        //TODO:  author internal template here

        //         <Frame repeat=self.children transform=get_transform(i)>
        //             <Yield index=i>
        //         </Frame>
        
        Rc::new(RefCell::new(Yield {
            id: String::from("stack_template_yield"),
            transform: Affine::default(),
        }))

    }
}

/*
TODO:
    [x] decide on API design, expected GUI experience
        - Direction (horiz/vert)
        - Gutter
        - Cell widths
    [x] expose a Stack element for consumption by engine
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
        [ ] "flattening yield" to support <Stack><Repeat n=5><Rect>...
        [ ] scopes:
            [ ] `i`
            [ ] braced templating {} ? or otherwise figure out `eval`
                - Code-gen?  piece together strings into a file and run rustc on it?
            [ ] calling "class methods" from templates, e.g. <Repeat n=5><Rect color="get_color(i)"

    
 */


impl RenderNode for Stack {


    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Stack's `Expressable` properties
    }

    fn get_align(&self) -> (f64, f64) { self.align }
    fn get_children(&self) -> RenderNodePtrList {

        // return the root of the internal template here — as long
        // as we capture refs to (c) and (d) below during Stack's `render` or `pre_render` fn,
        // we can happily let rendering just take its course,
        // recursing through the subtree starting with (e).
        //
        // example application render tree
        //          a( root )
        //              |
        //          b( Stack )
        //         /          \
        //    c( Rect )      d( Rect )
        //
        // example Stack (prefab) render tree
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
        // b: found a Stack, start rendering it
        //    get its children from the Engine (get_children)
        //    — these are the `adoptees` that will be passed to `Yield`
        //    and they need to be tracked.
        //    We can do this with a RenderNodeContext that we pass between recursive calls
        //    when rendering.  We can keep a stack of prefab "scopes," allowing `yield`'s render
        //    function to handily grab a reference to `adoptees[i]` when needed.  The stack
        //    enables prefabs to nest among themselves
        // e: is Stack::render()
        // f: first child of Stack — it's a Repeat;
        //    loop twice, first passing rendering onto a Frame (g), waiting for it to return,
        //    then passing onto the next Frame (i)
        // g: render the containing frame in the correct position,
        //    (plus clipping, maybe)
        // h: needs to "evaluate" into the rectangle itself — directs the
        //    flow of the render tree to (c) via the Context described in (b)
        // c: finally render the rectangle itself; return & allow recursion to keep whirring
        // i,j,d: repeat g,h,c

        //TODO:  return root of internal render tree here, instead of `self.children`
        //       (which are the adoptees)
        wrap_render_node_ptr_into_list(Rc::clone(&self.template.borrow().root))

        // Rc::clone(&self.children) //this logic is a placeholder & is wrong
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { Some((*self.size.0.read(), *self.size.1.read())) }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) {
        let size_raw = self.get_size().unwrap();
        return (
            match size_raw.0 {
                Size::Pixel(width) => {
                    width
                },
                Size::Percent(width) => {
                    bounds.0 * (width / 100.0)
                }
            },
            match size_raw.1 {
                Size::Pixel(height) => {
                    height
                },
                Size::Percent(height) => {
                    bounds.1 * (height / 100.0)
                }
            }
        )
    }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_origin(&self) -> (Size<f64>, Size<f64>) { self.origin }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn pre_render(&mut self, rtc: &mut RenderTreeContext) {
        //TODO:  calc & memoize the layout/transform for each cell of the stack
        //       probably need to do the memoization via a RefCell for mutability concerns,
        //       since pre_render happens during immutable render tree recursion

        rtc.runtime.borrow_mut().push_stack_frame(
            Rc::clone(&rtc.node.borrow().get_children())
        );
    }

    fn render(&self, _sc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {
        //TODO:  render cell borders if appropriate
    }

    fn post_render(&self, sc: &mut RenderTreeContext) {
        //clean up the stack frame for the next component
        sc.runtime.borrow_mut().pop_stack_frame();
    }

}

/* Things that a component needs to do, beyond being just a rendernode
    - declare a list of Properties [maybe this should be the same as RenderNodes?? i.e. change RenderNode to be more like this]
    - push a stackframe during pre_render and pop it during post_render
    - declare a `template` that's separate from its `children`
*/