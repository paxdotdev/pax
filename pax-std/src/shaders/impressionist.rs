use pax_engine::*;
use pax_runtime::api::{
    ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
    ShaderProgram, ShaderValue as V,
};

/// Experimental painterly filter with deterministic marks anchored to the container.
///
/// Requires WGPU and captures vectors and renderer-backed images. Native descendants
/// are unsupported. Source colors update independently of the procedural brush motion.
/// Vello CPU renders the children without this effect; use `@gpu` / `@cpu`
/// to provide an alternate appearance.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::impressionist::ImpressionistInstance")]
pub struct Impressionist {
    /// Brush treatment. Defaults to `ImpressionistMode::Basic`.
    pub mode: Property<ImpressionistMode>,
    /// Motion position, initially zero. Drives mark drift, breathing, and VanGogh's evolving swirls.
    /// Source content can animate independently; a fixed phase freezes the procedural motion.
    /// Advance at a chosen rate for speed control, or bind a timeline to scrub/ease/reverse motion.
    pub phase: Property<f64>,
    /// Deterministic mark seed. Defaults to zero.
    pub seed: Property<u32>,
    /// Brush scale in local logical pixels, clamped to 4–32. Each mode applies its own mark proportions.
    /// Also scales directional patterns, including VanGogh's whorls. Defaults to zero (effective size 4).
    pub size: Property<f64>,
}

/// Procedural brush treatments inspired by different painting techniques.
///
/// These change mark construction and pigment handling, not scene composition.
/// All modes retain seeded anchors on the surface; marks move around those anchors with `phase`.
#[pax]
#[engine_import_path("pax_engine")]
#[derive(PartialEq, Eq)]
pub enum ImpressionistMode {
    /// The original three scales of softly layered, oval brush marks.
    #[default]
    Basic,
    /// Broad directional sweeps, simplified color planes, and quiet bristle texture.
    Gauguin,
    /// Phase-driven evolving flows with pronounced light and shadow along paint ridges.
    VanGogh,
    /// Closely packed rectangular dabs with interwoven pale and chromatic pigment.
    Signac,
    /// Varied patches of parallel marks and nested arcs, with broken, wispy contours.
    Cezanne,
    /// Fine, dense, rounded points of color with restrained pale pigment.
    Seurat,
}

impl ImpressionistMode {
    fn shader_value(&self) -> u32 {
        match self {
            Self::Basic => 0,
            Self::Gauguin => 1,
            Self::VanGogh => 2,
            Self::Signac | Self::Seurat => 3,
            Self::Cezanne => 4,
        }
    }
}

// A shared pointillist kernel separates brush construction from its painter presets.
// Keep these controls private while the vocabulary is being proven visually.
struct PointillistPreset {
    spacing: f32,
    extent: [f32; 2],
    roundness: f32,
    jitter: f32,
    direction: [f32; 2], // Group angle variation and attraction to source contours.
    ground: f32,
    white: [f32; 2], // Ordinary pigment mix and additional mix for pale dabs.
    white_chance: f32,
    pigment: [f32; 2], // Saturation and warm/cool variation.
    relief: f32,
    motion: [f32; 2], // Travel along/across a dab, relative to its spacing.
}

impl PointillistPreset {
    fn for_mode(mode: &ImpressionistMode) -> Self {
        match mode {
            ImpressionistMode::Seurat => Self {
                spacing: 0.12,
                extent: [0.57, 0.54],
                roundness: 1.,
                jitter: 0.8,
                direction: [1.8, 0.],
                ground: 0.03,
                white: [0.015, 0.16],
                white_chance: 0.12,
                pigment: [1.12, 1.65],
                relief: 0.008,
                motion: [0.24, 0.2],
            },
            _ => Self {
                spacing: 0.21,
                extent: [0.67, 0.48],
                roundness: 0.12,
                jitter: 0.62,
                direction: [0.85, 0.7],
                ground: 0.12,
                white: [0.06, 0.4],
                white_chance: 0.28,
                pigment: [1.25, 1.5],
                relief: 0.025,
                motion: [0.25, 0.12],
            },
        }
    }

    fn uniforms(&self, size: f32) -> [V; 11] {
        [
            V::Float(size * self.spacing),
            V::Vec2(self.extent),
            V::Float(self.roundness),
            V::Float(self.jitter),
            V::Vec2(self.direction),
            V::Float(self.ground),
            V::Vec2(self.white),
            V::Float(self.white_chance),
            V::Vec2(self.pigment),
            V::Float(self.relief),
            V::Vec2(self.motion),
        ]
    }
}

const fn parameter(name: &'static str, ty: T) -> ShaderParameter {
    ShaderParameter { name, ty }
}

const MODE: ShaderParameter = parameter("mode", T::Uint);
const PHASE: ShaderParameter = parameter("phase", T::Float);
const SEED: ShaderParameter = parameter("seed", T::Uint);
const SIZE: ShaderParameter = parameter("size", T::Float);

const ORIENTATION: ShaderPass = ShaderPass {
    label: "orientation.wgsl",
    source: include_str!("orientation.wgsl"),
    parameters: &[SIZE],
    inputs: [ShaderInput::Subtree; 2],
    downsample: 2,
};

const fn strokes(
    label: &'static str,
    source: &'static str,
    parameters: &'static [ShaderParameter],
) -> ShaderPass {
    ShaderPass {
        label,
        source,
        parameters,
        inputs: [ShaderInput::Subtree, ShaderInput::Pass(0)],
        downsample: 1,
    }
}

