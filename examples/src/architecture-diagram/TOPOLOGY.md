# PAX-1010 — Architecture diagram in Pax

Status: broad operating model aligned; functional detail proposed for Zack's review. No diagram implementation yet.

Initially surveyed on 2026-09-29 at repository revision
`0903488598cd37213db27c5569cdf77880587c2e` (2026-09-21, `post-release cleaning`).
The worktree is now verified at `3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916`,
including PAX-1004 and PAX-1008. Opacity, mask-source capture and the replay
responsibility boundaries have been inspected at this base. Other sections retain
the initial survey and need a source refresh before implementation. This is
source/ancestry validation, not a new cross-platform execution claim.

### Base audit: subtree composition

| Reference inspected | PAX-1004 isolated opacity | PAX-1008 retained source capture |
| --- | --- | --- |
| Initial survey, `090348859` | Absent | Absent |
| PAX-1000 / `zb/paxflix`, `740e08cdc` | Present | Absent |
| Current worktree and audited local `main`, `3c46cd13d` | Present | Present |

PAX-1004's implementation is `a9b7aa7b0`. PAX-1008 adds live component-backed mask
sources and extends the retained surface machinery beyond opacity, through
`58b727cc2`, `3d2e62806` and the companion files in `3c46cd13d`. The earlier
Paxflix worktree audit found uncommitted rendering work; that work was inspected
read-only and is not needed to establish this mechanism.

## Explanatory model: how the machine operates

This revision responds to Zack's direction: explain how Pax works, like an engine
schematic. The prior category inventory is superseded. Start with mechanisms,
the state they act on, and the connections that transmit changes. Derive band
boundaries from that causal model.

The main subject is a running program. A smaller construction circuit explains
how it gets assembled. Draw the running system as coupled feedback loops; the
mechanisms below are not a claim that every update executes one fixed,
linear sequence. A property read can evaluate a dirty chain before the next
frame, native measurements can return later, and some updates affect only one
output path.

## Communication contract: parts, connections and assemblies

Use a complex CAD schematic of an internal combustion engine as the reference
for the communication model. Each part has a concrete identity. Its placement,
connections and participation in a larger assembly explain how the engine works.

- **On-diagram labels name the parts:** short nouns or noun phrases such as
  Replay planner, Tile planner, Dirty DAG, Chassis, Textures and Materials.
  Preserve established terms such as Control flow resolution. A label need not
  match a Rust type verbatim, but it must map to an evidenced responsibility.
- **Arrows explain operation:** show direction, what passes between parts, and
  the action or dependency. Put causal sentences on connections or in explainers,
  rather than making each component name a miniature process description.
- **Nesting describes assemblies:** shared state, ownership and interfaces must
  justify a boundary. An editorial grouping of related activity is not itself a
  component. Such grouping may guide placement without receiving its own box.
- **Popovers explain the part in operation:** responsibility, inputs, outputs,
  owned state, invalidation/lifetime rules, implementation anchors and limits.
  The static schematic must still expose the principal causal paths.

The bullets beneath labels in this draft are evidence and explainer material;
they are not all intended as visible diagram copy. Stable IDs describe topology
and can remain unchanged when a display label improves.

The schematic also serves as an architectural diagnostic. If a part is hard to
name, inspect whether the problem is wording, an overbroad grouping, or unclear
code responsibility. Record specific ownership conflicts, duplicate state,
implicit contracts or coupled policies when evidence establishes them. A long
label, multiple source files or numerous connections alone does not establish
technical debt. Preserve an awkward boundary in the working model until it is
understood; a tidy invented component must not conceal it. Diagnostic notes can
live in popovers or a later review overlay without crowding the core schematic.

## Naming used by the diagram

- **Instances** is the canonical term. `InstanceNode` supplies reusable,
  type-aware behavior; `ExpandedNode` is a concrete expanded instance with state.
  Keep these implementation names visible at the instantiation boundary.
- **Scene graph** is the diagram's consistent name for the live expanded
  component tree. Existing docs also call this the **expanded tree**. It includes
  components, primitives and control-flow nodes, not only component instances.
  Template ownership, render ancestry and semantic received children remain
  distinct relationships within that model. [S01, S07, S21]
- **Dirty DAG** names the reactive property mechanism: dependency connections,
  invalidation, demanded evaluation and registered effects. “Scene graph” never
  denotes this dependency graph. Property state participates in the Dirty DAG
  while belonging to instances in the scene graph. [S08]
- **Control flow resolution** names the mechanism by which reactive conditions,
  repetitions, routes and slots determine participating instances. [S22]
- **Chassis**, **cartridge**, **native elements**, **render layers**, **surfaces**,
  **tiles**, **punch-through**, and **LightFrame** retain their existing meanings.
  Functional labels such as “tile planner” identify code-backed responsibilities;
  they do not claim a public Rust type of that exact name.

## Proposed recursive mechanism tree

The broad operating model is aligned with Zack. The following functional detail
is proposed for review; visual implementation has not begun. Bold labels name
parts or explicitly marked editorial assemblies. The supporting prose and edge
tables describe their operation.

- **Program assembly** (`construct`)
  - **Source analyzer** (`construct.analyze`)
    - Rust component/type discovery plus Pax grammar/PAXEL parsing produce the
      manifest: templates, expressions/dependencies, settings, handlers, routes,
      timelines and resource metadata. [S02]
    - Standard-library and application components enter this same mechanism.
  - **Cartridge generator** (`construct.connect`)
    - Component/type/property/handler descriptors link names to compiled Rust
      behavior; startup data supplies definitions to the runtime.
    - Target compilation combines generated code, application/primitive Rust,
      engine and chassis. The release representation caveat below still applies.
      [S03–S06]
  - **Definition-to-instance traverser** (`construct.instantiate`)
    - `DefinitionToInstanceTraverser` reads templates and descriptors to build
      `InstanceNode` behavior, property initialization plans and handlers.
    - Chassis startup requests the main component. [S06–S07]
  - **Instance expander** (`construct.expand`)
    - Create `ExpandedNode` instances with state, IDs, scope and parent/child
      relations; connect bindings and register their effects in the Dirty DAG.
    - Mount/unmount updates runtime lookup and lifecycle registrations. Control
      flow resolution reuses this mechanism as the scene graph changes. [S07–S10]
