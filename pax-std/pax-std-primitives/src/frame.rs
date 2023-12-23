use core::option::Option;
use core::option::Option::Some;
use std::collections::HashMap;
use std::rc::Rc;

use pax_core::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::{Layer, RenderContext, Size};

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

    fn get_clipping_size(&self, expanded_node: &ExpandedNode) -> Option<(Size, Size)> {
        Some(self.get_size(expanded_node))
    }

    // fn handle_native_patches(
    //     &mut self,
    //     rtc: &mut RenderTreeContext<R>,
    //     computed_size: (f64, f64),
    //     transform_coeffs: Vec<f64>,
    //     _z_index: u32,
    //     _subtree_depth: u32,
    // ) {
    // let mut new_message: FramePatch = Default::default();
    // new_message.id_chain = rtc.get_id_chain(self.instance_id);
    // if !self.last_patches.contains_key(&new_message.id_chain) {
    //     let mut patch = FramePatch::default();
    //     patch.id_chain = new_message.id_chain.clone();
    //     self.last_patches
    //         .insert(new_message.id_chain.clone(), patch);
    // }
    // let last_patch = self.last_patches.get_mut(&new_message.id_chain).unwrap();
    // let mut has_any_updates = false;
    //
    // let val = computed_size.0;
    // let is_new_value = match &last_patch.size_x {
    //     Some(cached_value) => !val.eq(cached_value),
    //     None => true,
    // };
    // if is_new_value {
    //     new_message.size_x = Some(val);
    //     last_patch.size_x = Some(val);
    //     has_any_updates = true;
    // }
    //
    // let val = computed_size.1;
    // let is_new_value = match &last_patch.size_y {
    //     Some(cached_value) => !val.eq(cached_value),
    //     None => true,
    // };
    // if is_new_value {
    //     new_message.size_y = Some(val);
    //     last_patch.size_y = Some(val);
    //     has_any_updates = true;
    // }
    //
    // let latest_transform = transform_coeffs;
    // let is_new_transform = match &last_patch.transform {
    //     Some(cached_transform) => latest_transform
    //         .iter()
    //         .enumerate()
    //         .any(|(i, elem)| *elem != cached_transform[i]),
    //     None => true,
    // };
    // if is_new_transform {
    //     new_message.transform = Some(latest_transform.clone());
    //     last_patch.transform = Some(latest_transform.clone());
    //     has_any_updates = true;
    // }
    //
    // if has_any_updates {
    //     (*rtc.engine.runtime)
    //         .borrow_mut()
    //         .enqueue_native_message(pax_message::NativeMessage::FrameUpdate(new_message));
    // }
    // todo!()
    // }

    fn update(self: Rc<Self>, _expanded_node: &Rc<ExpandedNode>, _context: &mut RuntimeContext) {}

    fn handle_pre_render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &mut RuntimeContext,
        _rc: &mut Box<dyn RenderContext>,
    ) {
        // let tab = &expanded_node.computed_tab.as_ref().unwrap();

        // let width: f64 = tab.bounds.0;
        // let height: f64 = tab.bounds.1;
        // let _properties_wrapped: Rc<RefCell<dyn Any>> =
        //     rtc.current_expanded_node.borrow().get_properties();

        // let mut bez_path = BezPath::new();
        // bez_path.move_to((0.0, 0.0));
        // bez_path.line_to((width, 0.0));
        // bez_path.line_to((width, height));
        // bez_path.line_to((0.0, height));
        // bez_path.line_to((0.0, 0.0));
        // bez_path.close_path();

        // let transformed_bez_path = tab.transform * bez_path;

        // for (_key, rc) in rcs.iter_mut() {
        //     rc.save(); //our "save point" before clipping — restored to in the post_render
        //     rc.clip(transformed_bez_path.clone());
        // }
    }

    fn handle_post_render(
        &self,
        _context: &mut RuntimeContext,
        _rcs: &mut HashMap<String, Box<dyn RenderContext>>,
    ) {
        // for (_key, rc) in _rcs.iter_mut() {
        //     //pop the clipping context from the stack
        //     rc.restore();
        // }
    }

    fn handle_mount(&self, _node: &ExpandedNode, _context: &mut RuntimeContext) {
        // let id_chain = node.id_chain.clone();

        // //though macOS and iOS don't need this ancestry chain for clipping, Web does
        // let clipping_ids = ptc.get_current_clipping_ids();

        // let scroller_ids = ptc.get_current_scroller_ids();

        // let z_index = node.computed_z_index.unwrap();

        // ptc.enqueue_native_message(pax_message::NativeMessage::FrameCreate(AnyCreatePatch {
        //     id_chain,
        //     clipping_ids,
        //     scroller_ids,
        //     z_index,
        // }));
    }

    fn handle_unmount(&self, _expanded_node: &ExpandedNode, context: &mut RuntimeContext) {}

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
