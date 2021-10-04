
use piet_web::{WebRenderContext};
use crate::{Variable, Property, PropertyTreeContext, RenderNode, Size, Affine};


pub struct Stack {
    pub children: Vec<Box<dyn RenderNode>>,
    pub id: String,
    pub align: (f64, f64),
    pub origin: (Size<f64>, Size<f64>),
    pub transform: Affine,
    pub variables: Vec<Variable>,
}

/*
TODO:
    [ ] expose a Stack element for consumption by engine
    [ ] accept children, just like primitives e.g. `Group`
    [ ] author an internal template, incl. `yield`ing children and `repeating` inputs
        <frame repeat=self.children transform=get_transform(i)>
            <yield index=i>
        </frame>
        - need to be able to define/call methods on containing class (a la VB)
        - need to figure out polymorphism, Vec<T> (?) for repeat
        - need to figure out yield — special kind of rendernode?
    [x] decide on API design, expected GUI experience
        - Direction (horiz/vert)
        - Gutter
        - Cell widths
    
 */


impl RenderNode for Stack {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Stack's `Expressable` properties
    }

    fn get_align(&self) -> (f64, f64) { self.align }
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        Some(&self.children)
    }
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>> { Some(&mut self.children) }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
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

    }
    fn render(&self, _: &mut WebRenderContext, _: &Affine, _: (f64, f64)) {
        //TODO:  render cell borders if appropriate
    }

}