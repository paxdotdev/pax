# shader_effect
<!-- summary: Experimental, renderer-owned subtree filters for userland primitives. -->
<!-- tags: api, pax-runtime-api -->

Experimental, renderer-owned subtree filters for userland primitives.

Programs are Rust resources, not template values. Expose ordinary `Property<T>` inputs on
a primitive and resolve them into an instance through `InstanceNode::shader_effect`.

## Structs
### `ShaderEffect`
Current instance parameters and its conservative sampling/output support.

#### Properties
##### `program`
Type: &'`static` [`ShaderProgram`](../../api/pax-runtime-api/shader_effect.md#shaderprogram)

Immutable recipe shared across mounted instances.

##### `parameters`
Type: `Vec`<`Vec`<[`ShaderValue`](../../api/pax-runtime-api/shader_effect.md#shadervalue)>>

One ordered value list per pass, matching that pass's schema.

##### `outset`
Type: `f64`

Maximum absolute sampling displacement/output expansion on either local axis, in logical pixels.
Keep this envelope stable during animation to reuse captures and attachments.

#### Implementations
##### `encode_parameters`
<pre><code class="api-signature language-rust ignore">pub fn encode_parameters(&amp;self) -&gt; Result&lt;Vec&lt;Vec&lt;u8&gt;&gt;, String&gt;</code></pre>

Validate the recipe and encode its parameters. Shader syntax is checked by the GPU backend.

---

### `ShaderParameter`
A named field in the generated WGSL `parameters` uniform.

#### Properties
##### `name`
Type: &'`static` `str`

ASCII WGSL identifier used as `parameters.name`.

##### `ty`
Type: [`ShaderParameterType`](../../api/pax-runtime-api/shader_effect.md#shaderparametertype)

Uniform member type; values must match exactly.

---

### `ShaderPass`
An experimental WGSL fragment pass. Implement `fn effect(p: vec2<f32>) -> vec4<f32>`.
`p` and `sample_input0/1(p)` use effect-local logical pixels; output is premultiplied RGBA.
The engine supplies entry points, texture bindings, transparent-outside sampling and uniforms.

#### Properties
##### `label`
Type: &'`static` `str`

Asset/source name included in shader diagnostics.

##### `source`
Type: &'`static` `str`

WGSL function definitions, typically packaged with `include_str!`.

##### `parameters`
Type: &'`static` [[`ShaderParameter`](../../api/pax-runtime-api/shader_effect.md#shaderparameter)]

Ordered schema for the generated `parameters` uniform.

##### `inputs`
Type: [[`ShaderInput`](../../api/pax-runtime-api/shader_effect.md#shaderinput); 2]

Sources of `sample_input0` and `sample_input1`; repeat one source when only one is needed.

##### `downsample`
Type: `u32`

Resolution divisor: 1 is full resolution; 2, 4, 8 and 16 are supported.

---

### `ShaderProgram`
Immutable userland pass recipe. At most eight passes and 32 parameters per pass.

#### Properties
##### `label`
Type: &'`static` `str`

Human-readable name included in renderer diagnostics.

##### `passes`
Type: &'`static` [[`ShaderPass`](../../api/pax-runtime-api/shader_effect.md#shaderpass)]

Ordered passes; the last pass supplies the premultiplied output.

## Enums
### `ShaderInput`
Texture input to a pass. Earlier outputs share the capture's logical coordinate domain.

#### Variants
##### `Subtree`
The rendered descendants before this effect.

##### `Pass`(`usize`)
Zero-based index of an earlier pass in this program.

---

### `ShaderParameterType`
One host/shader parameter type. Every member occupies a 16-byte uniform slot.

#### Variants
##### `Float`
##### `Uint`
##### `Vec2`
##### `Vec4`
---

### `ShaderValue`
A resolved value, encoded explicitly rather than casting Rust structs into GPU memory.

#### Variants
##### `Float`(`f32`)
##### `Uint`(`u32`)
##### `Vec2`([`f32`; 2])
##### `Vec4`([`f32`; 4])
#### Implementations
##### `encode`
<pre><code class="api-signature language-rust ignore">pub fn encode(self) -&gt; Result&lt;[u8; 16], String&gt;</code></pre>

Encode one little-endian, zero-padded uniform slot; reject non-finite floats.

##### `ty`
<pre><code class="api-signature language-rust ignore">pub fn ty(self) -&gt; <a href="../../api/pax-runtime-api/shader_effect.md#shaderparametertype">ShaderParameterType</a></code></pre>

The uniform schema type represented by this value.
