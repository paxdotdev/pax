//! Experimental, renderer-owned subtree filters for userland primitives.
//!
//! Programs are Rust resources, not template values. Expose ordinary `Property<T>` inputs on
//! a primitive and resolve them into an instance through `InstanceNode::shader_effect`.

/// One host/shader parameter type. Every member occupies a 16-byte uniform slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderParameterType {
    Float,
    Uint,
    Vec2,
    Vec4,
}

/// A named field in the generated WGSL `parameters` uniform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShaderParameter {
    /// ASCII WGSL identifier used as `parameters.name`.
    pub name: &'static str,
    /// Uniform member type; values must match exactly.
    pub ty: ShaderParameterType,
}

/// A resolved value, encoded explicitly rather than casting Rust structs into GPU memory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderValue {
    Float(f32),
    Uint(u32),
    Vec2([f32; 2]),
    Vec4([f32; 4]),
}

impl ShaderValue {
    /// The uniform schema type represented by this value.
    pub fn ty(self) -> ShaderParameterType {
        match self {
            Self::Float(_) => ShaderParameterType::Float,
            Self::Uint(_) => ShaderParameterType::Uint,
            Self::Vec2(_) => ShaderParameterType::Vec2,
            Self::Vec4(_) => ShaderParameterType::Vec4,
        }
    }

    /// Encode one little-endian, zero-padded uniform slot; reject non-finite floats.
    pub fn encode(self) -> Result<[u8; 16], String> {
        let mut bytes = [0; 16];
        let (floats, count) = match self {
            Self::Uint(value) => {
                bytes[..4].copy_from_slice(&value.to_le_bytes());
                return Ok(bytes);
            }
            Self::Float(x) => ([x, 0., 0., 0.], 1),
            Self::Vec2([x, y]) => ([x, y, 0., 0.], 2),
            Self::Vec4(value) => (value, 4),
        };
        for (chunk, value) in bytes
            .chunks_exact_mut(4)
            .zip(floats.into_iter().take(count))
        {
            if !value.is_finite() {
                return Err("shader parameters must be finite".into());
            }
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }
}

/// Texture input to a pass. Earlier outputs share the capture's logical coordinate domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderInput {
    /// The rendered descendants before this effect.
    Subtree,
    /// Zero-based index of an earlier pass in this program.
    Pass(usize),
}

/// An experimental WGSL fragment pass. Implement `fn effect(p: vec2<f32>) -> vec4<f32>`.
/// `p` and `sample_input0/1(p)` use effect-local logical pixels; output is premultiplied RGBA.
/// The engine supplies entry points, texture bindings, transparent-outside sampling and uniforms.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderPass {
    /// Asset/source name included in shader diagnostics.
    pub label: &'static str,
    /// WGSL function definitions, typically packaged with `include_str!`.
    pub source: &'static str,
    /// Ordered schema for the generated `parameters` uniform.
    pub parameters: &'static [ShaderParameter],
    /// Sources of `sample_input0` and `sample_input1`; repeat one source when only one is needed.
    pub inputs: [ShaderInput; 2],
    /// Resolution divisor: 1 is full resolution; 2, 4, 8 and 16 are supported.
    pub downsample: u32,
}

/// Immutable userland pass recipe. At most eight passes and 32 parameters per pass.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderProgram {
    /// Human-readable name included in renderer diagnostics.
    pub label: &'static str,
    /// Ordered passes; the last pass supplies the premultiplied output.
    pub passes: &'static [ShaderPass],
}

/// Current instance parameters and its conservative sampling/output support.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderEffect {
    /// Immutable recipe shared across mounted instances.
    pub program: &'static ShaderProgram,
    /// One ordered value list per pass, matching that pass's schema.
    pub parameters: Vec<Vec<ShaderValue>>,
    /// Maximum absolute sampling displacement/output expansion on either local axis, in logical pixels.
    /// Keep this envelope stable during animation to reuse captures and attachments.
    pub outset: f64,
}

