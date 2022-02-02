# TODO


## Milestone: proof of concept engine

[x] Rendering 
[x] Components 
[x] Logging
[x] Stroke, color, fill
[x] Sizing
    [x] Browser resize support
    [x] None-sizing
    [x] Transform.align
    [x] Transform.origin
[x] Expression engine
    [x] variables, declaration & storage
    [x] node IDs
    [x] summonables
    [x] built-in vars like frame count
    [x] MVP rust closures + manifest of deps
[x] Spreads (née Stacks)
    [x] Decide `primitive` vs. userland `components`
    `components`
    [x] Internal template mechanism for components
    [x] Make `root` into a component definition
    [x] Control-flow `placeholder` (`placeholder`) for inputs/children
    [x] Ensure path forward to userland `placeholders`
    [x] Clipping & Frames
    [x] Control-flow `repeat` for cells & dividers inside template
    [x] Gutter
[x] Split out userland code
    [x] Add a third project to workspace, the sample project
    [x] (Further work to be done via compiler task)
[x] Timelines, transitions
[x] Refactors
    [x] Bundle Transform into "sugary transform," incl. origin & align; consider a separate transform_matrix property
    [x] Is there a way to better-DRY the shared logic across render-nodes?
e.g. check out the `get_size` methods for Frame and Spread
    [x] Maybe related to above:  can we DRY the default properties for a render node?
Perhaps a macro is the answer?
    Same with `scale`
    [x] Can we do something better than `(Box<......>, Box<.......>)` for `Size`?
    [x] Rename various properties, e.g. bounding_dimens => bounds
    [x] Take a pass on references/ownership in render_render_tree — perhaps &Affine should transfer ownership instead, for example
    [x] Better ergonomics for `wrap_render_node_ptr_into_list`
    [x] Evaluate whether to refactor the `unsafe` + PolymorphicType/PolymorphicData approach in expressions + scope data storage



## Milestone: "hello world" from .pax

[x] Compile base cartridge
    [x] Refactor PropertiesCoproduct to its own module
    [x] Sanity check "patch" ability for "blanks" (Properties, Expressions)
    [x] Demo app chassis running example project (`./serve.sh`)
        [x] Add stub macro for `pax`, derives
[x] baseline primitive(s) for hello world
    [x] import/package management
    [x] RIL -> PAX compatibility, or rewrite primitives
