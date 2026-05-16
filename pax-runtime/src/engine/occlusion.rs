use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use kurbo::{Affine, BezPath, Shape};
use pax_message::{borrow, MaskPathPatch, NativeMaskPatch, ScrollerPatch};
use pax_runtime_api::{bez_path_to_svg_path_data, Layer, Window};

use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};

use super::expanded_node::Occlusion;

// This pass still carries the historical "occlusion" name, but it no longer builds an
// alternating stack of vector/native compositing layers. It assigns z-order, computes native
// punch-through masks, and partitions canvas work into logical render layers. Layer 0 is the root
// surface stack; non-root layers are reserved for scroller-owned vector islands.

#[derive(Clone, Copy, Debug)]
/// Axis-aligned bounds used by the occlusion and native-mask pass.
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

    fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    fn new_from_transform_and_bounds_with_affine(
        t_and_b: TransformAndBounds<NodeLocal, Window>,
        affine: Affine,
    ) -> Self {
        let corners = t_and_b.corners();
        let mut x1 = f64::MAX;
        let mut y1 = f64::MAX;
        let mut x2 = f64::MIN;
        let mut y2 = f64::MIN;
        for c in corners {
            let c = affine * c;
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

    fn from_viewport(width: f64, height: f64) -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: width,
            y2: height,
        }
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        let intersection = Self {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        };
        (intersection.x2 > intersection.x1 && intersection.y2 > intersection.y1)
            .then_some(intersection)
    }

    fn as_array(&self) -> [f64; 4] {
        [self.x1, self.y1, self.x2, self.y2]
    }
}

#[derive(Clone)]
struct CoverageEntry {
    bounds: OcclusionBox,
    path: BezPath,
    clips: Vec<BezPath>,
    opacity: f64,
}

#[derive(Default)]
struct LayerCoverage {
    bounds: Option<OcclusionBox>,
    entries: Vec<CoverageEntry>,
}

impl LayerCoverage {
    fn push(&mut self, entry: CoverageEntry) {
        self.bounds = Some(match self.bounds {
            Some(bounds) => bounds.union(&entry.bounds),
            None => entry.bounds,
        });
        self.entries.push(entry);
    }
}

enum DrawableInfo {
    Canvas {
        layer_id: usize,
        entry: CoverageEntry,
    },
    Native {
        node: Rc<ExpandedNode>,
        layer: Layer,
        layer_id: usize,
        bounds: OcclusionBox,
        presentation_transform: Affine,
    },
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct NativeMaskStats {
    native_nodes: usize,
    updated_masks: usize,
    mask_entries: usize,
}

#[cfg(not(debug_assertions))]
#[derive(Default)]
struct NativeMaskStats;

/// Recompute z-order, native masks, and logical render-layer assignments for the tree.
pub fn update_node_occlusion(root_node: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    let pass_start = std::time::Instant::now();

    let mut drawables = Vec::new();
    let mut z_index = 0;
    let mut next_layer_id = 1;
    let viewport = ctx.globals().viewport.get();
    let viewport_bounds = OcclusionBox::from_viewport(viewport.bounds.0, viewport.bounds.1);
    ctx.clear_layer_scroller_owners();
    update_node_occlusion_recursive(
        root_node,
        ctx,
        0,
        None,
        &[],
        viewport_bounds,
        viewport_bounds,
        Affine::IDENTITY,
        &mut z_index,
        &mut next_layer_id,
        &mut drawables,
    );
    let _native_mask_stats = update_native_masks(&drawables, ctx);

    let new_layer_count = next_layer_id;
    if ctx.layer_count.get() != new_layer_count {
        ctx.layer_count.set(new_layer_count);
        ctx.enqueue_native_message(pax_message::NativeMessage::ShrinkLayersTo(
            new_layer_count as u32,
        ));
    }

    #[cfg(debug_assertions)]
    {
        let canvas_drawables = drawables
            .iter()
            .filter(|drawable| matches!(drawable, DrawableInfo::Canvas { .. }))
            .count();
        let native_drawables = drawables.len().saturating_sub(canvas_drawables);

        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        log::trace!(
            "occlusion pass: {}us, layers={}, scroller_layers={}, drawables={} canvas/{} native, native_masks={} updates/{} entries/{} nodes",
            pass_start.elapsed().as_micros(),
            new_layer_count,
            new_layer_count.saturating_sub(1),
            canvas_drawables,
            native_drawables,
            _native_mask_stats.updated_masks,
            _native_mask_stats.mask_entries,
            _native_mask_stats.native_nodes,
        );

        #[cfg(target_arch = "wasm32")]
        log::trace!(
            "occlusion pass: layers={}, scroller_layers={}, drawables={} canvas/{} native, native_masks={} updates/{} entries/{} nodes",
            new_layer_count,
            new_layer_count.saturating_sub(1),
            canvas_drawables,
            native_drawables,
            _native_mask_stats.updated_masks,
            _native_mask_stats.mask_entries,
            _native_mask_stats.native_nodes,
        );
    }
}

// Runtime is O(n^2) atm, but native punch-through work is grouped by logical render layer instead
// of by the old alternating vector/native layer stack.
fn update_node_occlusion_recursive(
    node: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    current_layer_id: usize,
    active_container: Option<u32>,
    active_clips: &[BezPath],
    viewport_bounds: OcclusionBox,
    active_clip_bounds: OcclusionBox,
    active_scroll_transform: Affine,
    z_index: &mut i32,
    next_layer_id: &mut usize,
    drawables: &mut Vec<DrawableInfo>,
) {
    fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
        if content <= viewport {
            return 0.0;
        }
        if !value.is_finite() {
            return 0.0;
        }
        value.max(0.0).min((content - viewport).max(0.0))
    }

