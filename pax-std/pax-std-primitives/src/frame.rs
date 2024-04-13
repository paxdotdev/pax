use core::option::Option;
use core::option::Option::Some;

use std::cell::RefCell;
use std::rc::Rc;

use kurbo::{Affine, BezPath};
use pax_runtime::api::{Layer, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// A primitive that gathers children underneath a single render node with a shared base transform,
/// like [`Group`], except [`Frame`] has the option of clipping rendering outside
/// of its bounds.
///
/// If clipping or the option of clipping is not required,
/// a [`Group`] will generally be a more performant and otherwise-equivalent
/// to [`Frame`], since `[Frame]` creates a clipping mask.
pub struct FrameInstance {
    base: BaseInstance,
}

impl InstanceNode for FrameInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::Canvas,
                    is_component: false,
                },
            ),
        })
    }

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        _context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
    ) {
        let transform = expanded_node.layout_properties.transform.get();
        let (width, height) = expanded_node.layout_properties.bounds.get();

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width, 0.0));
        bez_path.line_to((width, height));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0, 0.0));
        bez_path.close_path();

        let transformed_bez_path = <Affine>::from(transform) * bez_path;

        let layers = rcs.layers();
        let layers: Vec<String> = layers.iter().map(|s| s.to_string()).collect();

        for layer in layers {
            //our "save point" before clipping — restored to in the post_render
            rcs.save(&layer);
            rcs.clip(&layer, transformed_bez_path.clone());
        }
    }

    fn handle_post_render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
    ) {
        let layers = rcs.layers();
        let layers: Vec<String> = layers.iter().map(|s| s.to_string()).collect();
        for layer in layers {
            //pop the clipping context from the stack
            rcs.restore(&layer);
        }
    }

    fn handle_unmount(
        &self,
        _expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        use pax_std::primitives::Frame;

        match expanded_node {
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_f: &mut Frame| f.debug_struct("Frame").finish()),
            None => f.debug_struct("Frame").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