- **Runtime** (`operate`)
  - **Event dispatcher and handlers** (`operate.drive`)
    - Resolve input targets/bindings, supply typed event data and `NodeContext`,
      then invoke Rust handlers against their component state.
    - Handler writes, lifecycle callbacks, clocks/playheads and external results
      introduce changes into the same property mechanism. [S10–S11, S14]
  - **Dirty DAG** (`operate.react`)
    - **Properties and dependency links** (`operate.react.properties`):
      retain source/cached values and inbound/outbound dependency links.
    - **Dirty propagation** (`operate.react.dirty`): writes invalidate downstream
      computations and queue registered effects; configured cutoff computations
      can stop propagation when the accepted value has not changed.
    - **PAXEL evaluator and bindings** (`operate.react.evaluate`): reads demand
      current values from dirty dependencies; evaluate expression data in an
      instance's scope, or use eligible typed direct bindings.
    - **Effect queue** (`operate.react.effects`): the runtime drains registered
      consumers at defined lifecycle/clock/event boundaries. Effects materialize
      structural changes, native patches and render invalidation. The queue has
      a work budget; writes are not whole-scene transactions. [S08–S10]
  - **Control flow resolution** (`operate.structure`)
    - **Conditionals, repeats and routes** (`operate.structure.branches`):
      consume branch conditions, repeated values/identity and route state to
      select, create, reuse or remove instances.
    - **Slots** (`operate.structure.slots`): transport caller-provided
      content through component implementation structure to slot destinations;
      expose semantic received children to containers.
    - **Instance lifetime** (`operate.structure.lifetime`): keep
      active and exiting children distinct; retained exits can remain mounted
      until motion finishes. Structural resolution updates the scene graph and
      its participating Dirty DAG connections. [S07, S21–S22]
  - **Layout** (`operate.geometry`)
    - **Bounds/transform resolver** (`operate.geometry.transforms`): sizes,
      units, anchors, parent geometry and container placement produce reactive
      `TransformAndBounds` values.
    - **Measurement and container layout** (`operate.geometry.measurement`):
      native measurements and child layout hulls feed autosizing/arrangement;
      semantic active children and retained exits are separate inputs.
    - **Scroll and viewport state** (`operate.geometry.viewport`): content
      extent, scroll position and host viewport state determine the visible
      window, presentation coordinates and tile-planning inputs. [S07, S11]
    - **Scene geometry index** (`operate.geometry.index`): retain prepared
      geometry and spatial membership for visible canvas/native instances;
      answer region queries for replay and composition. Detached mask sources
      reuse prepared geometry without entering visible spatial indexes. [S37]
  - **Opacity scopes** (`operate.opacity`)
    - Preserve authored opacity ancestry for canvas subtree composition; retain
      multiplied effective opacity for native presentation and coverage estimates.
    - Feed scopes to the renderer so children compose before their boundary's
      opacity is applied. Native projection adapts effective opacity relative to
      the native parent. Native elements and separate Scroller surfaces still
      fade independently. [S35, S36]
  - **Primitive hooks** (`operate.project`)
    - Consume resolved geometry and typed properties; mark canvas nodes/layers
      dirty or emit native element patches as appropriate.
    - Expose geometry/paint through coverage-path and alpha-paint hooks for the
      compositor and mask mechanisms. Coverage may deliberately be conservative
      relative to animated visible pixels. [S07, S12–S14]
