**IMPORTANT NOTE:** management of list has been migrated to an external task tracker as of February 2023.
This historic "one-person tracker" remains as reference, and its contents are not authoritative or up-to-date.

## Milestone: proof of concept engine

```
[x] Rendering 
[x] Components 
[x] Logging
[x] Stroke, color, fill
[x] Sizing
    [x] Browser resize support
    [x] None-sizing
    [x] Transform.align
    [x] Transform.anchor
[x] Expression engine
    [x] variables, declaration & storage
    [x] node IDs
    [x] summonables
    [x] built-in vars like frame count
    [x] MVP rust closures + manifest of deps
[x] Stackers (née Stacks)
    [x] Decide `primitive` vs. userland `components`
    `components`
    [x] Internal template mechanism for components
    [x] Make `root` into a component definition
    [x] Control-flow `slot` (`slot`) for inputs/children
    [x] Ensure path forward to userland `slots`
    [x] Clipping & Frames
    [x] Control-flow `repeat` for cells & dividers inside template
    [x] Gutter
[x] Split out userland code
    [x] Add a third project to workspace, the sample project
    [x] (Further work to be done via compiler task)
[x] Timelines, transitions
[x] Refactors
    [x] Bundle Transform into "sugary transform," incl. anchor & align; consider a separate transform_matrix property
    [x] Is there a way to better-DRY the shared logic across render-nodes?
e.g. check out the `get_size` methods for Frame and Stacker
    [x] Maybe related to above:  can we DRY the default properties for a render node?
Perhaps a macro is the answer?
    Same with `scale`
    [x] Can we do something better than `(Box<......>, Box<.......>)` for `Size`?
    [x] Rename various properties, e.g. bounding_dimens => bounds
    [x] Take a pass on references/ownership in render_render_tree — perhaps &Affine should transfer ownership instead, for example
    [x] Better ergonomics for `wrap_render_node_ptr_into_list`
    [x] Evaluate whether to refactor the `unsafe` + PolymorphicType/PolymorphicData approach in expressions + scope data storage
```

## Milestone: Stacker

```
[x] decide on API design, expected GUI experience
    - Direction (horiz/vert)
    - Gutter
    - Cell widths
[x] expose a Stacker element for consumption by engine
[x] accept children, just like primitives e.g. `Group`
[x] author an internal template, incl. `slot`ing children and `repeating` inputs
    <Frame repeat=self.children transform=get_transform(i)>
        <Slot index=i>
    </Frame>
    - need to be able to define/call methods on containing class (a la VB)
    - need to figure out polymorphism, Vec<T> (?) for repeat
    - need to figure out slot — special kind of rendernode?
[x] Frame
    [x] Clipping
[x] Slot
[x] Repeat
    [x] "flattening yield" to support <Stacker><Repeat n=5><Rect>...
    [x] scopes:
        [x] `i`, `datum`
        [x] braced templating {} ? or otherwise figure out `eval`
            - Code-gen?  piece together strings into a file and run rustc on it?
            * Can achieve this with Expressions for now
        [x] calling "class methods" from templates, e.g. <Repeat n=5><Rect color="get_color(i)"
            * Can achieve with expressions
[x] Scopes & DI
    [x] Figure out dissonance between:  1. string based keys, 2. struct `Properties`
        and figure out how this plays out into the property DI mechanism.  Along the way,
        figure out how to inject complex objects (ideally with a path forward to a JS runtime.)
            - Option A:  write a macro that decorates expression definitions (or _is_ the exp. def.) and (if possible)
                         generates the necessary code to match param names to elsewhere-registered dependency streams (Angular-style)
            - Option A0: don't write the macro (above, A) yet, but write the expanded version of it by hand, incl. string deps (present-day approach)
                         ^ to achieve "DI," does this require a hand-rolled "monomorphization" of each expression invocation?
                         Or does a string dependency list suffice?  If the latter, we must solve a way to pass a data-type <D> through PolymorphicValue
                         Probably the answer is the _former_ — write the DI-binding logic manually alongside a string dep list (roughly how the macro unrolling will work)
    [x] Support getting self (or Scope) for access to Repeat Data
        - use-case: translate each element within a `repeat` by `i * k`
    [x] Quick pass on other relevant data to get from Scopes
[x] Layout
    [x] Primary logic + repeat via expression
    [x] Parameterize:
        - Gutter
        - (come back later for overrides; ensure design supports visual UX)
```

## Milestone: expressive RIL, hand-written

_RIL means Rust Intermediate Language, which is the
"object code" — i.e. compiler backend target — for Pax.  As the name suggests, RIL is valid Rust
(specifically, a subset of Rust.)  Pax relies on
`rustc` (via `cargo`) to convert RIL "object code" to machine code._

