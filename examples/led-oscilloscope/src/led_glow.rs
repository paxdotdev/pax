use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

/// A small colored halo underneath the original, crisp LED image. Requires WGPU.
#[pax]
#[primitive("led_oscilloscope::led_glow::LedGlowInstance")]
pub struct LedGlow {
    /// Distance between LED centers in local pixels; sets the halo's scale.
    pub pitch: Property<f64>,
    /// Halo opacity, clamped to 0–1. Zero preserves the source image.
    pub strength: Property<f64>,
}

static GLOW: ShaderProgram = ShaderProgram {
    label: "LED glow",
    passes: &[
        ShaderPass {
            label: "led-glow-x.wgsl",
            source: r#"
fn effect(p: vec2<f32>) -> vec4<f32> {
    let step = vec2<f32>(parameters.spacing, 0.0);
    var color = sample_input0(p) * 0.227027;
    color += (sample_input0(p - step) + sample_input0(p + step)) * 0.1945946;
    color += (sample_input0(p - 2.0 * step) + sample_input0(p + 2.0 * step)) * 0.1216216;
    color += (sample_input0(p - 3.0 * step) + sample_input0(p + 3.0 * step)) * 0.054054;
    color += (sample_input0(p - 4.0 * step) + sample_input0(p + 4.0 * step)) * 0.016216;
    return color;
}
"#,
            parameters: &[ShaderParameter {
                name: "spacing",
                ty: ShaderParameterType::Float,
            }],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 2,
        },
        ShaderPass {
            label: "led-glow-composite.wgsl",
            source: r#"
fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input1(p);
    if parameters.strength <= 0.0 { return source; }
    let step = vec2<f32>(0.0, parameters.spacing);
    var halo = sample_input0(p) * 0.227027;
    halo += (sample_input0(p - step) + sample_input0(p + step)) * 0.1945946;
    halo += (sample_input0(p - 2.0 * step) + sample_input0(p + 2.0 * step)) * 0.1216216;
    halo += (sample_input0(p - 3.0 * step) + sample_input0(p + 3.0 * step)) * 0.054054;
    halo += (sample_input0(p - 4.0 * step) + sample_input0(p + 4.0 * step)) * 0.016216;

    // Stationary fine grain catches the moving light without a separate animation clock.
    let grain = fract(sin(dot(floor(p), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    halo *= parameters.strength * (0.94 + 0.06 * grain);
    // Source-over keeps the fully lit bulb cores unchanged and the output premultiplied.
    return source + halo * (1.0 - source.a);
}
"#,
            parameters: &[
                ShaderParameter {
                    name: "spacing",
                    ty: ShaderParameterType::Float,
                },
                ShaderParameter {
                    name: "strength",
                    ty: ShaderParameterType::Float,
                },
            ],
            inputs: [ShaderInput::Pass(0), ShaderInput::Subtree],
            downsample: 1,
        },
    ],
};

impl LedGlow {
    fn recipe(&self) -> ShaderEffect {
        let pitch = self.pitch.get().clamp(1., 64.);
        // Fixed nine-tap support scales with the lamps, never with their changing occupancy.
        let spacing = (pitch * 0.18) as f32;
        ShaderEffect {
            program: &GLOW,
            parameters: vec![
                vec![ShaderValue::Float(spacing)],
                vec![
                    ShaderValue::Float(spacing),
                    ShaderValue::Float(self.strength.get().clamp(0., 1.) as f32),
                ],
            ],
            // Includes four taps on either axis and the half-resolution sampling footprint.
            outset: (pitch * 0.72 + 3.).ceil(),
        }
    }
}

pub struct LedGlowInstance {
    base: BaseInstance,
}

impl InstanceNode for LedGlowInstance {
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
        f.debug_struct("LedGlow").finish()
    }

    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        Some(node.with_properties_unwrapped(|p: &mut LedGlow| p.recipe()))
    }

    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}