- **Presentation** (`present`)
  - **Compositor** (`present.plan`)
    - **Order and layer partitioner** (`present.plan.layers`): traverse the
      scene graph in presentation order, assign z-order/logical render layers,
      and track scroller-owned layers/islands.
    - **Coverage and geometric clip resolver** (`present.plan.coverage`):
      combine transformed primitive coverage with Frame/Mask paths, inherited
      clips, opacity estimates and scroll/viewport coordinates.
      - A geometric Mask takes coverage from its source subtree. The source is
        maintained as auxiliary scene structure and is not ordinary visible
        content. Frame supplies rectangular/rounded rectangular coverage.
      - GPU drawing receives clip paths; native projection receives applicable
        frame/mask geometry. Geometry masking is distinct from painted alpha.
    - **Native occlusion / punch-through** (`present.plan.occlusion`): find
      drawn coverage above native elements on the relevant logical layer;
      transform intersecting coverage into native-local masks. Hash masks to
      avoid resending unchanged patches. Coverage uses geometry and opacity
      estimates, not final-pixel sampling. [S12, S18, S24]
    - **Tile planner** (`present.plan.tiles`): consume logical layer/host
      ownership, content extent, viewport/scroll, DPR and surface policy;
      produce a `LayerCanvasPlan` of physical surface regions.
      - Choose one surface or a bounded tile window according to dimensions,
        area/surface limits and scrollability.
      - Assign reusable physical slot identities, origins, host signatures and
        replay priorities; cover visible tiles and eligible prewarm regions.
      - Chassis supplies policy/limits and realizes the plan. Tiling limits
        graphics surfaces; it does not virtualize component instances. [S25]
  - **Render scheduling** (`present.work`; editorial assembly, not a standalone component)
    - **Dirty render planner** (`present.work.dirty`): inspect dirty nodes,
      layers and removals; select filtered traversal or required broader replay;
      skip canvas rendering when no canvas work is pending.
    - **Replay planner** (`present.work.replay`): derive replay regions and
      priority batches from changed surface layouts; coordinate active and
      pending surface batches through `SurfaceReplayCoordinator`. The scene
      geometry index selects intersecting nodes in response to those regions.
    - **Surface culler** (`present.work.cull`): send a node's drawing only to
      intersecting surface tiles, with conservative fallback where needed.
      These are separate decisions from choosing which tiles exist. [S10, S26, S37]
  - **Light collector** (`present.lights`):
    - Read `LightSource` and `AmbientLight` instances, transform eligible lights
      into layer space and supply layer lighting data.
    - `LightFrame` ancestry determines per-node direct-light eligibility masks;
      outside lights may enter, inside lights do not escape. Ambient is layer-wide.
    - Currently at most eight direct lights per layer's set; native elements
      and bitmap Images are not lit vector materials. Piet paints unlit.
      This resolution runs in the runtime and feeds the renderer; it is not
      inferred from pixels by the shader. [S29]
  - **Renderer** (`present.draw`)
    - **RenderContext adapter** (`present.draw.adapter`): route
      primitive drawing and clip state to each logical layer's physical
      surface renderer; translate coordinates/lights to its surface space.
      Web selects wgpu or the Piet canvas alternative. [S13, S26]
    - **Retained drawing records** (`present.draw.retained`):
      `WgpuRenderer` retains vector/image records by node ID, z-order, bounds,
      transforms and clip state. Geometry/resource caches reuse tessellation
      and GPU data when compatible inputs remain unchanged.
      These records feed both direct drawing and retained subtree capture;
      drawing-record reuse and composed-pixel reuse are separate mechanisms.
      [S27, S35]
    - **Subtree rasterizer** (`present.draw.capture`):
      - Build ordered draw/group plans from retained records and opacity scopes.
      - Render into reusable offscreen targets, retain bounded composed pixels,
        and invalidate them when content, clips, transforms or lighting changes.
      - Share the surface/cache machinery between opacity groups and detached
        alpha sources (`CaptureKey::Opacity` / `CaptureKey::Alpha`).
      - **Opacity compositor** (`present.draw.capture.opacity`): compose
        children first, then sample the result with the boundary's opacity.
        Changing only that boundary opacity can reuse its composed pixels. [S35]
    - **Vector pipeline** (`present.draw.vector`):
      - Prepare paths/strokes, optional smoothing and draw ranges; tessellate
        geometry and upload/reuse vertex/index/primitive data.
      - **Paints and materials** (`present.draw.materials`):
        solid/gradient fill supplies base color/alpha. Material coefficients
        supply ambient/diffuse/specular/roughness/emissive response to lighting.
      - The vector shader evaluates draw alpha, alpha-mask coverage and lighting;
        the subtree compositor applies enclosing group-opacity boundaries. [S27–S28, S35]
    - **Textures** (`present.draw.textures`):
      - Loaded RGBA images become cached/versioned GPU texture resources sampled
        by the image pipeline; their positioning follows scene geometry.
      - Mask coverage textures, stencil attachments and optional multisample
        render targets serve separate renderer purposes.
      - A GPU texture is not automatically a user-authored material texture map;
        the current vector Material API has no texture-map pipeline. [S28, S30]
    - **Stencil renderer** (`present.draw.stencil`):
      synchronize nested clip stacks, cache tessellated clip geometry and use
      stencil tests to restrict vector/image draws. [S31]
    - **Alpha mask** (`present.draw.alpha`):
      - Mount the source as live auxiliary scene structure: expand components,
        run logical lifecycle/reactivity, and route supported vector render hooks
        exclusively into the source capture rather than visible presentation.
      - Capture through the shared retained renderer, preserving source-local
        group opacity, transforms, Frame clips and nested vector Masks.
      - Extract alpha into `R8Unorm` coverage, optionally feather in two passes,
        and multiply with the consumer's enclosing alpha mask. Source capture
        is independent of the consumer's ancestor clips and opacity.
      - Cache unchanged source pixels and retire resources with source lifetime;
        sample coverage to modulate canvas draws. The masked content itself is
        not implicitly flattened. Native leaves, images and source-side Scrollers
        are unsupported sources; hit testing is unchanged. GPU-only alpha path.
        [S35, S36]
    - **GPU backend** (`present.draw.submit`):
      shared device/queue/pipeline resources and per-surface backends encode
      ordered draw batches, resolve optional multisampling and present surfaces.
      The host composes them with native elements. [S33]
  - **Native projection** (`present.patch`)
    - **Native patch generator** (`present.patch.diff`): primitive updates
      produce create/update/delete/style/transform messages; compositor supplies
      mask/layer changes. Native state does not pass through the vector renderer.
    - **Native opacity adaptation**: convert computed opacity into host-relative
      element opacity before patch application. [S14, S23]
- **Chassis** (`chassis`)
  - **Presentation host** (`present.compose`)
    - **Surface manager** (`chassis.surfaces`): realize tile/layer plans as
      canvas or native GPU surfaces; create/reuse/resize/place them and report
      handles, sizes, scale and readiness to the renderer.
    - **Native element manager** (`chassis.native`): apply the message queue to
      DOM elements or Swift/native views; manage text, controls, frames,
      scroller islands and supported native effects such as Apple Liquid Glass.
    - **Native masks** (`chassis.masks`): web realizes mask geometry
      through its SVG effect machinery; Apple has cached raster mask images
      and native clip/mask layers. These masks are distinct from GPU alpha-mask
      textures and from rasterizing native content itself.
    - Web boundary: Rust/Wasm ↔ TypeScript/DOM. Apple boundary: shared Rust
      C ABI ↔ Swift/native views. Both host graphics surfaces and native elements
      using the common scene coordinates/order. [S15–S16, S34]
  - **Input adapter** (`feedback.input`)
    - Translate native input/control events into `NativeInterrupt`; route to
      the identified element or use scene geometry for hit testing/dispatch.
      The handler writes back into the Dirty DAG. [S10, S14]
  - **Measurement and viewport adapter** (`feedback.measure`)
    - Native text/control sizing, scrolling and viewport changes return inputs
      to layout/coverage without requiring an application handler. [S11, S14]
  - **Frame scheduler and clocks** (`feedback.clock`)
    - Drive tick/render, lifecycle callbacks, effect draining and animation
      time. Dirty tracking controls work within this schedule. [S10–S11, S15–S16]
  - **Resource loader** (`feedback.resource`)
    - Resolve image/font/media references; return decoded data/readiness to
      the renderer or native element. Application-side asynchronous results
      can write properties. Loading differs by resource and target. [S01, S14–S16]