    let instance_node = borrow!(node.instance_node);
    let effect_clip_path = instance_node
        .resolve_effect_clip_path(node)
        .map(|clip| active_scroll_transform * clip);
    let has_effect_clip = effect_clip_path.is_some();
    let scrolls_content = instance_node.scrolls_content(node);
    let scroll_transform = if scrolls_content {
        let root_delegates_to_page_scroll = ctx.get_root_scroller_id() == Some(node.id.to_u32())
            && ctx.get_visual_viewport_state().is_some();
        if root_delegates_to_page_scroll {
            Affine::IDENTITY
        } else {
            let (scroll_x, scroll_y) = if ctx.get_root_scroller_id() == Some(node.id.to_u32()) {
                if let Some(visual) = ctx.get_visual_viewport_state() {
                    let visual_x = visual.page_scroll_x + visual.offset_x;
                    let visual_y = visual.page_scroll_y + visual.offset_y;
                    if visual_x.is_finite() && visual_y.is_finite() {
                        if let Some(state) = ctx.get_scroller_surface_state(node.id.to_u32()) {
                            let viewport_width = if visual.width.is_finite() {
                                visual.width
                            } else {
                                state.viewport_width
                            };
                            let viewport_height = if visual.height.is_finite() {
                                visual.height
                            } else {
                                state.viewport_height
                            };
                            (
                                clamp_offset(visual_x, state.content_width, viewport_width),
                                clamp_offset(visual_y, state.content_height, viewport_height),
                            )
                        } else {
                            (visual_x, visual_y)
                        }
                    } else {
                        ctx.get_scroller_surface_scroll(node.id.to_u32())
                            .or_else(|| instance_node.resolve_scroll_offset(node))
                            .unwrap_or((0.0, 0.0))
                    }
                } else {
                    ctx.get_scroller_surface_scroll(node.id.to_u32())
                        .or_else(|| instance_node.resolve_scroll_offset(node))
                        .unwrap_or((0.0, 0.0))
                }
            } else {
                ctx.get_scroller_surface_scroll(node.id.to_u32())
                    .or_else(|| instance_node.resolve_scroll_offset(node))
                    .unwrap_or((0.0, 0.0))
            };
            if scroll_x.abs() > f64::EPSILON || scroll_y.abs() > f64::EPSILON {
                let world_transform = Affine::from(node.transform_and_bounds.get().transform);
                let inverse_world =
                    Affine::from(node.transform_and_bounds.get().transform.inverse());
                world_transform * Affine::translate((-scroll_x, -scroll_y)) * inverse_world
            } else {
                Affine::IDENTITY
            }
        }
    } else {
        Affine::IDENTITY
    };
    let allow_scroller_vector_layers = ctx.globals().browser_allows_scroller_vector_layers.get()
        && (ctx
            .globals()
            .browser_allows_nested_scroller_vector_layers
            .get()
            || active_container.is_none());
    let materializes_native_surface = instance_node.materializes_native_surface(node);
    let materializes_native_surface_before_children = materializes_native_surface
        && instance_node.materializes_native_surface_before_children(node);
    let layer = if materializes_native_surface {
        Layer::Native
    } else {
        instance_node.base().flags().layer
    };
    drop(instance_node);