```
[x] Compile base cartridge
    [x] Refactor PropertiesCoproduct to its own module
    [x] Sanity check "patch" ability for "blanks" (Properties, Expressions)
    [x] Demo app chassis running example project (`./serve.sh`)
        [x] Add stub macro for `pax`, derives
[x] baseline primitive(s) for hello world
    [x] import/package management
    [x] RIL -> PAX compatibility, port primitives
        [x] Repeat, path from @foreach
[x] hand-write RIL first!
    [x] Handle control-flow with codegen
        [x] support with manual RIL, port old primitives
            [x] @for
                [x] syntax
                [x] with/out enumeration (`i`)
                [x] figure out scoping, e.g. addition of symbols to namespace, collision management
                [x] RepeatItem: manage scope, properties
            [x] @if
            [x] slot
            [x] frame
    [x] rendering hello world
    [x] proof of concept (RIL) for expressions
    [x] handle expressable + nestable, e.g. Stroke (should be able to set root as Expression, or any individual sub-properties)
    [x] proof of concept (RIL) for timelines
        [x] running on assumption that the problem is isomorphic to the Expression vtable problem
    [x] proof of concept (RIL) for actions
        [x] pax_lang::log
        [x] `Tick` support (wired up)
        [x] pencil in `Click`, but don't worry about raycasting yet (or do naive raycasting?? easy to PoC!)
        [x] sanity-check Repeat
    [x] port Repeat, etc. to latest RIL
[x] API cleanup pass
    [x] make consistent Size, Percent, Anchor, Align
    [x] finish-line @if
    [x] support in-.rs-file pax, as alternative choice vs. code-behind file
```

## Milestone: imported .pax

```
[x] Spend some cycles ideating demo deliverables/storyboard
[x] Port Stacker to be a pure component
    [x] figure out importing mechanism for core and/or other std primitives
        [x] Group, Slot (e.g. from Core)
        [x] Frame (e.g. from std — though if easier could just move to Core)
    [x] port stacker logic; design expression/method approach
        [-] pure expressions + helpers?
        [x] on_pre_render + manual dirty checking?
            [x] decision: no dirty checking at all for right now; expose imperative dirty-check API only when implementing dirty checking for Expressions
    [x] hook in existing layout calc logic
[x] Import and use Stacker in Root example
    [x] update example .pax as needed along the way
    [x] "expand the proof" of generated code & make it work manually
```


## Milestone: clickable square

```
[x] Action API
    [x] state management (.get/.set/etc.)
    [-] hooks into dirty-update system, to support expression dirty-watching
    [x] Instantiation, reference management, enum ID + addressing for method definitions &
        invocations
    [x] tween/dynamic timeline API
[x] revisit chassis-web implementation
    [x] rust/JS divide
        [x] Sandwich TS and rust (as current,) or
        [x] handle all cartridge work from rust, incl. generation of public API
[x] Event capture and transmission
    [x] Map inputs through chassis, native events (mouse, touch)
        [x] PoC with Web
        [x] PoC with macOS
    [x] tick event e2e
    [x] Message queue in runtime
    [x] Ray-casting? probably
    [x] Message bubbling/capture or similar solution
[x] Expressions
    [x] Write ExpressionTable harness, incl. mechanisms for:
        [x] vtable storage & lookup
        [x] Return value passing & caching
    [-] Sketch out design for parallelized expression computation (e.g. in WebWorkers)
    [-] Patch ExpressionTable into cartridge à la PropertyCoproduct
```


## Milestone: Eureka

