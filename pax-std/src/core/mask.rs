use std::iter;
use std::rc::Rc;

use crate::common::{native_surface_opacity, patch_if_needed};
use kurbo::Affine;
use pax_engine::*;
use pax_message::{AnyCreatePatch, FramePatch};
use pax_runtime::api::{
    bez_path_to_svg_path_data, borrow, borrow_mut, properties::UntypedProperty, use_RefCell, Layer,
    Property, RenderContext,
};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use_RefCell!();

/// Clips its first child by the unioned coverage path of its second child subtree.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::mask::MaskInstance")]
pub struct Mask {}

// Runtime instance backing `<Mask>`.
pub struct MaskInstance {
    base: BaseInstance,
}

impl MaskInstance {
    fn sync_mask_source_layout(expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let children = expanded_node.children.get();
        let mounted_children = borrow!(expanded_node.mounted_children);
        let children_changed = children.len() != mounted_children.len()
            || children
                .iter()
                .zip(mounted_children.iter())
                .any(|(child, mounted)| !Rc::ptr_eq(child, mounted));
        drop(mounted_children);

        let needs_parent_binding = children.iter().any(|child| {
            borrow!(child.render_parent)
                .upgrade()
                .is_none_or(|parent| !Rc::ptr_eq(&parent, expanded_node))
        });

        if children_changed || needs_parent_binding {
            expanded_node.attach_children(children.clone(), context, &expanded_node.parent_frame);
        }

        for child in children {
            Self::sync_mask_source_layout(&child, context);
        }
    }

    fn refresh_mask_source_layout(expanded_node: &ExpandedNode, context: &Rc<RuntimeContext>) {
        let mask_source = {
            let sidecar_children = borrow!(expanded_node.sidecar_children);
            sidecar_children.first().cloned()
        };
        if let Some(mask_source) = mask_source {
            Self::sync_mask_source_layout(&mask_source, context);
        }
    }

    fn mark_canvas_descendants_dirty(expanded_node: &ExpandedNode, context: &Rc<RuntimeContext>) {
        for child in expanded_node.children.get().iter() {
            if borrow!(child.instance_node).base().flags().layer == Layer::Canvas {
                context.mark_canvas_node_dirty(child.id);
                context.set_canvas_dirty(child.occlusion.get().render_layer_id);
            }
            Self::mark_canvas_descendants_dirty(child, context);
        }
    }

    fn resolve_mask_path(expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        let mask_source = {
            let sidecar_children = borrow!(expanded_node.sidecar_children);
            sidecar_children.first().cloned()
        };
        let Some(mask_source) = mask_source else {
            return None;
        };
        Self::resolve_subtree_coverage_path(&mask_source)
    }

    fn resolve_subtree_coverage_path(expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        let mut coverage = kurbo::BezPath::new();
        if let Some(path) =
            borrow!(expanded_node.instance_node).resolve_coverage_path(expanded_node)
        {
            coverage.extend(path.elements().iter().copied());
        }
        for child in expanded_node.children.get().iter() {
            if let Some(path) = Self::resolve_subtree_coverage_path(child) {
                coverage.extend(path.elements().iter().copied());
            }
        }

        if coverage.elements().is_empty() {
            None
        } else {
            Some(coverage)
        }
    }

    fn collect_mask_source_deps(expanded_node: &ExpandedNode, deps: &mut Vec<UntypedProperty>) {
        deps.push(expanded_node.transform_and_bounds.untyped());
        deps.extend(
            borrow!(expanded_node.properties_scope)
                .values()
                .cloned()
                .map(|v| v.get_untyped_property().clone()),
        );
        for child in expanded_node.children.get().iter() {
            Self::collect_mask_source_deps(child, deps);
        }
    }

