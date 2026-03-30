use std::iter;
use std::rc::Rc;

use crate::common::patch_if_needed;
use pax_engine::*;
use pax_message::{AnyCreatePatch, FramePatch};
use pax_runtime::api::{
    bez_path_to_svg_path_data, borrow, borrow_mut, use_RefCell, Layer, Property, RenderContext,
};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use_RefCell!();

/// Clips its first child by the coverage path of its second child.
///
/// Phase 1 only supports a direct geometry-backed primitive as the mask source.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::mask::MaskInstance")]
pub struct Mask {}

pub struct MaskInstance {
    base: BaseInstance,
}

impl MaskInstance {
    fn mark_canvas_descendants_dirty(expanded_node: &ExpandedNode, context: &Rc<RuntimeContext>) {
        for child in expanded_node.children.get().iter() {
            if borrow!(child.instance_node).base().flags().layer == Layer::Canvas {
                context.mark_canvas_node_dirty(child.id);
                context.set_canvas_dirty(child.occlusion.get().occlusion_layer_id);
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
        let path = borrow!(mask_source.instance_node).resolve_coverage_path(&mask_source);
        path
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
            occlusion_layer_id: 0,
        }));

        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        if children.len() != 2 {
            log::warn!("Mask expects exactly 2 direct children, got {}", children.len());
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
            for child in sidecar.iter() {
                child.recurse_control_flow_expansion(context);
            }
            expanded_node.attach_sidecar_children(sidecar, context, &expanded_node.parent_frame);
        }

        let weak_self_ref = Rc::downgrade(expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(FramePatch {
            id,
            ..Default::default()
        }));

        let mut deps = Vec::new();
        if let Some(mask_child) = borrow!(expanded_node.sidecar_children).first() {
            deps.push(mask_child.transform_and_bounds.untyped());
            deps.extend(
                borrow!(mask_child.properties_scope)
                    .values()
                    .cloned()
                    .map(|v| v.get_untyped_property().clone()),
            );
        }

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };

                    let clip_path = Self::resolve_mask_path(&expanded_node)
                        .map(|path| bez_path_to_svg_path_data(&path))
                        .unwrap_or_default();

                    let mut old_state = borrow_mut!(last_patch);
                    let mut patch = FramePatch {
                        id,
                        ..Default::default()
                    };

                    let updates = [
                        patch_if_needed(
                            &mut old_state.clip_content,
                            &mut patch.clip_content,
                            !clip_path.is_empty(),
                        ),
                        patch_if_needed(&mut old_state.clip_path, &mut patch.clip_path, clip_path),
                    ];

                    if updates.into_iter().any(|updated| updated) {
                        context.enqueue_native_message(pax_message::NativeMessage::FrameUpdate(
                            patch,
                        ));
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
        let total_layer_count = rtc.layer_count.get();
        let mut run_pre_render = false;
        for i in 0..total_layer_count {
            run_pre_render |= rtc.is_canvas_dirty(&i);
        }
        if !run_pre_render {
            return;
        }

        let Some(mask_path) = Self::resolve_mask_path(expanded_node) else {
            return;
        };

        let layers = rcs.layers();
        for layer in 0..layers {
            rcs.save(layer);
            rcs.clip(layer, mask_path.clone());
        }
    }

    fn handle_post_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        let total_layer_count = rtc.layer_count.get();
        let mut post_render = false;
        for i in 0..total_layer_count {
            post_render |= rtc.is_canvas_dirty(&i);
        }
        if !post_render {
            return;
        }
        if Self::resolve_mask_path(expanded_node).is_none() {
            return;
        }

        let layers = rcs.layers();
        for layer in 0..layers {
            rcs.restore(layer);
        }
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
