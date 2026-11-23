//! Bounded fragment-pass execution. Sources, pipelines and outputs have separate lifetimes so
//! changing a uniform never recompiles a pipeline or records the source scene again.
use super::effect_cells::{self, CellGrid, CellTexture};
use super::*;
use pax_runtime_api::{ShaderEffect, ShaderInput, ShaderParameterType, ShaderPass};
use wgpu::util::DeviceExt;

const MAX_EFFECT_BYTES: u64 = 256 * 1024 * 1024;

// Row layout matches EffectDomain in effect.wgsl.
pub(super) type EffectDomain = [[f32; 4]; 11];

// All supported pass divisors divide 64. Keep enough spare capacity for small
// resizes, and reclaim it once either axis shrinks substantially.
pub(super) fn storage_capacity(
    pixels: [u32; 2],
    previous: Option<[u32; 2]>,
    limit: u32,
) -> [u32; 2] {
    let previous = previous.unwrap_or_default();
    std::array::from_fn(|i| {
        let required = pixels[i];
        if previous[i] >= required && required > previous[i] / 4 {
            previous[i]
        } else {
            required.div_ceil(64).saturating_mul(64).min(limit)
        }
    })
}

pub(super) struct Effects {
    composite_layout: BindGroupLayout,
    composite: RenderPipeline,
    sampler: wgpu::Sampler,
    pipelines: PassPipelines,
    max_cell_dimension: u32,
    cells: Option<CellTexture>,
    output_bytes: u64,
    outputs: HashMap<u32, Output>,
}

struct PassOutput {
    view: TextureView,
    parameters: wgpu::Buffer,
    binding: BindGroup,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuEffectLighting {
    scene: data::GpuSceneLighting,
    origin: [f32; 4],
    tangent_x: [f32; 4],
    tangent_y: [f32; 4],
    normal: [f32; 4],
    mask: [u32; 4],
}

impl GpuEffectLighting {
    fn new(effect: &ShaderEffect, scene: &data::GpuSceneLighting) -> Self {
        let lighting = effect.lighting.unwrap_or_default();
        let frame = lighting.surface;
        let vector = |v: [f32; 3]| [v[0], v[1], v[2], 0.];
        Self {
            scene: if effect.lighting.is_some() {
                *scene
            } else {
                Default::default()
            },
            origin: vector(frame.origin),
            tangent_x: vector(frame.tangent_x),
            tangent_y: vector(frame.tangent_y),
            normal: vector(frame.normal),
            mask: [lighting.light_mask, 0, 0, 0],
        }
    }
}

struct Output {
    recipe: &'static pax_runtime_api::ShaderProgram,
    source: Option<Texture>,
    capacity: [u32; 2],
    domain: EffectDomain,
    domain_buffer: wgpu::Buffer,
    lighting: GpuEffectLighting,
    lighting_buffer: wgpu::Buffer,
    rect_buffer: wgpu::Buffer,
    passes: Vec<PassOutput>,
    composite: BindGroup,
    signature: Option<u64>,
    bytes: u64,
}

fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
fn sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

struct CompiledPass {
    prepare: Option<RenderPipeline>,
    shade: RenderPipeline,
}

// The label and source length select a bucket without reading the shader text.
// Equal recipes still share a pipeline, including separately declared copies.
#[derive(Clone, Copy)]
struct PassKey(&'static ShaderPass);

impl std::hash::Hash for PassKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&(self.0.label, self.0.source.len()), state);
    }
}

impl PartialEq for PassKey {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0) || self.0 == other.0
    }
}
impl Eq for PassKey {}

struct PassPipelines {
    cache: HashMap<(PassKey, bool), CompiledPass>,
    layout: BindGroupLayout,
    cell_layout: BindGroupLayout,
    format: TextureFormat,
}

