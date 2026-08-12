# Launch documentation coverage map

Status: phase-one discovery, gallery synthesis, and Checkpoint 1 review incorporated

Owner issue: PAX-975

Parent program: PAX-860

Last audited: 2026-08-03

This document defines a launch-bounded learning path for Pax. It is a planning artifact, not a draft of the finished chapters. It deliberately separates three bodies of material:

1. public prose that helps a builder evaluate, learn, and ship with Pax;
2. generated API reference for exact types and methods; and
3. internal architecture, design, and implementation records for maintainers.

The audit is grounded in the current source tree, examples, public API comments, tests, recent implementation history, and the Linear launch decisions in PAX-945, PAX-973, PAX-974, PAX-975, PAX-976, PAX-987, PAX-869, and parent PAX-860. Tickets and commits were treated as discovery leads, not proof on their own.

The governing launch claims are:

- Pax is a language-first GUI framework for Rust, with a declarative template language, reactive PAXEL expressions and properties, Rust application logic, a portable runtime, and a renderer.
- "Ready for builders" is a maturity transition, not a claim of universal production readiness or equal platform maturity.
- Current application targets are web, macOS, iOS, and iPadOS. Supported development workstations are a separate question and must be stated separately.
- The framework, compiler, runtime, CLI, hot reload, designtime protocols and implementations, source mapping, inspection, screenshots, and event-driving capability ship as open source. No companion product is required. A future companion would be additive.
- Exact performance, binary-size, accessibility, backend, and platform claims must be qualified and demonstrated rather than generalized.

## A. Current-state audit

### Public learning-path topology

The current `SUMMARY.md` is a flat sequence rather than a guided journey:

`Getting Started` -> language and state articles -> individual feature articles -> `Architecture` -> generated public and internal API pages.

That ordering has three consequences:

- A new reader reaches setup before receiving a concise answer to what Pax is, how its pieces fit, or why its constraints are useful.
- Strong reference-like chapters and very thin outline chapters appear as peers, so the navigation promises a more even level of completeness than exists.
- Public learning material, public API reference, and maintainer-facing internal APIs share the same top-level path. Design documents are correctly excluded from the navigation, but the internal generated API pages are not.

The current tree has no canonical public article for the complete developer workflow, platform support, build artifacts, deployment, practical runtime/performance behavior, limitations, or the shortest template-to-PAXEL-to-Rust learning loop. `performance-scale.md` is effectively empty and is not in `SUMMARY.md`.

### Article maturity, accuracy, and completeness

Coverage states below mean:

- **sufficient**: suitable as a launch anchor after normal verification and link cleanup;
- **revise**: the concept is substantially present but its framing or accuracy needs work;
- **expand**: a sound article needs important missing sections;
- **new subsection**: a bounded addition belongs in an existing article;
- **new article**: the current topology has no good owner;
- **defer**: useful material, but outside the launch spine.

| Current article | Current role and maturity | Accuracy, completeness, and link findings | Launch disposition |
| --- | --- | --- | --- |
| `getting-started.md` | Substantial workstation setup and CLI article. It also carries hot-reload configuration. | It does not introduce the project anatomy, guide the first edit, define a first-success checkpoint, or explain build output/deployment. Its generated-project assumptions may change under PAX-906. The active PAX-869 worktree adds web route metadata here, so it is read-only for this phase. | **Revise/split after coordination.** Let it own install/create/run; move the broader workflow to a dedicated article. |
| `template-language.md` | Short outline plus conditional `@settings` material. | Basic tree syntax, settings, selectors, IDs/classes, imports, event bindings, and z-order are named but not taught. The advanced conditional-settings section is more complete than the fundamentals. | **Expand.** Make this the canonical structural-language article. |
| `state-properties.md` | The strongest existing deep conceptual/API bridge. | It explains `Property<T>`, computed values, dependencies, subscriptions, and state patterns well, but assumes the reader already understands where properties sit in a Pax component and how templates see them. | **Revise lightly.** Add a short role/orientation section and link from the first guided app. Keep exact API detail here. |
| `data-binding-expressions.md` | Strong PAXEL reference covering bindings, operators, literals, units, colors/gradients, `$base`, and globals. | It is reference-shaped rather than tutorial-shaped. The launch path needs an earlier, smaller explanation of formula-style reactivity and of what PAXEL intentionally cannot do. | **Revise lightly plus add a conceptual on-ramp elsewhere.** |
| `event-handling-rust.md` | Brief outline with useful advanced coordinate, touch, and scroller notes and an embedded game example. | It lacks the basic handler signature, component/state relationship, reactive update loop, common event choices, and a compact click-to-property walkthrough. Advanced details arrive before the first success. | **Expand substantially.** |
| `control-flow.md` | Compact and useful syntax coverage for conditionals and keyed loops. | It needs one explanation of why keys matter, a link to component composition, and verification against current syntax. It need not become a large chapter. | **Revise lightly; keep or merge into a composition chapter.** |
| `components-composition.md` | Outline with relatively detailed slot/projection material. | It does not yet teach the `.rs`/`.pax` component pair, public fields, child composition, imports, defaults, or project organization. Slot detail is useful but out of sequence. | **Expand or merge with control flow.** |
| `routing.md` | Substantive routing guide with nesting, branches, route writes, and a working example. | It is one of the more launch-ready feature chapters. Route metadata is being added in the active PAX-869 worktree and must be semantically merged only after its prerequisite lands. | **Revise after coordination; otherwise keep.** |
| `layout-responsiveness.md` | Good autosize, padding, layout-role, and breakout-container material. | Core positioning, size constraints, alignment, transforms, and responsive composition are only outlined. Percentage `x`/`y` semantics are a recurring authoring surprise: they represent remaining travel after the element's own size, not raw parent coordinates. | **Expand substantially.** |
| `text-fonts-images.md` | Nine-line outline. | It does not teach text styles, wrapping, sizing, editing/input distinctions, fonts, or target caveats. It references examples that are not in the current tree. | **Merge into a launch visual-content article or write later as a deep article.** |
| `drawing-styling.md` | Nine-line outline. | It omits current vector/path, gradient, SVG import/ejection, and runtime drawing capabilities. | **New launch subsection in a visual-content article; deeper article later.** |
| `input-native-controls.md` | Detailed PhotoPicker section under a broad input heading. | Ordinary controls, two-way binding, focus, keyboard/pointer/touch input, sensors, native compositing, permissions, and the accessibility boundary are not coherently introduced. | **Restructure and expand.** Keep PhotoPicker as a platform-specific subsection or example. |
| `animation-motion.md` | Strong timeline, easing, enter/exit, interruption, and reflow material. | It is close to launch-ready after syntax/example verification and a simpler first example. It correctly names at least one limitation rather than implying complete transition coverage. | **Revise lightly; retain as canonical motion anchor.** |
| `compositing-effects.md` | Nine-line outline. | It does not explain canvas/native element islands, occlusion, masks, clipping, coordinate-space implications, or backend/platform caveats. | **New subsection/article within visual/native composition; avoid a stub nav item.** |
| `scrolling-viewports.md` | Short outline with autosize guidance. | It lacks a smallest working scroller, viewport/content sizing, events, native-element interaction, and performance/culling notes. It references a missing example. | **Expand or merge with input/native controls for launch.** |
| `architecture-runtime-cartridge.md` | Detailed kernel/cartridge implementation narrative. | It is too low-level for the core learning path, contains numerical size/performance examples that require current evidence, and mixes implemented behavior with older future-language and design-tool framing. Some material is valuable for maintainers and some can support a practical runtime explanation. | **Split/replace.** Write a builder-facing runtime/performance article and move or label the detailed architecture as an appendix. |
| `performance-scale.md` | One line, not navigated. | No usable content. | **Replace with a verified builder-facing runtime/performance article or remove the orphan during the later TOC edit.** |

### Cross-cutting duplication and missing connective tissue

The current articles duplicate concepts mostly by naming them in outlines, not by explaining them twice. The larger problem is that no article owns the connective model:

1. a `.pax` template declares content and structure;
2. PAXEL expressions read properties and derive values without side effects;
3. event bindings route interrupts to Rust;
4. Rust performs side effects and sets properties;
5. the reactive runtime updates affected content and rendering.

That loop should be stated once in `What is Pax?`, demonstrated once in the first guided interface, and then linked rather than re-explained in every reference chapter.

Other missing or weak links are:

- setup -> generated project anatomy -> first `.pax` edit;
- template bindings -> PAXEL reference -> `Property<T>` reference;
- event binding -> Rust handler -> state update;
- layout -> responsive settings -> autosize and breakpoint examples;
- native controls -> compositing and accessibility caveats;
- motion -> layout/reflow and interruption behavior;
- routing -> web route metadata and deployment fallback behavior;
- run/hot reload -> inspection, screenshots, logs, and event-driving workflow;
- targets -> renderer/backend behavior -> build artifacts -> deployment;
- prose concepts -> exact public API pages and runnable examples.

### Stale concepts and claims requiring correction or qualification