    let descendant_layer_id = if scrolls_content && allow_scroller_vector_layers {
        let layer_id = *next_layer_id;
        *next_layer_id += 1;
        layer_id
    } else {
        current_layer_id
    };
    if scrolls_content && allow_scroller_vector_layers {
        ctx.register_layer_scroller_owner(descendant_layer_id, node.id);
    }
    let descendant_container = if scrolls_content {
        Some(node.id.to_u32())
    } else {
        has_effect_clip
            .then(|| node.id.to_u32())
            .or(active_container)
    };
    let descendant_scroll_transform = active_scroll_transform * scroll_transform;
    let presented_bounds = OcclusionBox::new_from_transform_and_bounds_with_affine(
        node.transform_and_bounds.get(),
        active_scroll_transform,
    );
    let mut descendant_clip_bounds = active_clip_bounds;
    if scrolls_content {
        descendant_clip_bounds = descendant_clip_bounds
            .intersect(&presented_bounds)
            .unwrap_or(descendant_clip_bounds);
    }
    if let Some(effect_clip_bounds) = effect_clip_path
        .as_ref()
        .and_then(OcclusionBox::new_from_path)
    {
        descendant_clip_bounds = descendant_clip_bounds
            .intersect(&effect_clip_bounds)
            .unwrap_or(descendant_clip_bounds);
    }
    let mut descendant_clips = active_clips.to_vec();
    if let Some(clip_path) = effect_clip_path.clone() {
        descendant_clips.push(clip_path);
    }

    let presented_clip_bounds = if has_effect_clip || scrolls_content {
        Some(descendant_clip_bounds)
    } else {
        Some(active_clip_bounds)
    };

    if materializes_native_surface_before_children {
        let new_occlusion = Occlusion {
            render_layer_id: current_layer_id,
            z_index: *z_index,
            parent_frame: active_container,
        };

        if new_occlusion != node.occlusion.get() {
            let previous_occlusion = node.occlusion.get();
            let prev_layer = previous_occlusion.render_layer_id;
            ctx.set_canvas_dirty(prev_layer);
            ctx.set_canvas_dirty(new_occlusion.render_layer_id);
            node.occlusion.set(new_occlusion);
        }

        drawables.push(DrawableInfo::Native {
            node: Rc::clone(node),
            layer,
            layer_id: current_layer_id,
            bounds: presented_bounds,
            presentation_transform: active_scroll_transform,
        });
        *z_index += 1;
    }

    for child in node.children.get().iter().rev() {
        let cp = child.get_common_properties();
        let cp = borrow!(cp);
        let unclippable = cp.unclippable.get().unwrap_or(false);
        let (child_container, child_clips, child_clip_bounds) = if unclippable {
            (None, Vec::new(), viewport_bounds)
        } else {
            (
                descendant_container,
                descendant_clips.clone(),
                descendant_clip_bounds,
            )
        };

        update_node_occlusion_recursive(
            child,
            ctx,
            descendant_layer_id,
            child_container,
            &child_clips,
            viewport_bounds,
            child_clip_bounds,
            descendant_scroll_transform,
            z_index,
            next_layer_id,
            drawables,
        );
    }

    if materializes_native_surface_before_children {
        return;
    }

    if layer == Layer::DontCare && !has_effect_clip {
        return;
    }

    let new_occlusion = Occlusion {
        render_layer_id: current_layer_id,
        z_index: *z_index,
        parent_frame: active_container,
    };

    if scrolls_content {
        let content_layer_id = allow_scroller_vector_layers.then_some(descendant_layer_id as u32);
        let presentation_hash =
            hash_presentation_bounds(Some(presented_bounds), presented_clip_bounds);
        let presentation_changed = node.presentation_cache_hash.get() != presentation_hash;
        if node.browser_content_layer_id.get() != content_layer_id || presentation_changed {
            ctx.enqueue_native_message(pax_message::NativeMessage::ScrollerUpdate(ScrollerPatch {
                id: node.id.to_u32(),
                content_layer_id,
                presented_bounds: Some(presented_bounds.as_array()),
                presented_clip_bounds: presented_clip_bounds.map(|bounds| bounds.as_array()),
                ..Default::default()
            }));
            node.browser_content_layer_id.set(content_layer_id);
            node.presentation_cache_hash.set(presentation_hash);
        }
    }

    if has_effect_clip {
        let presentation_hash =
            hash_presentation_bounds(Some(presented_bounds), presented_clip_bounds);
        if node.presentation_cache_hash.get() != presentation_hash {
            ctx.enqueue_native_message(pax_message::NativeMessage::FrameUpdate(
                pax_message::FramePatch {
                    id: node.id.to_u32(),
                    presented_bounds: Some(presented_bounds.as_array()),
                    presented_clip_bounds: presented_clip_bounds.map(|bounds| bounds.as_array()),
                    ..Default::default()
                },
            ));
            node.presentation_cache_hash.set(presentation_hash);
        }
    }

    if new_occlusion != node.occlusion.get() {
        let previous_occlusion = node.occlusion.get();
        let prev_layer = previous_occlusion.render_layer_id;
        if layer == Layer::Canvas && prev_layer != new_occlusion.render_layer_id {
            ctx.enqueue_canvas_node_removal(prev_layer, node.id.to_u32());
        }
        if layer == Layer::Canvas {
            ctx.mark_canvas_node_dirty(node.id);
        }
        ctx.set_canvas_dirty(prev_layer);
        ctx.set_canvas_dirty(new_occlusion.render_layer_id);
        node.occlusion.set(new_occlusion);
    }