    fn mask_path_for_layer(
        mask_path: &kurbo::BezPath,
        expanded_node: &ExpandedNode,
        layer: usize,
        context: &RuntimeContext,
    ) -> kurbo::BezPath {
        let Some(owner_id) = context.get_layer_scroller_owner(layer) else {
            return mask_path.clone();
        };
        let Some(owner) = context.get_expanded_node_by_eid(owner_id) else {
            return mask_path.clone();
        };
        let owner_inverse = Affine::from(owner.transform_and_bounds.get().transform.inverse());
        let mut layer_transform = owner_inverse;

        // If this mask contains the scroller that owns the target canvas layer, the mask is fixed
        // in the scroller viewport while the OS/browser moves the canvas host in content
        // coordinates. Convert the viewport-local mask into content coordinates so the retained
        // canvas clip stays aligned with the native view mask during scroll.
        if owner.is_descendant_of(&expanded_node.id) {
            if let Some(state) = context.get_scroller_surface_state(owner_id.to_u32()) {
                let scroll_x = if state.presentation_scroll_x.is_finite() {
                    state.presentation_scroll_x
                } else {
                    state.scroll_x
                };
                let scroll_y = if state.presentation_scroll_y.is_finite() {
                    state.presentation_scroll_y
                } else {
                    state.scroll_y
                };
                layer_transform = Affine::translate((scroll_x, scroll_y)) * layer_transform;
            }
        }

        layer_transform * mask_path.clone()
    }

    fn layer_clip_is_handled_by_scroller_dom(
        expanded_node: &ExpandedNode,
        layer: usize,
        context: &RuntimeContext,
    ) -> bool {
        let Some(owner_id) = context.get_layer_scroller_owner(layer) else {
            return false;
        };
        let Some(owner) = context.get_expanded_node_by_eid(owner_id) else {
            return false;
        };

        // Browser-owned scroller islands mount their canvas host inside the native scroller DOM
        // subtree. When that subtree is already inside this Mask's CSS clip container, applying a
        // second vector stencil clip in content coordinates double-clips and drifts during native
        // scroll. The DOM ancestor clip is the authoritative clip for that island.
        owner.is_descendant_of(&expanded_node.id)
    }
}

