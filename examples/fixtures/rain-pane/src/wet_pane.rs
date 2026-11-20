use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

/// Fixture-local wet surface on the captured container plane, lit by scoped Pax lights.
/// The source must contain only renderer content. This is not a backdrop or material map.
/// The host owns the lights; this effect creates no emitters or light animation.
#[pax]
#[custom(Default)]
#[primitive("rain_pane::wet_pane::WetPaneInstance")]
pub struct WetPane {
    /// Animation time in seconds; starts at zero and is driven by the host.
    pub time: Property<f64>,
    /// Nominal descent speed in logical pixels/sec; defaults to 2.4.
    pub speed: Property<f64>,
    /// Arrival cadence multiplier (0–12), default 10; changes reschedule the analytic field.
    pub rain_frequency: Property<f64>,
    /// Fraction of active emitters (0–1); defaults to 0.69.
    pub wetness: Property<f64>,
    /// Size-spread multiplier (0.3–4) above the smallest primary radius; defaults to 0.7.
    pub drop_scale: Property<f64>,
    pub seed: Property<u32>,
    /// Specular highlight width (0.08–0.8), default 0.73; no diffuse body-colour fill.
    pub roughness: Property<f64>,
    /// Highlight brightness multiplier (0–2); defaults to 0.4.
    pub strength: Property<f64>,
    pub half_resolution: Property<bool>,
    pub identity: Property<bool>,
    pub quiet: Property<bool>,
}

impl Default for WetPane {
    fn default() -> Self {
        Self {
            time: Property::new(0.),
            speed: Property::new(2.4),
            rain_frequency: Property::new(10.),
            wetness: Property::new(0.69),
            drop_scale: Property::new(0.7),
            seed: Property::new(17),
            roughness: Property::new(0.73),
            strength: Property::new(0.4),
            half_resolution: Property::new(true),
            identity: Property::new(false),
            quiet: Property::new(false),
        }
    }
}

const PARAMETERS: &[ShaderParameter] = &[
    ShaderParameter {
        name: "time",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "speed",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "wetness",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "scale",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "seed",
        ty: ShaderParameterType::Uint,
    },
    ShaderParameter {
        name: "roughness",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "strength",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "extent",
        ty: ShaderParameterType::Vec2,
    },
    ShaderParameter {
        name: "quiet",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "frequency",
        ty: ShaderParameterType::Float,
    },
];

const PASS: ShaderPass = ShaderPass {
    label: "rain-pane.wgsl",
    source: include_str!("rain.wgsl"),
    parameters: PARAMETERS,
    inputs: [ShaderInput::Subtree; 2],
    downsample: 1,
};
static FULL: ShaderProgram = ShaderProgram {
    label: "Rain / full",
    passes: &[PASS],
};
static HALF: ShaderProgram = ShaderProgram {
    label: "Rain / half",
    passes: &[ShaderPass {
        downsample: 2,
        ..PASS
    }],
};
static IDENTITY_FULL: ShaderProgram = ShaderProgram {
    label: "Rain / identity capture",
    passes: &[ShaderPass {
        label: "rain-identity.wgsl",
        source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
        parameters: &[],
        inputs: [ShaderInput::Subtree; 2],
        downsample: 1,
    }],
};
static IDENTITY_HALF: ShaderProgram = ShaderProgram {
    label: "Rain / identity capture / half",
    passes: &[ShaderPass {
        downsample: 2,
        ..IDENTITY_FULL.passes[0]
    }],
};

impl WetPane {
    fn recipe(&self, bounds: (f64, f64)) -> ShaderEffect {
        let identity = self.identity.get();
        let values = vec![
            ShaderValue::Float(self.time.get() as f32),
            ShaderValue::Float(self.speed.get().clamp(0., 100.) as f32),
            ShaderValue::Float(self.wetness.get().clamp(0., 1.) as f32),
            ShaderValue::Float(self.drop_scale.get().clamp(0.3, 4.) as f32),
            ShaderValue::Uint(self.seed.get()),
            ShaderValue::Float(self.roughness.get().clamp(0.08, 0.8) as f32),
            ShaderValue::Float(self.strength.get().clamp(0., 2.) as f32),
            ShaderValue::Vec2([bounds.0 as f32, bounds.1 as f32]),
            ShaderValue::Float(if self.quiet.get() { 1. } else { 0. }),
            ShaderValue::Float(self.rain_frequency.get().clamp(0., 12.) as f32),
        ];
        ShaderEffect {
            program: if identity {
                if self.half_resolution.get() {
                    &IDENTITY_HALF
                } else {
                    &IDENTITY_FULL
                }
            } else if self.half_resolution.get() {
                &HALF
            } else {
                &FULL
            },
            lighting: if identity {
                None
            } else {
                Some(Default::default())
            },
            parameters: vec![if identity { vec![] } else { values }],
            // Only uniforms change during motion; the dark child can stay cached.
            outset: 0.,
        }
    }
}

pub struct WetPaneInstance {
    base: BaseInstance,
}
impl InstanceNode for WetPaneInstance {
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
        f.debug_struct("WetPane").finish()
    }
    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        let bounds = node.presentation_transform_and_bounds.get().bounds;
        Some(node.with_properties_unwrapped(|p: &mut WetPane| p.recipe(bounds)))
    }
    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_wgsl_validates_with_the_host_parameter_layout() {
        assert_eq!(PARAMETERS.len(), 10);
        assert!(PARAMETERS.len() <= 32);
        let mut source = String::from("struct Parameters {\n");
        for parameter in PARAMETERS {
            let ty = match parameter.ty {
                ShaderParameterType::Float | ShaderParameterType::CellSpacing { .. } => "f32",
                ShaderParameterType::Uint => "u32",
                ShaderParameterType::Vec2 => "vec2<f32>",
                ShaderParameterType::Vec4 => "vec4<f32>",
            };
            source.push_str(&format!("@align(16) @size(16) {}: {ty},\n", parameter.name));
        }
        source.push_str("};\n@group(0) @binding(1) var<uniform> parameters: Parameters;\n");
        // Validate against the actual entry/lighting wrappers: a stub cannot
        // catch restrictions caused by the host's visibility clipping branches.
        source.push_str(include_str!(
            "../../../../pax-gpu/src/render_backend/scene_lighting.wgsl"
        ));
        source.push_str(include_str!(
            "../../../../pax-gpu/src/render_backend/effect_lighting.wgsl"
        ));
        source.push_str(include_str!(
            "../../../../pax-gpu/src/render_backend/effect.wgsl"
        ));
        source.push_str(PASS.source);
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    }
}