- **Developer tools** (`develop`)
  - Inspect the live scene graph, read logs, capture composed output and drive
    input. Template reload changes permitted definitions/instances; opt-in logic
    reload rebuilds and activates supported target revisions. Both reload lanes
    are off in release. [S17]

## Subtree rasterization: validated reference and scope

At the current `3c46cd13d` reference, **subtree rasterization and retained
surfaces are a shared implemented mechanism**. PAX-1004 supplies group-opacity
composition; PAX-1008 reuses its ordered drawing, offscreen targets and retained
pixels for live vector-producing mask-source subtrees. This supersedes the
initial survey's conclusion that subtree capture should be future-only. [S35, S36]

Draw the common capture machinery once, with two consumers: **apply group
opacity** and **derive alpha-mask coverage**. Keep Apple native-mask rasterization
under the chassis: it realizes native occlusion masks and does not capture live
native content into these GPU subtree textures.

The implemented boundary is still per canvas/surface. Native controls/text and
separate Scroller surfaces are not captured as one mixed subtree image. Alpha
sources support vector leaves, not images, native leaves or source-side
Scrollers. The browser Piet renderer composes group opacity with reusable
temporary canvases, but does not retain composed pixels between dirty frames
or support alpha masking. General filters or arbitrary mixed-surface capture
are not established by these changes. No engine implementation is requested
or implied by this topology work.

## Connections that explain operation

The crucial coupling is **expanded instances ↔ property dependencies**:
instances provide scope/state and register consumers; changed dependencies
alter those instances and their presentation. Neither structure substitutes
for the other. Application properties belong to component state and also
participate in the Dirty DAG; do not draw them as copied state stores.

Each named edge must say what moves or what action it triggers. Use a separate
arrow for each direction of feedback. The table groups fan-out edges for reading;
the later diagram data should give each drawn arrow one source and one target.

| ID | Source → target | Relationship / payload | Why it matters |
| --- | --- | --- | --- |
| e01 | `construct.analyze` → `construct.connect` | Build: manifest definitions and type metadata | Connect declarative descriptions to concrete Rust behavior. |
| e02 | `construct.connect` → `construct.instantiate` | Startup data/code: templates and descriptor registry | Explain what crosses from build output into execution. |
| e03 | `construct.instantiate` → `construct.expand` | Construct: reusable node behavior and initialization plans | Distinguish a prototype from each live instance. |
| e04 | `construct.expand` → `operate.react` | Register: scoped properties, dependency links and effects | Explain how the reactive machine is wired. |
| e05 | `construct.expand` → `operate.structure` | Populate: concrete instances and parent/child ownership | Establish the scene graph that structural updates maintain. |
| e06 | `feedback.input` → `operate.drive` | Call: resolved Rust handler, event payload and context | Connect host input to application behavior. |
| e07 | `operate.drive` → `operate.react` | Write: changed source/clock/playhead values | Start downstream invalidation. |
| e08 | `operate.react` → `operate.structure` | Resolve: condition, list and route values; run affected consumers | Change the mounted topology. |
| e09 | `operate.structure` → `construct.expand` | Construct/retire: branch or repeated instances | Expansion is a runtime mechanism as well as a startup operation. |
| e10 | `operate.react` → `operate.geometry` | Resolve: sizes, transforms, measurement and viewport inputs | Explain how values become geometry. |
| e11 | `operate.geometry` → `operate.react` | Publish: computed bounds/transforms and measurements | Geometry participates in the Dirty DAG. This architectural return arrow does not imply a cycle in that DAG. |
| e12 | `operate.react` → `operate.project` | Resolve/run consumers: paint, text and native-control values | Some changes affect appearance without changing topology/layout. |
| e13 | `operate.structure`, `operate.geometry` → `operate.project` | Scene/geometry changes | Connect membership and placement to pending output work. |
| e14 | `operate.project` → `present.plan` | Invalidate: coverage/order/layer work | Replan shared composition when its inputs change. |
| e15 | `operate.project` → `present.draw` | Dirty canvas work and current primitive values | Update the drawn-content path. |
| e16 | `operate.project` → `present.patch` | Native element property patches | Native updates can occur independently of canvas drawing. |
| e17 | `present.plan` → `present.draw` | Render-layer assignments, clips and surface plan | Coordinate where/how drawn content is realized. |
| e18 | `present.plan` → `present.patch` | Native masks, layer/clip arrangement | Keep native and drawn coverage coherent. |
| e19 | `present.draw`, `present.patch` → `present.compose` | Surface content and updated native elements | Join the two visible output paths. |
| e20 | `present.compose` → `present.draw` | Host surface handles, backing size/scale and availability | The renderer draws into chassis-owned presentation resources. |
| e21 | `present.compose` → `feedback.input` | Host events/control notifications | Close the user-interaction loop. |
| e22 | `operate.geometry` → `feedback.input` | Target bounds/transforms | Input targeting depends on the scene that was resolved. |
| e23 | `present.patch` → `feedback.measure` → `operate.geometry` | Native measurement request/result, scroll/viewport observations | Show the physical host feeding layout back into the engine. |
| e24 | `feedback.clock` → `operate.drive`, `operate.react`, `present.draw` | Tick/render scheduling, clock advancement and effect-drain opportunities | Time/control drives work without a user event; this is not a strict phase-order assertion. |
| e25 | `operate.project` → `feedback.resource` → `present.draw`, `present.patch` | Resource reference/request and available content | Show the asynchronous resource side path. |
| e26 | `develop` → `construct.connect`, `construct.expand`, `feedback.input` | DEBUG: rebuild/reload or injected input | Development changes use explicit entry points. |
| e27 | `operate.structure`, `present.compose` → `develop` | DEBUG: inspection/capture results | Observe the live machine. |

## Detail connections across functional pieces

These edges expand the parent connections above. Preserve the separate jobs of
property invalidation, canvas dirtiness, tile selection and surface replay;
merging them into one “update” arrow would hide how work is controlled.