```
[x] resolve the `Any` question
    [x] defer until later; reassess if it becomes clear a lot of backtracking is emerging 
[x] parser
    [x] grammar definition, PEG
    [x] parse grammar into manifest
[x] grammar/parser updates:
    [x] `@for`
    [x] ranges: literal and symbolic
    [x] `@template` block, vs. top-level
[x] native rendering ++
    [x] message design/arch
    [x] runtime de/serialization
        [x] Data across FFI
            [x] explore raw C structs (decided: brittle)
            [x] Logging, strings, message-passing
            [x] CRUD operations and methods: PoC with `Text`
            [x] flexbuffers instead of fat c structs
                - Might want to revisit one day for fixed schema, might reduce Swift boilerplate?
    [x] ids
        [x] handle monotonic, instance-unique IDs
            - expose method in engine to generate new id
            - initial use-case: instance IDs for associating engine <> native Text instances
    [x] text support
        [x] handle_mount and handle_unmount
        [x] trigger mount/unmount lifecycle events in engine, `Conditional`, `Repeat`
        [x] hook up Text primitive + API
            [x] macOS
            [x] web -- reattach with updated messaging model
            [x] basic API pass: content, color, font, textSize
        [x] handle dirty-checking for `Patch` event population
    [x] native clipping support
        [x] rectilinear-affine frames
        [x] support macOS
        [x] support Web
    [x] click support
        [x] simple 2D ray-casting
            [x] Handle clipped content? and scrolled content? (i.e. check within accumulated clip bounds as well as object bounds)
        [x] inbound event arg-wrapping and dispatch
            [x] macos
            [x] web
        [x] sketch out bubbling/canceling, hierarchy needs
[x] dev env ++
    [x] support for different example projects
    [x] native macOS chassis + dev-harness
        [x] pax-chassis-macos (written in rust). responsible for:
            [x] accepting a CGContext pointer and rendering to it via Piet
            [x] managing user input channel, e.g. click/touch
            [x] managing native rendering channel, e.g. form controls, text
        [x] mac app dev-harness (written in swift). responsible for:
            [x] granting a piece of real estate (full window of simple mac app) to rendering with a CGContext.
            [x] passing CGContext to pax-chassis-coregraphics
            [x] handling resize
            [x] handling basic user input (e.g. click)
        [x] Debugging via LLDB
            [x] support debugging as necessary with macos dev-harness
            [x] IDE configs for each of: userland cartridge; core; std
[x] compiler + codegen
    [-] codegen Cargo.toml + solution for patching
    Note: decided to require manual Cargo setup for launch (solved by `generate` use-case)
        [x] manual
        [-] automated + file generation
        [-] Note use-case: pax-std also needs the same feature flags + deps.  Would be nice to automate!
    [x] .pax folder
        [x] manual .pax folder 'proof'
        [x] codegen + templating logic
    [x] generate `pub mod pax_reexports` via `pax_app` -- tricky because full parse is required to
        know how to build this tree.  Either: do a full parse during macro eval (possible! pending confirmation that parse_to_manifest can be called at macro-expansion time) or
        do some codegen/patching on the userland project (icky)
        (Tentative decision: refactor macro-time parse logic; probably do a full parse; return necessary dep strings along with pascal_identifiers)
        Escape hatch if the above doesn't work: use include_str!() along with a file that contains
        [x] Refactor parsing logic -- maybe start clean?
            [x] De-risk: verify ability to call macro-generated code at compile-time (parse_to_manifest)
            [x] Parse all of, recursively:
                [x] Template
                [x] Properties:
                    [x] Property key
                    [x] Property type, full module (import) path
                        [x] This will be tricky... either
                            [-] static analysis on the whole source file, looking for a `use ... KEYWORD` statement and infer paths
                            [-] dynamic analysis -- some kind of rustc API.....?
                            [x] dynamic analysis: `parse_to_manifest`-style `get_module_path`, called by parser
                                [x] parser returns fully qualified types in manifest by dynamically calling `get_module_path` on each discovered type during parsing
                                [x] refactor: punch `parser` features through all necessary Cargo.tomls — `impl PropertyManifestable` manually for `Size` and `Size2D`
                                [-] refactor: instead of `scoped_resolvable_types`, might need to `impl PropertyManifestable` for concrete nested generic types, e.g.
                                    instead of Vec::get_property_manifest and Rc::get_property_manifest, Vec<Rc<StackerCell>>::get_property_manifest
                                    -- this addresses compiler error `cannot infer type T for Rc<T>` (non-viable approach)
                                        [x] Alternatively: hard-code a list of prelude types; treat as blacklist for re-exporting
                                            -- note that this adds additional complexity/weakness around "global identifier" constraints, i.e. that as currently implemented, there can be no userland `**::**::Rc` or `**::**::Vec`  
                            [-] Require fully qualified types inside Property<...>, like `Property<crate::SomeType>` or `Property<pax_lang::api::Color>`  
                            [-] Make Property types `Pathable` or similar, which exposes a method `get_module_path()` that invoked `module_path!()` internally
                                Then -- `pax` macro can invoke `get_module_path()` just like `parse_to_manifest`
                                (Evaluated at compiletime) `Color::_get_module_path()`
                                    ^ wrong -- this would be evaluated at `parser bin`-time
                                Which returns `pax_runtime_api::Color`
                                Which gets codegenned into `pub mod pax_reexports`
                                Checksum: does this resolve at the right time
                                MAYBE we still need two phases:  one that parses template and property types, which allows codegen of `get_module_path` and `parse_to_manifest`
                                                                 and another that runs the parser bin
        [x] Codegen`.pax/reexports.partial.rs`
            [x] Basic logic
            [x] Filter prelude
            [x] fix bug: using `ctx` for property_manifests (maybe only a bug in primitives)
            [x] Hook up primitives + types
                -- possibly recycle logic that checks for `Property<...>`
            [x] fix bug: prelude types not showing up in PropertyManifests
                -- perhaps each propertymanifest should have:
                    -- fully qualified atomic types
                        -- for `pub mod pax_reexports`
                        -- for recursive calls to `get_property_manifest`
                        -- for TypesCoproduct generation
                        -- for cartridge codegen (imports)
                    -- In each of the above, it should be reasonable to handle prelude vs. imported types separately
                    -- Also: should this be called something else, like `dependencies` or `imports`? Instead of `PropertyManifests`.
                    -- What about `properties`?  For design tooling, will definitely be necessary.  Is it worth shimming `dependencies` into 
                       -- maybe `get_property_manifest` should just be `get_property_dependencies` ??
        [-] include_str!() `.pax/reexports.partial.rs` _at compile-time_ in macro logic
            -- the goal is for this types re-export to be present at the root of the *userland project* by the time
            the chassis is attached / compiled
            [-] might need to "create if doesn't exist," or otherwise guard the lifecycle of the include_str! per best practice
                -- a likely-viable approach: feature-gate two complementary `pax_app` macros, across binary values `is parser`
                   parser mode passes an empty string; prod mode assumes presence of .pax/reexports.partial.rs and hard-code-includes it
            [x] solution: write another macro, gated behind `not(feature="parser")`, which gets
                passed `env!("CARGO_MANIFEST_DIR")`
    [x] parser bin logic finish-line
        [x] macro
    [X] untangle dependencies between core, runtime entities (e.g. Transform, RenderTreeContext, InstanceNodePtrList), and cartridge
    [X] work as needed in Engine to accept external cartridge (previously where Component was patched into Engine)
    [x] update .pest and manifest-populating logic to latest language spec
    [x] support incremental compilation — not all #[pax] expansions (namely, side-effects) are expected to happen each compilation
        [-] NOTE: provisionally, this whole group is solved as not necessary, in light of the "parser binary" feature-flagged approach
        [x] science: determine how macros behave, caching of expansion, incremental compilation
        [x] allow macros to report their registrants and the appropriate module path + file path via IPC
        [x] sweep for files that have been removed from fs; update fsdb
        [x] determine
        [x] persist fsdb to disk, probably .pax/fsdb, probably binpack
        [x] enable manual full rebuild, e.g. via `pax compile --full | --force | etc.` (should be doable via rustc with something like INCREMENTAL=0, refer to rust incremental compilation docs)
    [x] architecture
        [x] compiler seq. diagram 
        [x] dependency diagram
    [x] two-stage compilation process
        [x] thread/process/IPC chassis
        [x] parser cargo feature
        [x] bin-running harness to execute parser (see https://stackoverflow.com/questions/62180215/renaming-main-rs-and-using-with-cargo)
        [-] TCP message passing (using stdio for now)
            [x] de/serialization for manifest
                [x] maybe normalize SelectorLiteralBlockDefinitions, if Serde can't elegantly de/serialize it
                [x] or de-normalize TemplateNodeDefinition!
            [-] coordination of TCP client/server from compiler main thread
        [x] parse and load .pax files
            [x] load file via macro
            [x] generate the parser bin logic via macro
                [x] manual
            [x] port minimal set of std entities (Rectangle, Group) to support manifest-gen 
            [x] traverse manifest of Component defs: parse .pax files, store in mem
            [x] (start with templates only)
    [x] thread for wrapping `cargo build`
    [x] sketch out .pax folder design
    [-] graceful shutdown for threaded chassis (at least: ctrl+c and error handling)
        [x] Alternatively: back out of async, given stdio for passing data from parser 
    [x] generate & embed reexports
        [x] parse properties into manifest
        [x] bundle pax_reexports into nested mods
        [x] load reexports.partial.rs into userland project
    [x] introduce `pax-cli`, import compiler to be a dep
        [-] `pax demo`?
        [-] `pax create` 
    [x] generate properties coproduct
        [x] retrieve userland crate name (e.g. `pax-example`) and identifier (e.g. `pax_example`) 
            [-] alternatively, hard-code a single dependency, something like "host", which always points to "../.." (relative to ".pax/properties-coproduct")
                -- don't think the above will work; cargo needs a symbol that maps to the target Cargo.toml
        [x] patch / generate Cargo.toml
            [x] include `pax-example = {path="../../"}`
                -- where `pax-example` is the userland crate name
        [x] PropertiesCoproduct
            -- coproduct of ComponentName
            [x] run through Tera template, iterating over dependencies
        [x] TypesCoproduct
            -- coproduct of all types from PropertyDefinitions 
            [x] if necessary, supporting type parsing & inference work for TypesCoproduct
            [x] run through Tera template
    [x] generate cartridge definition
        [x] prelude / hard-coded template
        [x] imports via `pax_example::pax_reexports::*`
            -- or, fully resolve every import when using
        [x] component factories
            [x] `compute_properties_fn` generation
            [x] `instantiate_main_component` generation, based on `pax_root` declaration
            [x] `properties: PropertiesCoproduct::Stacker(Stacker {...})` generation
            [x] PropertiesLiteral inline embedding vs. PropertiesExpression vtable id embedding
        [x] fully qualified resolution of expression symbols, e.g. `Transform2d`
            [-] Alternatively: don't support arbitrary imports yet:
                [-] Support referring to `T` for any Property<T> (already parsed + resolvable)
                [-] Import Transform2d::*, Color::*, and a few others via prelude
                    -- Note that each prelude import prohibits any other top-level symbols with colliding names; increases DX snafu likelihood via compiler errors
        [x] expression compilation
            [x] fully qualified resolution of expression symbols, e.g. `Transform2d`
                [-] Alternatively: don't support arbitrary imports yet:
                    [-] Support referring to `T` for any Property<T> (already parsed + resolvable)
                    [-] Import Transform2d::*, Color::*, and a few others via prelude
                        -- Note that each prelude import prohibits any other top-level symbols with colliding names; increases DX snafu likelihood via compiler errors
            [x] expression string => RIL generation
                [x] Pratt parser "hello world"
                [x] operator definitions to combine `px`, `%`, and numerics with operators `+*/-%`
                [x] grouping of units, e.g. `(5 + 10)%` 
                [x] boolean ops: `==`, `&&`, and `||`
                [x] parenthetical grouping  `(.*)`
                [x] Literals for strings, bools, ints, floats
                [x] Nested object references + injected context
                    [x] invocations for deriving values from scope
                    [x] type-matching
                    [x] Numeric type management, type inference / casting
                [x] QA
                    [x] fix: order / index of `vtable.insert` statements
                        -- ordering is jumbled
                        -- `3` is passed twice
                    [x] fix: import prelude types like Color::* so that `rgb()` and friends just work
                    [x] fix: duplicate invocation of `j` in `rgb(100 %, (100 - (j * 12.5)) %, (j * 12.5) %) `
                    [x] clean: remove "offset example" comment
                    [x] fix: tuples return usize(0) instead of Range()...
                    [x] fix: <<TODO>> for invocation of built-ins (either build built-in or refactor...)
                    [x] fix: need to fully qualify assc. fn. (helpers), e.g. a helper is currently called as `get_frame_size(` instead of `pax_reexports::...::get_frame_size()` 
            
    [x] generate / patch chassis cargo.toml
    
    [x] control flow
        [x] for
            [x] parse declaration `i`, `(i)`, `(i, elem)`
            [x] handle range literals 0..10 
            [x] shuttle data into RepeatInstance via Manifest
        [x] if
            [x] parse condition, handle as expression
        [x] slot
            [x] parse contents as expression/literal, e.g. `slot(i)` or `slot(0)`
[x] support inline (in-file) component def. (as alternative to `#[pax_file]` file path)
[x] e2e `pax run`
[x] publication to crates.io
    [x] reserve pax-engine crate on crates.io
    [x] update relative paths in all cargo.tomls, point to hard-coded published versions
        - can include both a relative path & version number, a feature of Cargo for this exact use-case
    [x] publish all crates
```


## Milestone: Alpha Launch

```
HIGH
zb [ ] hook up `pax_on` and basic lifecycle events
    [ ] consider design for async while doing this
    [ ] also requires hooking up `handler_registry` generation in cartridge
wj [ ] `Numeric` - "unlocks ability to multiply floats & ints"
    [ ] Introduce wrapper around literal numerics
    [ ] Build Into and From across expected primitive types (f64, usize, isize, etc.)
    [ ] Implement necessary Rust operators (`Mul`, etc.), following "truth table" for conversions
    [ ] Implement necessary non-Rust-builtin-operator operations, like `pow`, on Numeric directly
    [ ] Refactor Size (maybe take on Color, etc. along the way) to accept numerics 
        [ ] Chase this thread through the engine, refactoring to support Numeric where necessary (e.g. inside Size)
zb [ ] (Crude) error reporting through CLI
    [x] Parser errors (row, column and spatially local text)
        [ ] Better ParsingError handling, at least surfacing only the line_col info without the noise
        [ ] Stretch: explore a quick, friendlier error message PoC with "cascading errors in PEG" approach
    [x] Chassis compilation error
        [ ] Primitive source-maps
        - MVP: pipe / output stderr correctly?
        - Note: this only occurs during lib dev or when encountering a compiler bug in the wild
                Due to the above, this may not be a blocker on alpha launch
wj [ ] "code-behind file" as alternative to in-Rust macro
[x] turn off codesigning for dev harness
[ ] docs pass
    [ ] clean up codebase; reduce warnings
    [ ] README pass & updates
    [ ] update concepts, appendix articles
    [ ] Consider writing guides:
        [ ] Walk through building one or more examples (e.g. show reusable components, expressions, loops, lifecycle event handlers)
        [ ] Pipeline of compiling a project
        [ ] Writing a primitive
wj [ ] support `pax build` for distributable binaries (alternative to `pax run`)
    [ ] at least web + production build; consider the codesigning question separately
[ ] interactive demos for docs, website 
[ ] site
    [ ] branding
    [ ] content
        [ ] audience, outline, etc.
    [ ] demos
[ ] windows & linux dev envs
    [ ] web target only
    [ ] ensure e2e support & ergonomics
[ ] usability - high priority
    [x] figure out spurious stroke / outlining on macOS (clear Piet context?)
   zb [ ] solve pax-std feature-flagging / "roundup export" / raw coproduct memory access problem
        [ ] either `pax_primitive_exports` at crate root for external crates — or raw memory access into coproduct
   zb [ ] Patch in RIL generation for control flow — `Stacker` should work flawlessly, incl. `Slot` and `Repeat`
    [x] Figure out how to fork process properly in macOS without requiring debug mode
    [x] Fix native bindings / wasm compile issue (extricate pax-compiler)
    [x] pax-compiler "cache" — sometimes changes aren't propagated to `.pax/chassis/MacOS` —
        this is because the file copying logic for `include_dir` aborts when files already exist
        a solution: manually walk `include_dir` fs and write each file, instead of 
        using their provided one-shot method 
    [x] Any easy wins for compile times?
        [x] See if we can decouple pax-macro from pax-compiler — currently userland projects that rely on pax-macro thus rely on pax-compiler, which in turn has many dependencies and takes a long time to compile.
        [x] pre-npm-install for web builds?
        [x] package.json and Cargo.toml dependency cleaning; austerity measures 
   zb [ ] Refactor Color to accept percentage channel values; refactor Translate/etc. to support
        [ ] Also design ideal re: constant access -- maybe
            Color::Rgb(r,g,b) should be a literal enum after all? (instead of fn call)
            This enables Color::Red() & other "constants" to live under the same symbol, `Color`
   wj [ ] Introduce x= and y= values, separately from `Transform` (probably a last-applied Translate, in practice)
        [ ] consider revisiting transform API — keep current functionality, but also add translate=, rotate=, scale=
            -- this gracefully handles the use-case where complex or nested transforms are required (just use `rotate=120` instead of `transform={Transform2D::rotate(120)}`)
    [ ] profiling, CPU sanity-optimization (likely: impl. dep graph / dirty-tracking / caching for expressions)
   zb [ ] Production harness
        [ ] macos
            [ ] allow config of apple id / signing cert
                alt: can probably punt this, until concrete needs arise
        [ ] web
            [ ] decide if necessary; maybe same as dev harness for now
   zb [ ] Warning cleanup


MED
[ ] round out functionality
    [ ] Text APIs (and fix macOS regression)
    [x] Path primitive
        [x] API design
    [x] Ellipse primitive
[ ] polish up defaults & expected behavior (e.g. width of a Rect when unspecified)


```

## Milestone: usability & functionality++

```

[ ] Cargo cache bug (when changing the contents of @pax/pax-cartridge (template),
        `cargo clean` is sometimes needed to update the `include_dir` cache.)
        Is there a better solution than requiring `cargo clean` on lib-dev machines?
[ ] New units: `rad` and/or `deg` for rotation
[ ] @settings blocks
[ ] Runtime error handling
    [ ] chart out surface area
        [ ] divide by zero?  what else?
        [ ] vector access
    [ ] ideally: patch into source maps
[ ] consts
    -- decorate with `#[pax_const]`; copy tokens? or refer to orig?  consider component/namespace reqs
    -- consider also how to "coordinate between macros" 
[ ] Support async
    [ ] `Property` => channels 'smart object'; disposable `mut self` => lifecycle methods (support async lifecycle event handlers)
    [ ] Pencil out error handling (userland)
    [ ] update runtime to dispatch event handlers asynchronously, creating and passing "disposable self"s as needed
[ ] Dependency tracking & dirty-watching
    [ ] support "helpers", composable functions, which also serve the need of temporaries/`let`s
    [ ] dependency DAG + lazy evaluation, a la Excel
        [ ] cache last known values
        [ ] consider keeping a cache of `state tuple => output value`
    [ ] address use-case of `Property<Vec<T>>`, inserting/changing an element without setting the whole
        entity.  Might want to offer a `get_mut` API (keep an eye on async / ownership concerns)
        -- In fact, probably address this on the heels of a Property -> channel refactor, as the implications for this
        intersection are significant  
        -- Solution for now: 1. store Vec<T> inside Property<Vec<T>> — .get_mut() the Vec, mutate, then set() the updated value -- treat the entire property as dirty when this `set` occurs
[ ] usable error messages
    [ ] macro-time errors:
        [ ] write to stderr? or other pipe/file/channel readable by `pax-compiler`
    [ ] parsetime errors:
        [ ] pax syntax errors -- error handling with Pest (add "overflow" final rule to grammar at necessary junctures, e.g. `foo = bar | baz | error` — then handle `error` matches at parse-time)
        [ ] track line/column numbers; figure out how to report back to Rust/editor language server (if possible)
    [ ] compile-time errors:
        [ ] handle ad-hoc 
    [-] runtime errors: probably no special handling needed
[ ] `with` + event bindings
    [ ] vtable + wrapper functions for event dispatch; play nicely with HandlerRegistry; add `Scope` or most relevant thing to args list in HandlerRegistry
    [ ] update `handler_registry` closure codegen in cartridge — call methods with correct signature based on specified `with` 
    [ ] grammar+parser support
    [ ] single variables, with option parens (e.g. `with (i)` or `with i`, or `with (i,j,k)`)
    [ ] multiple variables in tuple `(i,j,k)`
[ ] Scrolling
    [ ] Create `Scroller` primitive -- =refer to `Text` primitive for closest reference: pax-std-primitives
        [ ] Register `Scroller` in `pax-std` and surface for use in `pax-example`
    [ ] native scrolling container, passes scroll events / position to engine
        - refer to how `click` events are handled from chassis-macos through the runtime
    [ ] bounds for scrolling container passed to chassis by engine; chassis instantiates a scrolling container
    [ ] attach native content as children of the native scroll container, so that native content scrolls natively with no need for Pax runtime intervention
        - if this introduces "jello" sync problems, instead try updating native elements positions through the vanilla CRUD operations — refer to how `Text` content is updated in `chassis-macos` or `chassis-web`
    [ ] Update `HandlerRegistry` to account for scroll event handlers; support `@scroll=self.some_handler` registration
```

## Milestone: form controls
```
[ ] `Text` upgrades:
    [ ] alignment horizontal within bounding box
    [ ] alignment vertical within bounding box
    [ ] auto-sizing?  (makes for super-easy alignment; requires letting OS render one frame, possibly off-screen, then applying that calculation.  Requires `interrupt` from chassis after size is measured.
[ ] Rich text display (Possibly support markdown as authoring format)
[ ] Dropdown list
[ ] Slider
[ ] Toggle buttons (iOS "on/off" style)
[ ] Button + content (arbitrary rendering content?  or just text to start?)
[ ] File controls
    [ ] Upload/open control?
    [ ] save/download control?
[ ] Text input boxes
    [ ] single-line
    [ ] multi-line?
    [ ] rich text?
    [ ] Numeric + stepper?
[ ] Databinding: event-based changes + two-way binding
```

## Milestone: iOS & Linux
```
[ ] Refactor ABI/FFI layer - Do before adding new targets, so that porting work & waste are reduced
    [ ] flexbuffers -> flatbuffers, with schema
    [ ] Port `pax-message/../lib.rs` to a Flexbuffer schema + codegenned Rust
    [ ] Update macOS messaging layers to support
    [ ] (Profile overhead of JSON serializing+parsing for Web; decide whether additional footprint cost is worth embedding a flatbuffer reader) 
[ ] Linux/GTK dev-harness & chassis 
    [ ] support `NativeMessage`s for all primitives and `std`
    [ ] dev-harness
[ ] ios app dev-harness & chassis
    [ ] support `NativeMessage`s for all primitives and `std`
    [ ] extract shared logic with macOS dev harness
    [ ] support ios simulator / physical device
    [ ] CLI hookups (--target)
[ ] Touch support
    [ ] tap / swipe support
    [ ] `clap` event, for "click or tap"
```


## Milestone: drawing++
```
[ ] Image primitive
    [ ] Bitmap support: .png, .jpg, maybe .svg, serialize as base64 and include encoding so that chassis can make sense of it
    [ ] API: how to support assets? check out `include_bytes!`
[ ] Gradient fills: shapes, text
[ ] Support interpolating Colors; see: `impl Interpolatable for Color {` ... 
[ ] Opacity (likely built-in property alongside `transform`, `size` -- it accumulates down render tree)
[ ] Palette built-in: perhaps `@palette { optional_nesting: {  } }
[ ] Path primitive + APIs for animations
[ ] Mask primitive (a Clipping primitive that allows a `Path` definition for bounds, instead of a rectangle)
[ ] Ellipse
[ ] macOS chassis improvements:
    [ ] Revisit loop timer?  (keep timestamps, target exact FPS, able to know when frames are dropped)
 
```



## Milestone: capabilities++
```
[ ] support stand-alone .pax files (no rust file); .html use-case
[ ] asset management -- enables fonts and images
    [ ] decide on approach: bundle into binary or work with chassis/dev-harness for bundling
    [ ] support async/http assets, relative paths + configurable prefix path, absolute paths, http/https
[ ] Text++
    [ ] custom fonts: embedded or webfonts
    [ ] basic text + paragraph API
        [ ] font family, face, weight, decoration
        [ ] "rich text" or "annotated text" to support mixed content
        [ ] paragraph: left-align/right-align/justify
        [ ] (rely on `Transform` for vertical centering)
[ ] Raster images
    [ ] Sizing / clipping API (e.g. cover, or anchor + stretch/maintain/fill/target-width/target-height/etc.)
```



## Milestone: embedded UI components
(instead of full-window Electron/Expo-like wrappers)

```
[ ] Runtime sharing -- allow cartridges to share/sideload a runtime instead of duplicating, e.g. for 100 individual component/cartridges in a codebase
[ ] Component SDK
    [ ] per-platform userland API (`mount`)
    [ ] `component-harness` alternative to `dev-harness` for embedding
    [ ] Wire up: properties/settings, placeholders,
    
```

## Milestone: timelines

```
[ ] Hook up PropertyTimeline
    [ ] refactor easing curve packaging, probably into enum
    [ ] refactor Tweenable, to support arbitrary types (dyn Tweenable) and impl for `fsize`
    [ ] support Tweenable for f64
    [ ] support Tweenable for `Transform`
[ ] ergonomic timeline API design in pax (probably JSON-esque {30: value, 120: other_value} where 30 is frame number
```


## Backlog

```
[ ] Imperative dirty-tracking API (a la MutationObserver)
[ ] Temporary lets in PAXEL?
    [ ] Or: easy composition, "helpers"
[ ] Support "skinning" the template subgrammar:
    [ ] KDL
    [ ] YAML
    [ ] JSON
[ ] "Media queries"
    [ ] built-ins for platforms (`@macos, @web`)
    [ ] `if` for dynamic application of properties, also responsive/screen-size concerns
[ ] Pax Browser
    - simple desktop app, mostly a dev harness, but also
      supports 
[ ] Revisit embedded literal strings across codebase for error messages, to reduce binary footprint (enum? codes?)
[ ] Reinvestigate Any as an alternative to Coproduct generation
    [ ] would dramatically simplify compiler, code-gen process
    [ ] would make build process less brittle
    [ ] roughly: `dyn InstanceNode` -> `Box<Any>`, downcast blocks instead of `match ... unreachable!()` blocks
        [ ] ensure compatibility with `Rc`!  Circa Q4'21, Any could not be downcast to Rc (e.g. `InstanceNodePtr`)
        [ ] de-globalize InstantiationArgs, e.g. `slot_index` and `source_expression`
        [ ] remove PropertiesCoproduct entirely (and probably repeat the process for TypesCoproduct)
        [ ] possibly remove two-stage compiler process
        [ ] sanity check that we can downcast from a given `Any` both to: 1. `dyn InstanceNode` (to call methods), and 2. `WhateverProperties` (to access properties)
        [ ] sanity check that `Any + 'static` will work with needs of `dyn InstanceNode` and individual properties
        [ ] check out downcast_rs crate as way to ease?
        [ ] ensure that `'static` constraints can be met!! e.g. with dynamically instantiated nodes in `Repeat`
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
[ ] Margin & padding?
    [ ] Decide whether to support, e.g. is there a simpler alternative w/ existing pieces?
    [ ] Decide whether to support ONE or BOTH
[ ] Component-level defaults ("default masks") for properties (think: design system) -- e.g. "if not specified, all Rectangle > Stroke is _this value_"
[ ] Frames: overflow scrolling
[ ] chassis:
    [ ] iOS
    [ ] Windows    
    [ ] Android
    [ ] Linux (GTK?)
[ ] Gradients
    [ ] Multiple (stacked, polymorphic) fills
    [ ] Gradient strokes?
[ ] Production compilation
    [ ] Generation of RIL, feature-gating `designtime`
[ ] Packaging & imports
    [ ] Ensure that 3rd party components can be loaded via vanilla import mechanism
[ ] JavaScript runtime
    [ ] First-class TypeScript support
    [ ] API design
        [ ] code-behind & decorator syntax
    [ ] Bindings to `runtime` API, plus IPC mechanism for triggering
[ ] Language server, syntax highlighting, IDE errors (VSCode, JetBrains)
[ ] Transform.skew
[ ] 3D renderers
[ ] Audio/video components
    [ ] "headless" components?
```

```
Creative development environment
for makers of
graphical user interfaces
```


<img src="compiler-sequence.png" />
<img src="runtime-arch.png" />
