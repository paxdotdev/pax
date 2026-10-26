use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

/// Colors a white foreground by source-message column before LED glow is applied.
#[pax]
#[primitive("led_oscilloscope::foreground::ForegroundInkInstance")]
pub struct ForegroundInk {
    pub pitch: Property<f64>,
    pub scroll: Property<usize>,
    pub period: Property<usize>,
    /// Zero is white; one is a fully saturated rainbow. Alpha is unchanged.
    pub saturation: Property<f64>,
}

static INK: ShaderProgram = ShaderProgram {
    label: "LED foreground color",
    passes: &[ShaderPass {
        label: "led-foreground.wgsl",
        source: r#"
fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    // Use the same integer source-column address as Display::compose. Never color by time.
    let column = u32(max(floor(p.x / parameters.pitch), 0.0));
    let address = (column + parameters.scroll) % parameters.period;
    let hue = f32(address % 112u) / 112.0;
    let wheel = abs(fract(vec3<f32>(hue) + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0);
    let rainbow = clamp(wheel - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
    let tint = mix(vec3<f32>(1.0), rainbow, parameters.saturation);
    return vec4<f32>(source.rgb * tint, source.a);
}
"#,
        parameters: &[
            ShaderParameter {
                name: "pitch",
                ty: ShaderParameterType::Float,
            },
            ShaderParameter {
                name: "scroll",
                ty: ShaderParameterType::Uint,
            },
            ShaderParameter {
                name: "period",
                ty: ShaderParameterType::Uint,
            },
            ShaderParameter {
                name: "saturation",
                ty: ShaderParameterType::Float,
            },
        ],
        inputs: [ShaderInput::Subtree; 2],
        downsample: 1,
    }],
};

impl ForegroundInk {
    fn recipe(&self) -> ShaderEffect {
        ShaderEffect {
            program: &INK,
            parameters: vec![vec![
                ShaderValue::Float(self.pitch.get().max(1.) as f32),
                ShaderValue::Uint((self.scroll.get() % self.period.get().max(1)) as u32),
                ShaderValue::Uint(self.period.get().max(1) as u32),
                ShaderValue::Float(self.saturation.get().clamp(0., 1.) as f32),
            ]],
            outset: 0.,
        }
    }
}

pub struct ForegroundInkInstance {
    base: BaseInstance,
}

impl InstanceNode for ForegroundInkInstance {
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
        f.debug_struct("ForegroundInk").finish()
    }

    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        Some(node.with_properties_unwrapped(|p: &mut ForegroundInk| p.recipe()))
    }

    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}
