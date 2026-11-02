use pax_engine::api::*;
use pax_engine::pax;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

/// Contributes viewport-local content above the ordinary scene. Multiple
/// declarations compose in authored traversal order, with earlier content above
/// later content. Its declaration still owns its lifetime and reactive scope.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::overlay::OverlayInstance")]
#[pax_document(children)]
pub struct Overlay {}

pub struct OverlayInstance {
    base: BaseInstance,
}

impl InstanceNode for OverlayInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
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
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn is_overlay(&self) -> bool {
        true
    }
    fn accepts_transferred_children(&self) -> bool {
        true
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Overlay").finish()
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let children = self.base.get_instance_children().borrow().clone();
        let env = node.stack.clone();
        let children = node.generate_children(
            children.into_iter().map(|n| (n, env.clone())),
            context,
            &node.parent_frame,
            true,
        );
        node.children.set(children);
    }
}
