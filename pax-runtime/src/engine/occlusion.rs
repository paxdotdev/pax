use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use pax_message::{borrow, OcclusionPatch};
use pax_runtime_api::{Layer, Window};

use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};

use super::expanded_node::Occlusion;

static DEBUG_ON: AtomicBool = AtomicBool::new(false);

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

    fn union(self, other: Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
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

#[derive(Default, Clone, Debug)]
struct OcclusionSet {
    bounds_native: Option<OcclusionBox>,
    bounds_canvas: Option<OcclusionBox>,
    pub layers_needed: u32,
}

struct NodeOcclusionData {
    occlusion_set: OcclusionSet,
    node: Rc<ExpandedNode>,
    children: Vec<(NodeOcclusionData, u32)>,
}

impl std::fmt::Debug for NodeOcclusionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("N")
            .field("s", &self.occlusion_set.layers_needed)
            .field("c", &self.children)
            .finish()
    }
}

impl OcclusionSet {
    fn merge_above(&mut self, above: &Self) -> u32 {
        if DEBUG_ON.load(Ordering::Relaxed) {
            log::debug!("Merging: {:?}, {:?}", self, above);
        }
        let mut layers_needed = self.layers_needed.max(above.layers_needed);
        let mut above_draw_container_offset = self.layers_needed;
        if let (Some(below_native), Some(above_canvas)) = (self.bounds_native, above.bounds_canvas)
        {
            if DEBUG_ON.load(Ordering::Relaxed) {
                log::debug!("merge_above: bellow native, above canvas");
            }
            if below_native.intersects(&above_canvas) {
                layers_needed = self.layers_needed + above.layers_needed + 1;
                above_draw_container_offset = self.layers_needed + 1;
            }
        }
        let bounds_native = match (self.bounds_native, above.bounds_native) {
            (None, None) => None,
            (Some(o), None) | (None, Some(o)) => Some(o),
            (Some(a), Some(b)) => Some(a.union(b)),
        };
        let bounds_canvas = match (self.bounds_canvas, above.bounds_canvas) {
            (None, None) => None,
            (Some(o), None) | (None, Some(o)) => Some(o),
            (Some(a), Some(b)) => Some(a.union(b)),
        };
        if DEBUG_ON.load(Ordering::Relaxed) {
            log::debug!("merge done");
        }
        *self = Self {
            bounds_native,
            bounds_canvas,
            layers_needed,
        };
        above_draw_container_offset
    }
}

pub fn update_node_occlusion(root_node: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    #[cfg(feature = "designtime")]
    {
        let usr_node = borrow!(ctx.userland_root_expanded_node);
        if let Some(node) = &*usr_node {
            // DEBUG_ON.store(true, Ordering::Relaxed);
            let dummy_data = calculate_occlusion_data(&node);
            // DEBUG_ON.store(false, Ordering::Relaxed);
            log::debug!("tree: {:#?}", dummy_data);
        }
    }
    let occlusion_data = calculate_occlusion_data(&root_node);
    let mut z_index = 0;
    update_node_occlusion_recursive(occlusion_data, ctx, 0, false, &mut z_index);
}

fn update_node_occlusion_recursive(
    occlusion_data: NodeOcclusionData,
    ctx: &RuntimeContext,
    layer_offset: u32,
    clipping: bool,
    z_index: &mut i32,
) {
    let node = &occlusion_data.node;
    for (child, offset) in occlusion_data.children {
        let cp = child.node.get_common_properties();
        let cp = borrow!(cp);
        let unclippable = cp.unclippable.get().unwrap_or(false);
        let clips = borrow!(child.node.instance_node).clips_content(&child.node);
        update_node_occlusion_recursive(
            child,
            ctx,
            layer_offset + offset,
            (clipping | clips) & !unclippable,
            z_index,
        );
    }

    let new_occlusion = Occlusion {
        occlusion_layer_id: layer_offset,
        z_index: *z_index,
        parent_frame: occlusion_data
            .node
            .parent_frame
            .get()
            .filter(|_| clipping)
            .map(|v| v.to_u32()),
    };
    let layer = borrow!(node.instance_node).base().flags().layer;
    if (layer == Layer::Native || borrow!(node.instance_node).clips_content(&node))
        && node.occlusion.get() != new_occlusion
    {
        let occlusion_patch = OcclusionPatch {
            id: node.id.to_u32(),
            z_index: new_occlusion.z_index,
            occlusion_layer_id: new_occlusion.occlusion_layer_id,
            parent_frame: new_occlusion.parent_frame,
        };
        ctx.enqueue_native_message(pax_message::NativeMessage::OcclusionUpdate(occlusion_patch));
    }
    node.occlusion.set(new_occlusion);
    *z_index += 1;
}

fn calculate_occlusion_data(node: &Rc<ExpandedNode>) -> NodeOcclusionData {
    let mut occlusion_set_self = OcclusionSet::default();
    let instance_node = borrow!(node.instance_node);
    let layer = instance_node.base().flags().layer;
    match layer {
        pax_runtime_api::Layer::Native => {
            occlusion_set_self.bounds_native = Some(OcclusionBox::new_from_transform_and_bounds(
                node.transform_and_bounds.get(),
            ))
        }
        pax_runtime_api::Layer::Canvas => {
            occlusion_set_self.bounds_canvas = Some(OcclusionBox::new_from_transform_and_bounds(
                node.transform_and_bounds.get(),
            ))
        }
        pax_runtime_api::Layer::DontCare => (),
    }
    let mut combined = OcclusionSet::default();
    let mut children = vec![];
    for child in node.children.get().iter().rev() {
        let child_data = calculate_occlusion_data(&child);
        let container_occlusion_offset = combined.merge_above(&child_data.occlusion_set);
        if DEBUG_ON.load(Ordering::Relaxed) {
            log::debug!("accum combined: {:?}", combined);
        }
        children.push((child_data, container_occlusion_offset));
    }
    combined.merge_above(&occlusion_set_self);
    NodeOcclusionData {
        occlusion_set: combined,
        node: Rc::clone(&node),
        children,
    }
}
