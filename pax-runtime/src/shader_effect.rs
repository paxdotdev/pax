//! Runtime glue for experimental GPU-only subtree filters.
//! Available with the opt-in `shaders` Cargo feature.
use crate::{ExpandedNode, RuntimeContext};
use pax_runtime_api::{borrow, Layer, LocalProperty, RenderContext};
use std::rc::Rc;

/// Subscribe an effect primitive's typed inputs and presentation geometry to canvas updates.
/// The default `InstanceNode::handle_mount` installs this for primitives with a shader recipe.
/// A primitive overriding that hook must call this after expanding its children.
pub fn bind_shader_effect(node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
    node.shader_effect_bound.set(true);
    let deps: Vec<_> = borrow!(node.properties_scope)
        .values()
        .map(|v| v.get_untyped_property().clone())
        .chain([
            node.presentation_transform_and_bounds.untyped(),
            #[cfg(feature = "3d")]
            node.presentation_geometry().untyped(),
            node.presentation_layer.untyped(),
            node.computed_opacity_scopes.untyped(),
            node.occlusion.untyped(),
            node.children.untyped(),
        ])
        .collect();
    let weak = Rc::downgrade(node);
    let context = Rc::clone(context);
    node.changed_listener.replace_with(LocalProperty::computed(
        move || {
            if let Some(node) = weak.upgrade() {
                context.mark_canvas_node_dirty(node.id);
                context.set_canvas_dirty(node.occlusion.get().render_layer_id);
            }
        },
        &deps,
    ));
}

impl ExpandedNode {
    pub(crate) fn render_shader_effect(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        renderer: &mut dyn RenderContext,
    ) -> bool {
        let Some(effect) = borrow!(self.instance_node).shader_effect(self) else {
            assert!(!self.shader_effect_bound.get(), "a mounted shader primitive must keep returning a recipe; use an identity pass to disable its effect");
            return false;
        };
        assert!(
            self.shader_effect_bound.get(),
            "shader primitives overriding mount must call bind_shader_effect"
        );
        if !renderer.is_gpu() {
            static WARNING: std::sync::Once = std::sync::Once::new();
            WARNING.call_once(|| log::warn!("CPU rendering preserves shader children without applying the effect; use @gpu / @cpu for an authored fallback"));
            return false;
        }
        let layer = self.occlusion.get().render_layer_id;
        let mut children = Vec::new();
        fn collect(node: &Rc<ExpandedNode>, layer: usize, children: &mut Vec<u32>) {
            if node.is_render_source() {
                return;
            }
            let instance = borrow!(node.instance_node);
            assert!(!matches!(instance.base().flags().layer, Layer::Native | Layer::NativeNonOccluding)
                && !instance.scrolls_content(node) && !instance.materializes_native_surface(node)
                && !node.is_unclippable(),
                "shader effects capture GPU content only: native controls/Text/NativeImage, descendant Scrollers, native materials and unclippable descendants are unsupported (node {:?})", node.id);
            if instance.base().flags().layer == Layer::Canvas {
                assert_eq!(
                    node.occlusion.get().render_layer_id,
                    layer,
                    "shader effect descendants must share one GPU layer"
                );
            }
            children.push(node.id.to_u32());
            if !node.shader_effect_bound.get() {
                for child in node.children.get().iter() {
                    collect(child, layer, children);
                }
            }
        }
        assert!(
            renderer.alpha_source_layer().is_none(),
            "shader effects inside detached mask sources are unsupported"
        );
        for child in self.children.get().iter() {
            collect(child, layer, &mut children);
        }
        let projection = context
            .canvas_surface_projection_for_node(self)
            .unwrap_or_else(|| {
                pax_runtime_api::math::ProjectedPlane::affine(
                    context.canvas_surface_transform_for_node(self),
                )
            });
        let replay = renderer.begin_projected_shader_effect(
            layer,
            self.id.to_u32(),
            self.occlusion.get().z_index,
            &effect,
            projection,
            self.presentation_transform_and_bounds.get().bounds,
            &self.computed_opacity_scopes.get(),
            &children,
        );
        // The captured children no longer participate in outer scene ordering.
        // Their composite must carry the effect's plane and Overlay/flight domain.
        #[cfg(feature = "3d")]
        renderer.set_node_depth_plane(
            layer,
            self.id.to_u32(),
            context.depth_plane_in_presentation(self, projection.depth_plane()),
        );
        if replay {
            fn collect_replay(node: &Rc<ExpandedNode>, ids: &mut Vec<u32>) {
                if node.is_render_source() {
                    return;
                }
                ids.push(node.id.to_u32());
                for child in node.children.get().iter() {
                    collect_replay(child, ids);
                }
            }
            let mut ids = Vec::new();
            for child in self.children.get().iter() {
                collect_replay(child, &mut ids);
            }
            // Capture replays unchanged drawing state. Invalidating geometry here
            // would make cold-surface preparation reject its own atomic fallback.
            context.mark_canvas_nodes_on_layer_dirty_by_id(layer, &ids);
        }
        for child in self.children.get().iter().rev() {
            child.recurse_render(context, renderer);
        }
        renderer.end_shader_effect(layer, self.id.to_u32());
        context.clear_canvas_node_dirty(&self.id);
        true
    }
}