impl InstanceNode for MaskInstance {
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
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::FrameCreate(AnyCreatePatch {
            id,
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            render_layer_id: 0,
        }));

        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        if children.len() != 2 {
            log::warn!(
                "Mask expects exactly 2 direct children, got {}",
                children.len()
            );
        }
        let this_mask_prop = Property::new(Some(expanded_node.id));

        let mut itr = children.iter().cloned();
        if let Some(content_child) = itr.next() {
            let content = expanded_node.generate_children(
                iter::once((content_child, Rc::clone(&env))),
                context,
                &this_mask_prop,
                true,
            );
            expanded_node.children.set(content);
        } else {
            expanded_node.children.set(Vec::new());
        }

        if let Some(mask_child) = itr.next() {
            let sidecar = expanded_node.create_children_detached(
                iter::once((mask_child, Rc::clone(&env))),
                context,
                &Rc::downgrade(expanded_node),
            );
            let sidecar = expanded_node.attach_sidecar_children(
                sidecar,
                context,
                &expanded_node.parent_frame,
            );
            for child in sidecar.iter() {
                child.recurse_control_flow_expansion(context);
            }
        }

        let weak_self_ref = Rc::downgrade(expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(FramePatch {
            id,
            ..Default::default()
        }));

        let mut deps = vec![
            expanded_node.transform_and_bounds.untyped(),
            expanded_node.computed_opacity.untyped(),
            expanded_node.occlusion.untyped(),
        ];
        if let Some(mask_child) = borrow!(expanded_node.sidecar_children).first() {
            Self::sync_mask_source_layout(mask_child, &context);
            Self::collect_mask_source_deps(mask_child, &mut deps);
        }

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };

                    Self::refresh_mask_source_layout(&expanded_node, &context);
                    let clip_path = Self::resolve_mask_path(&expanded_node)
                        .map(|path| bez_path_to_svg_path_data(&path))
                        .unwrap_or_default();

                    let mut old_state = borrow_mut!(last_patch);
                    let mut patch = FramePatch {
                        id,
                        ..Default::default()
                    };
                    let computed_tab = expanded_node.transform_and_bounds.get();
                    let (width, height) = computed_tab.bounds;

                    let updates = [
                        patch_if_needed(
                            &mut old_state.clip_content,
                            &mut patch.clip_content,
                            !clip_path.is_empty(),
                        ),
                        patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                        patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                        patch_if_needed(
                            &mut old_state.transform,
                            &mut patch.transform,
                            computed_tab.transform.coeffs().to_vec(),
                        ),
                        patch_if_needed(&mut old_state.clip_path, &mut patch.clip_path, clip_path),
                        patch_if_needed(
                            &mut old_state.opacity,
                            &mut patch.opacity,
                            native_surface_opacity(&expanded_node, &context),
                        ),
                        patch_if_needed(
                            &mut old_state.parent_frame,
                            &mut patch.parent_frame,
                            expanded_node.parent_frame.get().map(|v| v.to_u32()),
                        ),
                        patch_if_needed(
                            &mut old_state.z_index,
                            &mut patch.z_index,
                            expanded_node.occlusion.get().z_index,
                        ),
                    ];

                    if updates.into_iter().any(|updated| updated) {
                        context
                            .enqueue_native_message(pax_message::NativeMessage::FrameUpdate(patch));
                        Self::mark_canvas_descendants_dirty(&expanded_node, &context);
                    }
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::FrameDelete(
            expanded_node.id.to_u32(),
        ));
    }

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        let layers = rcs.layers();
        let has_dirty_layer = (0..layers).any(|layer| rtc.is_canvas_dirty(&layer));
        if !has_dirty_layer {
            return;
        }

        Self::refresh_mask_source_layout(expanded_node, rtc);
        let Some(mask_path) = Self::resolve_mask_path(expanded_node) else {
            return;
        };

        #[cfg(debug_assertions)]
        let mut applied_layers = 0;
        #[cfg(debug_assertions)]
        let mut scroller_dom_layers = 0;

        for layer in 0..layers {
            if !rtc.is_canvas_dirty(&layer) {
                continue;
            }
            if Self::layer_clip_is_handled_by_scroller_dom(expanded_node, layer, rtc) {
                #[cfg(debug_assertions)]
                {
                    scroller_dom_layers += 1;
                }
                continue;
            }
            // Mask clips are stack effects for descendants, not leaf draw nodes. Keep them
            // unbounded so every active layer renderer receives the matching save/clip state.
            if !rcs.begin_node(
                layer,
                expanded_node.id.to_u32(),
                expanded_node.occlusion.get().z_index,
                0,
            ) {
                continue;
            }
            rcs.save(layer);
            rcs.clip(
                layer,
                Self::mask_path_for_layer(&mask_path, expanded_node, layer, rtc),
            );
            let _ = rcs.end_node(layer, expanded_node.id.to_u32());
            #[cfg(debug_assertions)]
            {
                applied_layers += 1;
            }
        }

        #[cfg(debug_assertions)]
        log::trace!(
            "mask clip pre_render: node={}, total_layers={}, applied_layers={}, scroller_dom_layers={}",
            expanded_node.id.to_u32(),
            layers,
            applied_layers,
            scroller_dom_layers,
        );
    }

    fn handle_post_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        let layers = rcs.layers();
        let has_dirty_layer = (0..layers).any(|layer| rtc.is_canvas_dirty(&layer));
        if !has_dirty_layer {
            return;
        }
        if Self::resolve_mask_path(expanded_node).is_none() {
            return;
        }

        #[cfg(debug_assertions)]
        let mut restored_layers = 0;
        #[cfg(debug_assertions)]
        let mut scroller_dom_layers = 0;

        for layer in 0..layers {
            if !rtc.is_canvas_dirty(&layer) {
                continue;
            }
            if Self::layer_clip_is_handled_by_scroller_dom(expanded_node, layer, rtc) {
                #[cfg(debug_assertions)]
                {
                    scroller_dom_layers += 1;
                }
                continue;
            }
            rcs.restore(layer);
            #[cfg(debug_assertions)]
            {
                restored_layers += 1;
            }
        }

        #[cfg(debug_assertions)]
        log::trace!(
            "mask clip post_render: node={}, total_layers={}, restored_layers={}, scroller_dom_layers={}",
            expanded_node.id.to_u32(),
            layers,
            restored_layers,
            scroller_dom_layers,
        );
    }

    fn resolve_effect_clip_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        Self::resolve_mask_path(expanded_node)
    }

    fn clips_content(&self, expanded_node: &ExpandedNode) -> bool {
        Self::resolve_mask_path(expanded_node).is_some()
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Mask").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