impl ShaderEffect {
    /// Validate the recipe and encode its parameters. Shader syntax is checked by the GPU backend.
    pub fn encode_parameters(&self) -> Result<Vec<Vec<u8>>, String> {
        let passes = self.program.passes;
        if passes.is_empty() || passes.len() > 8 || self.parameters.len() != passes.len() {
            return Err("expected 1–8 passes and one parameter list per pass".into());
        }
        if !self.outset.is_finite() || self.outset < 0.0 {
            return Err("outset must be finite and nonnegative".into());
        }
        passes
            .iter()
            .zip(&self.parameters)
            .enumerate()
            .map(|(i, (pass, values))| {
                if !matches!(pass.downsample, 1 | 2 | 4 | 8 | 16) {
                    return Err(format!(
                        "pass {}: downsample must be 1, 2, 4, 8 or 16",
                        pass.label
                    ));
                }
                if pass
                    .inputs
                    .iter()
                    .any(|v| matches!(v, ShaderInput::Pass(n) if *n >= i))
                {
                    return Err(format!(
                        "pass {}: inputs must reference the subtree or an earlier pass",
                        pass.label
                    ));
                }
                if pass.parameters.len() > 32 || pass.parameters.len() != values.len() {
                    return Err(format!(
                        "pass {}: parameter count does not match schema (maximum 32)",
                        pass.label
                    ));
                }
                let mut names = std::collections::HashSet::new();
                let mut bytes = Vec::new();
                for (parameter, value) in pass.parameters.iter().zip(values) {
                    if parameter.name.is_empty()
                        || !names.insert(parameter.name)
                        || !parameter.name.chars().enumerate().all(|(i, c)| {
                            c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                        })
                    {
                        return Err(format!(
                            "pass {}: invalid or duplicate parameter {}",
                            pass.label, parameter.name
                        ));
                    }
                    if parameter.ty != value.ty() {
                        return Err(format!(
                            "pass {}: parameter {} expects {:?}, got {:?}",
                            pass.label,
                            parameter.name,
                            parameter.ty,
                            value.ty()
                        ));
                    }
                    bytes.extend(value.encode().map_err(|e| {
                        format!("pass {}, parameter {}: {e}", pass.label, parameter.name)
                    })?);
                }
                bytes.resize(bytes.len().max(16), 0);
                Ok(bytes)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "uniform contract",
        passes: &[ShaderPass {
            label: "typed",
            source: "",
            parameters: &[
                ShaderParameter {
                    name: "number",
                    ty: ShaderParameterType::Uint,
                },
                ShaderParameter {
                    name: "point",
                    ty: ShaderParameterType::Vec2,
                },
                ShaderParameter {
                    name: "color",
                    ty: ShaderParameterType::Vec4,
                },
            ],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        }],
    };
    #[test]
    fn typed_slots_preserve_integer_bits_and_padding() {
        let effect = ShaderEffect {
            program: &PROGRAM,
            parameters: vec![vec![
                ShaderValue::Uint(u32::MAX),
                ShaderValue::Vec2([1., -2.]),
                ShaderValue::Vec4([0.1, 0.2, 0.3, 0.4]),
            ]],
            outset: 0.,
        };
        let bytes = effect.encode_parameters().unwrap().remove(0);
        assert_eq!(bytes.len(), 48);
        assert_eq!(&bytes[0..4], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[4..16], &[0; 12]);
        assert_eq!(&bytes[16..20], &1f32.to_le_bytes());
        assert_eq!(&bytes[20..24], &(-2f32).to_le_bytes());
        assert_eq!(&bytes[24..32], &[0; 8]);
        let mut invalid = effect.clone();
        invalid.parameters[0][0] = ShaderValue::Float(1.);
        assert!(invalid
            .encode_parameters()
            .unwrap_err()
            .contains("expects Uint"));
        invalid = effect;
        invalid.parameters[0][1] = ShaderValue::Vec2([f32::NAN, 0.]);
        assert!(invalid.encode_parameters().unwrap_err().contains("finite"));
    }
    #[test]
    fn rejects_pass_cycles_and_invalid_support() {
        static CYCLE: ShaderProgram = ShaderProgram {
            label: "cycle",
            passes: &[ShaderPass {
                label: "self",
                source: "",
                parameters: &[],
                inputs: [ShaderInput::Pass(0); 2],
                downsample: 1,
            }],
        };
        let mut effect = ShaderEffect {
            program: &CYCLE,
            parameters: vec![vec![]],
            outset: 0.,
        };
        assert!(effect
            .encode_parameters()
            .unwrap_err()
            .contains("earlier pass"));
        effect.outset = f64::INFINITY;
        assert!(effect.encode_parameters().unwrap_err().contains("outset"));
    }
}
