use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

/// Experimental monochrome stencil corona. The children remain the CPU fallback.
///
/// Supply the two opaque, solid colors actually used by the source. The foreground
/// selects emitting coverage independently of its luminance. Source masks and
/// opacity are captured before the effect. Rays spread from every aperture edge,
/// including into the painted letter regions, while the emitting core stays crisp.
#[pax]
#[primitive("logo_corona::corona::CoronaInstance")]
pub struct Corona {
    pub foreground: Property<Color>,
    pub background: Property<Color>,
    /// Periodic motion position in cycles. The caller owns time and visibility.
    pub phase: Property<f64>,
    /// Emission-layer opacity; zero leaves only the surface treatment.
    pub emission: Property<f64>,
    /// Maximum projection distance in the canonical logo's local pixels, 0–120.
    pub reach: Property<f64>,
    /// Variation between short and long beams, 0–1.
    pub amplitude: Property<f64>,
    /// Distance attenuation exponent, 0.5–5.
    pub falloff: Property<f64>,
    /// Surface tonal depth, 0–0.35. Zero retains the original core colors.
    pub surface: Property<f64>,
}

const PARAMETERS: &[ShaderParameter] = &[
    ShaderParameter {
        name: "foreground",
        ty: ShaderParameterType::Vec4,
    },
    ShaderParameter {
        name: "background",
        ty: ShaderParameterType::Vec4,
    },
    ShaderParameter {
        name: "phase",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "emission",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "reach",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "amplitude",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "falloff",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "surface",
        ty: ShaderParameterType::Float,
    },
];

static PROGRAM: ShaderProgram = ShaderProgram {
    label: "Pax Corona",
    passes: &[
        ShaderPass {
            label: "corona-beams.wgsl",
            source: concat!(
                include_str!("corona_common.wgsl"),
                include_str!("corona_beams.wgsl")
            ),
            parameters: PARAMETERS,
            inputs: [ShaderInput::Subtree; 2],
            downsample: 2,
        },
        ShaderPass {
            label: "corona-composite.wgsl",
            source: concat!(
                include_str!("corona_common.wgsl"),
                include_str!("corona_composite.wgsl")
            ),
            parameters: PARAMETERS,
            inputs: [ShaderInput::Pass(0), ShaderInput::Subtree],
            downsample: 1,
        },
    ],
};

impl Corona {
    fn recipe(&self) -> ShaderEffect {
        let rgba = |c: Color| ShaderValue::Vec4(c.to_rgba_0_1().map(|v| v as f32));
        let values = vec![
            rgba(self.foreground.get()),
            rgba(self.background.get()),
            ShaderValue::Float(self.phase.get().rem_euclid(1.) as f32),
            ShaderValue::Float(self.emission.get().clamp(0., 1.) as f32),
            ShaderValue::Float(self.reach.get().clamp(0., 120.) as f32),
            ShaderValue::Float(self.amplitude.get().clamp(0., 1.) as f32),
            ShaderValue::Float(self.falloff.get().clamp(0.5, 5.) as f32),
            ShaderValue::Float(self.surface.get().clamp(0., 0.35) as f32),
        ];
        ShaderEffect {
            program: &PROGRAM,
            parameters: vec![values.clone(), values],
            // Reserve the whole tuning envelope plus reconstruction support so
            // changing reach cannot reallocate the capture on every slider event.
            outset: 144.,
        }
    }
}

pub struct CoronaInstance {
    base: BaseInstance,
}

impl InstanceNode for CoronaInstance {
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
        f.debug_struct("Corona").finish()
    }
    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        Some(node.with_properties_unwrapped(|p: &mut Corona| p.recipe()))
    }
    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}
