use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use kurbo::{Affine, BezPath, Shape};
use pax_message::{borrow, MaskPathPatch, NativeMaskPatch, OcclusionPatch};
use pax_runtime_api::{bez_path_to_svg_path_data, Layer, Window};

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
        if self.x2 <= other.x1 || other.x2 <= self.x1 {
            return false;
        }
        if self.y2 <= other.y1 || other.y2 <= self.y1 {
            return false;
        }
        true
    }

    fn new_from_transform_and_bounds(t_and_b: TransformAndBounds<NodeLocal, Window>) -> Self {
        let corners = t_and_b.corners();
        let mut x1 = f64::MAX;
        let mut y1 = f64::MAX;
        let mut x2 = f64::MIN;
        let mut y2 = f64::MIN;
        for c in corners {
            x1 = x1.min(c.x);
            y1 = y1.min(c.y);
            x2 = x2.max(c.x);
            y2 = y2.max(c.y);
        }
        OcclusionBox { x1, y1, x2, y2 }
    }

    fn new_from_path(path: &kurbo::BezPath) -> Option<Self> {
        let bounds = path.bounding_box();
        if bounds.is_zero_area() {
            return None;
        }
        Some(Self {
            x1: bounds.x0,
            y1: bounds.y0,
            x2: bounds.x1,
            y2: bounds.y1,
        })
    }
}

#[derive(Clone)]
struct CoverageEntry {
    bounds: OcclusionBox,
    path: BezPath,
    clips: Vec<BezPath>,
}

enum DrawableInfo {
    Canvas(CoverageEntry),
    Native {
        node: Rc<ExpandedNode>,
        layer: Layer,
        bounds: OcclusionBox,
    },
}

pub fn update_node_occlusion(root_node: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    let mut drawables = Vec::new();
    let mut z_index = 0;
    update_node_occlusion_recursive(root_node, ctx, None, &[], &mut z_index, &mut drawables);
    update_native_masks(&drawables, ctx);

    let new_layer_count = 1;
    if ctx.layer_count.get() != new_layer_count {
        ctx.layer_count.set(new_layer_count);
        ctx.enqueue_native_message(pax_message::NativeMessage::ShrinkLayersTo(
            new_layer_count as u32,
        ));
    }
}

// runtime is O(n^2) atm, but all native punchout work is now confined to one native overlay.
fn update_node_occlusion_recursive(
    node: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    active_container: Option<u32>,
    active_clips: &[BezPath],
    z_index: &mut i32,
    drawables: &mut Vec<DrawableInfo>,
) {
    let effect_clip_path = borrow!(node.instance_node).resolve_effect_clip_path(node);
    let has_effect_clip = effect_clip_path.is_some();

    let descendant_container = has_effect_clip.then(|| node.id.to_u32()).or(active_container);
    let mut descendant_clips = active_clips.to_vec();
    if let Some(clip_path) = effect_clip_path.clone() {
        descendant_clips.push(clip_path);
    }

    for child in node.children.get().iter().rev() {
        let cp = child.get_common_properties();
        let cp = borrow!(cp);
        let unclippable = cp.unclippable.get().unwrap_or(false);
        let (child_container, child_clips) = if unclippable {
            (None, Vec::new())
        } else {
            (descendant_container, descendant_clips.clone())
        };

        update_node_occlusion_recursive(
            child,
            ctx,
            child_container,
            &child_clips,
            z_index,
            drawables,
        );
    }

    let layer = borrow!(node.instance_node).base().flags().layer;
    if layer == Layer::DontCare && !has_effect_clip {
        return;
    }

    let new_occlusion = Occlusion {
        occlusion_layer_id: 0,
        z_index: *z_index,
        parent_frame: active_container,
    };

    if (matches!(layer, Layer::Native | Layer::NativeNonOccluding) || has_effect_clip)
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

    if new_occlusion != node.occlusion.get() {
        let prev_layer = node.occlusion.get().occlusion_layer_id;
        if layer == Layer::Canvas && prev_layer != new_occlusion.occlusion_layer_id {
            ctx.enqueue_canvas_node_removal(prev_layer, node.id.to_u32());
        }
        if layer == Layer::Canvas {
            ctx.mark_canvas_node_dirty(node.id);
        }
        ctx.set_canvas_dirty(prev_layer);
        ctx.set_canvas_dirty(new_occlusion.occlusion_layer_id);
        node.occlusion.set(new_occlusion);
    }

    match layer {
        Layer::Canvas => {
            if let Some(coverage_path) = borrow!(node.instance_node).resolve_coverage_path(node) {
                if let Some(bounds) = OcclusionBox::new_from_path(&coverage_path) {
                    drawables.push(DrawableInfo::Canvas(CoverageEntry {
                        bounds,
                        path: coverage_path,
                        clips: active_clips.to_vec(),
                    }));
                }
            }
        }
        Layer::Native | Layer::NativeNonOccluding => {
            drawables.push(DrawableInfo::Native {
                node: Rc::clone(node),
                layer,
                bounds: OcclusionBox::new_from_transform_and_bounds(node.transform_and_bounds.get()),
            });
        }
        Layer::DontCare => {}
    }

    *z_index += 1;
}

fn update_native_masks(drawables: &[DrawableInfo], ctx: &RuntimeContext) {
    let mut vector_above = Vec::<CoverageEntry>::new();

    for drawable in drawables.iter().rev() {
        match drawable {
            DrawableInfo::Canvas(entry) => vector_above.push(entry.clone()),
            DrawableInfo::Native { node, layer, bounds } => {
                let t_and_b = node.transform_and_bounds.get();
                let size = t_and_b.bounds;
                let entries = if *layer == Layer::Native {
                    let inverse = Affine::from(t_and_b.transform.inverse());
                    vector_above
                        .iter()
                        .filter(|entry| entry.bounds.intersects(bounds))
                        .map(|entry| MaskPathPatch {
                            path: bez_path_to_svg_path_data(&(inverse * entry.path.clone())),
                            clips: entry
                                .clips
                                .iter()
                                .map(|clip| bez_path_to_svg_path_data(&(inverse * clip.clone())))
                                .collect(),
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                let new_hash = if entries.is_empty() {
                    0
                } else {
                    hash_mask_entries(size, &entries)
                };
                if node.native_mask_hash.get() != new_hash {
                    node.native_mask_hash.set(new_hash);
                    ctx.enqueue_native_message(pax_message::NativeMessage::NativeMaskUpdate(
                        NativeMaskPatch {
                            id: node.id.to_u32(),
                            size_x: size.0,
                            size_y: size.1,
                            entries,
                        },
                    ));
                }
            }
        }
    }
}

fn hash_mask_entries(size: (f64, f64), entries: &[MaskPathPatch]) -> u64 {
    let mut hasher = DefaultHasher::new();
    size.0.to_bits().hash(&mut hasher);
    size.1.to_bits().hash(&mut hasher);
    entries.hash(&mut hasher);
    hasher.finish()
}
