# PAX-960 Light And Materials (Draft)

Authoring Date: 2026-06-03

<!-- summary: API and implementation proposal for light-reactive materials on Pax vector primitives. -->
<!-- tags: lighting, material, texture, gpu, drawing, camera, 3d -->

## Problem

Pax has a GPU vector renderer, but the public drawing surface still describes
2D primitives mainly through `fill`, `stroke`, opacity, transforms, and clipping.
That is enough for flat color and gradients, but it does not give authors a
first-class way to make vector surfaces react to light, depth, or material
response.

The requested direction is:

- a `material` property on drawing primitives
- a `LightSource` primitive with shape, intensity, color, 3D position, and
  direction
- a single effective ambient light per lighting domain, with a user-authored
  override for intentionally dark scenes
- default light interaction for vector elements when a light is present
- an API that does not paint the runtime into a corner before camera support
  and 3D primitives arrive

The first pass should make lighting feel native to Pax without requiring a full
3D scene graph, physically based renderer, or bitmap texture pipeline.

## Goals

- Preserve existing visual output for apps that do not use `LightSource`.
- Make light-reactive vector authoring declarative in Pax templates.
- Keep `fill` as the base color model; add material response alongside it rather
  than replacing it.
- Keep `LightSource` independent of z-order. A light should affect the relevant
  scene, not only nodes that happen to render after it.
- Give authors explicit control of ambient light so lit scenes can also be
  intentionally dark.
- Keep stock material presets extensible through user-authored helpers and a
  general material-parameter path.
- Use the existing WGPU vector pipeline for the initial implementation.
- Define coordinate semantics that can survive future camera and 3D element
  support.
- Make non-WGPU renderers degrade predictably instead of blocking the API.

## Non-goals

- A full PBR material model in the first pass.
- Image-backed albedo, normal, roughness, or displacement maps in the first
  pass.
- Lighting native platform controls in the first pass.
- Shadow maps, path-traced shadows, global illumination, bloom, or post effects.
- 3D mesh primitives, perspective cameras, or depth-buffered vector/native
  interop in this ticket.
- A CSS-style cascade for lights or materials.

## Existing Surface

Current vector primitives (`Rectangle`, `Ellipse`, `Path`, and `Line`) own Pax
properties, convert their local geometry into `kurbo` paths, and call the
backend-neutral `RenderContext` methods:

- `fill_with_opacity(layer, path, fill, opacity)`
- `stroke_with_opacity(layer, path, stroke, opacity)`

`PaxGpuRenderer` converts `Fill` into low-level `pax_gpu::Fill` data. The WGPU
geometry shader already receives per-primitive fill metadata, transforms,
opacity, colors, gradients, and a per-vertex normal field. Images use a separate
texture pipeline and are drawn as rectangular image resources.

That split matters:

- material data for vectors can fit naturally into the vector pipeline
- bitmap textures for arbitrary vector masks would require texture bind-group
  changes and batching decisions
- non-WGPU renderers can ignore new lighting metadata and keep drawing the same
  fill/stroke output

## Naming Recommendation

Use `material` as the author-facing primitive property. It is the correct term
for how a surface responds to light, and it leaves `texture` available for the
more precise future meaning of bitmap-backed texture data.

The docs should explicitly distinguish:

- `fill`: base color or gradient
- `material`: how the surface reacts to lights
- `Image`: a rectangular bitmap primitive
- `texture`: future bitmap-backed texture data, texture maps, or pattern fills

The authoring surface should not expose `texture` as an alias for `material`.
That would blur the future distinction between surface response and bitmap
resources.

## Proposed Authoring Model

### LightSource

`LightSource` should be a non-rendering primitive in `pax-std`, likely under the
drawing or effects namespace. It participates in template layout enough to
derive a 2D position, but it does not draw pixels or affect raycasting.

Conceptual template usage:

```pax
<LightSource
    x=50%
    y=20%
    z=260px
    shape=LightShape::Point
    radius=900px
    intensity=0.85
    color=rgba(255, 244, 220, 255)
/>

<LightSource
    shape=LightShape::Directional
    direction=Vector3::xyz(-0.25, -0.45, -1.0)
    intensity=0.35
    color=rgba(180, 210, 255, 255)
/>
```

Recommended first-pass properties:

```rust
pub struct LightSource {
    pub shape: Property<LightShape>,
    pub z: Property<Depth>,
    pub radius: Property<Size>,
    pub direction: Property<Vector3>,
    pub intensity: Property<f64>,
    pub color: Property<Color>,
    pub enabled: Property<bool>,
}

pub enum LightShape {
    Point,
    Directional,
}
```

