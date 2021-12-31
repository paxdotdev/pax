# TODO

## Milestone: "hello world" from .pax

[ ] Compile base cartridge
    [ ] Refactor PropertiesCoproduct to its own module
    [ ] Sanity check "patch" ability for "blanks" (Properties, Expressions)
    [ ] Demo app chassis (`pax run web`)
[ ] `pax` macros and compiler
    [ ] threaded chassis for jobs, e.g. server & shell `cargo`
    [ ] cleanup for threaded chassis, e.g. ctrl+c and error handling
    [ ] Idempotent server process startup from macros
    [ ] Phone home to server to register Components/Properties    
    [ ] mechanism to ensure every macro is invoked each compilation (or otherwise deterministically cached)
    [ ] Dump manifest to PropertiesCoproduct, patch, recompile (or hot reload)
[ ] legwork for `Definitions` and `Instances`
    [ ] Write `Definition` structs and refactor existing entities to `Instance` structs
    [ ] Write ORM methods for `Definitions`
    [ ] Attach CRUD API endpoints to `Definition` ORM methods via `designtime` server
[ ] parse and load .pax files
    [ ] Map parser logic (Pest token pairs) to CRUD API calls for `designtime` server
    [ ] traverse in-mem manifest of Component defs: parse files, pass to server
    [ ] figure out recompilation loop or hot-reloading of Properties and Expressions
[ ] render Hello World
    [ ] Manage mounting of Engine and e2e 



## Milestone: vNext

[ ] Production compilation
[ ] Expressions
    [ ] Transpile expressions to Rust (or choose another strategy)
    [ ] Write ExpressionTable harness, incl. mechanisms for:
        [ ] Dependency tracking & dirty-watching
        [ ] Return value passing & caching
        [ ] Sketch out design for parallelized expression computation (e.g. in WebWorkers)
    [ ] Patch ExpressionTable into cartridge a la PropertyCoproduct
[ ] Packaging & imports
    [ ] Ensure that 3rd party components can be loaded via vanilla import mechanism



## Backlog

[ ] JavaScript runtime
    [ ] First-class TypeScript support