| Claim or framing | Finding | Editorial treatment |
| --- | --- | --- |
| Future JavaScript/TypeScript or Python application logic described alongside current Rust support | Future language support is not current launch behavior. | Keep out of the main learning path. If mentioned at all, label it explicitly as future direction. |
| Designer-era framing | The old designer was removed; current OSS designtime tooling is CLI/protocol/source-mapping/inspection infrastructure, not the retired product story. | Do not revive designer-era mental models. Historical design records stay internal. |
| Blanket GPU claim | WebGPU is used where available/policy permits; the web runtime falls back to Piet/CPU on iOS WebKit or when WebGPU is unavailable. Lighting/material behavior is not identical on Piet. | Describe backend policy precisely. Do not equate every target with the same GPU path. |
| Numerical FPS, binary-size, or "zero work" claims | The source shows targeted dirty propagation, culling, and no-compute-at-rest work, but launch-wide numeric guarantees are not established by the current docs. | Explain mechanisms and link to measurements when reproducible; omit blanket numbers. |
| Full accessibility | Native controls/text and image-alt foundations exist, while reading order, tab order, broader annotations, and audit coverage remain incomplete under PAX-958. | State the current foundation and limits. Do not claim complete accessibility. |
| Equal target maturity | Runtime targets are web, macOS, iOS, and iPadOS, but backend and feature coverage vary. Apple native builds also require an appropriate macOS/Xcode host. | Separate application targets, development workstations, and feature/backend caveats. |
| Companion dependency | PAX-973 confirms the current framework and developer tooling ship OSS and are self-contained. | Say this directly. Do not imply a companion is required. |
| "Production-ready" | The accepted launch position is "ready for builders," not universal production readiness. | Use the maturity-transition wording consistently. |
| Text/scrolling example links | `scroll-island-mini` and `marketing-site` are not present in the current examples tree. | Replace with verified examples or create intentional placeholders owned by PAX-976/PAX-855. |
| Event-driving command | PAX-973 and the README describe event driving as shipped, but the current `pax-cli dev` command list exposes look, logs, ray-cast, selector, inspect, and source mutation without an obvious click/touch command. | Treat the capability as OSS-shipped per the accepted boundary, but locate and verify its actual public surface before documenting an exact workflow. Escalate if the public command is absent. |

### Concrete command and example verification needs

PAX-909 provides strong clean-workstation evidence for the source-linked workflow. PAX-906 still owns the release candidate and generated starter behavior, so the final public-package commands cannot be frozen from the current tree alone.

| Journey step | Candidate public surface | What must be verified before prose is final | Ownership/dependency |
| --- | --- | --- | --- |
| Install | `cargo install pax-cli` with any documented prerequisites | Install the actual release candidate from the public crate path on clean macOS, Ubuntu, and Windows workstations. Record Rust/Node/npm/wasm-pack prerequisites and distinguish web development from native Apple builds. | PAX-906 release; PAX-909 test method |
| Create | `pax-cli create <name>` and any `--example` option | Verify the generated source, command output, directory layout, first route/component, dependency version, and whether PAX-906 changes the default to `router-playground`. | PAX-906 blocker for stable screenshots/prose |
| First run | `pax-cli run --target=web` | Verify blank-machine first build, browser launch behavior, actual URL, visible success state, warnings, stop/restart behavior, and the first `.pax` hot reload. | Launch RC |
| Native run | `--target=macos`, `--target=ios`, `--target=ipados` | Verify exact spelling, simulator/device selection, Xcode requirements, signing boundary, and whether each documented path is run or build-only. | Launch RC on macOS |
| Build | `pax-cli build --target=<target>` | Verify debug/release flags, output locations, app/bundle names, web asset layout, and the difference between run, build, and release cartridge behavior. | Source plus launch RC |
| Template hot reload | Default debug `all`; `--hot-reload=pax` | Exercise a visible `.pax` edit on all four targets. Confirm disabled-lane behavior. | Runtime-resilience implementation plus launch RC |
| Logic hot reload | Default debug `all`; `--hot-reload=logic` | Exercise a Rust logic edit on web and macOS. Confirm iOS/iPadOS require rebuild/relaunch for logic changes. | Runtime-resilience implementation plus launch RC |
| Inspect and capture | `pax-cli dev status`, `look`, `inspect`, `logs`, `ray-cast`, `selector` | Capture exact help text and one minimal builder workflow. Confirm session discovery and output formats without presenting internal protocols as required knowledge. | Current CLI |
| Drive events | Current OSS event-driving surface | Locate the supported public command/API, drive one click/touch, and capture its selector/coordinate semantics. If no public surface exists, make that a launch tooling issue instead of inventing syntax. | PAX-973 boundary; CLI verification gap |
| Local docs/examples | `pax-cli docs list/open/search/examples` | Verify command names, browser behavior, offline expectations, example discovery, and which docs are generated/reference-only. Avoid asking a reader to rebuild docs. | Current CLI |
| Representative examples | Small starter plus layout, routing, motion, drawing, input/compositing, and backend-caveat examples | Build only the curated launch set against debug and release paths, then link stable source/demo URLs. Do not make all 38 examples a launch gate. | PAX-976; deeper examples PAX-855 |
| Web deployment | Built web directory plus static-host configuration | Validate base path, asset URLs, route fallback, route metadata output, refresh/deep-link behavior, caching expectations, and at least one real static host recipe. | PAX-869 metadata prerequisite; PAX-987 publication is separate |
| Apple distribution boundary | Built `.app`/simulator artifact | Explain what Pax produces versus what Xcode/signing/App Store workflows still require. Verify paths and avoid promising a one-command store deployment. | Launch RC |

### Public prose, API reference, and internal design separation

| Layer | Reader need | Material that belongs here | Material that does not |
| --- | --- | --- | --- |
| Public learning path | Evaluate Pax, get a first success, build a coherent app, find deeper features, and ship to a supported target. | Concepts, guided examples, supported workflows, caveats, stable cross-links, and selected public API links. | Compiler internals, protocol schemas, unresolved design drafts, exhaustive method lists. |
| Public API reference | Confirm exact Rust types, properties, events, methods, and component APIs. | Generated `pax-runtime-api` and `pax-std` documentation, with public examples and source-linked signatures. | Tutorial ordering or internal crate APIs presented as required builder knowledge. |
| Maintainer appendix/reference | Understand cartridges, compiler/runtime boundaries, render backends, protocols, and architectural invariants. | Curated, status-labeled architecture chapters; generated internal APIs for `pax-language`, `pax-runtime`, `pax-manifest`, `pax-message`, and `pax-gpu`; selected implemented design records. | Drafts silently presented as product guarantees. |
| Internal design/spec archive | Preserve decisions, proposals, retrospectives, and implementation plans. | `book/src/design/*`, explicitly dated and status-labeled. | Primary feature-card links or first-touch learning-path destinations. |

Recommendation: keep the public `pax-runtime-api` and `pax-std` reference reachable from the learning path, but place internal generated APIs under an explicitly named Maintainer Reference/Appendix or remove them from the primary public navigation. Do not add the entire design directory to `SUMMARY.md`; curate only stable, implemented architecture explanations that answer a builder or maintainer question.

## B. Independent builder-facing concept and feature inventory

This inventory was produced before consulting the separately delegated PAX-869 gallery catalog. It optimizes for builder comprehension rather than marketing-card appeal.

Legend:

- Criticality: **Launch-critical**, **Useful**, or **Post-launch**.
- State: **Shipped**, **Limited**, **Experimental**, or **Future**.
- Evidence abbreviates repo-relative source, article, example, or issue locations; each claim still needs final release-candidate verification where noted.