| ID | Source → target | What crosses / why |
| --- | --- | --- |
| d01 | `operate.react.dirty` → `operate.react.effects` | Registered consumers become pending; effect draining turns dependency invalidation into concrete work. |
| d02 | `operate.react.effects` → `operate.structure.branches`, `operate.geometry`, `operate.project` | Re-evaluate affected structure/layout/primitive consumers, according to their dependencies. |
| d03 | `operate.geometry.transforms`, `operate.opacity` → `present.plan.coverage` | Transformed geometry, inherited opacity and coverage estimates used for shared composition. |
| d04 | `present.plan.coverage` → `present.draw.stencil`, `present.plan.occlusion` | Geometric clip paths feed canvas clipping and the native occlusion calculation. |
| d05 | `present.plan.layers`, `operate.geometry.viewport` → `present.plan.tiles` | Layer/host ownership plus extents and visible window determine required surface regions. |
| d06 | `present.plan.tiles` → `chassis.surfaces` | `LayerCanvasPlan`: slot keys, origins, sizes, host signatures and replay priorities. |
| d07 | `chassis.surfaces` → `present.draw.adapter`, `present.work.replay` | Real surface handles/layout/readiness; changed or newly available surfaces require renderer synchronization and replay. |
| d08 | `present.work.dirty`, `present.work.replay`, `present.work.cull` → `present.draw.adapter` | Dirty traversal, scheduled surface batches and per-node surface scopes determine drawing and flushing work. The region-to-node selection connection is explicit in d21–d23. |
| d09 | `operate.project` → `present.draw.retained` | Primitive draw commands become/update retained node records through RenderContext. |
| d10 | `present.draw.retained` → `present.draw.vector`, `present.draw.textures` | Reused or updated vector resources and image draws feed their respective GPU pipelines. |
| d11 | `present.plan.layers`, `operate.geometry.transforms`, `operate.structure` → `present.lights` | Layer reachability, light geometry and LightFrame ancestry select per-layer lights and per-node eligibility. |
| d12 | `present.lights` → `present.draw.materials` | SceneLighting data and light masks combine with authored Material parameters in the vector shader. |
| d13 | `operate.opacity` → `present.draw.capture.opacity`, `present.patch.diff` | Authored opacity scopes control canvas group composition; effective opacity is adapted for native presentation. |
| d14 | `present.draw.alpha` → `present.draw.vector`, `present.draw.textures` | Cached raster mask texture sampled by canvas pipelines to multiply alpha; does not enter native projection. |
| d15 | `present.plan.occlusion` → `present.patch.diff` → `chassis.masks` | Native-local coverage/clip/opacity mask patches become SVG/native mask resources at the host. |
| d16 | `feedback.resource` → `present.draw.textures`, `chassis.native` | Decoded bitmap data or native font/media readiness reaches its actual consumer. |
| d17 | `present.draw.vector`, `present.draw.textures`, `present.draw.stencil`, `present.draw.alpha`, `present.draw.capture` → `present.draw.submit` | Prepared draws, captures and clip/coverage state are encoded into GPU work for the target surfaces. |
| d18 | `present.draw.submit`, `chassis.native` → `present.compose` | Rendered graphics surfaces and native views form the presented scene. |
| d19 | `present.draw.retained`, `operate.opacity` → `present.draw.capture` | Retained records, bounds, clips and opacity ancestry determine ordered subtree capture and content signatures. |
| d20 | `present.draw.alpha` → `present.draw.capture` → `present.draw.alpha` | Live source render hooks request a capture; composed source pixels return for alpha extraction, feathering and enclosing-mask multiplication. Draw as two directed edges. |
| d21 | `present.work.replay` → `operate.geometry.index` | Replay region request: find visible canvas instances intersecting the affected surface regions. |
| d22 | `operate.geometry.index` → `present.work.dirty` | Candidate node IDs are marked dirty; the engine reconciles targeted replay with other dirty work and removals before traversal. |
| d23 | `operate.geometry.index` → `present.work.cull` | Prepared coverage bounds determine which physical surfaces receive a node's draw. |

A useful rendering trace is **scroll changes → viewport state → tile planner →
chassis surface synchronization → visible-first replay → retained drawing →
GPU submission**. A mask trace is **mask-source property change → Dirty DAG →
live source update → retained subtree capture → alpha extraction/feather → shader
coverage sampling**. An opacity trace is **scope change → ordered group plan →
reuse or redraw composed pixels → apply boundary opacity**.
A light trace is **LightSource change → lighting collection/scope resolution →
material shading**, without turning LightFrame into an offscreen render layer.

## Boundary review: replay

The earlier label “Canvas render-work and replay planning” bundled distinct
parts. Inspection at `3c46cd13d` establishes these contracts:

| Part | Responsibility | Implementation anchor |
| --- | --- | --- |
| Replay planner | Surface regions, priority batches and pending replay lifetime | Layout synchronization in `PaxGpuRenderer`; priority helpers and `SurfaceReplayCoordinator` in `layer_surface.rs` |
| Scene geometry index | Prepared geometry and spatial node selection | `SceneGeometry`; `RuntimeContext::canvas_nodes_intersecting` / `request_canvas_replay` |
| Dirty render planner | Reconcile ordinary invalidation, replay requests and removals; select traversal or fallback | `PaxEngine::render` / `build_filtered_render_plan` |
| Surface culler | Route each draw to intersecting physical surfaces | `PaxGpuRenderer::begin_node_with_bounds` |

`SurfaceReplayCoordinator` explicitly documents that it owns surface indices
and replay regions, while the runtime scene index selects nodes. The old draft
incorrectly assigned both jobs to that coordinator. This is a confirmed modeling
correction, not an established code-debt finding. [S37]

For architectural review, trace who owns ordinary dirtiness versus replay
requests, and where conflicts force broader replay. `PaxEngine::render` contains
that reconciliation while the adapter owns surface batches and flush scope.
Determine whether the contracts are sufficient or policies are entangled before
proposing a new abstraction. The schematic should expose these connections;
renaming the entire assembly “Replay planner” would hide them.

## A trace the static diagram must explain

Use a small illustrative interaction to check the causal model:

1. A native button is activated; the chassis returns a control interrupt naming
   its element.
