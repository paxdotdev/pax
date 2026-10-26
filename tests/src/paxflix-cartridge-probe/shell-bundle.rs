use crate::*;
use pax_engine::pax_runtime::{self as rt, InstanceNode};
use std::rc::Rc;
pub fn capture(_: &NodeContext) {}
#[pax]
#[primitive("crate::bundle::OutletInstance")]
pub struct DeferredPaxflix {
    pub embedded: Property<bool>,
}
pub struct OutletInstance {
    base: rt::BaseInstance,
}
impl InstanceNode for OutletInstance {
    fn instantiate(args: rt::InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: rt::BaseInstance::new(
                args,
                rt::InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: rt::api::Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }
    fn base(&self) -> &rt::BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&rt::ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("DeferredPaxflix").finish()
    }
}