| Working key / title | Category and why a builder needs it | Criticality; current state and caveats | Current evidence | Canonical destination and coverage state | First-touch / first-aha role | Example or media need |
| --- | --- | --- | --- | --- | --- | --- |
| `pax-category` / What Pax is | Positioning. Lets an evaluator place Pax among Rust GUI frameworks and understand the language/runtime combination. | **Launch-critical; shipped.** "Ready for builders," not blanket production readiness. | PAX-945; compiler/runtime/CLI crates; root README. | New `what-is-pax.md`; **new article**. | First-touch: primary. | One architecture loop diagram and one representative app image. |
| `oss-boundary` / Self-contained open-source stack | Trust and workflow. Clarifies that building, running, hot reload, inspection, and capture do not require a companion product. | **Launch-critical; shipped.** Future companion work is additive. | PAX-973 final/correction comments; repository source. | `what-is-pax.md#open-source-and-self-contained`; **new subsection**. | First-touch reassurance. | No separate demo; link GitHub/source map. |
| `authoring-loop` / Template -> PAXEL -> Rust -> reactive update | Mental model. Prevents imperative logic in templates and makes the framework's division of responsibility memorable. | **Launch-critical; shipped.** Rust is the current application language. | `template-language.md`, `data-binding-expressions.md`, `event-handling-rust.md`, `state-properties.md`; runtime property APIs. | `what-is-pax.md#the-pax-authoring-loop` plus first-app demonstration; **new subsection**. | First-touch and first-aha backbone. | Four-step diagram and a single counter/card walkthrough. |
| `project-anatomy` / Pax project and component pair | Fundamentals. Shows where Cargo, Rust state/handlers, `.pax` templates, assets, and metadata live. | **Launch-critical; shipped.** Generated starter is pending PAX-906 changes. | `pax-compiler/files/new-project/*`; examples. | `getting-started.md#project-anatomy`; **new subsection** after PAX-906. | First success. | Annotated file tree. |
| `cli-first-run` / Install, create, run | Workflow. Provides the shortest path to a visible running application. | **Launch-critical; shipped but release-sensitive.** Clean-host package flow must be rerun for the launch RC. | `pax-cli` commands; PAX-909; PAX-906; current `getting-started.md`. | `getting-started.md`; **revise**. | First success gate. | Terminal transcript and one expected-result screenshot. |
| `template-tree` / Declarative UI tree and settings | Language fundamentals. Teaches element creation, property settings, IDs/classes, imports, bindings, and source order. | **Launch-critical; shipped.** Pax z-order is visually top-to-bottom: elements earlier in source render above later siblings. | `template-language.md`; parser/tests; examples. | `template-language.md`; **expand**. | First guided edit. | Small layered card example and tree/source comparison. |
| `paxel` / Formula-style expressions | Reactivity. Teaches side-effect-free bindings, derived values, operators, units, globals, and `$base`. | **Launch-critical; shipped.** Keep arbitrary side effects in Rust. | `data-binding-expressions.md`; language parser/tests. | `data-binding-expressions.md`; **sufficient/revise** plus a short first-app use. | First-aha: direct manipulation of a property updates UI. | Live derived label/style example. |
| `properties` / Reactive application state | State. Shows how Rust values participate in the dependency graph and how computed/subscribed properties work. | **Launch-critical; shipped.** Avoid making subscriptions the beginner default. | `state-properties.md`; `pax-runtime-api` property docs. | `state-properties.md`; **sufficient/revise**. | First-aha bridge from handler to UI. | Counter/filter state progression. |
| `rust-logic` / Handlers, lifecycle, and side effects | Application logic. Shows where network, filesystem, platform, and mutation work belongs. | **Launch-critical; shipped.** Rust is the supported launch language. | `event-handling-rust.md`; component macros and examples. | `event-handling-rust.md`; **expand**. | Completes first-aha loop. | Click handler changes a property; one async/platform call can be later. |
| `events-input-model` / Events and coordinates | Interaction. Builders need pointer/touch/click semantics, local/window coordinates, propagation, and capture boundaries. | **Launch-critical basics; shipped with caveats.** `local_point` and native/canvas boundaries matter; some controls capture touch. | Event APIs; `event-handling-rust.md`; `examples/src/mouse-animation`, `occlusion`; pain-point records. | `event-handling-rust.md#events-and-coordinates`; **expand**. | First interaction, then deeper troubleshooting. | Coordinate overlay/ray-cast illustration. |
| `control-flow` / Conditionals and keyed loops | Structure. Enables data-driven trees without imperative template code. | **Launch-critical; shipped.** Keys must be explained as identity, not decoration. | `control-flow.md`; examples/parser tests. | `control-flow.md` or composition chapter; **revise**. | Small-list first app or next step. | Add/remove/reorder list demo. |
| `components-slots` / Reusable components and composition | Architecture. Teaches ownership, public properties, defaults, nesting, slots, and projection. | **Launch-critical basics; shipped.** Advanced slot semantics can remain reference material. | `components-composition.md`; slot examples; component macros. | `components-composition.md`; **expand**. | Moves reader from toy to app. | Reusable card/button with one named/default slot. |
| `layout-core` / Position, size, alignment, transforms | Layout. Builders need a reliable spatial model before styling and motion make sense. | **Launch-critical; shipped.** Percentage positioning semantics need explicit explanation. | `layout-responsiveness.md`; `pax-std` layout types; examples; pain-point record. | `layout-responsiveness.md#the-coordinate-and-size-model`; **expand**. | First visual composition. | Interactive percent/px diagram or responsive card. |
| `units-responsive` / Units, breakpoints, and conditional settings | Layout/language. Shows fluid sizing, unit arithmetic, and platform/viewport-specific choices. | **Launch-critical; shipped.** Prefer `%` for responsive sizing; conditional settings syntax needs current verification. | `layout-responsiveness.md`, `template-language.md`, `responsive-helpers`, `adaptive-cards`. | `layout-responsiveness.md#responsive-layout` with link to settings syntax; **expand**. | First-aha visual payoff. | Narrow/wide side-by-side screenshot or resize clip. |
| `autosize-padding` / Content-driven layout | Layout. Explains intrinsic measurement, padding, breakout, and parent/child responsibilities. | **Useful; shipped.** Native/text measurement can vary by target. | `layout-responsiveness.md`; `auto-sized-containers`; PAX-788. | `layout-responsiveness.md#content-driven-size`; **sufficient/revise**. | Deepens responsive UI. | Autosize/breakout comparison. |
| `style-themes` / Reusable visual settings | Styling. Enables coherent design systems rather than scattered literals. | **Useful, near launch-critical; shipped.** `ImportSettings` and defaults need a beginner-level path; selector grammar has limitations. | `runtime-settings-themes`; PAX-864; template/settings source. | New styling subsection in layout/visual article; **new subsection**. | First-aha polish. | Theme toggle or branded component set. |
| `text-assets` / Text, fonts, images, and assets | Content. Necessary for any nontrivial app and for understanding native/rendered boundaries. | **Launch-critical basics; shipped with target caveats.** Current text article is a stub; image alt is a partial accessibility foundation. | `pax-std` text/image APIs; examples; current `text-fonts-images.md`. | `text-fonts-images.md#text-images-and-assets`; **expand**. | First practical UI. | Font/image asset walkthrough and text wrapping screenshot. |
| `vectors-gradients` / Shapes, colors, and gradients | Visual expression. Gives builders Pax-native illustration and styling vocabulary. | **Useful; shipped.** Verify backend parity for specific effects. | drawing primitives; gradient implementation/tests; `pax-logo`, `color-picker`; PAX-949. | `drawing-styling.md#shapes-and-gradients`; **expand**. | First visual wow. | Gradient card/logo. |
| `paths-svg` / Paths, SVG workflows, and runtime drawing | Creative tooling. Supports custom shapes, illustration, and authored/dynamic paths. | **Useful; shipped.** SVG import/ejection and runtime `draw_start`/`draw_end` have distinct workflows. | path APIs; CLI `svg-import`; `path-drawing`; PAX-967. | `drawing-styling.md#paths-and-svg`; **new subsection**, deeper article post-launch. | Deeper discovery/wow. | Handwriter/path-drawing clip and small SVG pipeline. |
| `native-controls` / Native UI elements and two-way binding | App capability. Explains how forms/media/platform controls coexist with rendered content. | **Launch-critical basics; shipped with varying target coverage.** Do not imply every control exists on every target. | `pax-std` native controls; examples; PAX-932. | `input-native-controls.md`; **expand/restructure**. | Shows real-app ceiling. | Small form + PhotoPicker, with platform matrix. |
| `compositing` / Canvas and native element islands | Runtime/visual. Builders need to understand occlusion, transforms, clipping, and when a native underlay is required. | **Useful; shipped with constraints.** Cross-island ordering and mask composition are not arbitrary DOM/canvas layering. | platform runtimes; `occlusion`, `neon-opacity`, `liquid-glass`; current `compositing-effects.md`; pain-point records. | `compositing-effects.md#native-compositing`; **expand**. | Deeper feature discovery. | Layer-stack diagram and occlusion demo. |
| `scrolling` / Scroll containers and content sizing | Layout/input. Teaches viewport/content roles, events, and performance implications. | **Launch-critical basics; shipped.** Native child/touch behavior and autosize rules need explicit caveats. | Scroller APIs; `rounded-scroller-tiles`, `scroll-garden`, stress examples. | `scrolling-viewports.md`; **expand**, or merge for a bounded launch chapter. | First real app list/feed. | Small list + scroll-event example. |
| `motion-timeline` / Timelines and easing | Motion. Introduces declarative animation and visual feedback. | **Useful, high launch value; shipped.** Custom closure easing is not declarative template syntax. | `animation-motion.md`; `timeline-playground`, `pax-logo`, `marionette`. | `animation-motion.md#timelines-and-easing`; **sufficient/revise**. | First-aha/wow. | 5–10 second clip or interactive embedded example. |
| `motion-structural` / Enter, exit, reflow, and interruption | Motion. Lets state and route changes remain spatially legible. | **Useful; shipped with named limitations.** Verify supported reflow cases and interruption semantics; do not promise every Stacker transition. | `animation-motion.md`; `transition-grid`; PAX-946 and interruption work. | `animation-motion.md#structural-transitions`; **sufficient/revise**. | Deeper app polish. | Transition-grid clip. |
| `routing-history` / Routes, branches, and history | App architecture. Enables multi-screen applications and URL/history-aware state. | **Launch-critical; shipped.** Route topology has static requirements on web. | `routing.md`; `router-playground`; router source/tests. | `routing.md`; **sufficient/revise**. | Moves reader from component demo to app. | Two-route starter or focused router demo. |
| `route-metadata` / Web metadata and static route topology | Deployment/discovery. Makes routed web apps shareable and indexable. | **Useful; implementation active in PAX-869 worktree.** Literal metadata and hosting fallback requirements must land before canonical docs. | PAX-869 branch comments/diff; not yet current-tree behavior. | `routing.md#web-route-metadata` after merge; **new subsection**. | Ship-stage concern. | Inspect generated route files and page metadata. |
| `targets` / Application targets vs development hosts | Platform. Prevents "runs everywhere" ambiguity and directs platform setup. | **Launch-critical; shipped targets: web, macOS, iOS, iPadOS.** Clean workstation validation covers macOS/Linux/Windows for supported development workflows; native Apple builds require macOS/Xcode. | CLI `RunTarget`; platform crates; PAX-876; PAX-909; PAX-945. | New `targets-build-deploy.md#targets-and-workstations`; **new article**. | Evaluation and ship gate. | Compact support matrix. |
| `renderer-policy` / WebGPU and Piet fallback | Runtime/performance. Sets honest expectations for capabilities and backend differences. | **Launch-critical caveat; shipped.** WebGPU where available/policy permits; Piet/CPU fallback on iOS WebKit or no WebGPU; lighting/materials render unlit on Piet. | web renderer selection; GPU/Piet backends; PAX-960; PAX-945. | `runtime-performance.md#render-backends`; **new subsection**. | Evaluation/deeper discovery. | Renderer selection diagram; no blanket benchmark. |
| `hot-reload` / Template and logic lanes | Workflow. Provides the tight authoring loop and explains target differences. | **Launch-critical; shipped.** Template lane all targets; Rust logic lane web/macOS; iOS/iPadOS require rebuild/relaunch for logic changes; release disables both. | CLI/runtime hot-reload code; runtime resilience design/tests; `getting-started.md`. | New `developer-workflow.md#hot-reload`; **new article**. | First edit and sustained workflow. | Short edit/reload clip and target matrix. |
| `dev-inspection` / Inspect, look, logs, selectors, ray-cast | Workflow. Gives builders eyes into a running scene and reproducible visual diagnostics. | **Useful, high launch value; shipped OSS.** Exact command outputs and stability need verification. | `pax-cli dev`; PAX-973; README. | `developer-workflow.md#inspect-a-running-app`; **new subsection**. | First troubleshooting success. | Terminal + screenshot sequence. |
| `event-driving` / Script userland interaction | Workflow/testing. Enables repeatable click/touch-driven visual validation. | **Useful; accepted as shipped OSS, public surface unresolved in this audit.** | PAX-973; README/AGENTS claim; no obvious current `pax-cli dev` subcommand. | `developer-workflow.md#drive-events` only after verification; **new subsection/verification blocker**. | Deeper testing, not first run. | One reproducible click/touch sequence. |
| `local-docs-examples` / Discover docs and examples from the CLI | Workflow. Lets builders explore without already knowing file paths or article names. | **Useful; shipped.** Generated API/internal result labeling must be clear. | `pax-cli docs`; docs index generator; example inventory. | `developer-workflow.md#local-docs-and-examples`; **new subsection**. | Discovery after first success. | Short command table, no media required. |
| `build-cartridge` / Debug and release program representations | Shipping/runtime. Explains why release behavior differs and what the build produces. | **Launch-critical basics; shipped.** Rich manifests and baked release cartridges must remain behaviorally aligned. | compiler cartridge generation; `pax-manifest` binary/program IR/rust manifest; architecture article/tests. | `targets-build-deploy.md#build-modes` with deeper maintainer appendix; **new subsection**. | Ship gate. | Debug/release flow diagram. |
| `deploy-web` / Host a web build | Deployment. A launch path is incomplete if it stops at local run. | **Launch-critical; shipped build output, documentation missing.** Route fallback/metadata and base-path details need verified recipes. | CLI build output; PAX-869 route work; PAX-987 publishing scripts as implementation reference only. | `targets-build-deploy.md#deploy-web`; **new subsection**. | Final journey step. | One static-host recipe and output tree. |
| `ship-apple` / Apple build and distribution boundary | Deployment. Clarifies what Pax automates and what Xcode/signing still owns. | **Launch-critical; shipped build/run targets with external platform requirements.** Store release is not a one-command Pax guarantee. | Apple chassis/CLI build paths; PAX-876. | `targets-build-deploy.md#build-for-apple-platforms`; **new subsection**. | Final journey step. | Build artifact screenshot/path and signing boundary note. |
| `runtime-performance` / Reactive work, culling, and render scheduling | Architecture/performance. Helps builders reason about cost without folklore or unsupported numbers. | **Useful, launch-critical for honest evaluation; shipped mechanisms.** Results depend on workload/backend/device. | runtime dirty propagation; culling; PAX-588/PAX-851; architecture source/tests. | New `runtime-performance.md`; **new article**. | Evaluation/deeper learning. | Mechanism diagram plus reproducible profiling recipe later. |
| `materials-lighting` / LightFrame and GPU materials | Advanced visual capability. Demonstrates the creative ceiling. | **Useful; limited.** WebGPU/WGSL path, maximum/current light constraints, Piet unlit; maps, shadows, 3D, and cameras are not current. | `materials`, `glow-buttons`; PAX-960/PAX-966; GPU source. | `drawing-styling.md#lighting-and-materials` as caveated discovery link; deeper **defer**. | Visual wow, not first-touch dependency. | Glow/material clip with backend label. |
| `liquid-glass` / Platform-specific visual material | Advanced native visual capability. Shows Apple-platform integration. | **Useful; limited/platform-specific.** Must not be presented as portable parity. | `liquid-glass`; platform component source. | Platform notes under visual/native composition; **new subsection or defer**. | Gallery/deeper discovery. | Apple-only demo with explicit badge. |
| `accessibility-status` / Current accessibility foundation | Quality/platform. Builders need to know present support and unresolved responsibilities. | **Launch-critical caveat; limited.** Foundations exist; reading/tab order, annotations, and full audits remain incomplete. | PAX-958; native/text/image APIs. | `input-native-controls.md#accessibility-status` and targets/status links; **new subsection**. | Evaluation and implementation caveat. | Compact current/limited matrix; no aspirational badge. |
| `future-platforms-languages` / Explicit non-goals for this launch | Scope. Prevents old architectural aspirations from being read as shipped behavior. | **Post-launch; future.** Windows/Linux/Android application targets and JS/Python application logic are not launch claims. | PAX-945 claims ledger; architecture history. | Optional roadmap link outside core docs; **defer**. | Not part of canonical journey. | None. |