2. Dispatch resolves the bound Rust handler and its component state. The handler
   increments a count property.
3. That write dirties the derived label value; it does not call a draw routine.
4. When the native Text consumer is evaluated/drained, its expression obtains the
   new count and the primitive emits a text update patch.
5. The native bridge updates the displayed text. If sizing depends on measured
   text, a measurement can return and change geometry/coverage in a later update.
6. A separate visual property change can dirty canvas rendering; a conditional
   dependency can instead change which instances exist. The Dirty DAG explains
   these different consequences of the same kind of property write.

This is an explanatory trace, not a claim that every property write redraws both
paths. A second trace, clock → playhead → transform → coverage/drawing, should be
recoverable from the same schematic without inventing another animation engine.

## Provisional vertical-band placement and edge semantics

Keep **vertical bands arranged left to right**, but make their widths and
contents follow the mechanisms. These are placement descriptions, not final
visible captions; printed assembly names should follow the component-label
contract above. Proposed placement:

- **Build the program:** source analysis and compilation, as a compact inlet.
- **Instantiate and wire:** definition-to-instance traversal, instance
  expansion, scope/dependency registration. Expose both build/runtime and
  reusable/concrete-instance seams here.
- **React and resolve:** the dominant region, containing the coupled Dirty DAG and scene graph, handlers, control flow resolution and reactive layout.
- **Draw and project:** shared coverage/layer planning branches into renderer
  calls and native patches.
- **Host the result:** graphics surfaces/native views, actual presentation,
  input/measurement/clock/resource adapters.

Forward value/presentation flow crosses the middle; input, measurement and
clock/control feedback have distinct labeled return routes. All arrows attach
to named mechanisms. The runtime/host boundary is explicit, with native messages
and graphics-surface interfaces as separate crossings. These are conceptual
responsibility boundaries, not claims of five processes or one pass per band.

Provisional legend:

- **Dashed arrow:** build-time transformation; label the produced representation.
- **Solid arrow:** runtime value/data dependency; label the value or patch.
- **Open arrowhead:** invocation/scheduling/registration; label the action.
- **Dotted arrow marked DEBUG:** optional development connection.
- Group boxes show mechanisms; persistent scene-graph/Dirty-DAG state must be recognizable
  inside those mechanisms. Written status badges mark supporting/future claims.
- Show only connections needed to explain assembly, operation or feedback.
  API inventories, component catalogs and exhaustive format details stay in the
  source reference. Do not reduce this to a waterfall-shaped pipeline.

Interaction remains undecided. Static output must explain these circuits fully;
popovers may add implementation anchors, not supply missing causal connections.

## Current, supporting, and excluded claims

1. **Release path needs precise wording.** The repository guidance calls out
   binary baking. The active generator at this revision passes
   `ctx.is_release && !ctx.should_run_designtime` to `use_rust_manifest`, then
   calls `rust_manifest::to_rust_expression`. The template still initializes a
   `PaxManifest`, and the traverser consumes it. Searches found binary/IR codec
   use in tests, not the normal startup spine. Show “release program embedded
   as Rust constructors” with `ProgramIR`/codecs in supporting notes; do not claim
   that release mounts a binary IR directly. Recheck if the intended base changes.
2. **PAXEL is current expression evaluation plus typed-binding fast paths.**
   The historical compiled-expression-vtable picture does not describe this
   revision. Rust handlers/helpers are compiled; that is a different boundary.
3. **Native + drawn composition is current, effect parity is limited.**
   Geometric masks can cover native/drawn content. Painted-alpha masks are
   GPU-canvas content features; they do not imply native/Piet parity. Native
   accessibility and control behavior also have target-specific limits.
4. **Performance mechanisms are not universal guarantees.** Dirty tracking,
   retained rendering, culling and tiling exist. They do not imply automatic
   list virtualization, zero frame callbacks at rest, or recovery from every
   graphics-device startup failure.
5. **Historical/future material stays out of the shipping graph.** No JS/Python
   application-language support, universal cartridge envelope, independent
   compiled-expression hot linking, or old designer/LLM/ORM workflow is claimed.
   No broad embedding API, audio system or VR/game-controller support is inferred
   from the old artwork. Existing design-time code may be acknowledged in detail
   as the current dev loop; its mere presence does not establish a product flow.
6. **Intentionally summarized:** detailed parser grammar, every std component,
   shader equations/cache eviction policies, every native wire payload and every
   platform-service API. The functional renderer/cache hierarchy is included;
   implementation algorithms remain behind source anchors.

## Evidence index

These are implementation anchors, not a crate-dependency legend. Paths are
relative to this file so the reference survives moving between worktrees.
S01–S34 document the initial survey. S35–S36 pin the effects evidence to the
audited commit, now also checked out here. S37 records the current replay boundary
audit. Functional display names do not imply matching types or modules.