`Depth` should be a logical-pixel scene-depth scalar, not `Size`. Percentages do
not have a clear meaning on the unbounded z axis. The authoring surface should
accept pixel depth such as `z=260px` and reject percent depth. If introducing a
new `Depth` value type is too much for the first implementation, a transitional
`Property<f64>` interpreted as logical pixels is acceptable, but it should not
pretend to support `%`.

`Spot` and `Area` are useful, but they add cone/area math and more API surface.
They should wait until point and directional lights are validated.

### AmbientLight

`AmbientLight` should be a non-rendering scene resource. Each lighting domain
has exactly one effective ambient light.

Conceptual usage:

```pax
<AmbientLight intensity=0.08 color=rgba(18, 22, 32, 255) />
<LightSource
    x=50%
    y=20%
    z=260px
    shape=LightShape::Point
    radius=900px
    intensity=0.85
    color=rgba(255, 244, 220, 255)
/>
```

Recommended first-pass properties:

```rust
pub struct AmbientLight {
    pub intensity: Property<f64>,
    pub color: Property<Color>,
    pub enabled: Property<bool>,
}
```

If multiple enabled `AmbientLight` nodes exist in one lighting domain, the one
with the highest render-tree precedence wins. In Pax terms, this should follow
the same ordering authors already use for visible z-order.

### Material On Vector Primitives

Add `material: Property<Material>` to vector drawing primitives:

- `Rectangle`
- `Ellipse`
- `Path`
- `Line`, applying to the stroke surface

Conceptual usage:

```pax
<Rectangle
    width=100%
    height=100%
    fill=@gradient {
        0%: rgba(40, 54, 78, 255)
        100%: rgba(80, 108, 150, 255)
    }
    material=Material::matte()
/>

<Ellipse
    width=160px
    height=160px
    fill=rgba(240, 238, 220, 255)
    material=Material::glossy(0.28)
/>

<Path
    elements={self.ribbon_path}
    fill=rgba(240, 128, 80, 255)
    material=Material::metallic(0.72)
/>
```

Recommended first-pass material shape:

```rust
pub enum Material {
    Lit(MaterialParams),
    Unlit,
}

pub struct MaterialParams {
    pub ambient: Property<f64>,
    pub diffuse: Property<f64>,
    pub specular: Property<f64>,
    pub roughness: Property<f64>,
    pub metallic: Property<f64>,
    pub emissive: Property<Color>,
    pub emissive_intensity: Property<f64>,
}
```

Stock materials should be constructors over `MaterialParams`, not closed enum
variants that require core-library extension for every new preset:

```rust
impl Material {
    pub fn matte() -> Material;
    pub fn glossy(roughness: f64) -> Material;
    pub fn metallic(roughness: f64) -> Material;
    pub fn emissive(color: Color, intensity: f64) -> Material;
    pub fn custom(params: MaterialParams) -> Material;
}
```

`Matte` should be the default. `Glossy` is the second most intuitive stock
material because it matches ordinary photographic language. `Metallic` is useful
for tinted specular response and familiar product/UI work. `Emissive` should
make a surface visibly self-lit in v1, but it should not illuminate other
surfaces until a later emission/global-lighting design.

`Unlit` is the explicit escape hatch for artwork or UI chrome that should ignore
scene lighting entirely.

The first extensible slice should be data-driven. Userland can define helpers,
components, or theme values that return `Material::custom(...)` without changing
core:

```rust
pub fn brushed_panel_material() -> Material {
    Material::custom(MaterialParams {
        // ...
    })
}
```

Custom shader-backed materials should be a later design. They require a shader
ABI, declared uniforms, backend capability checks, and batching rules. That is
useful, but it is a larger surface than this first material pass needs.

Avoid bitmap-backed material maps such as albedo, normal, or roughness textures
in the first slice. Those are important, but they change resource loading,
batching, UV semantics, and shader binding strategy.

## Default Behavior

Compatibility rule:

- with no authored lighting resources, rendering is identical to today
- with one or more authored lighting resources, vector primitives use their
  `material` response, defaulting to `Material::matte()`
- `Material::Unlit` opts a primitive out

The renderer should always resolve exactly one effective ambient term per
lighting domain:

- if there are no authored lighting resources, use identity lighting and skip
  diffuse/specular work so existing apps render unchanged
