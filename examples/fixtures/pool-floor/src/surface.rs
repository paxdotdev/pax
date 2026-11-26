use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

#[pax]
#[primitive("pool_floor::surface::WaterSurfaceInstance")]
pub struct WaterSurface {
    pub phase: Property<f64>,
    pub density: Property<f64>,
    pub amplitude: Property<f64>,
    pub full_resolution: Property<bool>,
    pub identity: Property<bool>,
}

const WATER: ShaderPass = ShaderPass {
    label: "Pool floor / caustics",
    source: include_str!("water.wgsl"),
    parameters: &[ShaderParameter {
        name: "wave",
        ty: ShaderParameterType::Vec4,
    }],
    inputs: [ShaderInput::Subtree; 2],
    downsample: 2,
};

const IDENTITY: ShaderPass = ShaderPass {
    label: "Pool floor / identity",
    source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
    ..WATER
};

const HALF: ShaderProgram = ShaderProgram {
    label: "Pool floor",
    passes: &[WATER],
};
const FULL: ShaderProgram = ShaderProgram {
    label: "Pool floor / full",
    passes: &[ShaderPass {
        downsample: 1,
        ..WATER
    }],
};
const IDENTITY_HALF: ShaderProgram = ShaderProgram {
    label: "Pool floor / identity",
    passes: &[IDENTITY],
};
const IDENTITY_FULL: ShaderProgram = ShaderProgram {
    label: "Pool floor / identity full",
    passes: &[ShaderPass {
        downsample: 1,
        ..IDENTITY
    }],
};

pub struct WaterSurfaceInstance {
    base: BaseInstance,
}

impl InstanceNode for WaterSurfaceInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::Canvas,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("WaterSurface").finish()
    }

    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        Some(node.with_properties_unwrapped(|p: &mut WaterSurface| {
            let density = p.density.get();
            let density = if density.is_nan() {
                1.
            } else {
                density.clamp(0.25, 3.)
            };
            let program = match (p.identity.get(), p.full_resolution.get()) {
                (false, false) => &HALF,
                (false, true) => &FULL,
                (true, false) => &IDENTITY_HALF,
                (true, true) => &IDENTITY_FULL,
            };
            // The identity comparison shares the cadence, lighting dependency and resolution.
            ShaderEffect {
                program,
                lighting: Some(Default::default()),
                outset: 0.,
                parameters: vec![vec![ShaderValue::Vec4([
                    p.phase.get() as f32,
                    density as f32,
                    p.amplitude.get().clamp(0., 1.) as f32,
                    program.passes[0].downsample as f32,
                ])]],
            }
        }))
    }

    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}