| Ref | Source | What it establishes |
| --- | --- | --- |
| S01 | [How Pax Runs](../../../pax-docs/book/src/how-pax-runs.md) | Builder-facing compiler/runtime/chassis, scene graph vs Dirty DAG, native/drawn surfaces, targets, debug/release. |
| S02 | [Static analysis](../../../pax-compiler/src/static_analysis.rs), `build_manifest_with_options` | Rust/Cargo discovery plus template parsing into `PaxManifest`. |
| S03 | [Compiler](../../../pax-compiler/src/lib.rs), `prepare_cartridge_sources`; [generation](../../../pax-compiler/src/cartridge_generation/mod.rs), `generate_cartridge_partial_rs` | Current release selection and generated program connections. |
| S04 | [Cartridge template](../../../pax-compiler/templates/cartridge_generation/cartridge.tera), `init_manifest`, `ComponentDescriptorRegistry`, `init_definition_to_instance_traverser` | Actual startup representation and registry/traverser contract. |
| S05 | [Rust manifest](../../../pax-manifest/src/rust_manifest.rs), `to_rust_expression`; [ProgramIR](../../../pax-manifest/src/program_ir.rs); [binary codec](../../../pax-manifest/src/binary.rs) | Release constructors; transitional IR and versioned serialization support. |
| S06 | [Runtime cartridge](../../../pax-runtime/src/cartridge.rs), `ComponentDescriptor`, `ErasedComponentDescriptor`, `DefinitionToInstanceTraverser`, `try_typed_property_binding` | Type-aware construction, handlers/scopes, typed bindings and expression fallback. |
| S07 | [Node behavior](../../../pax-runtime/src/rendering.rs), `InstanceNode`, `BaseInstance`; [expanded nodes](../../../pax-runtime/src/engine/expanded_node.rs), `ExpandedNode` | Reusable behavior vs concrete state, parent relations, active/mounted/exiting children, measurements/layout. |
| S08 | [Properties](../../../pax-runtime-api/src/properties/mod.rs); [dirty propagation](../../../pax-runtime-api/src/properties/graph_operations.rs); [effect scheduling](../../../pax-runtime-api/src/properties/properties_table.rs) | Reactive values, dependencies, dirty propagation and effect draining. |
| S09 | [PAXEL](../../../pax-language/src/interpreter/mod.rs), `PaxExpression`; [expression docs](../../../pax-docs/book/src/data-binding-expressions.md) | Parsed expression representation and evaluation. |
| S10 | [Engine](../../../pax-runtime/src/engine/mod.rs), `PaxEngine::tick`, `PaxEngine::render`; [runtime context](../../../pax-runtime/src/properties.rs), `RuntimeContext`; [events](../../../pax-docs/book/src/event-handling-rust.md) | Actual update/effect/lifecycle order, dirty rendering and handler-facing model. |
| S11 | [Layout](../../../pax-runtime/src/layout.rs); [motion docs](../../../pax-docs/book/src/animation-motion.md); [viewport docs](../../../pax-docs/book/src/scrolling-viewports.md) | Transforms, bounds, measurement, clocks/playheads, transitions and scrolling semantics. |
| S12 | [Occlusion](../../../pax-runtime/src/engine/occlusion.rs), `update_node_occlusion`; [surface layout](../../../pax-runtime/src/engine/layer_surface.rs); [tiling](../../../pax-runtime/src/engine/layer_tiling.rs) | Engine-owned order/native masks/layers and logical-to-physical surface mapping. |
| S13 | [GPU](../../../pax-gpu/src/render_context.rs), `WgpuRenderer`; [web render contexts](../../../pax-chassis-web/src/web_render_contexts.rs), `get_render_context` | Retained renderer, surface resources, WebGPU/Piet selection. |
| S14 | [Native messages](../../../pax-message/src/lib.rs), `NativeMessage`, `NativeInterrupt`; [Text](../../../pax-std/src/core/text.rs) | Native patch/feedback protocol; Text uses native text systems. |
| S15 | [Web chassis](../../../pax-chassis-web/src/lib.rs), `interrupt`, `tick`, `render`; [web interface](../../../pax-compiler/files/interfaces/web/src/index.ts), `renderLoop`, `processMessages`; [native elements](../../../pax-compiler/files/interfaces/web/src/classes/native-element-pool.ts) | Web host loop and Wasm/DOM realization. |
| S16 | [Apple bridge](../../../pax-chassis-common/src/core_graphics_c_bridge.rs), `pax_interrupt`, `pax_tick`, `pax_render`; [Swift native handling](../../../pax-compiler/files/swift/pax-swift-common/Sources/Rendering/NativeMessageHandling.swift) | Shared Apple C ABI and native presentation realization. |
| S17 | [Dev workflow](../../../pax-docs/book/src/developer-workflow.md); [reload policy](../../../pax-compiler/src/hot_reload.rs) | Target-specific reload lanes and dev inspection/capture/event tools. |
| S18 | [Compositing](../../../pax-docs/book/src/compositing-effects.md); [native controls/accessibility](../../../pax-docs/book/src/accessibility-native-controls.md) | Effect and target limitations. |
| S19 | [Historical notes](../../../pax-docs/book/src/architecture-runtime-cartridge.md) | Explicitly historical/proposed envelope and future-language directions. |
| S20 | [Animated logo](../pax-logo/src/animated_pax_logo.rs), `AnimatedPaxLogo::progress` | Current logo has a consumer-controlled progress property; `1.0` is the documented finished still pose. |

