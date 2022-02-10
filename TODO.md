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
 - Slight variation: generate a separate cartridge/blank project `pax-cartridge`
 - Split common dependencies from `core` & `cartridge` into `runtime` — it can be a leaf-node dependency of both, allowing common data structures/exchange
   - Some """driver""" logic may even need to jump over to cartridge, e.g. `Scope` and its dependants
 - If all of `runtime.rs` logic is moved into PropertiesCoproduct — this might be fixed!
   - (plus Timeline, plus Property... plus RenderTreeContext?)
   - (plus ComponentInstance...)

As a broader strategy, could step back and look at the architecture of Engine,
more carefully drawing boundaries between Runtime, Property, Timeline, Core, and PropertiesCoproduct

### on properties
2022-02-06

In userland, e.g. an Action, properties:
 - can read properties programmatically with .get()
 - can set properties programmatically with .set()
    - not v0, but would be nice to have a path to someday setting values other than literal values, e.g.
      to create Expressions and Timelines at runtime
In engine, properties:
 - need to `compute_in_place` (whether literal, timeline, or expression)
 - need to represent whether a property is literal, timeline, or expression
 - need to `read` the current (cached) value
 - need to instantiate with a starting value
 - need to support runtime mutations via userland `.set()`, plus accurate up-to-the-frame value retrieval via `.get()`
 - have dependencies on engine, via InjectionContext and `compute_in_place` parameters

Further: PropertiesCoproduct frames need to encapsulate Properties *instances* (the engine variant, if there are two variants)
        which suggests a dependency on Engine wherever {}Properties are generated

Are these Properties data structures the same or different?  The rubber meets the road in RIL —
are the macro-generated RootProperties/RectangleProperties and PropertiesCoproduct entities the same?

Note it's easier to generate RectangleProperties alongside Rectangle in cartridge-userland, but 
with an engine dependency they seem to need to exist in fully code-genned cartridge-runtime...

One possible tool to share the core Property definition is to split Property, PropertyLiteral,
PropertyExpression, and PropertyTimeline into pax_runtime_api (importable by both Engine & userland) — 
then to write traits/impls that allow engine to `compute_in_place` and `read`

*^ proceeding with this strategy*