    match layer {
        Layer::Canvas => {
            if let Some(coverage_path) = borrow!(node.instance_node).resolve_coverage_path(node) {
                let coverage_path = active_scroll_transform * coverage_path;
                if let Some(bounds) = OcclusionBox::new_from_path(&coverage_path) {
                    let opacity = borrow!(node.instance_node).resolve_coverage_opacity(node);
                    if opacity > f64::EPSILON {
                        drawables.push(DrawableInfo::Canvas {
                            layer_id: current_layer_id,
                            entry: CoverageEntry {
                                bounds,
                                path: coverage_path,
                                clips: active_clips.to_vec(),
                                opacity,
                            },
                        });
                    }
                }
            }
        }
        Layer::Native | Layer::NativeNonOccluding => {
            drawables.push(DrawableInfo::Native {
                node: Rc::clone(node),
                layer,
                layer_id: current_layer_id,
                bounds: presented_bounds,
                presentation_transform: active_scroll_transform,
            });
        }
        Layer::DontCare => {}
    }

    *z_index += 1;
}

fn update_native_masks(drawables: &[DrawableInfo], ctx: &RuntimeContext) -> NativeMaskStats {
    let mut vector_above = HashMap::<usize, LayerCoverage>::new();
    #[cfg(debug_assertions)]
    let mut stats = NativeMaskStats::default();
    #[cfg(not(debug_assertions))]
    let stats = NativeMaskStats::default();

    for drawable in drawables.iter().rev() {
        match drawable {
            DrawableInfo::Canvas { layer_id, entry } => {
                vector_above
                    .entry(*layer_id)
                    .or_default()
                    .push(entry.clone());
            }
            DrawableInfo::Native {
                node,
                layer,
                layer_id,
                bounds,
                presentation_transform,
            } => {
                #[cfg(debug_assertions)]
                {
                    stats.native_nodes += 1;
                }
                let t_and_b = node.transform_and_bounds.get();
                let size = t_and_b.bounds;
                let layer_coverage = vector_above.get(layer_id);
                let mut entries = if *layer == Layer::Native {
                    let has_overlap = layer_coverage
                        .and_then(|coverage| coverage.bounds)
                        .is_some_and(|coverage_bounds| coverage_bounds.intersects(bounds));
                    if has_overlap {
                        let inverse = Affine::from(t_and_b.transform.inverse())
                            * presentation_transform.inverse();
                        layer_coverage
                            .into_iter()
                            .flat_map(|coverage| coverage.entries.iter())
                            .filter(|entry| entry.bounds.intersects(bounds))
                            .map(|entry| MaskPathPatch {
                                path: bez_path_to_svg_path_data(&(inverse * entry.path.clone())),
                                clips: entry
                                    .clips
                                    .iter()
                                    .map(|clip| {
                                        bez_path_to_svg_path_data(&(inverse * clip.clone()))
                                    })
                                    .collect(),
                                opacity: Some(entry.opacity),
                            })
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                entries.sort_by(|lhs, rhs| {
                    lhs.path
                        .cmp(&rhs.path)
                        .then_with(|| lhs.clips.cmp(&rhs.clips))
                        .then_with(|| {
                            lhs.opacity
                                .map(f64::to_bits)
                                .cmp(&rhs.opacity.map(f64::to_bits))
                        })
                });

                let new_hash = if entries.is_empty() {
                    0
                } else {
                    hash_mask_entries(size, &entries)
                };
                if node.native_mask_hash.get() != new_hash {
                    node.native_mask_hash.set(new_hash);
                    #[cfg(debug_assertions)]
                    {
                        stats.updated_masks += 1;
                        stats.mask_entries += entries.len();
                    }
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

    stats
}

fn hash_mask_entries(size: (f64, f64), entries: &[MaskPathPatch]) -> u64 {
    let mut hasher = DefaultHasher::new();
    size.0.to_bits().hash(&mut hasher);
    size.1.to_bits().hash(&mut hasher);
    entries.hash(&mut hasher);
    hasher.finish()
}

fn hash_presentation_bounds(
    presented_bounds: Option<OcclusionBox>,
    presented_clip_bounds: Option<OcclusionBox>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    presented_bounds
        .map(|bounds| bounds.as_array().map(f64::to_bits))
        .hash(&mut hasher);
    presented_clip_bounds
        .map(|bounds| bounds.as_array().map(f64::to_bits))
        .hash(&mut hasher);
    hasher.finish()
}
