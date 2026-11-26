use pax_engine::pax;
#[cfg(feature = "designtime")]
use pax_message::borrow_mut;
use pax_message::{AnyCreatePatch, FramePatch, NativeMessage};
use pax_runtime::api::{LocalProperty, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

pax_message::use_RefCell!();

use pax_runtime::api::Layer;

/// A bounded viewport for an independently built application registered by the host.
/// Named guest sessions currently mount on macOS.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::inline_frame::InlineFrameInstance")]
pub struct InlineFrame {
    /// Name registered by the launcher or published in its trusted CLI descriptor directory.
    pub session: Property<String>,
}

// Runtime instance backing `<InlineFrame>`.
pub struct InlineFrameInstance {
    base: BaseInstance,
}

impl InstanceNode for InlineFrameInstance {
    fn property_requires_input_recompute(&self, _property_name: &str) -> bool {
        false
    }

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

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&pax_runtime::ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_g: &mut InlineFrame| {
                    f.debug_struct("InlineFrame").finish()
                })
            }
            None => f.debug_struct("InlineFrame").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn handle_mount(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let session = expanded_node
            .with_properties_unwrapped(|frame: &mut InlineFrame| frame.session.local());
        #[cfg(feature = "designtime")]
        if let Some(instance_node) = ctx
            .get_userland_root_instance_node()
            .filter(|_| session.get().is_empty())
        {
            // Legacy joint designer cartridges supply their root inside this runtime.
            let children = expanded_node.generate_children(
                vec![(instance_node, ctx.globals().stack_frame())],
                ctx,
                &expanded_node.parent_frame,
                true,
            );
            *borrow_mut!(ctx.userland_root_expanded_node) = children.first().cloned();
            expanded_node.children.set(children);
            return;
        }

        let id = expanded_node.id.to_u32();
        ctx.enqueue_native_message(NativeMessage::FrameCreate(AnyCreatePatch {
            id,
            parent_frame: expanded_node.parent_frame.get().map(|frame| frame.to_u32()),
            render_layer_id: 0,
        }));
        let placement = expanded_node.native_placement(ctx);
        let deps = [
            placement.untyped(),
            session.untyped(),
            expanded_node.presentation_transform_and_bounds.untyped(),
            expanded_node.parent_frame.untyped(),
            expanded_node.occlusion.untyped(),
            expanded_node.computed_opacity.untyped(),
        ];
        let node = Rc::downgrade(expanded_node);
        let ctx = Rc::clone(ctx);
        expanded_node
            .changed_listener
            .replace_with(LocalProperty::computed(
                move || {
                    let Some(node) = node.upgrade() else { return };
                    let (width, height) = node.presentation_transform_and_bounds.get().bounds;
                    let (owner, transform) = placement.get();
                    ctx.enqueue_native_message(NativeMessage::FrameUpdate(FramePatch {
                        id,
                        inline_session: Some(session.get()),
                        size_x: Some(width),
                        size_y: Some(height),
                        transform: Some(transform),
                        transform_scroller: Some(owner),
                        parent_frame: Some(node.parent_frame.get().map(|frame| frame.to_u32())),
                        z_index: Some(node.occlusion.get().z_index),
                        opacity: Some(crate::common::native_surface_opacity(&node, &ctx)),
                        clip_content: Some(true),
                        clip_is_rect: Some(true),
                        ..Default::default()
                    }));
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        #[cfg(feature = "designtime")]
        if !node.children.get().is_empty() {
            return;
        }
        node.changed_listener.replace_with(LocalProperty::default());
        ctx.enqueue_native_message(NativeMessage::FrameDelete(node.id.to_u32()));
    }
}