[ ] `pax-compiler`
    [x] architecture, seq. diagram
    [ ] two-stage compilation process
        [x] thread/process/IPC chassis
        [x] parser cargo feature
        [x] bin-running harness to execute parser (see https://stackoverflow.com/questions/62180215/renaming-main-rs-and-using-with-cargo)
        [ ] TCP message passing
            [x] de/serialization for manifest
                [x] maybe normalize SelectorLiteralBlockDefinitions, if Serde can't elegantly de/serialize it
                [x] or de-normalize TemplateNodeDefinition!
            [ ] coordination of TCP components from compiler main thread
        [x] parse and load .pax files
            [x] load file via macro
            [x] generate the parser bin logic via macro
                [x] manual
            [x] port minimal set of std entities (Rectangle, Group) to support manifest-gen 
            [x] traverse manifest of Component defs: parse .pax files, store in mem
            [x] (start with templates only)
    [x] thread for wrapping `cargo build`
    [x] sketch out .pax folder design
    [ ] graceful shutdown for threaded chassis (at least: ctrl+c and error handling)
    
[ ] compiler codegen
    [ ] codegen Cargo.toml + solution for patching
        [x] manual
        [ ] automated + file generation
    [ ] parser bin logic finish-line
        [ ] macro
    [ ] codegen PropertiesCoproduct
        [x] manual
        [ ] macro
        [ ] hook into compiler lifecycle
    [ ] serialize to RIL
        [ ] hand-write RIL first!
        [ ] normalize manifest, or efficient JIT traversal
            [ ] stack Settings fragments (settings-selector-blocks and inline properties on top of defaults)
            [ ] might need to codegen traverser!
                [ ] or might be able to be "dumb" (purely static) with codegen, relying on rustc to complain
        [ ] probably need to store property k/v/types in Manifest (and maybe fully qualified type paths)
        [ ] codegen RIL into source via `#[pax]` macro, to enable vanilla run-via-cargo (well, pax-compiler, but maybe there's still a path to bare cargo!)
        [ ] untangle dependencies between core, runtime entities (e.g. Transform, RenderTreeContext, RenderNodePtrList), and cartridge
    [ ] work as needed in Engine to accept external cartridge (previously where Component was patched into Engine)

[ ] render Hello World
    [ ] Manage mounting of Engine and e2e 

## Milestone: clickable square

[ ] Designtime
    [ ] codegen DefinitionToInstance traverser
        [ ] codegen in `#[pax]`: From<SettingsLiteralBlockDefinition>
            [ ] manual
            [ ] macro
    [ ] instantiator/traverser logic (codegen or library-coded)
    [ ] duplex websocket connection + handlers
        [ ] Write ORM (and maybe caching) methods for `Definitions`
        [ ] Attach CRUD API endpoints to `Definition` ORM methods via `designtime` server
    [ ] figure out recompilation loop or hot-reloading of Properties and Expressions
        [ ] incl. state transfer
[ ] Action API
    [ ] state management (.get/.set/etc.)
    [ ] hooks into dirty-update system
    [ ] Instantiation, reference management, enum ID + addressing for method definitions &
        invocations
    [ ] tween/dynamic timeline API
[ ] Event capture and transmission
    [ ] Map inputs through chassis, native events (mouse, touch)
        [ ] PoC with Web
    [ ] Message queue in runtime
    [ ] Ray-casting? probably
    [ ] Message bubbling/capture or similar solution

> What's our expression language MVP?
> - `==`, `&&`, and `||`
> - Parenthetical grouping `(.*)`
> - Literals for strings, bools, ints, floats
> - Nested object references + injected context

[ ] Expressions
    [ ] Transpile expressions to Rust (or choose another compilation strategy)
    [ ] Write ExpressionTable harness, incl. mechanisms for:
        [ ] vtable storage & lookup
        [ ] Dependency tracking & dirty-watching
        [ ] Return value passing & caching
    [ ] Sketch out design for parallelized expression computation (e.g. in WebWorkers)
    [ ] Patch ExpressionTable into cartridge à la PropertyCoproduct



## Backlog

[ ] Margin & padding?
    [ ] Decide whether to support, e.g. is there a simpler alternative w/ existing pieces?
    [ ] Decide whether to support ONE or BOTH
[ ] Ellipse
[ ] Path
[ ] Frames: overflow scrolling
[ ] Should (can?) `align` be something like (Size::Percent, Size::Percent) instead of a less explicit (f64, f64)?
[ ] PoC on macOS, iOS, Android, Windows
[ ] Image primitive
    [ ] Hook into `piet`s image rendering
    [ ] Asset management
[ ] Gradients
    [ ] Multiple (stacked, polymorphic) fills
[ ] Production compilation
    [  ] Generation of RIL, feature-gating `designtime`
[ ] Packaging & imports
    [ ] Ensure that 3rd party components can be loaded via vanilla import mechanism
[ ] Mixed mode, Web
    [x] Rust -> JS data bridge
    [x] DOM pooling & recycling mechanism
    [ ] Text primitives + basic styling
    [ ] Native-layer clipping (accumulate clipping path for elements above DOM elements, communicate as Path to web layer for foreignObject + SVG clipping)
    [ ] Form controls
    [ ] ButtonNative (vs. ButtonGroup/ButtonContainer/ButtonFrame?) (or vs. a click event on any ol element)
    [ ] Text input
    [ ] Dropdown
[ ] JavaScript runtime
    [ ] First-class TypeScript support
    [ ] API design
        [ ] code-behind & decorator syntax
    [ ] Bindings to `runtime` API, plus IPC mechanism for triggering
[ ] Language server, syntax highlighting, IDE errors (VSCode, JetBrains)
[ ] Transform.shear
[ ] Audio/video components
    [ ] "headless" components
[ ] Expression pre-compiler
    [ ] Enforce uniqueness and valid node/var naming, e.g. for `my_node.var.name`
    [ ] Parser for custom expression lang
[ ] Debugging chassis
[ ] Perf-optimize Rectangle (assuming BezPath is inefficient)


```
Creative development environment
for makers of
graphical user interfaces
```



## zb lab journal

### untangling Definitions, Values, and Patches
2022/01/27

We need "patches" to support stacking of sparse
definitions, e.g.  {foo: "bar"} + {baz: "qux"} => {foo: "bar", baz: "qux"}

How do these patches come to bear between .pax and the runtime?
Where do Expressions (and the exptable) fit in?
`Patch`ing also requires Default fallbacks — where do _those_ slot in?

Perhaps Patches can be combined (a la overriding `+`), and can
be distilled into values.  Perhaps a component, e.g., `Root` supports
`apply_patch(&mut self, patch: RootPatch)`

Where does this logic live (which feature/lifecycle stage?)
Seems like `designtime` is the key.  Not needed for runtime
(action value setting can be a different concern with a lighter footprint)

Two flavors of instantiating Definitions:
 - transpiling into RIL (hard-coded N/PIT; once & done)
 - dynamic traversal into N/PIT for designtime
    - accept changes in definitions
      - special handling for Expressions/exptable
    - separate definitions from values (e.g. maybe `patch`es for each?)
       Note: e.g. `Root` vs. `RootPatch` already does this; Root is a "value" container (can rep. patch)
       Perhaps start with use-case:  we need to `Patch`-stack in order to assemble sparse property
       DEFINITIONS from pax, e.g.
    - Do we really need to worry about values at all?  Those are already handled well by the runtime.
       when a user changes

One wrinkle re: managing the patch-stacking logic in the designtime:
this would require dynamic evaluation in order to generate RIL.
Which shouldn't strictly be necessary.  RIL SHOULD be
generatable directly from a PaxManifest. (Is this true?
is this some sort of purity-for-purity's-sake situation?)

Perhaps it isn't so bad for the compiler to load the cartridge + designtime in order
to traverse the manifest => 

1. normalize PaxManifest (into a single traversable tree with inline property values as frames of the bare structs, ready for RIL
   1. This requires collapsing stacked values, probably in a way that's distinct from the way the designtime does it (designtime deals in stacks of patches, vs RIL transpiler dealing with a normalized tree)
   2. This also requires transpiling + "blank"ing in an Expression table
   3. This also requires knowing property schema in the Manifest!  Thus far this hasn't been a thing.
      1. Need a way to universally qualify types, a la module_path!() [this might be tricky!]
      2. Alternatively, could do another bin-conversion trick a la parser, and rely on macros to make sense of property types on-the-fly
         1. (Note: it will be important to know property schema eventually, not least to expose to design tool)

Conclusion: further dynamic evaluation is unideal; requires more compilation loops

### expressions
2022-01-28

transpile @ {x + 5} into |&properties| -> { &properties.x + 5 } along with glue/injection logic,
dirty-handling logic, and vtable-like logic for storing & retrieving function (references)

handle type resolution as much as necessary; let rustc complain about type mismatches/etc. 

Expressions need to be dealt with in a few ways:
- parsed from 1. a template attribute, 2. a settings value (recursively within settings tree)
- looked up by ID, in RIL and in the DefinitionToInstance traverser
- hooked into with dirty-watching system, along with dependency DAG (choose which expressions to re-eval, when)
- future: hot-updated, via server message + recompiled binary/state-pass, when running the cartridge (compiler run mode, design tool)


- originally was thinking of a central vtable a la propertiescoproduct
  - this would make hot reloading easier (just replace the expressions sub-cartridge) — but it makes referencing difficult
    - maybe referencing isn't difficult with implicit return types!!
- am now thinking that each file generates its own expanded macros (via #[pax])
  - during compiler codegen phase, expressions are transpiled, surrounded by glue, and tagged/ID'd
  - for RIL, weave a code-genned pointer between a property instance and function (known from manifest)
    - e.g. `background_color: PropertyExpression(&expression_table.ae25534efc)`
  - for dynamic resolution, e.g. in designtime -
    - First of all, what does dynamic resolution mean?
      - It starts with a compiled cartridge + `designtime` feature, (already including RIL binary?) — which must already have an expression table compiled! (or capable of having it async loaded, FFI/etc.)
      - Then, a user changes the value of a property from a literal to an expression, or changes the definition of a current expression
      - Now, we must: 1. transpile the expression values to RIL, 
- if it yet becomes the case that we need to deal with explicitly stated return types on the expression lambdas:
  - expose fully qualified types in `pax_exports`, then track fully qualified import paths (e.g. pax_std::exports::foo) in manifest
  - expose naked functions at the root of codegenned `expression_table`, like  
```
pub fn x(input: i64) -> i64 {
    input + 6
}
```
- (cont.)
  - where the return type of the codegenned function is fully qualified via the nested-mod re-exports trick
  - (and where primitive types are enumerated & special-cased)
  - This likely also requires registering `DeeperStruct`s, e.g. via `#[pax struct]`


### helpers, injectables
2022-01-28

e.g. Engine.frames_elapsed, or a userland helper function hue_shift()

API thought: can continue the `#[pax ...]` convention, decorating a function declaration like so:
```
#[pax helper]
pub fn hue_shift() {
    //gather an entropic hue value from the world
}
```

### timelines, syntax in .pax
2022-01-29
implemented in core, but not yet sketched in the pax language / parser, is timeline support.

Timeline specs for a given property<T> are: 1. starting value: T, 2. Vec<{curve_in, ending_frame, ending_value: T}>

A jot of how the API may look in a paxy way:
```
background_color: Timeline {
    starting_value: Color::hsla()
    segments: [
        {}
    ],
    tween_strategy: ColorTweenStrategy::Hue,
}
```


### on RIL generation, consumption
2022-01-31

How is the generated RIL consumed?
 - Chassis reaches into cartridge and calls a method to get root component instance(s)
 - Chassis then passes instance to the Engine to start rendering


Is this the right time to rethink instance management? Could fix the
mess of Rc<RefCell<>> going on in core.

Broadly, instances could be stored centrally in some Vec<dyn RenderNode> (or hash)
This instance pool allows for safe passing of &mut throughout core

Finally, RIL can instantiate via this instance pool 

...

Update, after spending a day on revamping instance management (progress on branch zack/valiant-rc-refactor-jan-2022), it's not currently tenable.
The lifetime tangle through core/runtime/stackframes/properties/RenderNode is beyond my current skill to fix.
Rc<RefCell<>> is not a bottleneck problem, aside from aesthetics and a minor runtime penalty (noting that
an equivalent lifetime-based solution would still effectively reinvent RefCell via the InstancePool)


SO: for RIL, proceed with on-the-fly Rc<RefCell<>> instantiations, just like PoC renderer code


### on circular dependencies, PropertiesCoproduct
2022-02-02

Despite moderately careful planning, we've ended up with a circular dependency between:
 - userland cartridge
 - core
 - properties-coproduct
 - userland cartridge (to wrap properties types)

Ideas for mitigation:
 - codegen RootProperties _into_ pax-properties-coproduct instead of inline to the source file
   - Main drawback: this requires "ghosting" every type, annotating each subtype (or globally unique type name manifest)
   - Note also: the codegenned logic will depend on `runtime`, via `timeline` (at least) (`timeline` -> `Property` -> `RenderTreeContext` -> `Runtime` -> `Stack` -> `StackFrame` -> `PropertiesCoproduct`)
     - Could separate Timeline from Property, maybe — or revisit `compute_in_place` to see if something other than `RenderTreeContext` could be injected
 - Slight variation: generate a separate cartridge/blank project 
 - Split common dependencies from `core` & `cartridge` into `runtime` — it can be a leaf-node dependency of both, allowing common data structures/exchange
   - Some """driver""" logic may even need to jump over to cartridge, e.g. `Scope` and its dependants