impl PassPipelines {
    fn get(
        &mut self,
        device: &wgpu::Device,
        recipe: &'static ShaderPass,
        prepared: bool,
        stats: &mut crate::ResourceChurnStats,
    ) -> &CompiledPass {
        self.cache
            .entry((PassKey(recipe), prepared))
            .or_insert_with(|| {
                let source = shader_source(recipe, prepared)
                    .unwrap_or_else(|error| panic!("shader effect: {error}"));
                // Both entry points use the same author's code and uniform schema.
                // Validate and compile the module once when this recipe first appears.
                let module = shader_module(device, recipe.label, &source);
                let layouts = [&self.layout, &self.cell_layout];
                let prepare = prepared.then(|| {
                    pipeline(
                        device,
                        &module,
                        "fs_prepare_cells",
                        recipe.label,
                        &layouts[..1],
                        effect_cells::FORMAT,
                        1,
                        false,
                    )
                });
                let count = if prepared { 2 } else { 1 };
                let shade = pipeline(
                    device,
                    &module,
                    "fs_main",
                    recipe.label,
                    &layouts[..count],
                    self.format,
                    1,
                    false,
                );
                stats.effect_pipeline_compilations += count as u64;
                CompiledPass { prepare, shade }
            })
    }
}

fn shader_source(pass: &ShaderPass, prepared: bool) -> Result<String, String> {
    let mut source = String::from("struct Parameters {\n");
    if pass.parameters.is_empty() {
        source.push_str("@align(16) @size(16) unused: f32,\n");
    }
    for parameter in pass.parameters {
        let ty = match parameter.ty {
            ShaderParameterType::Float | ShaderParameterType::CellSpacing { .. } => "f32",
            ShaderParameterType::Uint => "u32",
            ShaderParameterType::Vec2 => "vec2<f32>",
            ShaderParameterType::Vec4 => "vec4<f32>",
        };
        source.push_str(&format!(
            "@align(16) @size(16) {}: {},\n",
            parameter.name, ty
        ));
    }
    let cells = effect_cells::declaration(pass);
    if cells.is_some() {
        source.push_str("pax_cell_rect: vec4<i32>,\n");
    }
    source.push_str("};\n@group(0) @binding(1) var<uniform> parameters: Parameters;\n");
    source.push_str(include_str!("scene_lighting.wgsl"));
    source.push_str(include_str!("effect_lighting.wgsl"));
    let pixels = if matches!(pass.inputs[0], ShaderInput::SolidColor(_)) {
        "domain.storage.xy"
    } else {
        "active_pixels(textureDimensions(input0))"
    };
    source.push_str(&include_str!("effect.wgsl").replace("PAX_INPUT0_ACTIVE_PIXELS", pixels));
    if pass
        .inputs
        .iter()
        .any(|input| matches!(input, ShaderInput::SolidColor(_)))
    {
        source.push_str(include_str!("effect_solid.wgsl"));
    }
    for (slot, input) in pass.inputs.iter().enumerate() {
        source.push_str(&format!(
            "fn sample_input{slot}(p: vec2<f32>) -> vec4<f32> {{\n"
        ));
        match input {
            ShaderInput::SolidColor([r, g, b, a]) => source.push_str(&format!(
                "return pax_effect_sample_solid(p, vec4<f32>({r}.0 / 255.0, {g}.0 / 255.0, {b}.0 / 255.0, 1.0) * ({a}.0 / 255.0));\n"
            )),
            _ => source.push_str(&format!(
                "let uv = input_uv(p);
                 let value = textureSampleLevel(input{slot}, input_sampler, storage_uv(uv, textureDimensions(input{slot})), 0.0);
                 return select(vec4<f32>(0.0), value, visible_local(p) && all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));\n"
            )),
        }
        source.push_str("}\n");
    }
    if let Some((i, _)) = cells {
        // Keep pixel-size queries inside prepare_cell anchored at the same
        // location in both the table and direct evaluation paths.
        source.push_str(&format!(
            "fn pax_evaluate_cell(c: vec2<i32>) -> vec4<f32> {{
            let previous = pax_effect_position;
            pax_effect_position = (vec2<f32>(c) + 0.5) * parameters.{};
            let value = prepare_cell(c);
            pax_effect_position = previous;
            return value;
        }}\n",
            pass.parameters[i].name
        ));
        if prepared {
            source.push_str(include_str!("effect_cell_lookup.wgsl"));
        } else {
            source.push_str(
                "fn load_cell(c: vec2<i32>) -> vec4<f32> {return pax_evaluate_cell(c); }\n",
            );
        }
    }
    source.push('\n');
    source.push_str(pass.source);
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|e| format!("{}: {}", pass.label, e.emit_to_string(&source)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| format!("{}: {}", pass.label, e.emit_to_string(&source)))?;
    // The wrapper owns all resources. Naga permits unused declarations to alias
    // their bindings, so checking only the binding range would admit extra resources.
    let resources: Vec<_> = module
        .global_variables
        .iter()
        .filter_map(|(_, variable)| variable.binding.as_ref())
        .collect();
    if module.entry_points.len() != 2 + usize::from(prepared)
        || resources.len() != 6 + usize::from(prepared)
        || resources.iter().any(|binding| {
            !(binding.group == 0 && binding.binding <= 5
                || prepared && binding.group == 1 && binding.binding == 0)
        })
    {
        return Err(format!(
            "{}: effect code cannot declare resources or entry points",
            pass.label
        ));
    }
    Ok(source)
}

