use kurbo::Affine;
use piet_web::WebRenderContext;



pub struct SceneGraph {
    pub root: Box<dyn RenderNode>
}

pub trait RenderNode
{
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>>;
    fn get_transform(&self) -> &Affine;
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine);
}