- if authored lighting resources exist and an enabled `AmbientLight` is present,
  use the topmost enabled `AmbientLight`
- if authored `LightSource` nodes exist but no enabled `AmbientLight` is present,
  use a modest default ambient light
- if an authored `AmbientLight` exists with low or zero intensity, honor it so
  authors can build intentionally dark scenes

A good starting shading model:

- effective ambient plus summed diffuse/specular contribution
- final color: `fill_color * lighting`, preserving original alpha handling

This satisfies "vectors interact by default when a light is present" while
preserving old apps that do not add lights, and it gives dark-scene authors a
first-class ambient override. `Material::Unlit` is separate from ambient light:
it means the material ignores scene lighting entirely.

## Coordinate Model

Use a simple scene-space convention now and keep it compatible with future
cameras:

- the current 2D canvas plane is `z = 0`
- `x` and `y` are logical Pax pixels in the same coordinate space used for
  vector rendering
- `z` is logical-pixel scene depth and only supports pixel/scalar depth, not `%`
- positive `z` points toward the viewer
- the default future camera can be considered orthographic, positioned on
  positive `z`, looking toward `z = 0`

For a point light, `x` and `y` should come from the `LightSource` node's
computed position, preferably the center of its bounds if width/height are set,
and `z` comes from the depth property. For a directional light, `direction` is
the emitted light direction in scene space.

This avoids inventing camera routing now while leaving room for a future
`Camera` primitive to transform scene coordinates before lighting.

## Scoping

`LightSource` should not be z-order sensitive. Template order is meaningful for
visible drawing, but a light is a scene resource rather than a visible element.

Recommended first scope:

- collect enabled lights and ambient-light overrides per logical canvas layer
  before rendering/replaying vector nodes
- every vector node in that logical canvas layer sees the same light list
- native layers and native controls are unaffected

Future scope extensions can add:

- `LightingScope` for subtree-local lighting
- named light groups
- per-primitive `lights=[...]` selection
- camera/light routing for 3D scenes

Layer-level collection keeps the initial implementation useful and avoids the
surprising behavior where a light only affects following siblings.

## Rendering Model

The first WGPU implementation should be single-pass forward lighting in the
existing vector fragment shader.

Recommended GPU data:

- a fixed-size light uniform/storage buffer, initially capped at a small number
  such as 8 or 16 lights per layer
- a material id or packed material fields per primitive
- world or layer-space fragment position passed from the vertex shader
- a default surface normal of `(0, 0, 1)`
- one effective ambient light per layer/domain

For point lights:

- compute vector from fragment position to light position
- attenuate by radius
- apply diffuse contribution from the surface normal
- optionally apply a cheap specular term based on roughness

For directional lights:

- use normalized direction
- no distance attenuation

For emissive materials:

- add the material's emissive contribution to the surface color
- do not feed emissive materials back into the scene's light list in v1

This model is intentionally modest. It gives dynamic lighting and surface
material response without requiring depth buffers, shadow maps, or mesh normals.

## Backend Fallbacks

WGPU should be the proving chassis.

Piet and other non-WGPU render contexts should render existing fill/stroke
output and ignore lights/materials, except `Material::Unlit`, which is equivalent
to current behavior anyway. A debug-only warning is acceptable when a scene uses
lights on a backend that cannot shade them, but runtime output should remain
usable.

This requires the backend-neutral `RenderContext` API to grow in a backward-safe
way. Prefer adding styled methods with default fallbacks:

```rust
fn fill_with_material_and_opacity(
    &mut self,
    layer: usize,
    path: kurbo::BezPath,
    fill: &Fill,
    material: &Material,
    opacity: f64,
) {
    self.fill_with_opacity(layer, path, fill, opacity);
}
```

The same pattern can be used for strokes. Existing callers keep working, while
vector primitives can opt into the richer call.

## Runtime And Compiler Boundary

This feature crosses the compiler/runtime boundary and must be handled as a
release-baked feature, not only as debug/runtime metadata.

Implementation areas to audit:

- `pax-runtime-api/src/drawing.rs` for `Material`, `LightShape`, `Vector3`, and
  `Depth`, plus helper constructors
- PaxValue coercion and `ToPaxValue` implementations for new public types
- `pax-std/src/drawing/*` primitive properties and dirty tracking
- a new `LightSource` primitive and runtime collection path
- `pax-runtime-api/src/rendering.rs` for material-aware render context methods
- `pax-runtime/src/engine/pax_gpu_render_context.rs` for lowering materials and
  lights into `pax-gpu`
