use std::rc::Rc;

use pax_message::{borrow, OcclusionPatch};
use pax_runtime_api::{Layer, Window};

use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};

use super::expanded_node::Occlusion;

#[derive(Clone, Copy, Debug)]
pub struct OcclusionBox {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl OcclusionBox {
    fn intersects(&self, other: &Self) -> bool {
        if self.x2 < other.x1 || other.x2 < self.x1 {
            return false;
        }
        if self.y2 < other.y1 || other.y2 < self.y1 {
            return false;
        }
        true
    }

    fn new_from_transform_and_bounds(t_and_b: TransformAndBounds<NodeLocal, Window>) -> Self {
        let corners = t_and_b.corners();
        let mut x2 = f64::MIN;
        let mut y2 = f64::MIN;
        let mut x1 = f64::MAX;
        let mut y1 = f64::MAX;
        for c in corners {
            x2 = x2.max(c.x);
            y2 = y2.max(c.y);
            x1 = x1.min(c.x);
            y1 = y1.min(c.y);
        }
        OcclusionBox { x1, y1, x2, y2 }
    }
}

pub fn update_node_occlusion(root_node: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    let mut occlusion_stack = vec![];
    let mut z_index = 0;
    update_node_occlusion_recursive(root_node, &mut occlusion_stack, ctx, false, &mut z_index);
    let max_layer = occlusion_stack
        .iter()
        .map(|(_, v, _)| *v)
        .max()
        .unwrap_or(0);
    ctx.enqueue_native_message(pax_message::NativeMessage::ShrinkLayersTo(max_layer));
}

// runtime is O(n^2) atm, could be improved by quadtree, or find an approximation
// method using the tree structure that works well.
fn update_node_occlusion_recursive(
    node: &Rc<ExpandedNode>,
    occlusion_stack: &mut Vec<(Layer, u32, OcclusionBox)>,
    ctx: &RuntimeContext,
    clipping: bool,
    z_index: &mut i32,
) {
    for child in node.children.get().iter().rev() {
        let cp = child.get_common_properties();
        let cp = borrow!(cp);
        let unclippable = cp.unclippable.get().unwrap_or(false);
        let clips = borrow!(child.instance_node).clips_content(&child);
        update_node_occlusion_recursive(
            child,
            occlusion_stack,
            ctx,
            (clipping | clips) & !unclippable,
            z_index,
        );
    }

    let layer = borrow!(node.instance_node).base().flags().layer;
    if layer != Layer::DontCare {
        let occlusion_box =
            OcclusionBox::new_from_transform_and_bounds(node.transform_and_bounds.get());
        let mut occlusion_index = 0;

        for (layer_type, occl_id, occl_box) in occlusion_stack.iter().rev() {
            if occlusion_box.intersects(occl_box) {
                occlusion_index = match (layer, layer_type) {
                    (Layer::Canvas, Layer::Native) => *occl_id + 1,
                    _ => *occl_id,
                };
                break;
            }
        }
        occlusion_stack.push((layer, occlusion_index, occlusion_box));

        let new_occlusion = Occlusion {
            occlusion_layer_id: occlusion_index,
            z_index: *z_index,
            parent_frame: node
                .parent_frame
                .get()
                .filter(|_| clipping)
                .map(|v| v.to_u32()),
        };
        if (layer == Layer::Native || borrow!(node.instance_node).clips_content(&node))
            && node.occlusion.get() != new_occlusion
        {
            let occlusion_patch = OcclusionPatch {
                id: node.id.to_u32(),
                z_index: new_occlusion.z_index,
                occlusion_layer_id: new_occlusion.occlusion_layer_id,
                parent_frame: new_occlusion.parent_frame,
            };
            ctx.enqueue_native_message(pax_message::NativeMessage::OcclusionUpdate(
                occlusion_patch,
            ));
        }
        node.occlusion.set(new_occlusion);
        *z_index += 1;
    }
}