## C. Proposed launch information architecture

### Design principles

The launch path should be tutorial-first, reference-connected, and intentionally shallow where a deep article would delay the spine. Each important concept gets one canonical owner and all other mentions use a short recap plus a deliberate "Read more" link. Article titles should name a builder task or mental model, not an internal crate.

Stable section anchors should be treated as public interfaces because the README and website cards will link to them. Prefer descriptive slugs and durable headings such as `#responsive-layout`, `#paths-and-svg`, `#native-controls`, and `#hot-reload`; avoid anchoring launch links to example names or temporary implementation labels.

### Proposed public path

The rows below are editorial units. A row may retain two current files when those files are already strong references, but it has one clear place in the journey.

| Proposed article | Purpose and reader outcome | Prerequisites and read-more relations | Owned concepts vs linked concepts | Current file action | Criticality | Example/media placeholder | Verification burden |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **What is Pax?** (`what-is-pax.md`) | Let a Rust developer decide whether Pax fits, then leave with the authoring loop, runtime/target boundary, OSS boundary, and honest maturity framing. | None. Read more to Getting Started, runtime/performance, targets, and GitHub. | Owns category, audience, template/PAXEL/Rust loop, portable-runtime summary, ready-for-builders wording, self-contained OSS statement. Links all feature detail. | **New.** Do not repurpose the internal architecture article. | Launch-critical | One loop diagram; one representative hero application image owned with PAX-976/PAX-869. | Claims-ledger review against PAX-945/PAX-973 and current target source. |
| **Install, create, and run** (`getting-started.md`) | Produce a visible local app from a clean workstation and identify the generated project files. It is the standalone destination for the primary Get Started CTA on the website, README, and other external surfaces. | **No reading prerequisite.** It may link back to What is Pax? for context and forward to First Pax Interface, Developer Workflow, and Targets/Build/Deploy. | Owns prerequisites, install, create, first web run, expected result, project anatomy, common setup failures. It includes only the minimum mental model needed to act and links rather than embedding full hot-reload/tooling docs. | **Keep and refocus after PAX-906/PAX-869 coordination.** Route metadata content may instead live under Routing/Deploy. | Launch-critical | Exact terminal transcript, generated tree, first-run screenshot. | Highest: clean macOS/Ubuntu/Windows RC install; final generated starter; actual first warnings/output. |
| **Build your first Pax interface** (`first-pax-interface.md`) | Guide one coherent edit from template content through a PAXEL binding and Rust event handler, then make it responsive and add one small motion affordance. | Getting Started. Read more to Templates, PAXEL/Properties, Events, Layout, and Motion. | Owns only the end-to-end guided experience and checkpoints. Deep syntax belongs to linked chapters. | **New.** Its source must match the final generated starter or an explicitly bundled example. | Launch-critical | Before/after screenshots and an optional 15-second interaction clip. | Build/run every snippet; hot reload template and logic paths; narrow/wide layout. |
| **Templates and UI structure** (`template-language.md`) | Teach how a Pax tree is declared, configured, selected, imported, layered, and connected to events. | First interface. Read more to Components/Control Flow, Layout, PAXEL, and Events. | Owns template grammar at builder level, `@settings`, IDs/classes, element ordering/z-order, bindings/imports. Links value semantics and handlers. | **Keep; expand/reorder from fundamentals to conditional settings.** | Launch-critical | Source/tree/layer diagram and tiny layered component. | Parser/examples for every syntax form; confirm selector and conditional-settings limits. |
| **Reactive values: PAXEL and Properties** (`data-binding-expressions.md` + `state-properties.md`) | Make the reactive spreadsheet-like model precise, from a simple binding through computed Rust properties and subscriptions. | First interface and Templates. Read more to Events and public property API. | PAXEL file owns expression syntax and units; Properties file owns Rust reactive data. A short shared overview states the boundary once. | **Keep both strong files; add mutual orientation and ordering.** Do not merge them into an unwieldy article. | Launch-critical | One dependency graph and one computed-value example. | Compile/evaluate all expression samples; verify API names and generated links. |
| **Events and Rust application logic** (`event-handling-rust.md`) | Teach handler wiring, event data, state mutation, lifecycle, side effects, and coordinate/capture basics. | First interface, Templates, Properties. Read more to Input/Native Controls and public event APIs. | Owns Rust-side response and event model. Links routing writes, animation triggers, and platform-specific controls. | **Keep; rewrite sequence and expand.** | Launch-critical | Click-to-property walkthrough and coordinate overlay. | Compile every handler; run pointer and touch path; verify local/window coordinates and propagation wording. |
| **Components, conditionals, and lists** (`components-composition.md`, with `control-flow.md` retained or merged) | Help the reader turn a single view into reusable components and data-driven structure. | Templates and reactive values. Read more to Layout and Routing. | Owns component pair, public inputs/defaults, nesting, slots, conditional trees, keyed identity. Links Rust state implementation. | **Expand `components-composition.md`; either keep `control-flow.md` as a compact linked reference or merge after outline review.** | Launch-critical | Reusable card/list with insert/reorder; slot example. | Build all variants and verify key/slot behavior. |
| **Layout, styling, and responsiveness** (`layout-responsiveness.md`) | Give a dependable spatial model and produce a polished narrow/wide interface using units, alignment, autosize, themes, and conditional settings. | Templates. Read more to Visual Content, Scrolling, and Motion. | Owns geometry, percentage semantics, units, responsive policy, autosize, padding, layout role, and basic styling/theme composition. Links drawing details. | **Keep; expand and retitle if needed.** | Launch-critical | Responsive side-by-side, percent-position diagram, autosize demo. | Run at several viewport sizes/targets; validate percentage and text/native measurement caveats. |
| **Text, fonts, and images** (`text-fonts-images.md`) | Teach common content, asset loading, text/image behavior, and the rendered/native distinction without requiring API archaeology. | Layout. Read more to Input/Accessibility and Compositing. | Owns text layout/style, fonts/assets, `Image`/`NativeImage`, selection/editability boundaries, and alternatives/format caveats. | **Keep and expand; recommended pending final Checkpoint 1 confirmation.** | Launch-critical basics | A focused shared visual-example component with source. | Verify assets/fonts, wrapping/selection/editability, images, alternatives, and target differences. |
| **Drawing and styling** (`drawing-styling.md`) | Introduce Pax's ordinary visual vocabulary, then provide durable discovery anchors for paths/SVG and qualified materials. | PAXEL and Layout. Read more to Motion and Compositing. | Owns primitives, fills/strokes, colors/gradients, themes, paths/SVG/draw ranges, and limited lighting/materials. | **Keep and expand; recommended pending final Checkpoint 1 confirmation.** | Useful, high launch/card value | Shared drawing example; path animation and labeled GlowButton proof. | Verify syntax, SVG subset/warnings, path ranges, theme behavior, and WGPU/Piet constraints. |
| **Compositing and effects** (`compositing-effects.md`) | Explain how rendered and native content layer, clip, mask, and occlude so builders can predict cross-surface results. | Layout and Drawing. Read more to Input/Native Controls and Runtime/Performance. | Owns opacity, masks, clipping, native islands/occlusion, and target/backend constraints. | **Keep and expand; recommended pending final Checkpoint 1 confirmation.** | Useful, high launch/card value | Shared compositing example, layer diagram, and native occlusion proof. | Verify mask contract, nesting, z-order, native projection, coordinate/target caveats. |
| **Input, native controls, and accessibility** (`input-native-controls.md`) | Show how rendered UI receives input and how native controls participate, including platform coverage and accessibility limits. | Events and Layout. Read more to Scrolling and Targets. | Owns ordinary controls, two-way bindings, focus/keyboard/pointer/touch overview, PhotoPicker as an example, native composition boundary, and accessibility status. | **Keep; restructure around basics rather than PhotoPicker.** | Launch-critical basics | Small form, PhotoPicker inset, control/platform matrix. | Exercise representative controls on each claimed target; audit accessibility wording with PAX-958. |
| **Scrolling collections** (`scrolling-viewports.md`) | Build a correctly sized scrollable list and understand viewport/content roles, events, native children, and culling implications. | Layout, Components/Lists, Events. Read more to Runtime/Performance. | Owns scroller construction and scroll-specific behavior. Links generic layout/events/performance. | **Keep and expand** if it can reach useful depth; otherwise merge into Layout for the launch spine and defer the standalone chapter. | Launch-critical for practical apps | Minimal list and one richer `scroll-garden`/tiles example. | Web and Apple smoke tests, touch capture, native-child clipping, content sizing. |
| **Routing and multi-screen apps** (`routing.md`) | Build a nested multi-screen app, write routes, understand history, and prepare routed web output. | Components and Events. Read more to Motion and Deploy. | Owns route hierarchy/history/branches and, after landing, literal web route metadata/static topology. Links hosting fallback to deployment. | **Keep; semantic merge with PAX-869 only after route-metadata prerequisite lands.** | Launch-critical | Focused two-route example or `router-playground` excerpt; route-flow diagram. | Router tests/example; browser history/deep links; metadata/static files after merge. |
| **Motion and transitions** (`animation-motion.md`) | Add a first timeline and understand easing, structural enter/exit/reflow, and interruption behavior. | Properties, Layout, and Events. Read more to Routing for screen transitions. | Owns declarative motion semantics and limitations. Links layout/state triggers. | **Keep; simplify the opening and verify later sections.** | Useful, high launch value | Small first timeline plus transition-grid clip. | Run all key syntax and interruption/reflow cases on representative backends. |
| **Developer workflow and tools** (`developer-workflow.md`) | Turn first-run success into a sustainable edit/inspect/capture/debug loop using hot reload, local docs/examples, logs, screenshots, and inspection. | Getting Started. Read more to Targets/Build and troubleshooting references. | Owns hot-reload lane matrix and `pax-cli dev/docs` workflows. Links source mapping/API internals only for maintainers. | **New.** Remove the long workflow digression from Getting Started after coordination. | Launch-critical | Hot-reload clip, `look` result, short command table. | Exact help/output verification; all-target template reload; web/macOS logic reload; locate public event-driving surface. |
| **Targets, build, and deployment** (`targets-build-deploy.md`) | Let a builder choose a target, distinguish host requirements, produce debug/release artifacts, deploy web output, and understand the Apple signing/distribution handoff. | Getting Started; Routing for routed web apps. Read more to Runtime/Performance. | Owns exact application targets, workstation matrix, build modes/output, release cartridges at a practical level, web hosting, and Apple boundary. Links internals to appendix. | **New.** | Launch-critical | Target/host matrix, build-output tree, one web deployment recipe. | Highest: launch RC on all three workstation OSes; all targets on macOS; route fallback/metadata; artifact paths. |
| **How Pax runs: runtime and performance** (`runtime-performance.md`) | Explain reactive invalidation, rendering/compositing, culling, backend selection, and performance reasoning without unsupported promises. | What is Pax and Layout. Read more to Maintainer Architecture and profiling APIs/tools. | Owns builder-facing runtime model, WebGPU/Piet policy, performance mechanisms, measurement guidance, and honest variability. | **New, replacing the empty performance page. Split useful sections out of `architecture-runtime-cartridge.md`; keep implementation detail in appendix.** | Useful; launch-critical for accurate evaluation | Reactive/render flow and backend-selection diagrams; reproducible profile recipe later. | Source/test audit; representative workload measurement if any numbers are retained. |
| **Public API Reference** | Let builders inspect exact supported Rust/component APIs after learning the concepts. | Any conceptual article. Links back to prose examples. | Owns generated `pax-runtime-api` and `pax-std` signatures. | **Keep generated, but group as Reference and improve cross-links/labels.** | Launch-critical reference | Generated examples only where maintained. | Regenerate and link-check; sample signature accuracy. |
| **Maintainer architecture and internal API** | Preserve compiler/runtime/cartridge/backend detail without interrupting the builder journey. | Runtime/Performance. | Owns internal crate APIs and curated, status-labeled architecture documents. | **Move/present as an Appendix or separate Maintainer Reference. Do not expose all design drafts as product docs.** | Useful, not on canonical launch journey | Architecture diagrams only where current. | Verify implemented-vs-proposed status and binary-baking coverage. |

