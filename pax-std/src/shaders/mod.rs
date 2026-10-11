//! Experimental shader containers for GPU-rendered subtrees.
//!
//! These primitives use the public `InstanceNode::shader_effect` extension point,
//! just as application-defined effects do. Requires the WGPU renderer.

macro_rules! effect_instance {
    ($instance:ident,$properties:ident) => {
        pub struct $instance {
            base: pax_runtime::BaseInstance,
        }

        impl pax_runtime::InstanceNode for $instance {
            fn instantiate(args: pax_runtime::InstantiationArgs) -> std::rc::Rc<Self> {
                std::rc::Rc::new(Self {
                    base: pax_runtime::BaseInstance::new(
                        args,
                        pax_runtime::InstanceFlags {
                            invisible_to_slot: false,
                            invisible_to_raycasting: true,
                            layer: pax_runtime::api::Layer::Canvas,
                            is_component: false,
                            is_slot: false,
                        },
                    ),
                })
            }
            fn base(&self) -> &pax_runtime::BaseInstance {
                &self.base
            }
            fn resolve_debug(
                &self,
                f: &mut std::fmt::Formatter,
                _: Option<&pax_runtime::ExpandedNode>,
            ) -> std::fmt::Result {
                f.debug_struct(stringify!($properties)).finish()
            }
            fn shader_effect(
                &self,
                node: &pax_runtime::ExpandedNode,
            ) -> Option<pax_runtime::api::ShaderEffect> {
                Some(node.with_properties_unwrapped(|p: &mut $properties| p.recipe()))
            }
            fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
                false
            }
        }
    };
}

pub mod gaussian_blur;
pub mod impressionist;

pub use gaussian_blur::*;
pub use impressionist::*;
