use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use pax_kit::*;
use std::rc::Rc;

/// A panel of gathered cloth. The shader supplies its uniform body colour.
/// The host supplies a cyclic phase and scoped Pax lights.
#[pax]
#[custom(Default)]
#[primitive("curtain_panel::sateen::SateenInstance")]
pub struct Sateen {
    pub phase: Property<f64>,
    /// Average fold pitch in logical pixels.
    pub spacing: Property<f64>,
    pub depth: Property<f64>,
    pub sheen: Property<f64>,
    pub roughness: Property<f64>,
    pub half_resolution: Property<bool>,
    pub identity: Property<bool>,
}

impl Default for Sateen {
    fn default() -> Self {
        Self {
            phase: Property::new(0.16),
            spacing: Property::new(187.),
            depth: Property::new(0.21),
            sheen: Property::new(0.25),
            roughness: Property::new(0.53),
            half_resolution: Property::new(true),
            identity: Property::new(false),
        }
    }
}

const fn float(name: &'static str) -> ShaderParameter {
    ShaderParameter {
        name,
        ty: ShaderParameterType::Float,
    }
}

const CLOTH: ShaderPass = ShaderPass {
    label: "Black sateen",
    source: include_str!("sateen.wgsl"),
    parameters: &[
        float("phase"),
        float("spacing"),
        float("depth"),
        float("sheen"),
        float("roughness"),
        ShaderParameter {
            name: "extent",
            ty: ShaderParameterType::Vec2,
        },
    ],
    inputs: [ShaderInput::SolidColor([19, 17, 20, 255]); 2],
    downsample: 1,
};
const IDENTITY: ShaderPass = ShaderPass {
    label: "Curtain comparison",
    source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
    // Include phase so playback invalidates the identity pass at the same cadence as the cloth.
    parameters: &[float("phase")],
    ..CLOTH
};

const fn program(label: &'static str, passes: &'static [ShaderPass]) -> ShaderProgram {
    ShaderProgram { label, passes }
}
static FULL: ShaderProgram = program("Sateen / full", &[CLOTH]);
static HALF: ShaderProgram = program(
    "Sateen / half",
    &[ShaderPass {
        downsample: 2,
        ..CLOTH
    }],
);
static IDENTITY_FULL: ShaderProgram = program("Identity / full", &[IDENTITY]);
static IDENTITY_HALF: ShaderProgram = program(
    "Identity / half",
    &[ShaderPass {
        downsample: 2,
        ..IDENTITY
    }],
);

impl Sateen {
    fn recipe(&self, (w, h): (f64, f64)) -> ShaderEffect {
        let identity = self.identity.get();
        let mut values = vec![ShaderValue::Float(self.phase.get().rem_euclid(1.) as f32)];
        if !identity {
            values.extend([
                ShaderValue::Float(self.spacing.get().clamp(100., 480.) as f32),
                ShaderValue::Float(self.depth.get().clamp(0., 2.) as f32),
                ShaderValue::Float(self.sheen.get().clamp(0., 1.5) as f32),
                ShaderValue::Float(self.roughness.get().clamp(0., 1.) as f32),
                ShaderValue::Vec2([w.max(1.) as f32, h.max(1.) as f32]),
            ]);
        }
        ShaderEffect {
            program: match (identity, self.half_resolution.get()) {
                (false, false) => &FULL,
                (false, true) => &HALF,
                (true, false) => &IDENTITY_FULL,
                (true, true) => &IDENTITY_HALF,
            },
            lighting: (!identity).then(ShaderLighting::default),
            parameters: vec![values],
            outset: 0.,
        }
    }
}

pub struct SateenInstance {
    base: BaseInstance,
}
impl InstanceNode for SateenInstance {
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
        f.debug_struct("Sateen").finish()
    }
    fn shader_effect(&self, node: &ExpandedNode) -> Option<ShaderEffect> {
        let bounds = node.presentation_transform_and_bounds.get().bounds;
        Some(node.with_properties_unwrapped(|cloth: &mut Sateen| cloth.recipe(bounds)))
    }
    fn property_requires_occlusion_recompute(&self, _: &str) -> bool {
        false
    }
}