### Anchor plan for cross-surface links

Before README and website links are finalized, reserve these canonical destinations:

| Capability link | Proposed canonical destination |
| --- | --- |
| Language-first mental model | `/what-is-pax/#the-pax-authoring-loop` |
| First project | `/getting-started/` |
| PAXEL and reactive bindings | `/data-binding-expressions/#reactive-bindings` |
| Rust properties | `/state-properties/#properties-and-computed-values` |
| Responsive layout | `/layout-responsiveness/#responsive-layout` |
| Components and keyed lists | `/components-composition/#data-driven-components` |
| Routing and history | `/routing/#routes-and-history` |
| Timelines and structural motion | `/animation-motion/#structural-transitions` |
| Text, fonts, and images | `/text-fonts-images/` |
| Paths and SVG | `/drawing-styling/#paths-and-svg` |
| Lighting and materials | `/drawing-styling/#lighting-and-materials` |
| Masks and native compositing | `/compositing-effects/#native-compositing` |
| Native controls and compositing | `/input-native-controls/#native-controls-and-compositing` |
| Hot reload and inspection | `/developer-workflow/#hot-reload` and `/developer-workflow/#inspect-a-running-app` |
| Targets and deployment | `/targets-build-deploy/#targets-and-workstations` and `/targets-build-deploy/#deploy-web` |
| Runtime and renderer behavior | `/runtime-performance/#render-backends` |