fn shader_module(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    fragment: &str,
    label: &str,
    layouts: &[&BindGroupLayout],
    format: TextureFormat,
    samples: u32,
    composite: bool,
) -> RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: layouts,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: composite.then_some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: composite.then_some(wgpu::DepthStencilState {
            format: crate::render_backend::STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                // Reflections and spatial back faces obey the same clip as front faces.
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: !0,
                write_mask: 0,
            },
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

impl Effects {
    fn new(backend: &RenderBackend<'_>) -> Self {
        let device = &backend.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Effect pass"),
            entries: &[
                uniform(0),
                uniform(1),
                texture(2),
                texture(3),
                sampler(4),
                uniform(5),
            ],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Effect composition"),
            entries: &[uniform(0), uniform(1), texture(2), sampler(3)],
        });
        let alpha = alpha_mask::sampling_layout(device);
        let module = shader_module(
            device,
            "Premultiplied effect composition",
            include_str!("effect_composite.wgsl"),
        );
        let composite = pipeline(
            device,
            &module,
            "fs_main",
            "Premultiplied effect composition",
            &[&composite_layout, &alpha],
            backend.surface_config.format,
            backend.sample_count,
            true,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Effect linear sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mut field = texture(0);
        field.ty = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let cell_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Effect cell lookup"),
            entries: &[field],
        });
        let can_render_cells = backend
            .context
            .adapter
            .get_texture_format_features(effect_cells::FORMAT)
            .allowed_usages
            .contains(effect_cells::USAGE);
        Self {
            max_cell_dimension: if can_render_cells {
                device.limits().max_texture_dimension_2d
            } else {
                0
            },
            cells: None,
            output_bytes: 0,
            composite_layout,
            composite,
            sampler,
            pipelines: PassPipelines {
                cache: HashMap::new(),
                layout,
                cell_layout,
                format: backend.surface_config.format,
            },
            outputs: HashMap::new(),
        }
    }

    pub(super) fn draw(&self, id: u32, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(output) = self.outputs.get(&id) {
            pass.set_pipeline(&self.composite);
            pass.set_bind_group(0, &output.composite, &[]);
            pass.draw(0..6, 0..1);
        }
    }

    pub(super) fn retain(&mut self, active: &HashSet<u32>) {
        self.outputs.retain(|id, output| {
            if active.contains(id) {
                true
            } else {
                self.output_bytes -= output.bytes;
                false
            }
        });
        self.pipelines.cache.retain(|(recipe, _), _| {
            self.outputs
                .values()
                .any(|o| o.recipe.passes.iter().any(|p| PassKey(p) == *recipe))
        });
        if self.cells.is_some() && !self.pipelines.cache.keys().any(|(_, prepared)| *prepared) {
            self.cells = None;
        }
    }
}