- `pax-runtime/src/engine/piet_render_context.rs` for fallback behavior
- `pax-gpu` CPU/GPU structs, batching hashes, buffers, and WGSL shaders
- manifest serialization, program IR, binary baking, and generated Rust
  manifest paths

Adding a property to primitives also means style/settings behavior should work
normally:

```pax
@settings {
    .surface {
        material: Material::glossy(0.35)
    }
}
```

## Tradeoffs

### `material` property vs `Fill::Material`

Putting material inside `Fill` is attractive because fill already controls
surface color. It is also too narrow: strokes need light response, future 3D
primitives need material response, and `fill` should stay useful as the base
color. Keep material separate.

### Layer-scoped lights vs subtree-scoped lights

Subtree scoping is more expressive, but it requires more scene graph machinery
and can be surprising with component boundaries. Layer-scoped lights are easier
to explain and avoid order sensitivity. Add scopes later when there is a clear
composition need.

### Procedural material response vs bitmap texture maps

Procedural material response is much cheaper to ship first because it can live
in the existing vector shader. Bitmap texture maps are valuable, but they
require image resource lifetime, texture binding, batching, and UV semantics for
arbitrary paths. Keep bitmap texture maps for a later slice.

### Data-driven materials vs shader-backed custom materials

`Material::custom(MaterialParams)` gives userland an immediate extension point:
projects can define named helpers, component defaults, and theme values without
requiring new core variants. Shader-backed custom materials are more powerful,
but they need a shader ABI and backend compatibility story. Keep arbitrary
material shaders out of the first implementation slice.

### Default lighting response vs explicit opt-in per primitive

Default response is more magical but matches the requested authoring model.
The compatibility guard is that no explicit light means no visual change, and
`Material::Unlit` gives local control.

### Singleton ambient light vs implicit ambient only

An implicit-only ambient term would make intentionally dark scenes awkward or
impossible. A singleton `AmbientLight` keeps the default scene forgiving while
giving authors direct control. Multiple ambient lights should resolve by
render-tree precedence rather than summing, so the model remains easy to reason
about.

## Recommended First Implementation Slice

1. Add public types for `Material`, `MaterialParams`, `AmbientLight`,
   `LightShape`, `Vector3`, and `Depth`.
2. Add `material` properties to vector primitives, defaulting to matte.
3. Add `LightSource` with point and directional support, and `AmbientLight`
   with singleton resolution.
4. Add a runtime lighting-resource collection pass per logical canvas layer
   before vector replay/rendering.
5. Extend `RenderContext` with material-aware fill/stroke methods that default to
   the old behavior.
6. Implement WGPU shader support for ambient, point light, directional light,
   matte/glossy/metallic/emissive material coefficients.
7. Keep Piet fallback unlit.
8. Add a focused example, ideally `examples/src/light-materials`, with a moving
   point light and at least three material variants.

## Tests And Validation

Coverage should include:

- PaxValue coercion and `ToPaxValue` roundtrips for new public types
- template compile coverage for inline `material` and `@settings` material usage
- template compile coverage for `AmbientLight`, including multiple ambient
  lights resolving to the topmost one
- depth coercion coverage that accepts logical pixel depth and rejects `%`
- manifest, program IR, binary manifest, and Rust manifest roundtrips
- WGPU unit coverage for material/light buffer packing where practical
- a browser or native screenshot smoke for the example with lights enabled
- a fallback smoke showing the same example renders without lighting on a
  non-WGPU path
- retained-renderer invalidation when a light changes without retessellating
  unchanged vector geometry

## Settled Design Choices

- Bitmap texture maps should not ship in the same first slice:
  keep `texture` reserved for bitmap-backed texture data and prove light-reactive
  `material` first.
- Spot lights should wait until point and directional lights prove the scene
  resource path.
- Lights should not affect `Image` primitives in the first vector pass. Image
  lighting needs separate texture shader work and material semantics for bitmap
  content.
- Default `Material::matte()` should respond subtly once a `LightSource` exists.
- Lights should not be author-order dependent; collect them as scene resources
  before rendering.
- Ambient light should be a singleton scene resource with a default effective
  value and a userland override. If duplicates exist, the topmost enabled
  `AmbientLight` wins.
- First-pass material extensibility should be data-driven through
  `Material::custom(MaterialParams)`, not arbitrary shader attachment.