static PAINT: ShaderProgram = ShaderProgram {
    label: "Surface anchored paint",
    passes: &[
        ORIENTATION,
        strokes(
            "strokes.wgsl",
            concat!(
                include_str!("brush_common.wgsl"),
                include_str!("strokes.wgsl"),
                include_str!("gauguin.wgsl"),
                include_str!("van_gogh.wgsl"),
                include_str!("cezanne.wgsl"),
                include_str!("impressionist.wgsl"),
            ),
            &[MODE, PHASE, SEED, SIZE],
        ),
    ],
};

static POINTILLIST: ShaderProgram = ShaderProgram {
    label: "Pointillist dabs",
    passes: &[
        ORIENTATION,
        strokes(
            "pointillist.wgsl",
            concat!(
                include_str!("brush_common.wgsl"),
                include_str!("pointillist.wgsl"),
            ),
            &[
                PHASE,
                SEED,
                SIZE,
                parameter("point_spacing", T::CellSpacing { padding: 2 }),
                parameter("point_extent", T::Vec2),
                parameter("point_roundness", T::Float),
                parameter("point_jitter", T::Float),
                parameter("point_direction", T::Vec2),
                parameter("point_ground", T::Float),
                parameter("point_white", T::Vec2),
                parameter("point_white_chance", T::Float),
                parameter("point_pigment", T::Vec2),
                parameter("point_relief", T::Float),
                parameter("point_motion", T::Vec2),
            ],
        ),
    ],
};

impl Impressionist {
    fn recipe(&self) -> ShaderEffect {
        let size = self.size.get().clamp(4., 32.) as f32;
        let mode = self.mode.get();
        let mut marks = vec![
            V::Float(self.phase.get() as f32),
            V::Uint(self.seed.get()),
            V::Float(size),
        ];
        // Bound the sampling halo from the shader's reach, not the longest
        // brush at maximum size. Orientation samples another 0.4×size from
        // each accepted anchor. In Gauguin the farthest direct color tap
        // (1.5 + 0.25 spacings) is already inside that bound.
        let (program, reach) = match mode {
            ImpressionistMode::Signac | ImpressionistMode::Seurat => {
                let preset = PointillistPreset::for_mode(&mode);
                marks.extend(preset.uniforms(size));
                (&POINTILLIST, (1.55 * preset.spacing + 0.4) * size)
            }
            _ => {
                marks.insert(0, V::Uint(mode.shader_value()));
                let reach = if mode == ImpressionistMode::Gauguin {
                    (1.75 * 1.35 + 0.4) * size
                } else {
                    96.
                };
                (&PAINT, reach)
            }
        };
        // Allow for filtering on the half-resolution orientation grid and
        // bucket changes so small size adjustments can reuse the capture.
        let outset = (8. * ((f64::from(reach) + 4.) / 8.).ceil()).min(96.);
        ShaderEffect {
            lighting: None,
            program,
            parameters: vec![vec![V::Float(size)], marks],
            outset,
        }
    }
}

effect_instance!(ImpressionistInstance, Impressionist);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painter_modes_round_trip_through_template_values() {
        for mode in [
            ImpressionistMode::Basic,
            ImpressionistMode::Gauguin,
            ImpressionistMode::VanGogh,
            ImpressionistMode::Signac,
            ImpressionistMode::Cezanne,
            ImpressionistMode::Seurat,
        ] {
            let value = mode.clone().to_pax_value();
            assert_eq!(ImpressionistMode::try_coerce(value).unwrap(), mode);
            let effect = Impressionist::default();
            effect.mode.set(mode);
            effect.recipe().encode_parameters().unwrap();
        }
        assert_eq!(
            Impressionist::default().mode.get(),
            ImpressionistMode::Basic
        );
    }

    #[test]
    fn painter_halo_covers_the_outermost_tap_without_full_size_word_captures() {
        let p = Impressionist::default();
        for (mode, size, expected) in [
            (ImpressionistMode::Signac, 32., 32.),
            (ImpressionistMode::Seurat, 32., 24.),
            (ImpressionistMode::Gauguin, 4., 16.),
            (ImpressionistMode::Gauguin, 32., 96.),
            (ImpressionistMode::VanGogh, 4., 96.),
        ] {
            p.mode.set(mode);
            p.size.set(size);
            p.phase.set(0.);
            assert_eq!(p.recipe().outset, expected);
            p.phase.set(9.);
            assert_eq!(p.recipe().outset, expected);
        }
    }

    #[test]
    fn pointillist_grid_follows_the_preset_and_current_brush_size() {
        let painter = Impressionist::default();
        for (mode, size, expected_spacing) in [
            (ImpressionistMode::Signac, 64., Some(32. * 0.21)),
            (ImpressionistMode::Seurat, 2., Some(4. * 0.12)),
            (ImpressionistMode::Gauguin, 20., None),
            (ImpressionistMode::Seurat, 16., Some(16. * 0.12)),
        ] {
            painter.mode.set(mode);
            painter.size.set(size);
            let recipe = painter.recipe();
            let spacing = recipe.program.passes[1]
                .parameters
                .iter()
                .zip(&recipe.parameters[1])
                .find(|(parameter, _)| parameter.ty == T::CellSpacing { padding: 2 })
                .map(|(_, value)| *value);
            assert_eq!(spacing, expected_spacing.map(V::Float));
            recipe.encode_parameters().unwrap();
        }
    }

    #[test]
    fn unknown_painter_mode_is_a_template_error() {
        let value = PaxValue::Enum(Box::new((
            "ImpressionistMode".into(),
            "Unknown".into(),
            vec![],
        )));
        assert!(ImpressionistMode::try_coerce(value).is_err());
    }
}