| S21 | [Runtime child ontology](../../../pax-docs/book/src/design/runtime-child-ontology.md); [Components](../../../pax-docs/book/src/components-composition.md) | Received/encapsulated/projected children and active/retained lifetime; canonical instance terminology. |
| S22 | [Conditional](../../../pax-runtime/src/conditional.rs); [Repeat](../../../pax-runtime/src/repeat.rs); [Router](../../../pax-runtime/src/router.rs); [Slot](../../../pax-runtime/src/slot.rs) | Concrete control flow resolution implementations. |
| S23 | [ExpandedNode](../../../pax-runtime/src/engine/expanded_node.rs), computed_opacity binding; [primitive helpers](../../../pax-std/src/common.rs), native_surface_opacity | Parent × local opacity in the Dirty DAG; adaptation relative to the native parent. |
| S24 | [Mask](../../../pax-std/src/core/mask.rs), resolve_subtree_coverage_path, collect_alpha_paints, handle_pre_render; [Frame](../../../pax-std/src/core/frame.rs) | Geometric/painted-alpha modes, source subtree handling, coverage hooks and supported rasterization boundary. |
| S25 | [Tile planning](../../../pax-runtime/src/engine/layer_tiling.rs), ScrollerTilingPolicy, scroller_canvas_plan_with_policy; [Apple plan bridge](../../../pax-chassis-common/src/core_graphics_c_bridge.rs), pax_get_layer_canvas_plan | Single/tiled surface choice, bounded windows, slot reuse, priorities and chassis-supplied policy. |
| S26 | [Surface replay](../../../pax-runtime/src/engine/layer_surface.rs), SurfaceReplayCoordinator; [GPU adapter](../../../pax-runtime/src/engine/pax_gpu_render_context.rs), PaxGpuRenderer, LayerRenderer | Physical tile renderer sets, spatial replay/culling, visible-first scheduling and surface synchronization. |
| S27 | [Retained renderer](../../../pax-gpu/src/render_context.rs), WgpuRenderer, RetainedNode, VectorGeometryCache, VectorResourceCache | Retained drawing records, transforms/clips and shared vector caches, distinct from raster subtree images. |
| S28 | [Vector shader](../../../pax-gpu/src/render_backend/geometry.wgsl), fs_main, apply_lighting; [drawing API](../../../pax-runtime-api/src/drawing.rs), Fill, Material, SceneLighting | Paint, opacity, alpha sampling and material/light shading; eight-light bound. |
| S29 | [Light resources](../../../pax-std/src/drawing/lighting.rs); [light collection](../../../pax-runtime/src/properties.rs), collect_scene_lighting_for_layer; [lighting docs](../../../pax-docs/book/src/drawing-styling.md#lighting-and-materials) | LightFrame ancestry, per-layer light selection, per-node masks, ambient semantics and backend limits. |
| S30 | [Texture renderer](../../../pax-gpu/src/render_backend/texture.rs); [image shader](../../../pax-gpu/src/render_backend/textures.wgsl); [GPU resource helpers](../../../pax-gpu/src/render_backend/gpu_resources.rs) | Bitmap texture sampling, texture-resource reuse, and separate multisample target resources. |
| S31 | [Stencil renderer](../../../pax-gpu/src/render_backend/stencil.rs), StencilRenderer | Nested geometric clipping and cached stencil geometry. |
| S32 | [Alpha-mask rasterizer](../../../pax-gpu/src/render_backend/alpha_mask.rs), AlphaMasks; [mask shader](../../../pax-gpu/src/render_backend/alpha_mask.wgsl); [feather shader](../../../pax-gpu/src/render_backend/alpha_blur.wgsl) | R8 coverage textures, cached source rasterization, two-pass feather and parent-mask multiplication. |
| S33 | [GPU backend](../../../pax-gpu/src/render_backend/mod.rs), GpuContext, RenderBackend | Shared device/queue/pipelines, per-surface state, draw submission and optional multisample resolve. |
| S34 | [Web layer manager](../../../pax-compiler/files/interfaces/web/src/classes/render-layer-context.ts), RenderLayerManager, SvgEffectManager; [Apple rendering](../../../pax-compiler/files/swift/pax-swift-common/Sources/Rendering/Rendering.swift), RasterizedNativeMaskImageCache | Chassis surface/native hosting; web SVG masks and Apple raster/native mask realization. |
| S35 | At `3c46cd13d`: [shared capture backend](https://github.com/paxdotdev/pax/blob/3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916/pax-gpu/src/render_backend/capture.rs), `CaptureKey`, `RetainedSurfaces`; [opacity planner](https://github.com/paxdotdev/pax/blob/3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916/pax-gpu/src/render_context/opacity.rs); [source capture](https://github.com/paxdotdev/pax/blob/3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916/pax-gpu/src/render_context/source_capture.rs) | Shared offscreen allocation/cache/lifetime, ordered group composition, source capture, alpha extraction and content invalidation. Inspected from local Git, not fetched from GitHub. |
| S36 | At `3c46cd13d`: [Mask](https://github.com/paxdotdev/pax/blob/3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916/pax-std/src/core/mask.rs), `begin_alpha_source` / `recurse_render_alpha_source`; [compositing article](https://github.com/paxdotdev/pax/blob/3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916/pax-docs/book/src/compositing-effects.md); [capture regressions](https://github.com/paxdotdev/pax/blob/3c46cd13dfdc7fb4ac5992aa6c44672e4fc1f916/pax-gpu/src/render_backend/retained_clip_tests.rs) | Live component sources, supported leaves and backend/surface limits; regression scenarios for opacity, source clips, reveal, feather, nesting, reuse and retirement. Tests inspected, not rerun in this audit. |
| S37 | At `3c46cd13d`: [replay coordinator](../../../pax-runtime/src/engine/layer_surface.rs), `SurfaceReplayCoordinator`; [scene index](../../../pax-runtime/src/scene_geometry.rs), `SceneGeometry`; [runtime requests](../../../pax-runtime/src/properties.rs), `canvas_nodes_intersecting`, `request_canvas_replay`; [render planning](../../../pax-runtime/src/engine/mod.rs), `PaxEngine::render`, `build_filtered_render_plan`; [surface adapter](../../../pax-runtime/src/engine/pax_gpu_render_context.rs), layout synchronization, `begin_node_with_bounds` | Separate owners for surface replay scheduling, node selection, dirty traversal and draw-to-surface culling. Coordinator comments explicitly exclude scene-node selection. |

Historical prior art remains unchanged at
`/Users/zack/Desktop/pax-architecture-2023-12-06.svg`.
SHA-256: `6204a2dc4c0e506489e82d9812e6da22a80e6d8e556726357135f37c187739a8`.
The full 10928.6 × 1419.8 bounds were rendered locally for inspection and the
text was also read from the SVG. Preserve its boundary/flow ideas, not its old
product or implementation claims. This local path is a reference, not a runtime
dependency of the future example.

## Approval checkpoint and subsequent deliverables

The broad operating model is aligned with Zack. Review this next layer of
functional hierarchy and the explicit subtree-rasterization boundary before
visual implementation. Band names and interaction choice remain provisional.

After approval: textual wireframe → spatial/type/edge specification → real Pax
example → screenshot/content iteration. Keep stable node/edge IDs and content
data separate from layout; derive labels from that data and keep this explanatory
reference aligned, without building a graph-editor framework. Start with a
single small Rust content table and Pax components unless implementation reveals
a simpler arrangement.

Proposed docs location is a concise reference/embed in **How Pax Runs**, with a
maintainer-facing link near the internal reference entry. Review that placement
before editing the public learning path. The example will carry run/export/update
guidance and reproducible still settings, including the logo pose. Public docs,
generated API pages, engine behavior, and published snapshots are unchanged in
this phase. No build is needed to validate this words-only checkpoint.

Export validation must capture the composed scene, including native Text and
drawn paths, rather than only a GPU canvas. Font/resource readiness and the fixed
viewport must be part of the later reproducibility check.