These are proposed contracts, not current URLs. Final slugs and heading text should be approved before prose so PAX-974, PAX-869, and PAX-987 can link once and avoid churn.

## D. Canonical launch journey

The shortest navigable path should be seven visible successes, not an exhaustive reading assignment:

1. **Place Pax.** Read `What is Pax?` and understand that a Pax app combines declarative `.pax` templates, formula-style PAXEL/reactive properties, Rust logic, and a portable runtime targeting web and Apple platforms.
2. **See it run.** Install the launch CLI, create the final starter, run it on web, and confirm the expected screen. The page shows the generated file tree and one recovery path for common setup failures.
3. **Complete the authoring loop.** Change template content, bind it to a property with PAXEL, handle a click in Rust, set the property, and watch the UI update. Use hot reload where the target supports the changed lane.
4. **Make it a real interface.** Extract a small component, render a keyed list or conditional region, then make the layout respond at narrow and wide sizes. This is the first practical aha: the same declarative model covers structure, state, and responsive visual behavior.
5. **Choose a deeper feature.** Follow deliberate links to routing, motion, drawing/paths, native controls, scrolling, or inspection. The guided app uses only one small motion flourish; feature chapters provide breadth without blocking the core path.
6. **Form an app.** Add or inspect a second route, understand history and route-specific web considerations, and confirm the target/backend caveats relevant to the chosen feature set.
7. **Build and ship.** Choose web, macOS, iOS, or iPadOS; distinguish the development host requirements; build the appropriate debug/release artifact; deploy the web output or hand the Apple artifact to the normal signing/distribution workflow.

At each step the next link should be explicit. A reader should never need the generated API reference to finish steps 1–4, but each conceptual chapter should link the exact public API when the reader is ready to generalize.

### Candidate first guided application

The guided application should be small enough to explain line by line and rich enough to exercise the full authoring loop. It should also be the actual source copied by the default `pax-cli create`, so the README excerpt, Getting Started screenshot, first tutorial, generated files, and first support questions all describe the same program.

The current evidence creates a useful spectrum:

- PAX-974's README drafts a clickable square/counter that cleanly demonstrates template -> PAXEL -> Rust handler -> property -> reactive update.
- `examples/src/increment` is similarly compact, but its pre-render tick loop, remote font, continually changing color, and animation make the smallest program noisier than necessary.
- `examples/src/router-playground` is a strong feature example but has many components, nested routers, responsive chrome, stores, and several hundred lines of route/motion content before dependencies are counted. It teaches routing before it teaches Pax.
- the current `examples/src/starter-project` is a much larger multi-view/calculator artifact and is not a clean launch starter.
- a copy of the launch website would maximize first-frame polish but would couple first touch to a large, fast-changing, routed content application and make the primary tutorial circular.

| Starter option | First impression | Learning and maintenance cost | Recommendation |
| --- | --- | --- | --- |
| Copy of the launch website | Strongest visual proof and direct brand continuity. | Very high complexity; circular website/docs dependency; content, routing, metadata, and launch-specific assets obscure the language model. | Do not use as the default. It can remain a curated advanced example if its source is suitable for copying. |
| Existing feature example such as `router-playground` | Demonstrates a credible multi-screen app immediately. | The reader opens a large tree before learning the component pair or reactive loop. Changes to the feature example destabilize setup docs. | Offer through `--example`, not as the default. |
| Any example selected by name | Excellent exploration and reuse; matches the direction already written into PAX-906. | The repository also contains stress tests, labs, implementation probes, and platform-specific examples that should not all become public starter contracts. | Ship a selection mechanism, but expose a curated, tested catalog and make unsupported/internal examples unavailable or clearly advanced. |
| Minimal clickable square | Clearest possible authoring loop and already aligns with the README draft. | Risks reading as visually generic and does not show component structure unless expanded. | Keep its conceptual core, not necessarily its exact presentation. |
| Purpose-built polished starter | Can show the same five-step loop with a distinctive first frame, responsive composition, and one secondary component. | Requires a small new/renamed canonical example and coordinated ownership across PAX-906/PAX-974/PAX-975/PAX-976. | **Recommended default.** Treat it as a product surface, not a disposable sample. |

Recommended starter contract:

1. A polished interactive card/tile rather than a bare square, visible without explanation and attractive on desktop and phone sizes.
2. One main component plus one small secondary component, each with a `.rs`/`.pax` pair, so the generated file tree teaches composition without becoming an application architecture lesson.
3. One `Property<usize>` or small enum, one click handler, and two or three plainly readable PAXEL derivations for label, color, position, or rotation.
4. At most one subtle eased response. No router, store, lifecycle tick loop, network font, native control, backend-limited effect, or platform permission in the default.
5. A bounded source target: roughly one screen of Rust and one to two screens of Pax per component. Every line should be explainable in the first tutorial.
6. The README uses a verified excerpt from this source; Getting Started runs it unchanged; First Pax Interface edits it; PAX-976 may show it but remains free to choose a more ambitious hero example.
7. `pax-cli create <name> --example=<curated-name>` provides opt-in examples such as routing, fireworks, responsive layout, or paths. A `--list-examples` surface or equivalent should print stable names, short descriptions, target caveats, and which one is the default.

This recommendation changes PAX-906's proposed default away from the current `router-playground`, so it requires an explicit cross-ticket decision before implementation. The documentation should not freeze screenshots, paths, or prose until that decision lands.

## E. Sequential editorial plan

PAX-860's editorial process is part of the acceptance criteria: outline/research first, Zack feedback, then one chapter at a time. No parallel chapter delegation is appropriate because the terminology and examples must converge sequentially.

### Checkpoint 1: approve the spine before prose

Review this coverage map and decide:

- the split between `What is Pax?`, setup, and the first guided interface;
- the final generated starter/tutorial relationship with PAX-906;
- whether visual-content stubs become one bounded launch article;
- whether control flow remains a compact separate reference or joins Components;
- whether internal generated APIs move to a maintainer appendix or leave the public nav;
- the stable slug/anchor contracts required by PAX-974 and PAX-869.

No existing prose or `SUMMARY.md` should change before this checkpoint, except a narrowly necessary factual correction approved to avoid publishing a false claim.

### Checkpoint 2: establish first-touch and terminology

Write and review, one at a time:

1. `What is Pax?`
2. `Install, create, and run`
3. `Build your first Pax interface`

For each article: agree on a short outline, select/verify its example, draft prose, run the commands/snippets, then request feedback before proceeding. After these three, check that the same terms and authoring loop agree with the README direction from PAX-974 and the accepted PAX-945 claims ledger.

### Checkpoint 3: complete the authoring model

Proceed one article at a time:

4. Templates and UI structure
5. PAXEL expressions
6. Properties and application state
7. Events and Rust application logic
8. Components, conditionals, and lists
9. Layout, styling, and responsiveness

Bounded verification: compile every code sample; run one common guided project instead of inventing six disconnected mini-apps; exercise wide/narrow layouts and click/touch paths; link exact public APIs. Review the complete template/PAXEL/Rust loop with Zack before branching into feature breadth.

### Checkpoint 4: cover app and creative feature areas

Proceed one article at a time:

10. Routing and multi-screen apps, incorporating PAX-869 route metadata only after its prerequisite lands
11. Motion and transitions
12. Input, native controls, and accessibility
13. Scrolling collections
14. Text, fonts, and images
15. Drawing and styling
16. Compositing and effects

Bounded verification: select the smallest representative set from PAX-976, build each against the current debug and release paths, and capture only media that has a stable source and platform label. Deeper per-feature examples stay with PAX-855.

### Checkpoint 5: make the workflow shippable

Proceed one article at a time:

17. Developer workflow and tools
18. Targets, build, and deployment
19. How Pax runs: runtime and performance
20. Public API/Maintainer Appendix navigation and status labeling

Bounded verification:

- rerun the public install/create/run flow on clean macOS, Ubuntu, and Windows against the release candidate;
- run or build web, macOS, iOS, and iPadOS on the supported host, recording exact artifacts and external requirements;
- verify template hot reload on all targets and Rust logic hot reload on web/macOS;
- verify inspection, screenshots, logs, local docs/examples, and the actual event-driving surface;
- validate one routed static web deployment, including refresh/fallback and metadata behavior;
- regenerate API docs and run link/navigation checks;
- avoid making all examples, all design documents, or publication infrastructure part of this gate.

PAX-987 owns versioned publication/upload. This plan produces a verified source tree and local documentation build only; it does not publish.

### Final editorial pass

After the chapters are individually accepted:

1. edit `SUMMARY.md` once to express the approved Start/Core/App/Ship/Reference hierarchy;
2. remove dead example references and duplicate introductions;
3. check every README/website/API cross-link and stable anchor;
4. ensure platform, backend, accessibility, performance, maturity, and OSS caveats use the accepted vocabulary;
5. run the bounded command/example/link matrix;
6. only then move PAX-975 to In Review and leave the concise Linear update requested by the issue.

## F. Known overlap and gallery synthesis

### Active overlap that remains read-only in phase one

The active PAX-869 main worktree at `/Users/zack/.codex/worktrees/1b54/pax` currently changes these public-doc files:

| File | Observed work | PAX-975 phase-one policy |
| --- | --- | --- |
| `pax-docs/book/src/getting-started.md` | Adds web-site/route metadata configuration. | Audit only. Do not edit until the prerequisite lands or Zack coordinates the semantic merge. |
| `pax-docs/book/src/routing.md` | Adds literal route metadata, generated static route files, dynamic head behavior, and hosting fallback guidance. | Audit only. Reserve the Routing/Deploy ownership decision; do not copy the branch text. |
| `pax-docs/book/src/design/pain-points.md` | Records new authoring/tooling pain points, including percentage positioning and settings-selector issues. | Audit only. Do not add phase-one notes despite the standing AGENTS.md practice. Revisit after overlap clears. |

The PAX-974 README rewrite is also not current-tree prose. Its proposed `/get-started/` link strengthens the need to approve stable docs slugs before merging either surface.

### Ensemble method and result

The independent inventory above was completed before reading the PAX-869 artifact at `pax-docs/book/src/design/website-feature-gallery-catalog.md` in its delegated worktree. That catalog was then compared by capability, state, evidence, caveat, proof asset, and destination. Its twelve-card shortlist aligns unusually well with the docs inventory: all twelve are real builder-facing capabilities, not merely internal mechanisms. The differences are mostly about article ownership and whether an advanced visual capability belongs in the launch learning path.

### Intersection: high-confidence launch priorities and final destinations

The final destination is expressed as a source file and stable heading contract. PAX-987 still needs to decide whether the published URL ends in `.html`, uses clean paths, or provides both; card code should consume the publication-generated URL rather than guessing from the source filename.

| Gallery rank and card | Matching docs concept | Recommended canonical destination | Launch docs gap that must close | Ensemble recommendation |
| --- | --- | --- | --- | --- |
| 1. Templates for interfaces. Rust for application logic. | `authoring-loop`, `template-tree`, `rust-logic` | `what-is-pax.md#the-pax-authoring-loop`, with the guided example linked immediately | New mental-model article and first guided interface; expand template/event fundamentals | **Keep.** This is the clearest shared lead capability. Prefer the mental-model anchor over sending first-touch traffic directly into grammar reference. |
| 2. One interface model, four current targets | `targets`, `renderer-policy` | `targets-build-deploy.md#targets-and-workstations` | New target/host matrix, backend caveats, and build/deploy article | **Keep.** Name web, macOS, iOS, and iPadOS every time; do not imply equal maturity or native Windows/Linux. |
| 3. Reactive expressions with real units | `paxel`, `properties`, `units-responsive` | `data-binding-expressions.md#reactive-bindings`, with a sibling link to `#literals` and Properties | Add tutorial on-ramp and stable headings; verify operators/units | **Keep.** The gallery's PAXEL-plus-units decomposition is visually legible and technically sound. |
| 4. Native controls inside a rendered scene | `native-controls`, `compositing`, `accessibility-status` | `input-native-controls.md#native-controls-and-compositing` | Rewrite the current PhotoPicker-first chapter; add control/target matrix, compositing explanation, and accessibility boundary | **Keep.** The card's qualification must travel with it. |
| 5. Motion belongs to the component | `motion-timeline`, `motion-structural` | `animation-motion.md#structural-transitions`, with the timeline introduction linked above it | Simplify the first example and reverify enter/exit/interruption/reflow limitations | **Keep.** `transition-grid` is strong current proof; no FPS language. |
| 6. Responsive geometry without a CSS detour | `layout-core`, `units-responsive` | `layout-responsiveness.md#responsive-layout` | Add core geometry and percentage-position semantics; verify responsive settings | **Keep.** Preserve the gallery's "not CSS compatibility" qualification. |
| 7. Layout that measures its content | `autosize-padding` | `layout-responsiveness.md#autosize` | Existing section is strong; add dependency-cycle caveat and link from responsive fundamentals | **Keep.** This is high-confidence, useful, and already near a launch-grade destination. |
| 8. Paths that draw themselves | `paths-svg`, `motion-timeline` | `drawing-styling.md#paths-and-svg` | No narrative path/SVG guide; document subset import, warnings, draw ranges, and explicit non-capabilities | **Keep if the narrative anchor lands.** Do not leave the card pointed only at generated API docs at launch. |
| 9. Scoped light for vector materials | `materials-lighting`, `renderer-policy` | `drawing-styling.md#lighting-and-materials` | Add a concise builder guide and WGPU/Piet matrix, eight-light limit, ambient/layer behavior, and explicit deferrals | **Keep conditionally.** It is the strongest advanced-rendering proof, but must remain `Limited` and should be replaced by an overflow card if the guide or labeled media misses the launch gate. |
| 10. Routing stays declarative | `routing-history` | `routing.md#routes-and-history` | Current article is solid; merge route-metadata material after its prerequisite and verify deep-link/history behavior | **Keep.** Exclude experimental RouteCard gesture claims. |
| 11. Hot reload, in two explicit lanes | `hot-reload` | `developer-workflow.md#hot-reload` | New workflow article and all-target lane verification | **Keep.** The two-lane caveat is part of the value, not footnote noise. |
| 12. Inspect the interface you are running | `dev-inspection`, plus gallery-specific `ExampleHost` proof | `developer-workflow.md#inspect-a-running-app`, with a link to the `ExampleHost` public API/example | New tooling guide; distinguish CLI scene inspection from ExampleHost's explicit source manifest and from source mutation | **Keep if simplified.** One card currently combines two related but distinct workflows; the docs anchor must make their boundary obvious. |

### Website-only or visually decomposed capabilities

These are not missing from the technical model, but the gallery treats them as more specific visual stories than the launch spine needs as independent chapters:

| Gallery capability | Docs interpretation | Recommendation |
| --- | --- | --- |
| `LightFrame` as a distinct visual story | Part of the broader limited lighting/materials concept. | Keep the specific GlowButton proof on the card; teach it as a subsection, not a top-level learning-path article. |
| `ExampleHost` source drawer | A public component/example that supports inspectable demos, not the general scene-inspection mechanism. | Document authoring in Developer Workflow and link its API. Preserve the explicit-manifest qualification. |
| Handwriter single-line text | A limited path/text specialization. | Use as path-drawing media or an advanced subsection; do not create a launch chapter or default card. |
| Carousel/scroll snapping | A useful scrolling specialization. | Add to the expanded Scrolling article if verified; keep as overflow while narrative docs are thin. |
| PhotoPicker, sensors, and Liquid Glass | Narrow or platform-dependent examples of input/native integration. | Use as proof inside platform-labeled sections. Keep as overflow rather than default cards. |
| Runtime themes / `ImportSettings` | A visually compelling styling subsystem. | Add to Layout/Styling; retain as first replacement candidate if a default limited card fails verification. |
| RouteCard/RouteModal | Experimental higher-level routing presentation. | Defer until PAX-930 and gesture/public-contract work are complete. Base Router remains the launch claim. |
| Inherited opacity and canvas/native image choice | Builder-relevant visual details currently stranded behind stubs/API pages. | Cover in Visual Content, but do not allocate default cards unless the article and distinct proof justify them. |

### Docs-only foundational work that should remain off the gallery

The gallery correctly omits or relegates these topics even though the learning path cannot:

- a true `What is Pax?` category/maturity/OSS explanation;
- installation prerequisites, generated project anatomy, exact create/run recovery, and the PAX-906 starter decision;
- the basic handler -> property -> reactive-update loop, lifecycle, and coordinate semantics;
- component definition, public inputs/defaults, control flow, keyed identity, and slot ownership;
- element order/z-order and percentage positioning semantics;
- ordinary text/image/asset handling and common styling/theme practices;
- scrolling construction even if Carousel is not a card;
- accessibility status and platform-control coverage;
- route fallback/metadata configuration and the client-side/SSR boundary;
- exact build modes, release cartridges, artifacts, web deployment, and Apple signing handoff;
- public API versus maintainer API/design-document separation;
- practical runtime/backend/performance reasoning without numeric promises.

These concepts should not be turned into filler cards. They are the connective tissue that lets a builder reproduce the capabilities shown by the gallery.

### Contradictions and editorial review points

Most state and claim judgments agree. The following decompositions need an explicit decision:

1. **Exact event-driving surface.** Both artifacts accept PAX-973's OSS boundary. The gallery labels scene inspection and event-driving as shipped and cites `pax-cli dev`, but its concrete workflow is ray-cast/inspect/select plus `touch` source mutation; this audit did not find an obvious public CLI command that emits a userland click/touch event. Before either article says "drive events," identify the public command/API and demonstrate it. Otherwise describe the verified operations precisely and open the missing public surface as a launch tooling gap.
2. **One Visual Content article or three repaired chapters.** The gallery recommends independently expanding Drawing & Styling, Compositing & Effects, and Text/Fonts/Images. The docs-first proposal combines their launch essentials into one economical Visual Content tour because all three current articles are nine-line stubs. Decide whether stable card anchors and long-term discoverability outweigh the extra prose/verification burden of three articles. Recommendation: one bounded launch article now, then split only when each child topic can support a real chapter.
3. **Source filename versus public URL contract.** The gallery uses deterministic current mdBook `.html` URLs, while PAX-974 proposes `/get-started/` and this map proposes new source slugs. Decide the published canonical URL and redirect policy before hard-coding CTAs. Source filenames and stable heading IDs should be approved now; PAX-987 should emit or preserve the external URL contract.
4. **Inspectable Interfaces card scope.** The gallery combines `ExampleHost` source presentation with CLI inspection. They reinforce the same theme but are separate systems and `ExampleHost` is not automatic arbitrary source recovery. Recommendation: keep one card only if its summary says "expose example source and inspect running scenes" rather than implying a single automatic inspector.
5. **Lighting as a default launch card.** Both inventories call it limited and agree on WGPU/Piet constraints. The decision is editorial, not factual: its visual proof is unusually strong, but it adds the heaviest qualification burden and currently has only API/internal-design destinations. Keep it only if the short guide and platform-labeled media are ready; otherwise replace it with Runtime Themes or Native Scrolling from overflow.
6. **Performance placement.** Both inventories reject performance mechanisms and numbers as cards. The docs inventory still treats a builder-facing runtime/performance explanation as necessary for evaluation. Keep that article off the gallery and lead with verified mechanisms, not numeric guarantees.