impl RenderBackend<'_> {
    pub(crate) fn render_effect(
        &mut self,
        id: u32,
        effect: &ShaderEffect,
        parameters: &mut [Vec<u8>],
        projection: pax_runtime_api::math::ProjectedPlane,
        signature: u64,
        source_globals: &data::GpuGlobals,
        aligned_rect: [f32; 4],
        local_size: [f32; 2],
        scene_lighting: &data::GpuSceneLighting,
        stats: &mut crate::ResourceChurnStats,
    ) {
        let lighting = GpuEffectLighting::new(effect, scene_lighting);
        for (pass, bytes) in effect.program.passes.iter().zip(parameters.iter_mut()) {
            if effect_cells::declaration(pass).is_some() {
                bytes.resize((pass.parameters.len() + 1) * 16, 0);
            }
        }
        let (source, rect) = if effect.program.samples_subtree() {
            let (texture, rect) = self
                .retained_surfaces
                .as_ref()
                .unwrap()
                .effect_source(capture::CaptureKey::EffectSource(id));
            (Some(texture), rect)
        } else {
            (None, aligned_rect)
        };
        let mapping = projection.mapping();
        let inverse = mapping
            .inverse()
            .expect("shader effect requires an invertible projection");
        let rows = |t: pax_runtime_api::math::PlanarProjection| {
            std::array::from_fn::<_, 3, _>(|r| {
                [t.0[r] as f32, t.0[3 + r] as f32, t.0[6 + r] as f32, 0.]
            })
        };
        let a = rows(inverse);
        let b = rows(mapping);
        let clips = projection.camera_clip_rows().map(|r| {
            [
                r[0] as f32,
                r[1] as f32,
                r[2] as f32,
                projection.clips_depth() as u8 as f32,
            ]
        });
        let pixels = [
            (rect[2] * source_globals.dpr[0]).round() as u32,
            (rect[3] * source_globals.dpr[1]).round() as u32,
        ];
        let capacity = match &source {
            Some(source) => [source.width(), source.height()],
            None => storage_capacity(
                pixels,
                self.effects
                    .as_ref()
                    .and_then(|e| e.outputs.get(&id))
                    .map(|o| o.capacity),
                self.device.limits().max_texture_dimension_2d,
            ),
        };
        let storage = [
            pixels[0] as f32,
            pixels[1] as f32,
            capacity[0] as f32,
            capacity[1] as f32,
        ];
        let domain = [
            rect,
            a[0],
            a[1],
            a[2],
            b[0],
            b[1],
            b[2],
            clips[0],
            clips[1],
            storage,
            [local_size[0], local_size[1], 0., 0.],
        ];
        let final_downsample = effect.program.passes.last().unwrap().downsample;
        let composite_rect = [
            rect,
            [
                pixels[0].div_ceil(final_downsample) as f32
                    / capacity[0].div_ceil(final_downsample) as f32,
                pixels[1].div_ceil(final_downsample) as f32
                    / capacity[1].div_ceil(final_downsample) as f32,
                0.,
                0.,
            ],
        ];
        if self.effects.is_none() {
            self.effects = Some(Effects::new(self));
        }
        let effects = self.effects.as_mut().unwrap();
        if effects.outputs.get(&id).is_none_or(|o| {
            o.source != source
                || o.capacity != capacity
                || (!std::ptr::eq(o.recipe, effect.program) && o.recipe != effect.program)
        }) {
            let required: u64 = effect
                .program
                .passes
                .iter()
                .map(|p| {
                    4 * u64::from(capacity[0].div_ceil(p.downsample))
                        * u64::from(capacity[1].div_ceil(p.downsample))
                })
                .sum();
            let used = effects.output_bytes - effects.outputs.get(&id).map_or(0, |o| o.bytes);
            assert!(
                used + required <= MAX_EFFECT_BYTES,
                "shader effect {} exceeds the 256 MiB per-tile output budget",
                effect.program.label
            );
            effects.output_bytes = used + required;
            if effects
                .cells
                .as_ref()
                .is_some_and(|c| c.bytes() > MAX_EFFECT_BYTES - effects.output_bytes)
            {
                effects.cells = None;
            }
            let domain_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Effect coordinates"),
                    contents: bytemuck::cast_slice(&domain),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
            let lighting_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Effect scene lighting"),
                        contents: bytemuck::bytes_of(&lighting),
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });
            let source_view = source.as_ref().map(|t| t.create_view(&Default::default()));
            let mut passes: Vec<PassOutput> = Vec::new();
            for (recipe, values) in effect.program.passes.iter().zip(parameters.iter()) {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(recipe.label),
                    size: wgpu::Extent3d {
                        width: capacity[0].div_ceil(recipe.downsample),
                        height: capacity[1].div_ceil(recipe.downsample),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.surface_config.format,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let parameters =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Effect parameters"),
                            contents: values,
                            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                        });
                let input = |input| match input {
                    ShaderInput::Subtree => source_view.as_ref().unwrap(),
                    // Both slots share a bind group layout. Constant sampling never reads this view.
                    ShaderInput::SolidColor(_) => self.alpha_masks.transparent_view(),
                    ShaderInput::Pass(n) => &passes[n].view,
                };
                let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(recipe.label),
                    layout: &effects.pipelines.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: domain_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: parameters.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(input(recipe.inputs[0])),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(input(recipe.inputs[1])),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&effects.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: lighting_buffer.as_entire_binding(),
                        },
                    ],
                });
                passes.push(PassOutput {
                    view,
                    parameters,
                    binding,
                });
            }
            let rect_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Effect output rectangle"),
                    contents: bytemuck::cast_slice(&composite_rect),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
            let composite = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Effect output"),
                layout: &effects.composite_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: rect_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&passes.last().unwrap().view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&effects.sampler),
                    },
                ],
            });
            stats.effect_texture_allocations += passes.len() as u64;
            stats.effect_texture_bytes += required;
            effects.outputs.insert(
                id,
                Output {
                    recipe: effect.program,
                    source,
                    capacity,
                    domain,
                    domain_buffer,
                    lighting,
                    lighting_buffer,
                    rect_buffer,
                    passes,
                    composite,
                    signature: None,
                    bytes: required,
                },
            );
        }
        let output = effects.outputs.get_mut(&id).unwrap();
        if output.domain != domain {
            // Coordinate changes reuse pass textures and bindings. Only a new
            // source texture or recipe changes their resource dependencies.
            self.queue
                .write_buffer(&output.domain_buffer, 0, bytemuck::cast_slice(&domain));
            self.queue.write_buffer(
                &output.rect_buffer,
                0,
                bytemuck::cast_slice(&composite_rect),
            );
            output.domain = domain;
            output.signature = None;
        }
        if output.signature == Some(signature) {
            stats.effect_cache_hits += 1;
            return;
        }
        if bytemuck::bytes_of(&output.lighting) != bytemuck::bytes_of(&lighting) {
            self.queue
                .write_buffer(&output.lighting_buffer, 0, bytemuck::bytes_of(&lighting));
            output.lighting = lighting;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(effect.program.label),
            });
        for (i, (pass, recipe)) in output.passes.iter().zip(effect.program.passes).enumerate() {
            let values = &mut parameters[i];
            let grid = CellGrid::for_pass(
                recipe,
                &effect.parameters[i],
                &domain,
                effects.max_cell_dimension,
            );
            let grid = grid.filter(|g| {
                CellTexture::ensure(
                    &mut effects.cells,
                    &self.device,
                    &effects.pipelines.cell_layout,
                    g.size,
                    MAX_EFFECT_BYTES - effects.output_bytes,
                    stats,
                )
            });
            let pipelines = effects
                .pipelines
                .get(&self.device, recipe, grid.is_some(), stats);
            let field = if let Some(grid) = grid {
                let rect = [
                    grid.origin[0],
                    grid.origin[1],
                    grid.size[0] as i32,
                    grid.size[1] as i32,
                ];
                values
                    .last_chunk_mut::<16>()
                    .unwrap()
                    .copy_from_slice(bytemuck::bytes_of(&rect));
                let cells = effects.cells.as_ref().unwrap();
                draw_pass(
                    &mut encoder,
                    "Effect cell preparation",
                    &cells.view,
                    grid.size,
                    pipelines.prepare.as_ref().unwrap(),
                    &pass.binding,
                    None,
                    stats,
                );
                Some(&cells.binding)
            } else {
                None
            };
            self.queue.write_buffer(&pass.parameters, 0, values);
            let size = pixels.map(|v| v.div_ceil(recipe.downsample));
            draw_pass(
                &mut encoder,
                recipe.label,
                &pass.view,
                size,
                &pipelines.shade,
                &pass.binding,
                field,
                stats,
            );
        }
        output.signature = Some(signature);
        self.enqueue_command_buffer(encoder.finish());
    }
}

