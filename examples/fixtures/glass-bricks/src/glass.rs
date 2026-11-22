use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

/// Opaque-backed pressed glass. An unlit painted child supplies coverage and the
/// CPU appearance. Scene lights belong to the host, on either side of the pane.
#[pax]
#[custom(Default)]
#[primitive("glass_bricks::glass::GlassBricksInstance")]
pub struct GlassBricks {
    /// Repeat distance in local logical pixels, including the mortar joint.
    pub pitch: Property<f64>,
    /// Apparent depth of the inset lip and absorption through the body (0–1).
    pub thickness: Property<f64>,
    /// Face waviness and highlight spread (0–1).
    pub roughness: Property<f64>,
    /// Strength of the concentrated light along the recessed inner lip (0–3).
    pub inner_bezel: Property<f64>,
    /// Strength of the concentrated light along the outer shoulder (0–3).
    pub outer_bezel: Property<f64>,
    /// Return the source pixel with identical capture and lighting dependencies.
    pub identity: Property<bool>,
}

impl Default for GlassBricks {
    fn default() -> Self {
        Self {
            pitch: Property::new(98.),
            thickness: Property::new(0.7),
            roughness: Property::new(0.42),
            inner_bezel: Property::new(1.),
            outer_bezel: Property::new(1.),
            identity: Property::new(false),
        }
    }
}

static GLASS: ShaderProgram = ShaderProgram {
    label: "Pressed glass bricks",
    passes: &[ShaderPass {
        label: "glass-bricks.wgsl",
        source: include_str!("glass.wgsl"),
        inputs: [ShaderInput::Subtree; 2],
        downsample: 1,
        parameters: &[
            ShaderParameter {
                name: "pitch",
                ty: ShaderParameterType::Float,
            },
            ShaderParameter {
                name: "thickness",
                ty: ShaderParameterType::Float,
            },
            ShaderParameter {
                name: "roughness",
                ty: ShaderParameterType::Float,
            },
            ShaderParameter {
                name: "inner_bezel",
                ty: ShaderParameterType::Float,
            },
            ShaderParameter {
                name: "outer_bezel",
                ty: ShaderParameterType::Float,
            },
            ShaderParameter {
                name: "identity",
                ty: ShaderParameterType::Uint,
            },
        ],
    }],
};

pub struct GlassBricksInstance {
    base: BaseInstance,
}

impl InstanceNode for GlassBricksInstance {
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
        f.debug_struct("GlassBricks").finish()
    }
    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        Some(
            node.with_properties_unwrapped(|p: &mut GlassBricks| ShaderEffect {
                program: &GLASS,
                lighting: Some(Default::default()),
                parameters: vec![vec![
                    ShaderValue::Float(p.pitch.get().clamp(32., 320.) as f32),
                    ShaderValue::Float(p.thickness.get().clamp(0., 1.) as f32),
                    ShaderValue::Float(p.roughness.get().clamp(0., 1.) as f32),
                    ShaderValue::Float(p.inner_bezel.get().clamp(0., 3.) as f32),
                    ShaderValue::Float(p.outer_bezel.get().clamp(0., 3.) as f32),
                    ShaderValue::Uint(p.identity.get() as u32),
                ]],
                outset: 0.,
            }),
        )
    }
    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}