### Card deferrals and launch-blocking docs gaps

Defer RouteCard/RouteModal, standalone Liquid Glass, standalone PhotoPicker, Carousel/native scrolling, Handwriter, sensors, Markdown Text, accessibility, performance mechanisms, release size, designer/visual editing, generic media/video, future logic languages, and future native targets as default launch cards. Some remain valid overflow or article examples; the rejected claims remain rejected.

To support the recommended default twelve, launch docs must at minimum complete:

1. `what-is-pax.md` and the first guided authoring loop;
2. `targets-build-deploy.md` with exact target/host/backend distinctions;
3. the PAXEL on-ramp and expanded template/events/component fundamentals;
4. the native-controls/compositing/accessibility section;
5. layout fundamentals plus stable responsive/autosize anchors;
6. a public paths/SVG guide;
7. a limited lighting/materials guide if card 9 remains;
8. `developer-workflow.md` for hot reload and inspection, including resolution of event-driving wording;
9. a verified Routing merge after PAX-869's prerequisite;
10. PAX-987 publication/redirect work so all CTAs resolve to the new book.

## G. Checkpoint 1 review and remaining decisions

Zack's 2026-08-04 review establishes these decisions:

1. **The three-step first-touch split is approved:** `What is Pax?` -> `Install, create, and run` -> `Build your first Pax interface`.
2. **Getting Started is also a standalone external front door.** The website, README, and other Get Started CTAs may link directly to it. It assumes intent to try Pax, not prior navigation from the beginning of the book.
3. **Builder and maintainer reference will be separated.** Public `pax-runtime-api`/`pax-std` remain visible; internal APIs and curated architecture move under a clearly labeled appendix/reference.
4. **Stable slugs and anchors will be frozen early**, with a version-aware publication contract rather than ad hoc surface-specific URLs.

Two editorial choices remain before the first chapter outline: the generated starter contract and the visual-content article decomposition.

### Clarification: visual-content topology versus examples

The earlier "one bounded Visual Content article" proposal was about navigation and concept ownership, not about whether examples should be embedded. The current book contains three separate nine-line stubs:

- `text-fonts-images.md`;
- `drawing-styling.md`; and
- `compositing-effects.md`.

Combining them into `visual-content.md` would reduce launch prose and eliminate three weak peer articles, but it would also discard useful long-term destinations and give website cards less precise anchors. Keeping all three creates better reference homes but requires enough verified content that they no longer feel like placeholders.

The `/examples` and `ExampleHost` idea improves either topology. It should be treated as a shared proof system:

1. author one cohesive visual-example suite with small named components for text/images, drawing/styling, and compositing/effects;
2. expose the same components in `/examples` as visually polished discovery surfaces;
3. embed or host the relevant component beside each article with an explicit `ExampleHost` source manifest;
4. keep essential prose and copyable source in the article so learning does not depend on a live embed, WebGPU availability, or JavaScript execution;
5. label target/backend-specific examples at the embed boundary; and
6. let PAX-976 own the launch example catalog/hosting and PAX-855 own deeper chapter examples, while PAX-975 owns the canonical explanation and article placement.

Revised recommendation: **keep and expand the three existing articles rather than introduce `visual-content.md`**, but bound their launch roles tightly:

| Article | Launch-owned concepts | Deliberate links rather than duplication |
| --- | --- | --- |
| `text-fonts-images.md` | Text layout/style, fonts/assets, rendered `Image` versus native `NativeImage`, selectable/editable text boundaries, image alternatives, and supported-format caveats. | Link controls/accessibility to Input; masks to Compositing; detailed properties to public APIs. |
| `drawing-styling.md` | Vector primitives, fills/strokes, colors/gradients, inherited styling/themes, paths, SVG import/ejection, draw ranges, and a clearly advanced/limited lighting section. | Link unit/expression syntax to PAXEL, motion to Animation, masking/layer semantics to Compositing. |
| `compositing-effects.md` | Source order/z-order recap, opacity, masks, clipping, rendered/native element islands, occlusion, and backend/target constraints. | Link basic geometry to Layout, native controls to Input, and renderer selection to Runtime/Performance. |

This costs more than one combined article, but the tailored-example plan makes each page independently useful and supplies precise, durable destinations for gallery cards. If launch time becomes the binding constraint, the fallback is to finish Drawing/Styling and Compositing first, put a verified text/images essentials section in Input/Layout, and defer a full standalone Text article rather than publish another stub.

### Versioned docs and primary CTA contract

The existing PAX-987 publisher is a good foundation. It already uploads `/<version>/` with `public,max-age=31536000,immutable`, treats the bucket root as mutable latest with `no-cache`, publishes a no-cache `versions.json`, and has a path-preserving version picker. The bucket root, not a literal `/latest/` prefix, is the current latest implementation.

Recommended public URL contract:

| Purpose | Public shape | Policy |
| --- | --- | --- |
| Primary current docs | `https://docs.pax.dev/getting-started/` | Canonical external CTA. Standalone and mutable with the current release. |
| Exact release docs | `https://docs.pax.dev/0.39.0/getting-started/` | Immutable after successful publication. Version picker preserves the page and fragment where the destination exists. |
| Compatibility aliases | `/get-started/`, `/getting-started.html`, and optionally `/latest/getting-started/` | Permanent redirect to the canonical bare latest path. Do not expose multiple equivalent current URLs in navigation. |
| Historical filename alias | `/0.39.0/getting-started.html` | Redirect within the same version, never silently to latest. |

`/getting-started/` is preferable to `/get-started/` because it matches the source article and the current PAX-974 README. Existing or older `/get-started/` links should remain valid through a permanent redirect. The bare docs root is the product-facing latest alias; a literal `/latest/` directory may exist internally, but should redirect or rewrite invisibly rather than become a second public URL family.

Publication recommendations for PAX-987:

1. **Validate strict SemVer and enforce immutability.** Refuse to overwrite a non-empty released prefix unless an explicit emergency-repair flag and audit note are supplied. The current script labels a prefix immutable through caching but does not prevent a later `sync --delete` from changing it.
2. **Build once, upload the immutable version first, verify it, then advance latest.** The same bytes should back the versioned snapshot and latest alias. A failed immutable upload must not change the public latest pointer or manifest.
3. **Make latest a pointer/alias where infrastructure allows.** The cleanest model is a CloudFront rewrite from bare paths to the chosen immutable version prefix, updated only after verification. If latest remains a copied tree, put it in a separately deletable prefix or otherwise remove stale latest-only objects without risking semver directories.
4. **Invalidate only mutable surfaces.** Refresh the bare/latest alias, redirects, and `/versions.json`; avoid the current `/*` invalidation because immutable semver paths should remain cacheable and untouched. AWS recommends versioned names/directories because they make roll-forward, rollback, cache behavior, and logging more controllable: [CloudFront versioned content guidance](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/UpdatingExistingObjects.html).
5. **Keep mutable cache behavior explicit.** Latest HTML, redirects, and the manifest should revalidate (`max-age=0, must-revalidate` or equivalent); stable-name latest CSS/JS should also revalidate unless the build fingerprints them. Ensure the CloudFront cache policy has minimum TTL 0, because a positive minimum can override `no-cache`: [CloudFront expiration guidance](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/Expiration.html).
6. **Make rollback a pointer change.** Never rebuild an old version to roll back. Repoint latest to a previously verified immutable prefix, update the manifest, and invalidate only the mutable alias.
7. **Enrich the manifest.** In addition to version, label, path, and release date, record the source revision and optional support status (`latest`, `supported`, `archived`, `prerelease`). Use a SemVer-aware comparator; the current custom split key should be verified for prerelease/build-metadata ordering.
8. **Define missing-page behavior.** Changing versions should preserve the article path and anchor. If that page did not exist, show a same-version not-found page with links to that version's home/search; do not silently jump to latest.
9. **Choose one canonical URL per page.** Latest pages should self-canonicalize to the bare latest path; versioned pages should self-canonicalize within their version because the version context is meaningful. Aliases should redirect, internal links and sitemaps should use canonical URLs, and canonical tags should be emitted in source HTML. Google treats redirects and `rel=canonical` as strong signals and recommends consistent internal links: [Google canonical URL guidance](https://developers.google.com/search/docs/crawling-indexing/consolidate-duplicate-urls).
10. **Add publication smoke tests.** Check the bare CTA, an exact-version page, CSS/JS, version switching, a legacy alias, a missing page, route refresh, response cache headers, canonical tags, and a rollback rehearsal before PAX-987 publishes.

### Remaining decisions before Checkpoint 2

1. Approve or modify the **purpose-built polished starter plus curated `--example` overrides** recommendation, including changing PAX-906's proposed default away from `router-playground`.
2. Approve or modify the revised **three focused visual articles backed by shared `/examples`/`ExampleHost` proof** recommendation.

Once those are resolved, Checkpoint 1 is complete. The next sequential deliverable is the outline and evidence packet for `What is Pax?`, not prose for multiple chapters.
