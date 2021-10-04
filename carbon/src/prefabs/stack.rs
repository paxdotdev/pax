use std::cell::{RefCell};
use piet_web::{WebRenderContext};
use crate::{Variable, Property, PropertyTreeContext, RenderNode, Size, Affine, SceneGraphContext, SceneGraph};


pub struct Stack {
    pub children: Vec<Box<dyn RenderNode>>,
    pub internal_scene_graph: RefCell<SceneGraph>,
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
    [ ] Yield
    [ ] Repeat
        [ ] "flattening yield" to support <Stack><Repeat n=5><Rect>...
        [ ] scopes:
            [ ] `i`
            [ ] Handlebars templating {} ? or otherwise figure out `eval`
                - Code-gen?  piece together strings into a file and run rustc on it?
            [ ] calling "class methods" from templates, e.g. <Repeat n=5><Rect color="get_color(i)"

    
 */


impl RenderNode for Stack {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Stack's `Expressable` properties
    }

    fn get_align(&self) -> (f64, f64) { self.align }
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {


        // return the root of the internal template here — as long
        // as we capture refs to (c) and (d) below during Stack's `render` or `pre_render` fn,
        // we can happily let rendering just take its course,
        // recursing through the subtree starting with (e).
        //
        // example application scene graph
        //          a( root )
        //              |
        //          b( Stack )
        //         /          \
        //    c( Rect )      d( Rect )
        //
        // example Stack (prefab) scene graph
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
        //    We can do this with a SceneGraphContext that we pass between recursive calls
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
        //    flow of the scene graph to (c) via the Context described in (b)
        // c: finally render the rectangle itself; return & allow recursion to keep whirring
        // i,j,d: repeat g,h,c

        //TODO:  return root of internal scene graph here, instead of `self.children`
        //       (which are the adoptees)

        Some(&self.children)
    }
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>> { Some(&mut self.children) }
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
    fn pre_render(&self) {
        //TODO:  calc & memoize the layout/transform for each cell of the stack
        //       probably need to do the memoization via a RefCell for mutability concerns,
        //       since pre_render happens during immutable scene graph recursion
        //


    }
    fn render(&self, _: &mut WebRenderContext, _: &SceneGraphContext) {
        //TODO:  render cell borders if appropriate
        //TODO:  capture a reference to the application-scene-graph-provided children,
        //       into a `frame` that will ride with the SceneGraphContext
        //TODO:  in order to actually adopt an adoptee, we need to either
        //       mutate the adoptees Vec, or keep an index alongside.
        //       Let's try making the SceneGraphContext mutable.
    }

}