fn draw_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    view: &TextureView,
    size: [u32; 2],
    pipeline: &RenderPipeline,
    binding: &BindGroup,
    field: Option<&BindGroup>,
    stats: &mut crate::ResourceChurnStats,
) {
    let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render.set_pipeline(pipeline);
    render.set_bind_group(0, binding, &[]);
    if let Some(field) = field {
        render.set_bind_group(1, field, &[]);
    }
    let [width, height] = size;
    render.set_viewport(0., 0., width as f32, height as f32, 0., 1.);
    render.draw(0..3, 0..1);
    stats.effect_passes += 1;
    stats.effect_pixels += u64::from(width) * u64::from(height);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::ShaderParameter;
    #[test]
    fn shader_pipeline_keys_preserve_content_equality_on_hash_collisions() {
        const PASS: ShaderPass = ShaderPass {
            label: "test",
            source: "one",
            parameters: &[],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        };
        static FIRST: ShaderPass = PASS;
        static COPY: ShaderPass = PASS;
        static OTHER: ShaderPass = ShaderPass {
            source: "two",
            ..PASS
        };

        let mut keys = HashMap::new();
        keys.insert(PassKey(&FIRST), 1);
        keys.insert(PassKey(&OTHER), 2);
        assert_eq!(keys.get(&PassKey(&COPY)), Some(&1));
        assert_eq!(keys.get(&PassKey(&OTHER)), Some(&2));
    }

    #[test]
    fn rejects_additional_resources_on_engine_bindings() {
        for binding in 0..=5 {
            let source = format!(
                "@group(0) @binding({binding}) var extra: texture_2d<f32>;\n\
                 fn effect(p: vec2<f32>) -> vec4<f32> {{ return sample_input0(p); }}"
            );
            let pass = ShaderPass {
                label: "extra-resource.wgsl",
                source: Box::leak(source.into_boxed_str()),
                parameters: &[],
                inputs: [ShaderInput::Subtree; 2],
                downsample: 1,
            };
            let error = shader_source(&pass, false).expect_err("extra resources must be rejected");
            assert!(error.contains("extra-resource.wgsl: effect code cannot declare resources"));
        }
    }

    #[test]
    fn generated_uniform_layout_and_diagnostics() {
        let pass=ShaderPass {label:"user-color.wgsl",source:"fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p) * parameters.color; }",parameters:&[ShaderParameter {name:"color",ty:ShaderParameterType::Vec4}],inputs:[ShaderInput::Subtree;2],downsample:1};
        shader_source(&pass, false).unwrap();
        let mut invalid = pass.clone();
        invalid.source = "fn effect(p: vec2<f32>) -> vec4<f32> { return parameters.typo; }";
        let error = shader_source(&invalid, false).unwrap_err();
        assert!(error.contains("user-color.wgsl") && error.contains("typo"));
        invalid = pass;
        invalid.source="@compute @workgroup_size(1) fn extra() {} fn effect(p: vec2<f32>) -> vec4<f32> { return vec4<f32>(0.0); }";
        assert!(shader_source(&invalid, false)
            .unwrap_err()
            .contains("entry points"));
    }
}