#### Re: Transform as a Property —
 - Transform has a special method in the rendering lifecycle, `compute_matrix_in_place`.
 - This is called in a subtly different context than `compute_in_place` for computableproperties — namely, it's called with the context of calculated hierarchical bounds (container size, etc.)
 - Further, every RenderNode is expected to manage Transform, via get_transform
 - Ergonomically, it would be ideal to treat any of the sub-properties of Transform as a plain Property,
 - e.g. so that rotation.z can be an expression while scale[x,y] are literal values
 - (Further, there seems to be no reason this can't continue recursively, with the `operations` API)

Question: given the above, should `transform` be expected as a member of `{}Properties`, or should we hoist it to be a top-level property of `{}Instance`?
 - In the world where it's hoisted to be an `Instance` property:
   - We can still `compute_in_place` by special-casing `.transform` whenever we handle `compute_in_place` for properties — 
   - that is, `.properties.compute_in_place()` and `.transform.compute_in_place()`.  To spell out further: `transform` is treated as a `ComputableProperty` in the engine
   - in every way except for being part of the PropertiesCoproduct.
 - This also suggests an opaque-to-user special-handling of Transform during compilation.  Namely,
   - the user addresses Transform just as they would any Property, e.g. through Settings and through
   - runtime .get/.set APIs.  However — in RIL and engine (.transform.set), Transform is special cased


### Transform API
2022-02-07

What's a reasonable & ergonomic API for transforms, which:
 - is terse & expressive in PAXEL
 - is terse & expressive in the action API
 - is thorough and enables specifying arbitrary transform order 
 - 
Some ideas —

#### Array for operation sequence, enums for declaring operations
```
<Rectangle id="meow">

@settings {
    #meow {
        transforms: [Transform::Rotate(Angle::Degrees(27.8))]
    }
}
```
Pros: highly expressive
Cons: verbose (esp. enums)

#### More CSS-like?
```
<Rectangle id="meow">

@settings {
    #meow {
        transform: rotate(32) scale(1.2)
    }
}
```
pros: expressive & terse
cons: new DSL


#### Recursive?
```
<Rectangle id="meow">

@settings {
    #meow {
        transform: {
            operations: [
                Transform {
                    scale: [1.2, 2.2]
                },
                Transform {
                    translate: [400.0px, 300.0px]
                }
            ]
        }
    }
}
```
Pros: expressive and aligned with RIL
Cons: verbose, esp. nesting `operations` and reincantation of `Transform`


#### fusion of operation sequence + recursive?
either accept polymorphic Transform values (array or Transform) —
or surface monomorphic top-level properties (`transform-sequence : []Transform` and `transform: Transform`)
```
<Rectangle id="meow">

@settings {
    #meow {
        transform-sequence:[
            {
                scale: [1.2, 2.2]
            },
            {
                translate: [400.0px, 300.0px]
            }
        ]
    }
}
```



#### Require transform sequences to be handled with expressions?
con: runtime penalty (maybe?  maybe it's equivalent given expression caching!)


```
<Rectangle id="meow">

@settings {
    #meow {
        transform: @{
            Transform::scale(1.4, 2.2) * Transform::rotate(120deg) * Transform::translate(200px, 100px)
        }
    }
}
```

The above is quite nice.  The single-transform case is easily handled as a literal, as is the "manually expand matrix" case,
and the "combine transform" case is easily & elegantly handled with an expression.

Can also expose a matrix2d method on Transform for manual computation:
```
Transform::matrix2d(a,b,c,d,e,f)

representing:
| a c e |
| b d f |
| 0 0 1 |
```

Another note: Kurbo's `Affine` (used for pax's 2d backends) currently handles all of this with similar
ergonomics.  Would it make sense to (selectively) expose these APIs directly (e.g. impl'ing local
traits as necessary to inject behavior) — or should there be a stand-alone glue layer between
the user-facing Transform API and the rendering Transform API?
^ decision: yes expose new middle object

Finally:  it's not so crazy to introduce a special "transform list" syntax and
supporting it with the parser, e.g.:
`transform: scale(1.5, 1.5) * translate(100.0, 100.0)`
instead of wrapping in an expression.
But it's a tiny readability difference, ultimately `@{}` vs not.

Decision:
`Size` lives alongside `Properties` and `Children` as a top-level member of an `instance`.
Design GUI can special-case these built-ins
In the (unusual) case where a size is explicitly not desired (e.g. Group), then
it must be handled as a primitive (i.e. one that defines `fn get_size() { None }`)

one more decision:
Add `position` as a property? (essentially `.x` and `.y` — but consistent with ergonomics of giving `transform` and `size` their own buckets)
This would act as an affine translation, after origin and align calculation but before remaining transform calculation
Currently it's not necessary because `translate` is effectively equivalent.

If `position` were added, given that it's purely ergonomic (approachability), consider
whether to add aliases like `.x`, `.y`, 



### On `compute_in_place` for generated userland components
2022-02-08

Using `Spread` as a reference, `compute_in_place` manually iterates over
properties and calls `compute_in_place`.

We don't want users to worry about this; we want to autogenerate the `compute_in_place` code for
properties.  Problematically, we can only call `compute_in_place` in the cartridge runtime context
(due to dependency on engine,) but we don't currently have metadata knowledge of properties
in that context.

One possibility:  expose an iterator that returns a sequence of Box<dyn Property> (`Property<WHAT>` though)...

Another possibility: separate the `rtc` across a trait boundary, allowing a similar maneuver
as `dyn ComputableProperty` in Engine

Note also: `Spread` created its own RenderNode as its subtree root, with a single child `Component`
Should this be the general approach?  Is there a benefit to doing this?
(beyond the necessary ability to write `compute_in_place` logic for arbitrary properties,
though note that this could be generalized by exposing an iterator over

two options:
    - expose RenderTreeContext via pax_runtime_api, untangle as needed, e.g. through traits or closure-passing
    - codegen `compute_properties_fn` closures in RIL, cartridge-runtime; add properties intelligence to parser

For the former, conceptually it's a tough split.  the RenderTreeContext is squarely conceptually attached to the runtime.
The reason for the attachment is fundamentally to access StackFrame & Scope, which are used for runtime calculations (e.g. of Expressions)
Thus, and given that property schemas will need to be understood by the parser eventually:
*Decision: codegen `compute_properties_fn` closures in RIL, cartridge-runtime; add properties intelligence to parser*

We need a fn:
```
compute_properties_fn: |mut properties: PropertiesCoproduct, rtc: &RenderTreeContext|{
    if let PropertiesCoproduct::Root(mut properties_cast) = properties {
        //Note: this is code-genned based on parsed knowlege of the properties
        //      of `Root`
        properties_cast.deeper_struct.compute_in_place(rtc);
        properties_cast.current_rotation.compute_in_place(rtc);
        properties_cast.num_clicks.compute_in_place(rtc);
    }
},
```

This requires only knowing property names, not even types/import paths (extra easy)

Update: achieved apparently functional `compute_in_place` 
Next steps: pencil in second rectangle
— then bite of expressions with manually written harness code, because there's a potential design dead end
here if we hit a wall with wiring up Expressions, Properties, Scopes, etc.



### on expressions
expressions will be transpiled to Rust — so some semantics will likely
carry over, e.g. symbol names, :: for namespace access or associated functions, etc.

dependency graph: via expressions, properties may depend on other properties
Expressions may also depend on globals/constants like @frames_elapsed 
future/theoretical: expressions may depend on descendents' values, via selector (`@('#abc').num_clicks
Expressions may depend on other 'helper' expressions, or perhaps vanilla 
functions that handle their own dirty-notifications
Expressions may have no dependencies, e.g. @{ 1+1 } 

Numeric literals need special handling re: float & ints
Should cast implicitly between int/float when necessary
Perhaps study JS/AS as model

#### Symbol resolution
Symbols in scope may be:
1. properties on `self`
2. "helpers" (methods, or special macro-decorated expressions) on `self`
3. imported symbols, in scope in the context of `whatever.rs` — alternatively, maybe can capture in closure??
4. maybe "special" imported symbols, essentially a pre-imported expression `std`



Some example transpilations:

```
@{1 + 1}
1 + 1
```

```
@{
Color::rgba(
    Math::abs(
        Math::sin(num_clicks / 10.0)
    ),
    1.0,
    0.0,
    1.0
)}

Color::rgba(
    Math::abs(
        Math::sin(num_clicks / 10.0)
    ),
    1.0,
    0.0,
    1.0
)}

```

### Journey of an expression
2022-02-09 

1. authored in .pax
2. parsed by parser; lives as string in Definition object, passed from parser binary to compiler
3. transpiled by compiler —> String (of Rust code, ready for cartridge runtime)
4. codegenned into cartridge runtime, as closure in ExpressionTable

In RIl (cartridge runtime), 



### More dependency graph untangling
2022-02-10

*Property can't depend on Engine*, due to 
the inverted dependency relationship between
cartridge and engine.  This is not news, but is worth pointing out
as the crux of this issue.

Previously we tried "sideloading" behavior
via a trait, which didn't work (is there yet a way
to make this work? one possibility is to declare
`Properties` objects (like `RectangleProperties`) in
a scope that has access to the necessary bits of `rtc`

Probably solid approach A:
- remove `compute_in_place`
- give every Property a UUID — register
    uuid -> &property (maybe ComputedProperty!) in a global hashmap
    instead of compute_in_place, look up
    a given property in each of 
    `expression` global map and `timeline`
    global map.  if present, evaluate.
    What does evaluate mean here?  It means
    storing a cached computed value 
- instead of `compute_in_place`...
(this might run into the same problem with dep. graph, trait side-loading)


**Probably solid approach B:**

- keep `compute_in_place`, but pass it a `dyn`
object, e.g. of a simple `Receiver` object (probably `impl Receiver for CarbonEngine`)

- pass the property's string ID to that receiver object
when evaluating compute_in_place

- `Receiver` (probably Engine) pops from this stack
(or removes from singular register) of string ID, uses that string ID to route `rtc`
to the right table & Fn for evaluation (Expression, Timeline)



Re: storing the ExpressionTable — there's a wrinkle in that each return type for `||{}` makes for a unique
type signature.  Either we can give a PropertiesCoproduct treatment to return types — or MAYBE we can give a PropertiesCoproduct treatment to the `Fn`s themselves.

static HashMap<String, ExpressionLambdaCoproduct> {
    "aef132": ExpressionLambdaCoproduct::
}

```



get_expression_evaluator_by_id(id: &str) {
    
}
```