use std::iter;
use std::rc::Rc;

use pax_engine::pax;
use pax_runtime::api::{borrow, use_RefCell, Layer};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use_RefCell!();

/// Mounts a non-rendering subtree whose component descendants export selector settings.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::import_settings::ImportSettingsInstance")]
pub struct ImportSettings {}

pub struct ImportSettingsInstance {
    base: BaseInstance,
}

impl InstanceNode for ImportSettingsInstance {
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
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let sidecar_children = expanded_node.create_children_detached(
            children.iter().cloned().zip(iter::repeat(env)),
            context,
            &Rc::downgrade(expanded_node),
        );
        let sidecar_children = expanded_node.attach_sidecar_children(
            sidecar_children,
            context,
            &expanded_node.parent_frame,
        );
        for child in sidecar_children.iter() {
            child.recurse_control_flow_expansion(context);
        }
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let sidecar_children = std::mem::take(&mut *expanded_node.sidecar_children.borrow_mut());
        for child in sidecar_children {
            child.recurse_unmount(context);
        }
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("ImportSettings").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
