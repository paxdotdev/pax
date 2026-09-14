# Launch documentation coverage map

Status: Initial prose reviewed through Routing and Primitives; navigation/reference integration in progress. The first-interface tutorial remains queued for a stable PAX-993 handoff. Launch verification and publication gates are still open; see the latest checkpoints below.

Owner issue: PAX-975

Parent program: PAX-860

Last audited: 2026-09-12, including the standalone Primitives review and the navigation/reference pass.

Sections A–J retain the original discovery and approved planning history;
later lettered checkpoints record implementation, review, and superseding
decisions. Descriptions of thin or missing articles in that early audit are
historical, not the current chapter status.

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
| `getting-started.md` | Substantial workstation setup article that now also documents formatting, the `.pax`-only hot-reload default and opt-in logic lane, web `public/` files, and project metadata. | Despite its title and CTA role, it still does not walk through `pax-cli create`, the first run, expected screen, project anatomy, first edit, build output, or deployment. Its generated-project assumptions must be replaced with the stabilized PAX-993 contract after that work lands. | **Revise/split after coordination.** Make install/create/run its standalone opening; move the broader sustained workflow to a dedicated article. |
| `template-language.md` | Short fundamentals outline followed by substantive dynamic ID/class and conditional `@settings` reference material. | Basic tree syntax, settings blocks, imports, event bindings, source ordering/z-order, and component invocation are still named rather than taught. The newest selector features are more complete than the language fundamentals. | **Expand and reorder.** Make this the canonical structural-language article without losing the new dynamic-class detail. |
| `state-properties.md` | The strongest existing deep conceptual/API bridge. | It explains `Property<T>`, computed values, dependencies, subscriptions, and state patterns well, but assumes the reader already understands where properties sit in a Pax component and how templates see them. | **Revise lightly.** Add a short role/orientation section and link from the first guided app. Keep exact API detail here. |
| `data-binding-expressions.md` | Initial prose now starts with a shared reactive panel, followed by values/units, choices, structured data, helpers, globals, and a compact reference. | The invalid color-object example and blanket `$base` ordering claim are corrected. Dynamic indexing, boolean guards, and the precedence table remain reserved for the PAX-995 handoff. See sections L–M. | **Initial prose review; retain useful reference and stage drawing-specific material for its owner chapter.** |
| `event-handling-rust.md` | Brief outline with useful advanced coordinate, touch, and scroller notes and an embedded game example. | It lacks the basic handler signature, component/state relationship, reactive update loop, common event choices, and a compact click-to-property walkthrough. Advanced details arrive before the first success. | **Expand substantially.** |
| `control-flow.md` | Compact and useful syntax coverage for conditionals and keyed loops. | It needs one explanation of why keys matter, a link to component composition, and verification against current syntax. It need not become a large chapter. | **Merge into the Components and Composition chapter.** |
| `components-composition.md` | Outline with relatively detailed slot/projection material. | It does not yet teach the `.rs`/`.pax` component pair, public fields, child composition, imports, defaults, or project organization. Slot detail is useful but out of sequence. | **Expand or merge with control flow.** |
| `routing.md` | Substantive routing guide with nesting, branches, route writes, and a working example. | It is one of the more launch-ready feature chapters. PAX-869 reports route metadata implemented in its separate mainline worktree, but that implementation and prose are not present in this rebased tree and still require a coordinated semantic merge. | **Revise after coordination; otherwise keep.** |
| `layout-responsiveness.md` | Good autosize, padding, layout-role, and breakout-container material. | Core positioning, size constraints, alignment, transforms, and responsive composition are only outlined. Percentage `x`/`y` semantics are a recurring authoring surprise: they represent remaining travel after the element's own size, not raw parent coordinates. | **Expand substantially.** |
| `text-fonts-images.md` | A partial article that now teaches `TextStyle`, local and hosted font values, asset/remote image sources, reactive image URLs, and raw RGBA construction. | It still needs wrapping/sizing behavior, selectable/editable distinctions, rendered versus native images, image fitting, supported-format/target caveats, alternatives/accessibility, and a verified example. | **Expand.** It is now a viable durable article rather than a stub. |
| `drawing-styling.md` | Nine-line outline. | It omits current vector/path, gradient, SVG import/ejection, and runtime drawing capabilities. | **Expand as the dedicated Drawing and Styling article; defer exhaustive leaf-API coverage.** |
| `input-native-controls.md` | Detailed PhotoPicker section under a broad input heading. | Ordinary controls, two-way binding, focus, platform coverage, permissions, and the accessibility boundary are not coherently introduced. Pointer/touch mechanics belong with Events rather than under the native-controls heading. | **Rename to and restructure as `accessibility-native-controls.md`.** Keep PhotoPicker as a platform-specific subsection or example and preserve a compatibility path from the old URL. |
| `animation-motion.md` | Strong timeline, easing, enter/exit, interruption, and reflow material. | It is close to launch-ready after syntax/example verification and a simpler first example. It correctly names at least one limitation rather than implying complete transition coverage. | **Revise lightly; retain as canonical motion anchor.** |
| `compositing-effects.md` | Nine-line outline. | It does not explain canvas/native element islands, occlusion, masks, clipping, coordinate-space implications, or backend/platform caveats. | **Expand as the dedicated Compositing and Effects article; avoid a stub nav item.** |
| `scrolling-viewports.md` | Short outline with autosize guidance. | It lacks a smallest working scroller, viewport/content sizing, events, native-element interaction, and performance/culling notes. It references a missing example. | **Expand if it can reach useful depth; otherwise merge its essentials into Layout and defer the standalone chapter.** |
| `architecture-runtime-cartridge.md` | Detailed kernel/cartridge implementation narrative. | It is too low-level for the core learning path, contains numerical size/performance examples that require current evidence, and mixes implemented behavior with older future-language and design-tool framing. Some material is valuable for maintainers and some can support a practical runtime explanation. | **Split/replace.** Write the standalone builder-facing `how-pax-runs.md` and move or label the detailed architecture as an appendix. |
| `performance-scale.md` | One line, not navigated. | No usable content. | **Replace with `how-pax-runs.md` or remove the orphan during the later TOC edit.** |

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

PAX-909 provides strong clean-workstation evidence for the source-linked workflow. PAX-993 now owns the generated starter, curated `--example` contract, and canonical-source bundling mechanism; PAX-906 retains release execution and publish-channel validation. The final public-package commands cannot be frozen from the current tree alone.

| Journey step | Candidate public surface | What must be verified before prose is final | Ownership/dependency |
| --- | --- | --- | --- |
| Install | `cargo install pax-cli` with any documented prerequisites | Install the actual release candidate from the public crate path on clean macOS, Ubuntu, and Windows workstations. Record Rust/Node/npm/wasm-pack prerequisites and distinguish web development from native Apple builds. | PAX-906 release; PAX-909 test method |
| Create | `pax-cli create <name>` and `--example=<curated-name>` | Verify the PAX-993-selected default, generated source, command output, directory layout, component pair, dependency versions, available-example behavior, and canonical-source snapshot. | PAX-993 implementation; PAX-906 publish-channel validation |
| First run | `pax-cli run --target=web` | Verify blank-machine first build, browser launch behavior, actual URL, visible success state, warnings, stop/restart behavior, and the first `.pax` hot reload. | Launch RC |
| Native run | `--target=macos`, `--target=ios`, `--target=ipados` | Verify exact spelling, simulator/device selection, Xcode requirements, signing boundary, and whether each documented path is run or build-only. | Launch RC on macOS |
| Build | `pax-cli build --target=<target>` | Verify debug/release flags, output locations, app/bundle names, web asset layout, and the difference between run, build, and release cartridge behavior. | Source plus launch RC |
| Template hot reload | Default debug `pax`; `--hot-reload=pax` | Exercise a visible `.pax` edit on all four targets. Confirm disabled-lane behavior. | Runtime-resilience implementation plus launch RC |
| Logic hot reload | Opt in with `--hot-reload=all` or select `--hot-reload=logic` | Exercise a Rust logic edit on web and macOS. Confirm the default does not rebuild logic and iOS/iPadOS require rebuild/relaunch for logic changes. | Runtime-resilience implementation plus launch RC |
| Inspect and capture | `pax-cli dev status`, `look`, `inspect`, `logs`, `ray-cast`, `selector` | Capture exact help text and one minimal builder workflow. Confirm session discovery and output formats without presenting internal protocols as required knowledge. | Current CLI |
| Drive events | Current OSS event-driving surface | Locate the supported public command/API, drive one click/touch, and capture its selector/coordinate semantics. If no public surface exists, make that a launch tooling issue instead of inventing syntax. | PAX-973 boundary; CLI verification gap |
| Local docs/examples | `pax-cli docs list/open/search/examples` | Verify command names, browser behavior, offline expectations, example discovery, and which docs are generated/reference-only. Avoid asking a reader to rebuild docs. | Current CLI |
| Representative examples | Small starter plus layout, routing, motion, drawing, input/compositing, and backend-caveat examples | Build only the curated launch set against debug and release paths, then link stable source/demo URLs. Do not make all complete examples or all 41 gallery call-outs a launch gate. | PAX-976; deeper examples PAX-855 |
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
| `project-anatomy` / Pax project and component pair | Fundamentals. Shows where Cargo, Rust state/handlers, `.pax` templates, assets, and metadata live. | **Launch-critical; shipped.** The generated-project contract is evolving in PAX-993 and is not yet a stable prose dependency. | PAX-993 worktree; bundled-example registry and create tests. | `getting-started.md#project-anatomy`; **new subsection** after the PAX-993 handoff stabilizes. | First success. | Annotated file tree. |
| `cli-first-run` / Install, create, run | Workflow. Provides the shortest path to a visible running application. | **Launch-critical; shipped but release-sensitive.** Clean-host package flow must be rerun for the launch RC. | `pax-cli` commands; PAX-909; PAX-906; current `getting-started.md`. | `getting-started.md`; **revise**. | First success gate. | Terminal transcript and one expected-result screenshot. |
| `template-tree` / Declarative UI tree and settings | Language fundamentals. Teaches element creation, property settings, IDs/classes, imports, bindings, and source order. | **Launch-critical; shipped.** Pax z-order is visually top-to-bottom: elements earlier in source render above later siblings. | `template-language.md`; parser/tests; examples. | `template-language.md`; **expand**. | First guided edit. | Small layered card example and tree/source comparison. |
| `paxel` / Formula-style expressions | Reactivity. Teaches side-effect-free bindings, derived values, operators, units, globals, and `$base`. | **Launch-critical; shipped.** Keep arbitrary side effects in Rust. | `data-binding-expressions.md`; language parser/tests. | `data-binding-expressions.md`; **sufficient/revise** plus a short first-app use. | First-aha: direct manipulation of a property updates UI. | Live derived label/style example. |
| `properties` / Reactive application state | State. Shows how Rust values participate in the dependency graph and how computed/subscribed properties work. | **Launch-critical; shipped.** Avoid making subscriptions the beginner default. | `state-properties.md`; `pax-runtime-api` property docs. | `state-properties.md`; **sufficient/revise**. | First-aha bridge from handler to UI. | Counter/filter state progression. |
| `rust-logic` / Handlers, lifecycle, and side effects | Application logic. Shows where network, filesystem, platform, and mutation work belongs. | **Launch-critical; shipped.** Rust is the supported launch language. | `event-handling-rust.md`; component macros and examples. | `event-handling-rust.md`; **expand**. | Completes first-aha loop. | Click handler changes a property; one async/platform call can be later. |
| `events-input-model` / Events and coordinates | Interaction. Builders need pointer/touch/click semantics, local/window coordinates, propagation, and capture boundaries. | **Launch-critical basics; shipped with caveats.** `local_point` and native/canvas boundaries matter; some controls capture touch. | Event APIs; `event-handling-rust.md`; `examples/src/mouse-animation`, `occlusion`; pain-point records. | `event-handling-rust.md#events-and-coordinates`; **expand**. | First interaction, then deeper troubleshooting. | Coordinate overlay/ray-cast illustration. |
| `control-flow` / Conditionals and keyed loops | Structure. Enables data-driven trees without imperative template code. | **Launch-critical; shipped.** Keys must be explained as identity, not decoration. | `control-flow.md`; examples/parser tests. | `components-composition.md#control-flow-and-identity`; **merge/revise**. | Small-list first app or next step. | Add/remove/reorder list demo. |
| `components-slots` / Reusable components and composition | Architecture. Teaches ownership, public properties, defaults, nesting, slots, and projection. | **Launch-critical basics; shipped.** Advanced slot semantics can remain reference material. | `components-composition.md`; slot examples; component macros. | `components-composition.md`; **expand**. | Moves reader from toy to app. | Reusable card/button with one named/default slot. |
| `layout-core` / Position, size, alignment, transforms | Layout. Builders need a reliable spatial model before styling and motion make sense. | **Launch-critical; shipped.** Percentage positioning semantics need explicit explanation. | `layout-responsiveness.md`; `pax-std` layout types; examples; pain-point record. | `layout-responsiveness.md#the-coordinate-and-size-model`; **expand**. | First visual composition. | Interactive percent/px diagram or responsive card. |
| `units-responsive` / Units, breakpoints, and conditional settings | Layout/language. Shows fluid sizing, unit arithmetic, and platform/viewport-specific choices. | **Launch-critical; shipped.** Prefer `%` for responsive sizing; conditional settings syntax needs current verification. | `layout-responsiveness.md`, `template-language.md`, `responsive-helpers`, `adaptive-cards`. | `layout-responsiveness.md#responsive-layout` with link to settings syntax; **expand**. | First-aha visual payoff. | Narrow/wide side-by-side screenshot or resize clip. |
| `autosize-padding` / Content-driven layout | Layout. Explains intrinsic measurement, padding, breakout, and parent/child responsibilities. | **Useful; shipped.** Native/text measurement can vary by target. | `layout-responsiveness.md`; `auto-sized-containers`; PAX-788. | `layout-responsiveness.md#content-driven-size`; **sufficient/revise**. | Deepens responsive UI. | Autosize/breakout comparison. |
| `style-themes` / Reusable visual settings | Styling. Enables coherent design systems rather than scattered literals. | **Useful, near launch-critical; shipped.** `ImportSettings` and defaults need a beginner-level path; selector grammar has limitations. | `runtime-settings-themes`; PAX-864; template/settings source. | New styling subsection in layout/visual article; **new subsection**. | First-aha polish. | Theme toggle or branded component set. |
| `text-assets` / Text, fonts, images, and assets | Content. Necessary for any nontrivial app and for understanding native/rendered boundaries. | **Launch-critical basics; shipped with target caveats.** Current text article is a stub; image alt is a partial accessibility foundation. | `pax-std` text/image APIs; examples; current `text-fonts-images.md`. | `text-fonts-images.md#text-images-and-assets`; **expand**. | First practical UI. | Font/image asset walkthrough and text wrapping screenshot. |
| `vectors-gradients` / Shapes, colors, and gradients | Visual expression. Gives builders Pax-native illustration and styling vocabulary. | **Useful; shipped.** Verify backend parity for specific effects. | drawing primitives; gradient implementation/tests; `pax-logo`, `color-picker`; PAX-949. | `drawing-styling.md#shapes-and-gradients`; **expand**. | First visual wow. | Gradient card/logo. |
| `paths-svg` / Paths, SVG workflows, and runtime drawing | Creative tooling. Supports custom shapes, illustration, and authored/dynamic paths. | **Useful; shipped.** SVG import/ejection and runtime `draw_start`/`draw_end` have distinct workflows. | path APIs; CLI `svg-import`; `path-drawing`; PAX-967. | `drawing-styling.md#paths-and-svg`; **new subsection**, deeper article post-launch. | Deeper discovery/wow. | Handwriter/path-drawing clip and small SVG pipeline. |
| `native-controls` / Native UI elements and two-way binding | App capability. Explains how forms/media/platform controls coexist with rendered content. | **Launch-critical basics; shipped with varying target coverage.** Do not imply every control exists on every target. | `pax-std` native controls; examples; PAX-932. | `accessibility-native-controls.md#native-controls`; **expand/restructure**. | Shows real-app ceiling. | Small form + PhotoPicker, with platform matrix. |
| `compositing` / Canvas and native element islands | Runtime/visual. Builders need to understand occlusion, transforms, clipping, and when a native underlay is required. | **Useful; shipped with constraints.** Cross-island ordering and mask composition are not arbitrary DOM/canvas layering. | platform runtimes; `occlusion`, `neon-opacity`, `liquid-glass`; current `compositing-effects.md`; pain-point records. | `compositing-effects.md#native-compositing`; **expand**. | Deeper feature discovery. | Layer-stack diagram and occlusion demo. |
| `scrolling` / Scroll containers and content sizing | Layout/input. Teaches viewport/content roles, events, and performance implications. | **Launch-critical basics; shipped.** Native child/touch behavior and autosize rules need explicit caveats. | Scroller APIs; `rounded-scroller-tiles`, `scroll-garden`, stress examples. | `scrolling-viewports.md`; **expand**, or merge for a bounded launch chapter. | First real app list/feed. | Small list + scroll-event example. |
| `motion-timeline` / Timelines and easing | Motion. Introduces declarative animation and visual feedback. | **Useful, high launch value; shipped.** Custom closure easing is not declarative template syntax. | `animation-motion.md`; `timeline-playground`, `pax-logo`, `marionette`. | `animation-motion.md#timelines-and-easing`; **sufficient/revise**. | First-aha/wow. | 5–10 second clip or interactive embedded example. |
| `motion-structural` / Enter, exit, reflow, and interruption | Motion. Lets state and route changes remain spatially legible. | **Useful; shipped with named limitations.** Verify supported reflow cases and interruption semantics; do not promise every Stacker transition. | `animation-motion.md`; `transition-grid`; PAX-946 and interruption work. | `animation-motion.md#structural-transitions`; **sufficient/revise**. | Deeper app polish. | Transition-grid clip. |
| `routing-history` / Routes, branches, and history | App architecture. Enables multi-screen applications and URL/history-aware state. | **Launch-critical; shipped.** Route topology has static requirements on web. | `routing.md`; `router-playground`; router source/tests. | `routing.md`; **sufficient/revise**. | Moves reader from component demo to app. | Two-route starter or focused router demo. |
| `route-metadata` / Web metadata and static route topology | Deployment/discovery. Makes routed web apps shareable and indexable. | **Useful; implementation active in PAX-869 worktree.** Literal metadata and hosting fallback requirements must land before canonical docs. | PAX-869 branch comments/diff; not yet current-tree behavior. | `routing.md#web-route-metadata` after merge; **new subsection**. | Ship-stage concern. | Inspect generated route files and page metadata. |
| `targets` / Application targets vs development hosts | Platform. Prevents "runs everywhere" ambiguity and directs platform setup. | **Launch-critical; shipped targets: web, macOS, iOS, iPadOS.** Clean workstation validation covers macOS/Linux/Windows for supported development workflows; native Apple builds require macOS/Xcode. | CLI `RunTarget`; platform crates; PAX-876; PAX-909; PAX-945. | New `targets-build-deploy.md#targets-and-workstations`; **new article**. | Evaluation and ship gate. | Compact support matrix. |
| `renderer-policy` / WebGPU and Piet fallback | Runtime/performance. Sets honest expectations for capabilities and backend differences. | **Launch-critical caveat; shipped.** WebGPU where available/policy permits; Piet/CPU fallback on iOS WebKit or no WebGPU; lighting/materials render unlit on Piet. | web renderer selection; GPU/Piet backends; PAX-960; PAX-945. | `how-pax-runs.md#render-backends`; **new subsection**. | Evaluation/deeper discovery. | Renderer selection diagram; no blanket benchmark. |
| `hot-reload` / Template and logic lanes | Workflow. Provides the tight authoring loop and explains target differences. | **Launch-critical; shipped.** `.pax` reload is the debug default on all targets. Rust logic reload is opt-in on web/macOS; iOS/iPadOS require rebuild/relaunch for logic changes; release disables both lanes. | CLI/runtime hot-reload code; runtime resilience design/tests; `getting-started.md`. | New `developer-workflow.md#hot-reload`; **new article**. | First edit and sustained workflow. | Short edit/reload clip and target matrix. |
| `dev-inspection` / Inspect, look, logs, selectors, ray-cast | Workflow. Gives builders eyes into a running scene and reproducible visual diagnostics. | **Useful, high launch value; shipped OSS.** Exact command outputs and stability need verification. | `pax-cli dev`; PAX-973; README. | `developer-workflow.md#inspect-a-running-app`; **new subsection**. | First troubleshooting success. | Terminal + screenshot sequence. |
| `event-driving` / Script userland interaction | Workflow/testing. Enables repeatable click/touch-driven visual validation. | **Useful; accepted as shipped OSS, public surface unresolved in this audit.** | PAX-973; README/AGENTS claim; no obvious current `pax-cli dev` subcommand. | `developer-workflow.md#drive-events` only after verification; **new subsection/verification blocker**. | Deeper testing, not first run. | One reproducible click/touch sequence. |
| `local-docs-examples` / Discover docs and examples from the CLI | Workflow. Lets builders explore without already knowing file paths or article names. | **Useful; shipped.** Generated API/internal result labeling must be clear. | `pax-cli docs`; docs index generator; example inventory. | `developer-workflow.md#local-docs-and-examples`; **new subsection**. | Discovery after first success. | Short command table, no media required. |
| `build-cartridge` / Debug and release program representations | Shipping/runtime. Explains why release behavior differs and what the build produces. | **Launch-critical basics; shipped.** Rich manifests and baked release cartridges must remain behaviorally aligned. | compiler cartridge generation; `pax-manifest` binary/program IR/rust manifest; architecture article/tests. | `targets-build-deploy.md#build-modes` with deeper maintainer appendix; **new subsection**. | Ship gate. | Debug/release flow diagram. |
| `deploy-web` / Host a web build | Deployment. A launch path is incomplete if it stops at local run. | **Launch-critical; shipped build output, documentation missing.** Route fallback/metadata and base-path details need verified recipes. | CLI build output; PAX-869 route work; PAX-987 publishing scripts as implementation reference only. | `targets-build-deploy.md#deploy-web`; **new subsection**. | Final journey step. | One static-host recipe and output tree. |
| `ship-apple` / Apple build and distribution boundary | Deployment. Clarifies what Pax automates and what Xcode/signing still owns. | **Launch-critical; shipped build/run targets with external platform requirements.** Store release is not a one-command Pax guarantee. | Apple chassis/CLI build paths; PAX-876. | `targets-build-deploy.md#build-for-apple-platforms`; **new subsection**. | Final journey step. | Build artifact screenshot/path and signing boundary note. |
| `runtime-performance` / Reactive work, culling, and render scheduling | Architecture/performance. Helps builders reason about cost without folklore or unsupported numbers. | **Useful, launch-critical for honest evaluation; shipped mechanisms.** Results depend on workload/backend/device. | runtime dirty propagation; culling; PAX-588/PAX-851; architecture source/tests. | New `how-pax-runs.md`; **new article**. | Evaluation/deeper learning. | Mechanism diagram plus reproducible profiling recipe later. |
| `materials-lighting` / LightFrame and GPU materials | Advanced visual capability. Demonstrates the creative ceiling. | **Useful; limited.** WebGPU/WGSL path, maximum/current light constraints, Piet unlit; maps, shadows, 3D, and cameras are not current. | `materials`, `glow-buttons`; PAX-960/PAX-966; GPU source. | `drawing-styling.md#lighting-and-materials` as caveated discovery link; deeper **defer**. | Visual wow, not first-touch dependency. | Glow/material clip with backend label. |
| `liquid-glass` / Platform-specific visual material | Advanced native visual capability. Shows Apple-platform integration. | **Useful; limited/platform-specific.** Must not be presented as portable parity. | `liquid-glass`; platform component source. | Platform notes under visual/native composition; **new subsection or defer**. | Gallery/deeper discovery. | Apple-only demo with explicit badge. |
| `accessibility-status` / Current accessibility foundation | Quality/platform. Builders need to know present support and unresolved responsibilities. | **Launch-critical caveat; limited.** Foundations exist; reading/tab order, annotations, and full audits remain incomplete. | PAX-958; native/text/image APIs. | `accessibility-native-controls.md#current-support` and targets/status links; **new subsection**. | Evaluation and implementation caveat. | Compact current/limited matrix; no aspirational badge. |
| `future-platforms-languages` / Explicit non-goals for this launch | Scope. Prevents old architectural aspirations from being read as shipped behavior. | **Post-launch; future.** Windows/Linux/Android application targets and JS/Python application logic are not launch claims. | PAX-945 claims ledger; architecture history. | Optional roadmap link outside core docs; **defer**. | Not part of canonical journey. | None. |

## C. Proposed launch information architecture

### Design principles

The launch path should be tutorial-first, reference-connected, and intentionally shallow where a deep article would delay the spine. Each important concept gets one canonical owner and all other mentions use a short recap plus a deliberate "Read more" link. Article titles should name a builder task or mental model, not an internal crate.

Stable section anchors should be treated as public interfaces because the README and website cards will link to them. Prefer descriptive slugs and durable headings such as `#responsive-layout`, `#paths-and-svg`, `#native-controls`, and `#hot-reload`; avoid anchoring launch links to example names or temporary implementation labels.

### Proposed public path

The rows below are editorial units. A row may retain two current files when those files are already strong references, but it has one clear place in the journey.

| Proposed article | Purpose and reader outcome | Prerequisites and read-more relations | Owned concepts vs linked concepts | Current file action | Criticality | Example/media placeholder | Verification burden |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **What is Pax?** (`what-is-pax.md`) | Let a Rust developer decide whether Pax fits, then leave with the authoring loop, runtime/target boundary, OSS boundary, and honest maturity framing. | None. Read more to Getting Started, How Pax Runs, Targets/Build/Deploy, and GitHub. | Owns category, audience, template/PAXEL/Rust loop, portable-runtime summary, ready-for-builders wording, self-contained OSS statement. Links all feature detail. | **New.** Do not repurpose the internal architecture article. | Launch-critical | One loop diagram; one representative hero application image owned with PAX-976/PAX-869. | Claims-ledger review against PAX-945/PAX-973 and current target source. |
| **Install, create, and run** (`getting-started.md`) | Produce a visible local app from a clean workstation and identify the generated project files. It is the standalone destination for the primary Get Started CTA on the website, README, and other external surfaces. | **No reading prerequisite.** It may link back to What is Pax? for context and forward to First Pax Interface, Developer Workflow, and Targets/Build/Deploy. | Owns prerequisites, install, create, first web run, expected result, project anatomy, common setup failures. It includes only the minimum mental model needed to act and links rather than embedding full hot-reload/tooling docs. | **Keep and refocus around the stabilized PAX-993 create contract.** Route metadata content may instead live under Routing/Deploy. | Launch-critical | Exact terminal transcript, generated tree, first-run screenshot. | Highest: clean macOS/Ubuntu/Windows RC install; stable generated-project handoff; actual first warnings/output; PAX-906 publish-channel validation. |
| **Build your first Pax interface** (`first-pax-interface.md`) | Guide one coherent edit through the same project produced by Getting Started. | Getting Started. Read more to Templates, PAXEL/Properties, Events, Components, Layout, and Motion. | Owns only the end-to-end guided experience and checkpoints. It does not introduce another tutorial project or explain every decorative feature. Deep syntax belongs to linked chapters. | **New.** Reuse the stabilized PAX-993 starter rather than introducing another project. | Launch-critical | Before/after screenshots and an optional short interaction clip. | Freeze the exact edit sequence only after PAX-993 stabilizes; then build/run every snippet and verify the relevant hot-reload and responsive behavior. |
| **Templates and UI structure** (`template-language.md`) | Teach how a Pax tree is declared, configured, selected, imported, layered, and connected to events. | First interface. Read more to Components/Control Flow, Layout, PAXEL, and Events. | Owns template grammar at builder level, `@settings`, IDs/classes, element ordering/z-order, bindings/imports. Links value semantics and handlers. | **Keep; expand/reorder from fundamentals to conditional settings.** | Launch-critical | Source/tree/layer diagram and tiny layered component. | Parser/examples for every syntax form; confirm selector and conditional-settings limits. |
| **Reactive values: PAXEL and Properties** (`data-binding-expressions.md` + `state-properties.md`) | Make the reactive spreadsheet-like model precise, from a simple binding through computed Rust properties and subscriptions. | First interface and Templates. Read more to Events and public property API. | PAXEL file owns expression syntax and units; Properties file owns Rust reactive data. A short shared overview states the boundary once. | **Keep both strong files; add mutual orientation and ordering.** Do not merge them into an unwieldy article. | Launch-critical | One dependency graph and one computed-value example. | Compile/evaluate all expression samples; verify API names and generated links. |
| **Events and Rust application logic** (`event-handling-rust.md`) | Teach handler wiring, event data, state mutation, lifecycle, side effects, and coordinate/capture basics. | First interface, Templates, Properties. Read more to Accessibility and Native Controls and public event APIs. | Owns Rust-side response and event model. Links routing writes, animation triggers, and platform-specific controls. | **Keep; rewrite sequence and expand.** | Launch-critical | Click-to-property walkthrough and coordinate overlay. | Compile every handler; run pointer and touch path; verify local/window coordinates and propagation wording. |
| **Components, conditionals, and lists** (`components-composition.md`, absorbing `control-flow.md`) | Help the reader turn a single view into reusable components and data-driven structure. | Templates and reactive values. Read more to Layout and Routing. | Owns component pair, public inputs/defaults, nesting, slots, conditional trees, keyed identity. Links Rust state implementation. | **Expand `components-composition.md` and merge the useful control-flow material into it.** Preserve stable compatibility anchors if the old page has external links. | Launch-critical | Reusable card/list with insert/reorder; slot example. | Build all variants and verify key/slot behavior. |
| **Layout, styling, and responsiveness** (`layout-responsiveness.md`) | Give a dependable spatial model and produce a polished narrow/wide interface using units, alignment, autosize, themes, and conditional settings. | Templates. Read more to Visual Content, Scrolling, and Motion. | Owns geometry, percentage semantics, units, responsive policy, autosize, padding, layout role, and basic styling/theme composition. Links drawing details. | **Keep; expand and retitle if needed.** | Launch-critical | Responsive side-by-side, percent-position diagram, autosize demo. | Run at several viewport sizes/targets; validate percentage and text/native measurement caveats. |
| **Text, fonts, and images** (`text-fonts-images.md`) | Teach common content, asset loading, text/image behavior, and the rendered/native distinction without requiring API archaeology. | Layout. Read more to Accessibility/Native Controls and Compositing. | Owns text layout/style, fonts/assets, `Image`/`NativeImage`, selection/editability boundaries, and alternatives/format caveats. | **Keep and expand.** | Launch-critical basics | A focused shared visual-example component with source. | Verify assets/fonts, wrapping/selection/editability, images, alternatives, and target differences. |
| **Drawing and styling** (`drawing-styling.md`) | Introduce Pax's ordinary visual vocabulary, then provide durable discovery anchors for paths/SVG and qualified materials. | PAXEL and Layout. Read more to Motion and Compositing. | Owns primitives, fills/strokes, colors/gradients, themes, paths/SVG/draw ranges, and limited lighting/materials. | **Keep and expand.** | Useful, high launch value | Shared drawing example; path animation and labeled GlowButton proof. | Verify syntax, SVG subset/warnings, path ranges, theme behavior, and WGPU/Piet constraints. |
| **Compositing and effects** (`compositing-effects.md`) | Explain how rendered and native content layer, clip, mask, and occlude so builders can predict cross-surface results. | Layout and Drawing. Read more to Accessibility/Native Controls and How Pax Runs. | Owns opacity, masks, clipping, native islands/occlusion, and target/backend constraints. | **Keep and expand.** | Useful, high launch value | Shared compositing example, layer diagram, and native occlusion proof. | Verify mask contract, nesting, z-order, native projection, coordinate/target caveats. |
| **Accessibility and native controls** (`accessibility-native-controls.md`, replacing/renaming `input-native-controls.md`) | Give evaluators a directly discoverable, candid contract for native controls and the current accessibility foundation. | Events and Layout. Read more to Text/Images, Compositing, Scrolling, and Targets. | Owns native controls, two-way bindings, focus/keyboard behavior, platform coverage, image alternatives, current assistive-technology foundations, and explicit PAX-958 gaps. Pointer/touch mechanics remain in Events. | **Rename and restructure around the support contract rather than PhotoPicker.** Preserve a compatibility link from the old path. | Launch-critical basics | Small form, PhotoPicker inset, control/platform/accessibility matrix. | Exercise representative controls on each claimed target; audit wording and gaps with PAX-958. |
| **Scrolling collections** (`scrolling-viewports.md`) | Build a correctly sized scrollable list and understand viewport/content roles, events, native children, and culling implications. | Layout, Components/Lists, Events. Read more to How Pax Runs. | Owns scroller construction and scroll-specific behavior. Links generic layout/events/performance. | **Keep and expand** if it can reach useful depth; otherwise merge into Layout for the launch spine and defer the standalone chapter. | Launch-critical for practical apps | Minimal list and one richer `scroll-garden`/tiles example. | Web and Apple smoke tests, touch capture, native-child clipping, content sizing. |
| **Routing and multi-screen apps** (`routing.md`) | Build a nested multi-screen app, write routes, understand history, and prepare routed web output. | Components and Events. Read more to Motion and Deploy. | Owns route hierarchy/history/branches and, after landing, literal web route metadata/static topology. Links hosting fallback to deployment. | **Keep; semantic merge with PAX-869 only after route-metadata prerequisite lands.** | Launch-critical | Focused two-route example or `router-playground` excerpt; route-flow diagram. | Router tests/example; browser history/deep links; metadata/static files after merge. |
| **Motion and transitions** (`animation-motion.md`) | Add a first timeline and understand easing, structural enter/exit/reflow, and interruption behavior. | Properties, Layout, and Events. Read more to Routing for screen transitions. | Owns declarative motion semantics and limitations. Links layout/state triggers. | **Keep; simplify the opening and verify later sections.** | Useful, high launch value | Small first timeline plus transition-grid clip. | Run all key syntax and interruption/reflow cases on representative backends. |
| **Developer workflow and tools** (`developer-workflow.md`) | Turn first-run success into a sustainable edit/inspect/capture/debug loop using hot reload, local docs/examples, logs, screenshots, and inspection. | Getting Started. Read more to Targets/Build and troubleshooting references. | Owns hot-reload lane matrix and `pax-cli dev/docs` workflows. Links source mapping/API internals only for maintainers. | **New.** Remove the long workflow digression from Getting Started after coordination. | Launch-critical | Hot-reload clip, `look` result, short command table. | Exact help/output verification; all-target template reload; web/macOS logic reload; locate public event-driving surface. |
| **Targets, build, and deployment** (`targets-build-deploy.md`) | Let a builder choose a target, distinguish host requirements, produce debug/release artifacts, deploy web output, and understand the Apple signing/distribution handoff. | Getting Started; Routing for routed web apps. Read more to How Pax Runs. | Owns exact application targets, workstation matrix, build modes/output, release cartridges at a practical level, web hosting, and Apple boundary. Links internals to appendix. | **New.** | Launch-critical | Target/host matrix, build-output tree, one web deployment recipe. | Highest: launch RC on all three workstation OSes; all targets on macOS; route fallback/metadata; artifact paths. |
| **How Pax runs: runtime and performance** (`how-pax-runs.md`) | Explain reactive invalidation, rendering/compositing, culling, backend selection, and performance reasoning without unsupported promises. | Authoring fundamentals and Layout. Read more from What is Pax; link onward to Maintainer Architecture and profiling APIs/tools. | Owns builder-facing runtime model, WebGPU/Piet policy, performance mechanisms, measurement guidance, and honest variability. | **New standalone article, replacing the empty performance page. Split useful sections out of `architecture-runtime-cartridge.md`; keep implementation detail in appendix.** | Useful; launch-critical for accurate evaluation | Reactive/render flow and backend-selection diagrams; reproducible profile recipe later. | Source/test audit; representative workload measurement if any numbers are retained. |
| **Public API Reference** | Let builders inspect exact supported Rust/component APIs after learning the concepts. | Any conceptual article. Links back to prose examples. | Owns generated `pax-runtime-api` and `pax-std` signatures. | **Keep generated, but group as Reference and improve cross-links/labels.** | Launch-critical reference | Generated examples only where maintained. | Regenerate and link-check; sample signature accuracy. |
| **Maintainer architecture and internal API** | Preserve compiler/runtime/cartridge/backend detail without interrupting the builder journey. | How Pax Runs. | Owns internal crate APIs and curated, status-labeled architecture documents. | **Move/present as an Appendix or separate Maintainer Reference. Do not expose all design drafts as product docs.** | Useful, not on canonical launch journey | Architecture diagrams only where current. | Verify implemented-vs-proposed status and binary-baking coverage. |

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
| Accessibility and native controls | `/accessibility-native-controls/#current-support` and `/accessibility-native-controls/#native-controls` |
| Hot reload and inspection | `/developer-workflow/#hot-reload` and `/developer-workflow/#inspect-a-running-app` |
| Targets and deployment | `/targets-build-deploy/#targets-and-workstations` and `/targets-build-deploy/#deploy-web` |
| Runtime and renderer behavior | `/how-pax-runs/#render-backends` |

These are proposed contracts, not current URLs. Final slugs and heading text should be approved before prose so PAX-974, PAX-869, and PAX-987 can link once and avoid churn.

## D. Canonical launch journey

The shortest navigable path should be seven visible successes, not an exhaustive reading assignment:

1. **Place Pax.** Read `What is Pax?` and understand that a Pax app combines declarative `.pax` templates, formula-style PAXEL/reactive properties, Rust logic, and a portable runtime targeting web and Apple platforms.
2. **See it run.** Install the launch CLI, create the final starter, run it on web, and confirm the expected screen. The page shows the generated file tree and one recovery path for common setup failures.
3. **Complete the authoring loop.** Change template content, bind it to a property with PAXEL, handle a click in Rust, set the property, and watch the UI update. Use hot reload where the target supports the changed lane.
4. **Make it a real interface.** Extract a small component, render a keyed list or conditional region, then make the layout respond at narrow and wide sizes. This is the first practical aha: the same declarative model covers structure, state, and responsive visual behavior.
5. **Choose a deeper feature.** Follow deliberate links to routing, motion, drawing/paths, native controls, scrolling, or inspection. Feature chapters provide breadth without blocking the core path.
6. **Form an app.** Add or inspect a second route, understand history and route-specific web considerations, and confirm the target/backend caveats relevant to the chosen feature set.
7. **Build and ship.** Choose web, macOS, iOS, or iPadOS; distinguish the development host requirements; build the appropriate debug/release artifact; deploy the web output or hand the Apple artifact to the normal signing/distribution workflow.

At each step the next link should be explicit. A reader should never need the generated API reference to finish steps 1–4, but each conceptual chapter should link the exact public API when the reader is ready to generalize.

### First guided application dependency

The project produced by default `pax-cli create` should remain the single
project used by the README, Getting Started, and the first guided interface.
PAX-993 owns its exact content, bundled-example registry, and create behavior;
`examples/src/*` remains the canonical source. Because that work is still
changing, this map intentionally does not freeze a starter name, visual
concept, component tree, property, interaction, screenshot, or tutorial edit.

PAX-975 should consume a stable command/tree/first-edit handoff from PAX-993
and must not design another starter in parallel. Until then, Getting Started can
be outlined around the invariant install -> create -> run journey, but its
generated file tree, expected screen, and exact tutorial continuation remain
explicit verification gates.

## E. Sequential editorial plan

PAX-860's editorial process is part of the acceptance criteria: outline/research first, Zack feedback, then one chapter at a time. No parallel chapter delegation is appropriate because the terminology and examples must converge sequentially.

### Checkpoint 1: approved spine

Checkpoint 1 completed on 2026-08-15. The accepted decisions are recorded in Section G. The public prose and `SUMMARY.md` remained unchanged during discovery; article work now proceeds through one outline/evidence review and one prose review at a time.

### Checkpoint 2: establish first-touch and terminology

Write and review, one at a time:

1. `What is Pax?`
2. `Install, create, and run`
3. `Build your first Pax interface`

For each article: agree on a short outline, select/verify its example, draft prose, run the commands/snippets, then request feedback before proceeding. After these three, check that the same terms and authoring loop agree with the README direction from PAX-974 and the accepted PAX-945 claims ledger.

On 2026-09-06, Zack approved the tutorial outline and advancing to Templates
while PAX-993's first-edit handoff remains in flux. This changes the drafting
sequence only; the public reading order and the tutorial's launch requirement
remain unchanged. Resume the tutorial from its accepted outline after handoff.

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
12. Accessibility and native controls
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

## F. Website-gallery awareness and ownership boundary

The 2026-08-12 rebase adds 41 authored `FeatureCard` records under `examples/src/pax-website/src/feature_gallery/`. They are useful discovery evidence: they confirm the breadth builders may want to explore and reinforce that Drawing/Styling and Developer Workflow are among the current prose gaps. They do not create a parallel documentation backlog.

Zack's 2026-08-12 direction makes the ownership boundary explicit:

- **PAX-869 owns the gallery:** card selection, copy, categories, state/target treatment, destinations, visual annealing, and filling out gallery examples.
- **PAX-975 owns the learning path:** mental model, first success, canonical exposition, chapter relationships, command verification, and deliberate read-more links.
- **PAX-976/PAX-855 own broader example curation and chapter-example production.** PAX-975 chooses where an example improves comprehension; it does not require a runnable embed for every capability.

Accordingly, PAX-975 will not edit gallery source, maintain a second feature-callout list, decide card badges, or gate completion on all 41 cards. It will verify builder-facing behavior independently and write the minimum coherent concepts required to understand and compose the shipped system. PAX-869 may link cards to the resulting stable articles/anchors after it decides the website presentation.

The gallery does not replace these docs-only foundations:

- what Pax is, its maturity, OSS boundary, and template/PAXEL/Rust mental model;
- install/create/run, generated project anatomy, and first successful edit;
- component definitions, ownership, defaults, control flow, keyed identity, and slots;
- element ordering/z-order and percentage positioning semantics;
- accessibility status and platform support boundaries;
- targets versus development hosts, build artifacts, deployment, and Apple signing handoff;
- public versus maintainer API/design material; and
- practical runtime/rendering/performance reasoning without product-wide numeric promises.

Two cross-surface facts remain relevant to docs verification even though PAX-975 does not own the cards: Rust logic reload is opt-in rather than the default, and the accepted event-driving claim still lacks an obvious public `pax-cli dev` click/touch command in this tree. Document only the verified public behavior.

## G. Approved Checkpoint 1 decisions

Zack's reviews through 2026-08-15 establish these decisions:

1. **The three-step first-touch split is approved:** `What is Pax?` -> `Install, create, and run` -> `Build your first Pax interface`.
2. **Getting Started is also a standalone external front door.** The website, README, and other Get Started CTAs may link directly to it. It assumes intent to try Pax, not prior navigation from the beginning of the book.
3. **Builder and maintainer reference will be separated.** Public `pax-runtime-api`/`pax-std` remain visible; internal APIs and curated architecture move under a clearly labeled appendix/reference.
4. **Stable slugs and anchors will be frozen early**, with a version-aware publication contract rather than ad hoc surface-specific URLs.
5. **The canonical starter is delegated to PAX-993.** `examples/src/*` remains authoritative; PAX-975 consumes the finished create/run/tree/first-edit contract rather than owning its design or packaging.
6. **The feature gallery remains under PAX-869.** PAX-975 uses it as discovery evidence but does not own card editing, destinations, badges, or example completion.
7. **The first tutorial uses the PAX-993 starter in place.** It will not introduce another hello-world project. PAX-993 owns the evolving source and packaging; PAX-975 owns the tutorial prose after receiving a stable handoff.
8. **Runtime and rendering receive a standalone builder article:** `how-pax-runs.md` after the authoring fundamentals.
9. **Accessibility and native-control coverage receive a directly discoverable article:** `accessibility-native-controls.md`, with a compatibility path from the old input/native-controls location.
10. **The topology defaults are approved:** separate Text, Drawing, and Compositing articles; control flow taught with Components; one Targets/Build/Deploy article; and a separate Developer Workflow article.

### Clarification: visual-content topology versus examples

The earlier "one bounded Visual Content article" proposal was about navigation and concept ownership, not about whether examples should be embedded. The current book now contains one useful partial article and two nine-line stubs:

- `text-fonts-images.md` now owns hosted/local fonts and reactive image-source basics;
- `drawing-styling.md` remains a stub despite drawing, styling, path, SVG, and material concepts needing a builder-facing home; and
- `compositing-effects.md` remains a stub despite native/rendered composition, masks, clipping, and occlusion needing a builder-facing home.

Combining them into `visual-content.md` would reduce launch prose, but it would discard the newly useful text/media anchors and blur three different builder questions: content, drawing, and composition. Keep all three canonical owners for the learning model itself, independent of website-card needs.

The `/examples` and `ExampleHost` idea improves either topology. It should be treated as a shared proof system:

1. select the strongest existing example or author a small named component for each article's central idea;
2. expose reusable proofs under `/examples` when PAX-976 adopts them;
3. embed or host a relevant component beside an article with an explicit `ExampleHost` source manifest when doing so materially improves comprehension;
4. keep essential prose and copyable source in the article so learning does not depend on a live embed, WebGPU availability, or JavaScript execution;
5. label target/backend-specific examples at the embed boundary; and
6. let PAX-976 own the launch example catalog/hosting and PAX-855 own deeper chapter examples, while PAX-975 owns the canonical explanation and article placement.

Approved topology: **keep and expand the three existing articles rather than introduce `visual-content.md`**, but bound their launch roles tightly and require a runnable embed only where it materially improves comprehension:

| Article | Launch-owned concepts | Deliberate links rather than duplication |
| --- | --- | --- |
| `text-fonts-images.md` | Text layout/style, fonts/assets, rendered `Image` versus native `NativeImage`, selectable/editable text boundaries, image alternatives, and supported-format caveats. | Link controls/accessibility to Accessibility and Native Controls; masks to Compositing; detailed properties to public APIs. |
| `drawing-styling.md` | Vector primitives, fills/strokes, colors/gradients, inherited styling/themes, paths, SVG import/ejection, draw ranges, and a clearly advanced/limited lighting section. | Link unit/expression syntax to PAXEL, motion to Animation, masking/layer semantics to Compositing. |
| `compositing-effects.md` | Source order/z-order recap, opacity, masks, clipping, rendered/native element islands, occlusion, and backend/target constraints. | Link basic geometry to Layout, native controls to Accessibility and Native Controls, and renderer selection to How Pax Runs. |

This costs more than one combined article. If launch time becomes the binding constraint, preserve the current Text article and finish Drawing/Styling plus Compositing to a concise orientation-and-caveats standard; defer exhaustive examples and leaf-API coverage rather than collapsing the topology.

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

Checkpoint 1 is complete. `What is Pax?` and Getting Started have completed
their outline and initial prose reviews. The first-interface tutorial outline
is accepted; its exact steps depend on the PAX-993 handoff. With Zack's approval,
Templates is the next active article, starting with its outline review.

## H. `What is Pax?` outline and evidence packet

**Status:** Outline and initial prose accepted 2026-09-04. The current
`what-is-pax.md` is the approved starting point; final navigation and link
checks remain part of the launch-wide editorial pass.

### Reader promise and editorial boundary

The article should take roughly five minutes and require no prior Pax knowledge. A Rust developer evaluating GUI frameworks should leave able to:

1. describe Pax in one accurate sentence;
2. explain the template -> PAXEL/property -> Rust handler -> reactive update loop;
3. distinguish the shared authoring/runtime model from target-specific rendering and native integration;
4. name the current application targets and understand the pre-1.0 maturity boundary; and
5. choose the standalone Getting Started path without reading the rest of the book first.

Target length: approximately 1,000-1,400 words plus one durable authoring-loop
diagram. A representative application image is optional once its source and
framing are stable.

This article owns category, audience, mental model, high-level runtime/target scope, OSS boundary, and maturity. It must not become a feature inventory, installation guide, syntax reference, architecture specification, performance benchmark, or target deployment guide.

### Proposed article skeleton

#### `# What is Pax?`

- Lead with the accepted category: **Pax is a language-first GUI framework for Rust.**
- Expand once: declarative `.pax` templates, reactive PAXEL/properties, Rust application logic, and a portable runtime.
- Name the current outcomes immediately: web, macOS, iOS, and iPadOS.
- Identify the launch reader: Rust developers and Rust-capable creative technologists who want application structure without giving up direct control over layout, vectors, composition, and motion.
- Add one representative application image only after its source and framing are stable; the article does not depend on a starter screenshot.

#### `## A language built for interfaces`

- A Pax component is a Rust type paired with a `.pax` template.
- The template owns interface structure, settings, bindings, control flow, event routing, responsive choices, and motion declarations.
- PAXEL expressions read properties and derive display/layout values without becoming the place for side effects.
- Rust owns state changes, event handlers, data and platform integration, and other imperative work.
- Avoid comparing every construct to HTML/CSS or React; one brief contrast is enough if it clarifies the division of responsibility.

#### `## The Pax authoring loop`

- Present the canonical four-beat loop:
  1. the template declares structure and an event binding;
  2. PAXEL derives visible values from reactive properties;
  3. an event invokes a Rust handler; and
  4. Rust updates a property, causing dependent values, layout, and rendering work to update.
- Keep this conceptual rather than binding the article to the changing starter. The first tutorial will own the concrete end-to-end trace after PAX-993 stabilizes.
- Introduce the spreadsheet analogy once: Pax tracks dependencies like formula cells, but the output is an interface.
- Read more to Templates, PAXEL, Properties, Events/Rust, and the first guided interface.

#### `## One interface model, several real targets`

- Explain at one level of abstraction: the compiler prepares a program/cartridge; the runtime expands the reactive scene; a target chassis integrates rendering, input, native elements, and platform services.
- State the exact current application targets: web, macOS, iOS, and iPadOS.
- Explain that rendering is target-aware: current web builds select WebGPU where policy/support permits and use a Piet/CPU fallback where necessary; Apple builds use their native chassis and GPU path.
- Mention native text, controls, and scrolling as supporting proof of target integration, not the headline definition of Pax.
- Read more to How Pax Runs and Targets/Build/Deploy. Keep cartridges, binary baking, renderer internals, and artifact paths out of this article.

#### `## Why builders choose Pax`

- Organize the explanation under the accepted three ideas without turning them into marketing-card headings:
  - **A language built for interfaces:** concise declarative structure and derived values, with Rust as the escape hatch and application layer.
  - **Real application architecture with a higher creative ceiling:** components, state, routing, events, and responsive layout share a scene with vectors, paths, masks, composition, and motion.
  - **A portable runtime built to ship:** one interface model reaches current web and Apple targets while retaining target-specific integration and fallback behavior.
- Treat human/agent tractability as a consequence of structured source and inspectable tools, not the category headline.
- Keep philosophy to one sentence about expanding the expressive range of software; reserve the “love letter to the web” argument for the launch post.

#### `## Current scope and maturity`

- Define **ready for builders** as a maturity transition: coherent and runnable enough to evaluate and build with, not 1.0 or universal production readiness.
- State that Pax remains pre-1.0, APIs can evolve, and feature/backend maturity differs by target.
- Rust is the current application language. Do not present future JavaScript/Python logic as current capability.
- Accessibility has shipped foundations through native text/controls and image alternatives; broader reading order, tab order, annotation, and audit work remains open. Link the dedicated Accessibility and Native Controls article.
- The current framework, compiler, runtime, CLI, hot reload, designtime tooling, inspection, screenshots, and event-driving capability ship OSS. No companion is required; any future companion is additive.

#### `## Start building`

- Primary CTA: **Install, create, and run** at `/getting-started/`, explicitly described as a standalone path.
- Link the standalone Getting Started path without naming or describing the changing generated project.
- Secondary paths: inspect examples/source or read How Pax Runs, depending on whether the reader wants proof or architecture.

### Evidence and claims ledger for this article

| Planned claim | Current evidence | Allowed precision / verification note |
| --- | --- | --- |
| Pax is a language-first GUI framework for Rust. | Accepted PAX-945 category; current README opening. | Use as the category sentence. “UI engine, language, and portable runtime” is the technical expansion, not a competing headline. |
| `.pax` templates own declarative structure, bindings, control flow, events, settings, and motion. | `pax-language/src/pax.pest`; macro template-source validation; current language docs/examples. | Say declarative by grammar. Do not imply arbitrary Rust statements execute inside templates. |
| PAXEL derives reactive values; Rust owns side effects and state changes. | PAX-945 vocabulary; `pax-language` expression/property-resolution code; runtime computed-property/cartridge paths; README example. | Keep PAXEL side-effect-free in public wording. Current application language is Rust. |
| Property changes dirty dependent work. | `pax-runtime-api` Property APIs; runtime property graph, cartridge dependency collection, engine dirty render planning. | Explain the mechanism qualitatively. Do not promise zero idle work or minimal work for every workload. |
| Compiler -> cartridge/program -> runtime -> chassis is the high-level execution model. | `pax-compiler::prepare_cartridge_sources`; `building::build_project_with_cartridge`; `pax-manifest::program_ir`; runtime engine; web/Apple chassis crates. | One paragraph only; implementation/binary-baking detail belongs in How Pax Runs and the maintainer appendix. |
| Current application targets are web, macOS, iOS, and iPadOS. | `pax-compiler::RunTarget`; CLI possible values; README support table; PAX-945 claims ledger. | Always name the four. Development workstations are a separate matrix. |
| Rendering is target-aware, not universally one backend. | `pax-chassis-web::get_render_context`; `pax-chassis-common` GPU path; PAX-945 renderer policy. | WebGPU where policy/support permits, Piet/CPU fallback where required. Avoid “all rendering is GPU.” |
| Pax combines application structure with vectors/composition/motion. | Runtime/standard-library APIs and current routing, layout, drawing, compositing, and motion examples/tests. | Use representative categories, not an exhaustive feature list or equal-maturity claim. |
| Native elements participate in the Pax scene. | Runtime rendering/native message path; `pax-std` text/forms/scroller/media; PAX-945 vocabulary. | Supporting architecture. Avoid implying every primitive is a native widget or every control exists on every target. |
| Pax is ready for builders and remains pre-1.0. | Accepted PAX-945 maturity definition; README Project Status. | Never translate this into “production-ready,” “finished,” or equal maturity everywhere. |
| The current framework and developer tooling ship OSS without a required companion. | PAX-973 final and correction comments; repository licenses and source. | A future companion may be mentioned only as additive future work, if mentioned at all. |
| Accessibility foundations are present but incomplete. | PAX-945 claims ledger; native text/controls and image-alternative APIs; PAX-958 tracking. | Link the dedicated article. No certification, comprehensive screen-reader, or full-accessibility claim. |

### Claims deliberately excluded

- “Runs anywhere,” “any screen,” “write once, run everywhere,” or native Linux/Windows/Android support today.
- Blanket 240 FPS, universal GPU rendering, zero-compute-at-rest, fixed memory/size, or other product-wide numeric promises.
- Full accessibility, production readiness, API stability, or feature parity across targets.
- Pax Designer or a future companion as the product, prerequisite, or current workflow.
- Future JavaScript/Python application logic as shipped behavior.
- A long catalogue of standard-library components or gallery features.

### Media placeholder

**Required: authoring-loop visual**

Dynamics to show:

- distinct template, PAXEL/property, Rust handler, and reactive-update responsibilities;
- one directional event path into Rust and one dependency-driven update path back to the interface; and
- conceptual flow without exposing compiler/runtime internals prematurely.

Three candidate treatments:

1. **Four-beat loop:** four compact nodes around a cycle, with “event” and “property update” distinguished by arrow style.
2. **Split component:** `.pax` source on the left, Rust on the right, and a running interface below; highlighted lines connect to the visible state change.
3. **Spreadsheet-to-pixels strip:** a property cell fans out to text/layout/visual properties, with the Rust handler as the only mutation point.

Recommendation: use the four-beat loop in this article for longevity and reserve an annotated starter-source treatment for the first tutorial after PAX-993 stabilizes. Do not add a second architecture diagram here.

### Verification before prose review

1. Recheck the article's category, maturity, target, accessibility, and OSS sentences against PAX-945/PAX-973 immediately before drafting.
2. Keep starter-specific names, files, behavior, and media out of this article while PAX-993 is changing.
3. Validate every read-more destination against the approved filename/anchor plan, even if the destination article is still an outline.
4. Search the draft for excluded claims and for accidental future-as-present language.
5. Keep runtime/backend detail consistent with `how-pax-runs.md`; do not duplicate its eventual explanation.

### Editorial choices for Zack at this outline checkpoint

1. **Opening hierarchy:** lead with the category sentence, then place “Pax is ready for builders” in Current Scope and Maturity. Alternative: use the launch flag as the opening headline. **Recommendation:** category first; it answers the docs reader's question faster and keeps the maturity phrase meaningful rather than promotional.
2. **Philosophical register:** include one sentence about expanding software's expressive range, while leaving the “love letter to the web” thesis to the launch post. Alternative: omit the vision entirely from the launch docs entry. **Recommendation:** keep one restrained sentence so the creative motivation is present without delaying the mental model.

Both recommended choices were approved on 2026-09-04 and are reflected in the
prose draft.

## I. `Getting Started` outline and evidence packet

**Status:** Outline and initial prose accepted 2026-09-06, including the
cross-platform opening that identifies web as this guide's first-run target.
Public-package, clean-workstation validation remains a release gate.

### Reader promise and editorial boundary

This page is Pax's primary external CTA destination. It must work for a reader
who already wants to try Pax and has not read `What is Pax?` or any other
chapter. The reader should leave with:

1. the correct prerequisites for their development workstation;
2. a CLI installed from the public release channel;
3. a newly generated project running in a web browser;
4. a small, accurate map of the generated project; and
5. an obvious next step when the run succeeds or a useful recovery path when it
   does not.

The primary path should take about ten minutes excluding toolchain downloads
and first compilation. The page should not promise a fixed elapsed time.

This article owns workstation preparation, CLI installation, project creation,
the first explicit web run, the visible success checkpoint, minimal project
anatomy, and first-touch troubleshooting. It does not own the first authored
interface change, the complete hot-reload model, formatting, build artifacts,
deployment, project metadata, public web files, or native-target setup beyond a
brief boundary and link.

### Proposed article skeleton

#### `# Getting Started`

- Promise one result: a new Pax project running in the browser.
- State that the page is self-contained and targets the public CLI release,
  rather than a source checkout of the Pax monorepo.
- Offer a compact prepared-workstation path near the top, with a direct link to
  the operating-system prerequisites for readers who need them.

#### `## Quick start`

- Use one canonical command sequence:

  ```sh
  cargo install pax-cli
  pax-cli create my-first-project
  cd my-first-project
  pax-cli run --target=web
  ```

- Keep `--target=web` explicit even though web is currently the CLI default; it
  teaches the target boundary and remains easy to scan from external CTAs.
- Explain that the first installation and build can take time without quoting a
  duration.
- Describe the observable success state only after the release candidate and
  PAX-993 output are stable: terminal message, local URL/browser behavior, and
  the actual generated application screen.
- Give the stop command or process behavior once verified.

#### `## Prepare your workstation`

- Begin with a compact support statement: macOS, Debian/Ubuntu Linux, and
  Windows support the public web install/create/run path; Apple application
  targets require macOS and Xcode.
- Provide separate, copyable macOS, Debian/Ubuntu, and Windows subsections.
- State prerequisites at the public-package boundary. The source-linked test
  harness installs Node.js/npm because it can rebuild ignored TypeScript
  artifacts; normal published-CLI users should receive those artifacts and
  should not be told to install Node unless the release-candidate test proves
  otherwise.
- Verify rather than assume the exact Rust toolchain, WebAssembly target,
  `wasm-pack` version/installation flags, Linux system packages, Windows MSVC
  and Clang components, and macOS Command Line Tools requirement.
- Keep contributor/source-checkout setup out of this page.

#### `## Install the Pax CLI`

- Make `cargo install pax-cli` the canonical release-channel command.
- Verify whether launch prose should add `--locked`, an explicit version, or an
  upgrade/reinstall note. Do not copy source-linked `--path` or `--force`
  commands into the public journey.
- Add one version check only if the CLI exposes a stable, useful command and the
  release-candidate transcript confirms its output.

#### `## Create a project`

- Document `pax-cli create <destination>` without naming or describing the
  changing default project.
- Explain the destination rule and existing-directory failure in one sentence
  after verifying the final behavior.
- Treat `--example=<name>` and example discovery as an optional callout only
  after PAX-993 stabilizes its public contract. The default path must not depend
  on choosing an example.
- Capture the exact successful terminal handoff after the generated-project
  contract lands.

#### `## Run it on the web`

- Change into the generated directory and run
  `pax-cli run --target=web`.
- Explain which terminal should remain open, whether the browser opens
  automatically, the local address, and how to stop the process, using an
  observed release-candidate transcript rather than assumptions.
- Define first success visually and with a lightweight technical check. Do not
  teach template editing here; the next tutorial owns the first change.

#### `## Project anatomy`

- Include a small annotated tree only after the PAX-993 handoff stabilizes.
- Explain only the durable roles: Cargo manifest, Rust application logic,
  `.pax` templates, assets, launch wrapper if present, and generated `.pax/`
  build state.
- Mark generated/build directories as non-source and avoid enumerating every
  file.
- Link the component pair to the first guided interface and the deeper
  Templates/Properties/Events chapters.

#### `## When something goes wrong`

- Cover only high-frequency first-touch failures with exact symptoms and a
  bounded remedy: CLI not on `PATH`, missing Rust/WebAssembly tooling, missing
  OS build dependencies, destination already exists, occupied local port, and
  the distinction between a long first build and a stalled build.
- Point compiler or template errors to their eventual troubleshooting owners;
  do not turn Getting Started into a general error catalogue.
- Never suggest monorepo path patches or library-development flags to public
  users.

#### `## Where to go next`

- Primary continuation: build the first Pax interface using the project that is
  already running.
- Secondary links: developer workflow for hot reload/inspection, targets and
  deployment for native or release builds, and `What is Pax?` for readers who
  arrived directly and now want the mental model.
- Keep the CTA ending concise; advanced reference sections should not continue
  below it once their destination articles exist.

### Current-content disposition

| Current `getting-started.md` material | Launch destination |
| --- | --- |
| macOS/Linux/Windows setup | Keep, but tighten around public-package prerequisites and current clean-host evidence. |
| repeated per-OS create/run snippets | Consolidate into one canonical Quick Start before the OS setup choices. |
| formatting | Move to Developer Workflow. |
| full hot-reload policy and precedence | Move to Developer Workflow; Getting Started may mention only what the first run does. |
| web `public/` files | Move to Targets, Build, and Deployment. |
| project metadata and Apple packaging keys | Move to Targets, Build, and Deployment. |
| asset-directory role | Keep one line in Project Anatomy; link deeper image/asset coverage later. |

Until those destination articles are written, preserve the existing advanced
sections below the primary path under a clearly secondary heading. Relocate
them only when their new canonical homes exist so the sequential rewrite does
not temporarily discard accurate public guidance.

### Evidence and verification ledger

| Planned claim or command | Current evidence | Verification gate before prose is final |
| --- | --- | --- |
| `cargo install pax-cli` is the public installation path. | README and current Getting Started; crates release workflow. | Install the launch candidate from the real public registry on each supported workstation. |
| `pax-cli create <destination>` creates a standalone Rust-backed Pax project. | Current CLI/compiler; PAX-993 evolving bundled-project implementation. | Wait for PAX-993's stable handoff; inspect output, failure behavior, dependency versions, and source/build separation. |
| `pax-cli run --target=web` is the canonical first run. | CLI default/target enum, README, create output direction. | Record browser launch, URL, terminal output, shutdown, and visible result from the public package. |
| Web first touch is supported on macOS, Debian/Ubuntu, and Windows. | First-touch VM harness and README support table. | Rerun against the public launch candidate rather than the source-linked workflow. |
| Published-CLI users do not need Node.js/npm for normal first touch. | First-touch harness explicitly distinguishes its source-linked TypeScript rebuild from packaged artifacts. | Confirm the published CLI contains the required web interface bundle and no clean-host step invokes npm. |
| Apple targets require macOS/Xcode. | compiler Apple build path and accepted target/workstation language. | Keep detailed simulator/device/signing setup for Targets, Build, and Deployment. |
| The generated tree has a stable component/source anatomy. | PAX-993 worktree and generated-project tests. | Freeze no filenames, screenshots, or explanation until its changing default project is accepted and merged. |
| The old publish-channel mismatch is resolved for launch. | PAX-896 recorded historical crate/template skew; PAX-906 owns release validation. | A project created by the published CLI must complete a web build without path patches or monorepo access. |

### Bounded verification run

Before this article is marked launch-ready, capture one transcript per clean
workstation from the public release candidate. The source-checked draft can
receive editorial review while this release validation remains outstanding:

1. install the documented prerequisites and `pax-cli`;
2. create into a new directory outside the Pax repository;
3. inspect the generated dependency sources for accidental local paths;
4. run the explicit web target and confirm the expected application is served;
5. stop and restart once; and
6. exercise each documented first-touch recovery at least synthetically or
   through a narrow automated test.

Source-linked smoke results remain useful diagnostics, but they are not a
substitute for this public-package path.

### Editorial choices for Zack at this outline checkpoint

1. **First visible section:** put the four-command Quick Start before the
   operating-system instructions, with a clear “need prerequisites?” link.
   Alternative: require everyone to choose an OS first. **Recommendation:**
   commands first; it serves experienced Rust users and makes the external CTA
   feel immediate without hiding setup help.
2. **Definition of first success:** stop after a verified running application
   and project-anatomy orientation. Alternative: include a tiny `.pax` edit in
   this page. **Recommendation:** stop after run; the next article can give the
   first edit enough context and keeps the three-step opening spine distinct.
3. **Transition while destination articles are absent:** retain formatting,
   hot-reload, public-files, and metadata reference below the new primary path,
   then move each section when its owner article is written. Alternative:
   remove or create placeholder destinations immediately. **Recommendation:**
   retain temporarily so this sequential process never reduces currently
   accurate coverage.

All three recommendations were approved before drafting.

### Draft disposition and checks — 2026-09-06

- The four-command Quick Start leads the page. Workstation instructions are
  separate from runtime targets, and installation/create/run explanations are
  shared across OS sections. The existing Windows setup script is unchanged,
  inside a collapsible section.
- Project anatomy covers durable file roles only. No PAX-993 project name,
  behavior, screenshot, or exact generated tree is frozen in this draft.
- Formatting, hot reload, public files, and project metadata are unchanged
  apart from heading depth beneath Further Reference. Their fragment IDs and
  the old `development-environment-setup` anchor are preserved.
- The PAX-869 worktree's pending Getting Started changes are route metadata
  additions. This draft does not edit that reference body or import unmerged
  metadata behavior.
- The current CLI source confirms `--version`, `create <destination>`,
  `run --target=web`, and `--verbose`. The compiler rejects an existing create
  destination. Its web servers print a loopback URL and select an available
  port; the CLI handles Ctrl-C and cleans up child processes. These are source
  checks, not a claimed public-package end-to-end run.
- OS setup was compared with `scripts/first-touch/README.md` and its guest
  prerequisites. The former Ubuntu validation sentence was removed because
  source-linked smoke evidence does not establish a clean public-package run.
  The retained dependency lists still need the release-candidate check above.
- `cargo install pax-cli` remains unpinned in this draft. Once the release
  version is known, PAX-987 should render the matching exact CLI version in
  immutable docs. Cargo supports exact version selection with `--version`;
  adding `--locked` also needs validation against the packaged lockfile.
  Reference: [Cargo install](https://doc.rust-lang.org/cargo/commands/cargo-install.html).
- Validation passed: mdBook 0.4.47 build; all 29 rendered local links/anchors
  in this article; syntax-only parsing of all nine shell blocks; exact
  preservation of the Windows setup commands and four reference bodies;
  and `git diff --check`. The Windows disclosure and its code render in HTML.
- A previously built local CLI advertises legacy flags/defaults that disagree
  with current source. It was excluded as proof of the current release. No
  workstation packages were installed or upgraded for this prose pass.
- The next-step links currently use existing chapters. Add the approved
  First Pax Interface, Developer Workflow, Targets/Build/Deploy, and What is
  Pax destinations when those pages are wired into the book. `SUMMARY.md`
  remains unchanged.

Getting Started's initial prose review is complete. Section J is the next
outline/evidence packet. Do not begin the tutorial's prose before its outline
review and stable PAX-993 first-edit handoff.

## J. `Build your first Pax interface` outline and evidence packet

**Status:** Outline accepted, 2026-09-06. Prose awaits the stable PAX-993
first-edit handoff. No tutorial prose or example changes.
Proposed file: `first-pax-interface.md`. Launch-critical.

### Reader promise and scope

Starting with the project already running from Getting Started, make one
coherent, personal change to its interface and understand how that change works.
The reader should finish able to locate a component's Rust/template pair,
change its appearance, bind a value, update it in an event handler, and check
the result at narrow and wide sizes.

Use the same generated project throughout. PAX-993 owns the starter's design
and source. This chapter owns the guided editing sequence and its checkpoints.
Keep the essential interaction small enough that the reader can explain the
whole template -> expression/property -> Rust handler -> visible update loop.
Explain deeper syntax through links to the canonical chapters.

Prerequisites: Getting Started completed; an editor; basic Rust familiarity.
What is Pax? is optional context. The tutorial supplies the Pax concepts needed
at each step without requiring the reader to visit another chapter mid-step.

### Proposed sequence

| Section | Reader action and outcome | Checkpoint |
| --- | --- | --- |
| Find the part you'll change | Locate the main component and the small child involved in the edit. Explain `#[file(...)]`, the paired `.pax` file, and the child tag using those actual files. | The reader knows which source owns the visible element. |
| Make a visible change | Personalize one literal visual setting in the selected template. Introduce only the units, placement, or style needed for that edit. | Save the `.pax` file and see the change in the running app. |
| Give the interface a value to follow | Introduce or adapt one simple `Property<T>` and replace the literal with a PAXEL binding. Show its initialization and one derived value. | The rendered result follows the property's value. |
| Respond to an action | Add or adapt one input binding and a short Rust handler that sets that property. Read the entire small loop together. | Activate it twice and observe the expected state and visual changes. |
| Pass the value to a component | Follow one value from parent to an existing child's public property. Keep the child's implementation details bounded. | The child updates from the parent's value without a second copy of the state. |
| Check a smaller screen | Resize, inspect the relevant percentage sizing or breakpoint, and adjust one layout rule if the selected edit needs it. | The personalized interface remains legible and usable at both recorded sizes. |

End with a brief recap of the three authoring roles and direct read-more links.
The core path ends after the resize check. Lists, slots, routing, animation
internals, asset pipelines, lighting, masking, and performance belong to their
own chapters. Existing visual richness can remain in the running project
without making its entire implementation a prerequisite for the first edit.

### Learning and verification choices

- **Prefer modifying a small existing feature.** The exact property and action
  must come from the accepted starter's first-edit handoff. Avoid a property
  that an animation or per-frame hook continually overwrites. If no isolated
  edit is available, return that concrete teaching need to PAX-993 before prose.
- **Read and use an existing child component.** Defer component extraction,
  module-organization choices, slots, and custom events to Components and
  Composition. This keeps file-management work from interrupting the loop.
- **Keep the default reload policy.** The first template edit demonstrates Pax
  hot reload. At the Rust edit, explicitly stop and rerun
  `pax-cli run --target=web`. Link to optional web/macOS logic reload rather
  than changing the reader's configuration. Do not promise state preservation
  across either reload or restart.
- **Teach ordering where it matters.** If the edit layers text or another
  foreground element over a background, explain that earlier siblings appear
  on top at that point in the example.
- **Use reproducible checkpoints.** Each step names the file, complete changed
  declaration or handler, expected result, and one likely recovery. Avoid
  unexplained ellipses inside code the reader must paste.

### Current evidence and handoff boundary

| Evidence inspected | What it establishes | Remaining boundary |
| --- | --- | --- |
| PAX-993 issue and all comments; PAX-975's starter handoff comment | Default `create` and canonical `examples/src/*` ownership; stable first-edit/tree/media handoff is an acceptance criterion. | The August 13 Ink & Light handoff is stale relative to current source. PAX-993 is In Progress. |
| PAX-993 worktree at `88e978da8`, plus its current uncommitted changes | `examples/bundled-cli-examples.toml` and CLI source currently name Living Quilt as the default. The example, renderer, and masking code are still changing. | Current names, screenshots, behavior, and backend caveats are internal discovery evidence; they are not frozen tutorial content. |
| Current starter `src/lib.rs`, `src/lib.pax`, `logo_card.rs/.pax`, `quilt_scene.rs/.pax` | Actual main/child component pairing, property inputs, reactive template reads, click handler, and responsive sizing. | The root interaction includes ripple collections and frame-driven updates. Select a smaller first-edit surface rather than requiring readers to understand that algorithm. |
| `pax-runtime-api/src/properties/mod.rs`; `examples/src/increment/src/lib.rs/.pax` | `Property<T>::get/set`, typed event handler, and template-derived output. | Increment is a source cross-check only; the learner stays in the default generated project. |
| `pax-runtime/src/engine/expanded_node.rs`, `dispatch_pointer_activation` | A lone `@click` handler also receives touch activation through the shared dispatch path. | Exercise the selected control's real hit area and interaction; pointer activation does not establish keyboard or accessibility behavior. |
| `template-language.md`, `data-binding-expressions.md`, `state-properties.md`, `event-handling-rust.md`, `components-composition.md`, `layout-responsiveness.md` | Current vocabulary and canonical ownership of concepts beyond the walkthrough. | Several chapters still need their own prose pass; the tutorial must remain self-contained for its essential steps. |
| PAX-993 `pax-cli/tests/create_bundled_examples.rs` and bundle implementation | Tests cover default/override creation, manifest sanitization, and an opt-in source-linked web run. | This is inspected test code, not a test result from this pass or proof of the final published package. |

The current source check was read-only. No changes were made in the PAX-993
worktree, and no stale bundle or local CLI was used to claim that the changing
project matches what a public release creates.

### Example and media plan

Use one interactive example of the same tutorial project, pinned to the docs
release and accompanied by the exact source files. Three possible presentation
treatments for that one project: an end-state app with source tabs; a before/after
toggle that highlights the edit; or an interaction beside a short annotated
handler/binding excerpt. Recommend the end-state app with source tabs, keeping
the sequential edits in the article.

Allow two supporting media placeholders: an annotated component/template source
pair, and a narrow/wide comparison of the completed edit. Capture both only
after the accepted project and edit sequence are stable. ExampleHost or the
docs embed should reuse canonical source; no separate tutorial example or
gallery expansion is proposed here.

### Bounded verification before prose is accepted

1. Record the accepted PAX-993 source revision, synchronized bundle, generated
   project tree, and first-edit contract.
2. Create the default project outside the monorepo using the matching CLI;
   perform each tutorial edit in order, building at every Rust checkpoint.
3. Verify the first `.pax` reload and the explicit Rust stop/rebuild/restart.
4. Exercise the interaction, including repeat activation, and compare the
   final layout at narrow and wide viewports. Label browser touch emulation
   separately from actual mobile-target verification.
5. Build the final web release output and confirm the edited behavior there.
   Keep source-linked tests distinct from the published-package launch gate.
6. Check every snippet, media/source revision, and local link. Confirm that
   commands agree with Getting Started and that no starter internals have
   become unexplained prerequisites.

### Editorial choices for Zack

1. **Walkthrough depth:** recommend one small modification of the existing
   generated app. A reconstruction of the full starter would substantially
   expand this chapter and duplicate later feature explanations.
2. **Composition depth:** recommend passing a value into an existing child.
   Save extracting a new component from scratch for Components and Composition.

Both recommendations and advancing to Templates were approved on 2026-09-06.
The tutorial stays queued for the stable first-edit handoff; no replacement
starter or parallel chapter drafting is authorized.

## K. `Templates and UI structure` outline and evidence packet

**Status:** Outline and initial prose accepted, 2026-09-06. Zack approved the
rewritten `template-language.md` as a starting point and advancing to the PAXEL
outline. Launch-critical; final media and cross-chapter verification remain.

### Reader promise and chapter boundary

Learn to read and author a Pax template: recognize its tree, set values, connect
state and events, predict which overlapping element is in front, and understand
where a setting comes from. Use one small layered interface as the running
example, drawing on existing example source rather than adding another starter.

The eventual reading path follows the first-interface tutorial. This chapter
should also be usable after Getting Started, with a brief orientation to the
component/template pair. Familiarity with HTML or CSS is useful background,
but their text-node, selector, cascade, and element-order rules should not be
assumed to apply to Pax.

This article owns template structure, authoring syntax, selector/settings
mechanics, and element order. PAXEL owns value syntax and expression semantics;
Properties owns reactive Rust state; Events owns handler behavior; Components
and Control Flow own reusable component construction, slots, and list identity;
Layout owns sizing and positioning; Drawing owns visual design and materials;
Motion owns timelines. Give just enough orientation to recognize linked forms.

### Current-content audit before the rewrite

- The opening was five outline bullets. It needed a concrete template
  that teaches tags, nested elements, properties, and text before selectors.
- IDs/classes and conditional settings contain useful current reference.
  Preserve their details and existing `ids-and-classes` and
  `conditional-settings` anchors while putting fundamentals first.
- The chapter lacked a concrete element-order example, an explanation
  of `=` on tags versus `:` in settings, a scoped settings model, and an
  introduction to `ImportSettings`.
- Class-list order and imported settings are separate precedence dimensions.
  Avoid presenting a single CSS-like specificity rule that incorrectly makes
  a local ID selector outrank every imported class.

### Proposed article sequence

| Section / anchor | Purpose and content | Boundary or read more |
| --- | --- | --- |
| Read a template / `reading-a-template` | Show a small complete tree and its Rust `#[file(...)]` association. Identify PascalCase component tags, opening/closing and self-closing tags, parent/child/sibling relationships, and multiple root elements. Use `Text text=...` for visible text. Introduce `//` comments in a valid location. | Defining reusable components and Rust module organization stay in Components. Mention embedded templates as an alternate source form only. |
| Set a value / `setting-values` | Read an attribute as a property assignment. Introduce literals, units, `{...}` expressions, an event binding, and a native control's `bind:` syntax in a compact recognition table. Explain `self` in the containing component's scope. | No operator catalogue, property-graph lesson, or handler implementation walkthrough; link PAXEL, Properties, and Events. |
| Arrange the tree / `element-order` | Explain earlier siblings in front of later siblings with foreground/text/background from the same example. Distinguish nesting, draw order, and spatial layout. | Layout owns anchors, positioning, flow containers, and responsiveness. Compositing owns masks/native occlusion and exceptional layering behavior. |
| Reuse settings / `ids-and-classes` | Start from inline settings, move repeated values to `@settings`, then introduce a named `id` and one class. Progress to `class=["base", "selected"]`, reactive class values, and the local precedence rules. | Keep supported syntax and error/normalization notes here as compact reference after the first example. |
| Choose settings conditionally / `conditional-settings` | Show one `if`/`else` around selector blocks, preferably with a viewport condition. Explain the difference between selecting property settings and conditionally creating elements. | The condition is a PAXEL boolean. Container-aware responsive design belongs in Layout; structural `if`/`for` belongs in Control Flow. |
| Share settings / `imported-settings` | Introduce a small `ImportSettings` provider from the existing theme example. Explain component scope, provider order, imported-versus-local layers, and final inline overrides for ordinary settings. | Link `$base` to PAXEL and palette/theme design to Drawing. Dynamic provider architecture and animation precedence should not dominate this introduction. |
| Recognize the rest / `read-more` | Briefly identify structural `if`, `for`, `slot(...)`, and `@timeline` as forms readers will encounter, with direct owner links. Add a formatter/workflow link. | Keep this a short navigation paragraph/list, not another syntax reference. |

In the settings sequence, first show the ordinary local case: matching classes
in class-list order, then matching ID settings, then inline values. Introduce
imported providers as an additional ordered layer in the following section.
Keep timeline interaction out of this simplified ladder and name that boundary.

### Source-backed authoring rules and verification notes

| Finding | Current evidence | Editorial consequence |
| --- | --- | --- |
| Multiple top-level elements, matching tags, self-closing tags, and explicit comments are accepted forms. | `pax-language/src/pax.pest`, `pax_component_definition`, `open_tag`, `matched_tag`, `comment`; `pax-manifest/src/parsing.rs`, template-node construction. | Do not invent a mandatory single-root wrapper. Use a wrapper only when the example needs a shared container. |
| Literal inner content is unfinished despite having a grammar rule. | `pax-manifest/src/parsing.rs`, `Rule::node_inner_content`, explicitly calls `unimplemented!`. | Teach `<Text text="..." />`. Do not publish `<Text>"..."</Text>` based on grammar comments alone. |
| `.pax` selectors are `.class` and `#id`. The shared selector API also accepts type names. | Grammar `selector`; `pax-manifest/src/selectors.rs`, `SelectorExpr::parse`; runtime settings tests also construct type selectors programmatically. | Keep inspection/query syntax separate from authorable `@settings` syntax. Do not teach a bare `Text { ... }` selector. |
| Classes are strings or string lists, including reactive expressions; duplicate names keep their final position. | `pax-runtime/src/engine/expanded_node.rs`, `normalize_selector_classes`; `pax-language/src/formatting/mod.rs`, class tests. | Explain ordered lists explicitly. A space-separated string is not a multi-class spelling. Keep invalid-value and empty-value rules from the current article. |
| Classes precede IDs within one settings layer; imported layers follow component settings, then ordinary inline values win. | `pax-runtime/src/cartridge.rs`, `append_selector_layer_entries`, `resolve_runtime_settings_with_layers_for_node`; tests `selector_cascade_is_type_then_classes_then_id_then_inline` and `runtime_settings_resolve_columns_and_provenance_by_layer`. | Show local precedence first, then layer precedence. Avoid a universal CSS-specificity analogy. |
| `ImportSettings` mounts non-rendering providers and applies their settings through the containing component; provider expressions retain provider scope. | `pax-std/src/core/import_settings.rs`; `imported_settings_layers_for_node` in cartridge; provider collection in expanded node; provider-scope tests. | Explain explicit sharing and component scope. Do not imply an automatically global stylesheet or that provider `self` refers to each styled node. |
| Settings conditions contain selector blocks or nested conditions, not per-property `if` statements inside a selector. | Grammar `settings_conditional_body`; parsing `parse_settings_conditional`; runtime condition evaluation/tests. | Use a boolean outer condition and link ternary property values to PAXEL. `class` assignment inside a selector/settings block is rejected. |
| Render traversal visits siblings in reverse source order. | `pax-runtime/src/engine/expanded_node.rs`, `recurse_render`; layered Text/Rectangle source in `examples/src/increment` and `runtime-settings-themes`. | Illustrate the earlier-sibling-in-front rule visually; keep runtime traversal terminology out of the beginner explanation. |

Source checks also exposed two adjacent verification debts: the grammar's
comment about requiring an element/multiple settings blocks is not a reliable
description of the current parse paths, and the current PAXEL `$base` paragraph
compresses timeline precedence more broadly than the runtime layer construction
alone establishes. Do not copy either claim into new prose. Reconcile the latter
with motion-specific tests during the PAXEL/Motion passes. No adjacent source,
prose, or overlapping pain-points file was changed here.

### Example and media plan

Use small, source-backed excerpts of existing examples for the fundamentals;
`runtime-settings-themes` supplies imported-provider evidence and
`responsive-helpers` supplies conditional-settings evidence. Avoid carrying
their full application complexity or network-font dependencies into the core
lesson. New snippets must be compiled before prose review.

Budget one principal interactive example plus an optional existing theme embed,
with source tabs tied to the docs version. The principal example should expose
element order, a literal-to-binding change, and class/inline overrides. Three
presentation ideas for the later media pass: a layered panel with selectable
source lines; an exploded foreground/background view; or a property inspector
showing the value contributed by each settings layer. Recommend the layered
panel, with one supporting source/tree/layer diagram. These are presentation
options for existing material, not a request to build another starter or gallery.

### Bounded verification before prose review

1. Validate each syntax form through manifest parsing and compilation, not just
   the grammar or formatter. Include text attributes, multiple roots, class
   lists/reactive classes, nested values, and conditional settings.
2. Build and inspect the small layered example on web; reverse the relevant
   siblings once to confirm the diagram and restore the intended order.
3. Exercise class order, ID override, inline override, and two imported
   providers. Check scope at a child-component boundary and a provider-owned
   reactive value. Use existing runtime tests as supporting evidence.
4. Resize across the documented condition and confirm inactive settings fall
   back correctly. Explain structural changes separately.
5. Check the final examples against release baking as well as debug behavior,
   since selector metadata and conditional settings cross that boundary.
6. Preserve existing anchors and link the current canonical articles. Add the
   final tutorial/workflow links when those pages exist; keep `SUMMARY.md`
   unchanged during this article pass.

The outline pass inspected source and test definitions; it did not execute
the proposed examples or claim new runtime test results.

### Editorial choices for Zack

1. **Settings ownership:** recommend keeping the short `ImportSettings` and
   precedence explanation here, beside local settings. Drawing and Styling
   can focus on visual vocabulary, palettes, and how to design a theme.
2. **Depth:** recommend a readable fundamentals-first chapter, with precise
   class/selector restrictions collected after their introductory examples.
   Keep full control-flow, PAXEL, and timeline syntax in their owner chapters.

Both recommendations were approved on 2026-09-06. The draft follows this
sequence, using one small panel to introduce inline values, local settings,
and an imported theme. Existing `ids-and-classes` and `conditional-settings`
anchors remain intact. `SUMMARY.md` remains unchanged; its navigation label
will be reconciled with the new chapter title during the final topology pass.

### Draft verification — 2026-09-06

- Rebuilt the current-source CLI with `cargo build --offline -p pax-cli`.
- All seven fenced Pax snippets compile together in a temporary web fixture.
  The fixture also compiles the recognition table's state binding, event
  binding, and `Textbox` read/write binding. Its snippet files were compared
  byte-for-byte with the chapter after testing. No new public example or
  starter was added, and no existing example source was modified.
- Browser inspection confirms that the inline and local-settings versions
  render the same panel, and the imported provider supplies the green surface
  and larger corner radius. Temporarily moving the background before the text
  hides the text; the documented source order was restored and checked after
  reloading the page.
- The conditional panel measures 320px wide in a 1280px viewport, 452px in a
  500px viewport, and 320px after returning to the wide viewport. Both browser
  captures and `pax-cli dev selector .panel` support this check. The temporary
  viewport override was cleared.
- Both `pax-cli build --target=web --libdev` and the same command with
  `--release` pass for the fixture. The release output was served locally and
  inspected at wide and narrow viewports: local and imported settings, text
  layering, and the responsive condition render correctly through the release
  cartridge. This is current-source web verification, not a published-CLI
  installation test or an Apple-target smoke test.
- Fourteen focused tests pass: seven `pax-runtime` `settings_` tests; two
  `selector_class` tests; three `pax-language` formatting tests; and the
  `pax-manifest` tests `parses_top_level_settings_conditionals` and
  `binary_manifest_round_trips_template_maps_without_json_keys`. These cover
  class normalization, local/layer precedence, inactive-condition fallback,
  provider-owned values and conditions, parsing, and binary roundtripping.
- The mdBook build passes, as do all 27 rendered local links/anchors in this
  article and `git diff --check`.

Two preview observations remain follow-up checks, not established general
runtime bugs: the first hidden-browser capture lacked canvas fills until a
viewport change, and restoring the sibling order through hot reload left the
text occluded until a page reload. The restored source rendered correctly after
reload, and the subsequent release preview rendered correctly on first capture.
Recheck both in a visible browser during the media/workflow pass; no
framework workaround or public claim was added. Keep these notes here while
the shared pain-points file is under the PAX-869 overlap restriction.

The diagram and interactive example remain explicit production placeholders.
Templates initial prose is accepted. The first-interface tutorial remains
queued behind the stable PAX-993 handoff; PAXEL is the active outline below.

## L. `PAXEL and reactive values` outline and evidence packet

**Research checkpoint:** Outline reviewed on 2026-09-06. Keep
`data-binding-expressions.md` as the canonical destination. The initial
research pass changed no PAXEL prose. Zack subsequently confirmed all three reproduced expression
behaviors as bugs and requested fixes in a separate issue-backed worktree.
[PAX-995](https://linear.app/paxdev/issue/PAX-995/fix-paxel-reactive-indexing-boolean-short-circuiting-and-precedence)
owns those fixes before the chapter's final contract is established.

Implementation task: `01a07842-700a-7d51-b638-5d6f6d3dc6ff`, in
`/Users/zack/.codex/worktrees/a594/pax`, starting from
`f13299220228f181fe9bbdb7de4f9492b4aa828d`. The task brief links PAX-995;
its Linear discussion records the worktree and return handoff to this docs task.

### Reader promise and chapter boundary

Learn to turn application state into interface values: a label, a dimension,
a color, or a choice of content. Understand what a formula reads, what makes
its result update, and where Rust enters when the work needs a state change,
a reusable calculation, or a side effect.

Start after Templates, with a short reminder of `Property<T>` fields so that
Properties is not a hidden prerequisite. Use the panel already introduced in
Templates, with a few state-driven values; do not introduce another starter or
depend on the changing PAX-993 design. One input feeding several visible values
is the principal learning example.

This chapter owns expression syntax, template-side reactive bindings, values
and units, choosing values, expression scope, helper calls, and `$base` meaning.
Properties owns Rust handles, computed-property construction, dependency lists,
subscriptions, effects, and graph internals. Events owns state-changing handlers.
Templates owns settings precedence and structure; Layout owns geometry; Drawing
owns visual recipes; Motion owns timeline behavior; Control Flow owns repeated
or conditional element trees.

### Proposed chapter sequence

| Section / stable anchor | Reader outcome | Boundary / read more |
| --- | --- | --- |
| Reactive bindings / `reactive-bindings` (retain `bindings` alias) | Follow a literal-to-`{self.title}`-to-derived-value progression. Explain automatic dependency wiring, cached derived values, and the update loop in one small example. Distinguish a derived value from a control's `bind:` read/write connection. | Link Rust field updates to Properties and Events; avoid duplicating their implementation walkthroughs. |
| Values and units / `literals` | Read numbers, strings, booleans, colors, lists, contextual objects, and `None`. Use `{100% - 48px}` and `{(self.progress * 100)%}`. Explain that the receiving property supplies the expected type; `%` is a unit and `%%` is modulo. | Layout explains the reference dimension; Drawing explains color spaces and property-specific list positions; Motion explains time/frame interpretation. |
| Choosing a value / `choosing-values` | Use a boolean ternary and optional-value fallback. Distinguish `None` from false, zero, and an empty string. Explain which branches are evaluated, subject to the correctness decisions below. | Link structural `if` to Control Flow and conditional settings to Templates. `??` does not recover from evaluation errors. |
| Reading structured data / `structured-values` | Read fields and lists, construct a derived list or object, and recognize an exclusive range. Explain the currently different boundaries of literal objects and literal lists. | Dynamic indexing requires the dependency fix/decision below. Rust owns loading and reshaping data; Control Flow owns `for`, keys, and item identity. |
| Functions and Rust helpers / `function-calls` | Use `Math::min`, `Math::max`, and `Math::len`; recognize enum constructors. Explain the registered `Type::function(...)` call boundary and optionally show one small pure Rust helper with explicit inputs. | No general Rust method invocation, mutation, or I/O inside formulas. Link richer computed-state work to Properties. |
| Built-in values / `built-in-globals` | Discover viewport facts, runtime platform/OS facts, clocks, and sensors in a compact reference. Explain that `$base` is contextual, separately below. | Layout owns responsive recipes, Motion owns clocks, Input/Events owns sensor permissions and target caveats. Detection fields do not advertise additional build targets. |
| Building on a previous setting / `base` | Use `$base` to adjust the earlier value of the same property, with one ordinary settings/inline example linked back to Templates. | Keep timeline-specific examples and precedence in Motion. Do not equate `$base` with a parent property or a previous animation frame. |
| Operators and troubleshooting / `operators`, `common-gotchas` | Keep a compact, accurate lookup for operators, grouping, valid input types, bounds checks, expression errors, and keeping helpers pure. | Freeze precedence and guard guidance only after the decisions below. Link runtime diagnostics to the workflow chapter when it exists. |

### Current reference material: preserve, correct, relocate

- Keep useful option/coalescing semantics and adjacent unit-suffix rules. The
  current opening can become a short worked progression rather than starting
  with a syntax inventory.
- Preserve existing public anchors: `bindings`, `operators`, `literals`,
  `base`, `built-in-globals`, `loops`, `function-calls`, and `common-gotchas`.
  The `loops` anchor can introduce the small range example and link onward.
- Keep one example each of contextual object and list values here. Full
  corner-radius arity diagrams and linear/radial gradient recipes belong in
  Drawing. Retain that material as end-reference during the PAXEL rewrite;
  relocate it when Drawing is authored so no information disappears between
  sequential chapter passes.
- Replace the current nested `{r, g, b, a}` object assigned to `fill`: the
  interpreter constructs that object, but `Fill::try_coerce` rejects it. Use
  a real struct property to teach objects and `rgb(...)`/`rgba(...)` for color.
- Narrow the current universal `$base` precedence sentence. Resolution has
  selector, imported-provider, timeline-selector, inline, and special
  timeline/transition overlay paths. `stack_with_base`, common-property
  fallbacks, and the base-symbol tests establish the previous-value meaning;
  they do not justify a single blanket statement that every timeline is last.
  Recheck the existing timeline snippet through full compilation/runtime before
  moving it to Motion.
- Separate platform from OS detection: `$web` can coexist with `$macos`, for
  example. `$android`, `$windows`, and `$linux` are known OS facts, including
  browser hosts; current build targets remain web, macOS, iOS, and iPadOS.
  Neither these fields nor sensor availability establish workstation support.

### Reproduced expression findings requiring a decision

These are current-source findings, not inferred from issue status. No runtime
or language implementation was changed in this pass.

| Finding | Reproduction and source | Recommended disposition |
| --- | --- | --- |
| Dynamic index changes can leave a binding stale. | `items=[10,20]`, `index=0`, and a runtime property built from `items[index]` yield 10. After `index.set(1)`, the value remains 10; writing `items` again makes it 20. `DependencyCollector` records only `items`, ignoring the accessor's expression dependencies. The probe uses the actual `build_component_property` path. | Fix dependency collection and cover nested/index-expression cases before promising fully reactive dynamic access. Include release-baked dependency metadata in the verification. Do not teach artificial extra dependencies as the canonical pattern. |
| Boolean operators evaluate both operands. | Both `false && (items[9] == 1)` and `true \|\| (items[9] == 1)` return an out-of-bounds error. `PaxInfix::compute` evaluates both operands. A ternary skips its unused branch, while `items[9] ?? 4` still errors because fallback is for `None`. | Decide whether to adopt short-circuit boolean semantics. Recommend doing so in a focused language task; until then, document the actual distinction and use ternaries for guards. |
| Operator grouping differs from familiar arithmetic/boolean precedence. | `1 + 2 > 2` produces numeric 1, while `(1 + 2) > 2` produces true. `true \|\| false && false` produces false; `2 * 3 %% 2` produces 2. `get_pax_pratt_parser` puts comparisons above arithmetic, gives `&&`/`\|\|` one level, and gives modulo a higher level than multiplication/division. | Make an explicit language decision before publishing a lasting precedence table. Recommend conventional grouping, with an audit of affected examples/tests and release parity. Use explicit parentheses in this chapter's mixed expressions. |

Zack confirmed all three findings as bugs on 2026-09-06. PAX-995 was filed
first, with reproductions, corrected outcomes, regression/formatter checks,
an existing-expression compatibility audit, and debug/release-baking parity
requirements. Its dedicated implementation worktree owns the fixes; this docs
task retains the prose and will reverify the corrected contract after handoff.

### Additional source checks and bounded verification

- `pax-language/src/interpreter/computable.rs` and `property_resolution.rs`
  establish evaluation and dependency behavior. Both ternary branches and both
  coalescing operands contribute static dependencies even though unused value
  branches are skipped during evaluation. Avoid implying dynamic subscriptions
  to only the chosen branch.
- Literal-object fields already accept expression values, as shown by
  `runtime-settings-themes/src/sans_typography_theme.pax`. Parser probes accept
  `<Text style={font_size: {self.size}} />` and whole-object expressions.
  Literal-list members such as `sizes=[{self.size}, None]` fail parsing, while
  the whole-list expression `sizes={[self.size, None]}` parses. PAX-990 remains
  a useful lead for broader literal nesting, not proof that every object-field
  expression is unsupported. Full typed snippets must compile before prose review.
- `pax-runtime-api/src/pax_value/functions.rs` defines the built-in helpers.
  `#[helpers]` exposes public associated functions without a `self` receiver;
  `#[has_helpers]` suppresses the default empty registration implementation.
  Generated cartridges register helpers for their type table. Verify a complete
  reachable helper example in debug and release before teaching the setup.
  Helpers must receive changing inputs explicitly and remain side-effect-free.
- `Globals::stack_frame`, `TargetInfo`, and `Viewport` provide the globals
  contract. `$viewport.width`/`height` are numeric logical-pixel values, so an
  expression-derived size needs the appropriate suffix/contextual conversion.
- Seventy-two focused tests passed: 54 interpreter tests, 12 runtime
  `base_symbol` tests, four runtime `globals_expose` tests, and two macro
  helper-visibility tests. A temporary native diagnostic executable separately
  reproduced the three findings above and the invalid color-object coercion.
  Passing existing tests does not mean those untested behaviors are correct.
- Before prose review: compile each final Pax/Rust snippet, exercise a shared
  input feeding several visible values, verify `bind:` control updates,
  conditional/fallback behavior, unit arithmetic, nested object/list forms,
  helper calls, and `$base`; compare debug and baked release results. Then
  rebuild mdBook and validate the retained/new anchors. No new full web or
  release fixture was run during this outline pass.

### Example/media plan and editorial checkpoint

Budget one principal interactive example and one supporting diagram. The
example shows an input, the formulas that read it, and the resulting label,
size, and color. Reuse the Templates panel and existing example patterns;
source tabs must remain tied to the docs version.

Three possible treatments: a panel with an editable property and highlighted
source; a compact worksheet with formula/result pairs; or a split view whose
dependency arrows light up as a control changes. Recommend the panel with
source tabs, plus a small one-input/multiple-results diagram. Advanced grammar
probes and failure cases belong in tests/reference, not the first interaction.

Zack approved proceeding with the conceptual opening, compact reference,
staged drawing recipes, and one small pure helper example. PAXEL remains the
single active chapter. Drafting can proceed while PAX-995 verifies the three
fixes; the final expression contract will be rechecked after its handoff.

## M. PAXEL initial prose checkpoint

**Status:** Initial prose approved by Zack on 2026-09-07, with the Common
Gotchas section removed at his request. Its legacy anchor is retained at the
operator reference so existing deep links still resolve. This draft
does not depend on importing unmerged PAX-995 changes. It uses current-source
behavior and reserves the three affected guarantees below for a later pass.

### Editorial treatment

- A complete Rust declaration and the Templates panel introduce one slider
  feeding a label, percentage width, and conditional color. A textbox shows
  the read/write connection separately. This is a small teaching panel, not
  another starter or a dependency on PAX-993's changing design.
- The formula model leads; values, units, choices, structured data, registered
  helpers, globals, and `$base` follow. Rust property graph construction and
  side effects remain owned by Properties and Events.
- The helper section includes `#[has_helpers]`, the separate `#[helpers]`
  implementation, a public receiver-free function, and an explicit reactive
  argument. It is compiled as part of a reachable component in both modes.
- The invalid color-object fill is removed. `$base` now has a focused
  class-to-inline example; there is no blanket timeline precedence promise.
- Existing corner-radius, gradient, and timeline-relative material remains
  as end-reference until the Drawing and Motion passes give it a canonical
  home. Platform and OS facts remain distinct from build-target support.
- One interactive-panel placeholder and one dependency-diagram placeholder
  retain their production briefs and three alternative treatments. They are
  not presented as completed embeds.

### Reserved PAX-995 verification points

| Destination | Work after verified handoff and coordinated integration |
| --- | --- |
| Structured values | Add a changing-index example and verify that changing the index alone invalidates the bound result, including nested access and baked metadata. The current draft uses a constant index. |
| Choosing values / operators | Add boolean guard guidance only after lazy `&&`/`||` behavior and required-operand errors are verified. The draft makes no short-circuit promise and teaches no workaround. |
| Operators | Add the exact precedence/associativity table after the parser, formatter, existing-expression audit, and debug/release results are reviewed. The current table is a purpose-grouped syntax lookup. |

PAX-995's implementation task has provided a completed handoff on PAX-975 and
is In Review, but its work has not been merged into this worktree. Zack owns
commits and integration. These three
items gate final chapter readiness, not prose review or subsequent sequential
article research after that review.

### Verification performed in this worktree

- All 19 Pax fences were extracted into a temporary app and compiled against
  this worktree with `pax-cli build --target=web --libdev`, then with
  `--release`. The two Rust fences are composed according to the helper
  instructions; other snippet components supply the fields named in prose.
  Both builds passed, including release cartridge generation/baking.
- Live browser checks in debug and release verified initial 25 percent
  progress, a change to 50 percent, the half-width green bar, the derived
  label, the pure helper's formatted text, and textbox edits reflected in the
  heading. The debug pass also exercised 75 percent.
- A temporary native probe passed 23 assertions for the draft's value,
  coercion, fallback, function, and reactive label/width examples. It uses the
  actual `build_component_property` path and confirms `??` does not catch
  invalid list access. The 12 existing `base_symbol` runtime tests also passed.
- `mdbook build` and `git diff --check` passed. The link check validated all
  20 chapter links and nine retained/new anchors: `bindings`,
  `reactive-bindings`, `operators`, `literals`, `base`, `built-in-globals`,
  `loops`, `function-calls`, and `common-gotchas`. The same check confirms
  the compiled Pax fences match the article exactly.
- `SUMMARY.md`, adjacent public articles, gallery content, and the overlapping
  pain-points file were not changed by this prose pass. Sidebar labels and
  previous/next ordering remain a coordinated navigation pass.

Local verification artifacts (temporary, not publication inputs):

- `/tmp/pax-975-paxel-check.FtFjgG`: snippet app and chapter-link check.
- `/tmp/pax-975-paxel-probe.wh0qsR/src/bin/docs_contract.rs`: native assertions.
- `/tmp/pax-975-docs-review.awrOqW`: rebuilt mdBook preview.

Workflow observations retained here under the pain-points overlap restriction:
the first release-page capture used a fallback serif font; reloading restored
the expected sans-serif rendering, as observed in the earlier Templates pass.
Do not treat first-load font behavior as resolved. The temporary app's
`pax-cli run` command returned before a discoverable dev session appeared;
live checks used loopback static servers for the successful debug/release
build outputs. No hot-reload or dev-session guarantee was verified by this
pass, and no framework workaround was added to public prose.

PAXEL prose is accepted as a starting point. Properties research/outline is
now active, while the three PAX-995 verification items remain visible until
coordinated integration. No publication or issue-status completion is implied
by this chapter checkpoint.

## N. `State and Properties` outline and evidence packet

**Status:** Outline approved by Zack on 2026-09-07. The resulting
`state-properties.md` draft is at its prose checkpoint; see section O.
PAX-995's three reserved expression guarantees do not block this chapter.

### Reader promise and chapter boundary

After this chapter, a builder can choose and initialize component state,
change a scalar or collection from Rust, and explain how that change reaches
the interface. They can then create a reusable computed value and recognize
when a node-scoped subscription is appropriate.

Keep the current file and its substantial reference material. Add the missing
everyday-state introduction and improve the order; this does not call for a
new state-management architecture or a second tutorial project. Reuse the
small Field notes panel from Templates/PAXEL, independent of PAX-993's starter.

- **Own here:** `Property<T>` values and handles, initialization, access and
  mutation, Rust computed properties, explicit dependencies, graph identity,
  subscriptions, and propagation cutoffs.
- **Link to PAXEL:** expression syntax, automatic template dependency wiring,
  registered pure helpers, and the control's `bind:` connection. Show enough
  template context to make the Rust example understandable.
- **Link to Events:** event payloads, lifecycle ordering, asynchronous work,
  and platform-facing side effects. Use one complete handler without turning
  this into the event catalogue.
- **Link to Components:** parent/child inputs, shared handles, and scoped
  stores. Explain handle sharing here; teach where state lives across a
  component tree in Components. That article still needs its own outline.
- **Link to Motion and the API reference:** easing/transition behavior and
  the full method surface. Keep graph storage and scheduling implementation
  details out of the main learning path.

### Proposed article sequence

| Section / anchor | Reader outcome and treatment |
| --- | --- |
| State in a component / `properties-and-computed-values` | Connect the `title` and `progress` fields to the already-familiar template. Establish `Property<T>` as reactive state and distinguish ordinary Rust data without promising that arbitrary types can be fields of a `#[pax]` component. Briefly orient readers who arrive directly from an API or website link. |
| Initial values / `initial-values` | Explain generated defaults, `Property::new`, and a complete `#[custom(Default)]` implementation for nonzero/nonempty initial state. Reserve `on_mount` for setup that needs the mounted context or existing graph connections; acknowledge the simpler initialization used in the preceding panel. Parent-authored inputs belong to Components. |
| Reading and updating state / `reading-and-updating-state` | Follow a small handler through `get` and `set`; add one `Property<Vec<String>>` update. Explain that `get` clones a value and `update` performs get/mutate/set. Retain the compact method table, including `read` and `set_if_neq`; place each important constraint next to its method. |
| Property handles / retain `property-handles` | Show the same value through two cloned handles. Contrast a handle with a value snapshot, including the fact that cloning a value containing nested property handles can still share those inner handles. Explain why closures capture handles and why replacing a field handle is different from updating its existing graph node. |
| Computed properties / retain `computed-properties` | Extend the panel with one derived Rust label and its complete mount setup. Explain when a reusable Rust computation is useful, explicit dependency lists, `.untyped()`, lazy evaluation, and `replace_with`. Update source inputs to change derived state; a `set` on a computed field does not detach its evaluator. Avoid cycles and side effects in evaluators. |
| Subscriptions and effects / retain `subscriptions-and-effects` | Show a short node-scoped observation of progress. Explain the initial scheduled callback, later coalesced updates, explicit dependencies, synchronous draining, and cleanup on unmount. `clear_subscriptions` removes all subscriptions on that node; this API returns no individual cancellation handle. |
| How updates travel / retain `the-property-graph` | Provide the small dependency diagram and the accurate eager-invalidation/lazy-evaluation explanation. Clarify that subscribers and cutoff boundaries also schedule reactive work; the laziness explanation is about ordinary computed values, not a promise that every callback waits for a visible read. |
| Propagation cutoffs / retain `propagation-cutoffs` | Preserve the existing pointer-to-bucket example and precise true/false semantics as advanced end-reference. Explain last-accepted values, first evaluation, synchronous settling, and the fact that a cutoff still evaluates its own function. No blanket performance promise. |
| Choosing a pattern / retain `performance-guidelines` | End with a compact choice guide: PAXEL for a template formula, computed property for reusable derived Rust state, subscription for a bounded effect. Link Events as the next chapter and retain the generated property API link. |

The everyday path through state, handles, and computed values is launch-critical.
Subscriptions need a sound explanation; cutoff tuning is useful advanced
reference and should not feel like a prerequisite for building an interface.
No visible Common Gotchas section is proposed.

### Source-backed audit and verification ledger

| Evidence | Consequence for the draft |
| --- | --- |
| `pax-runtime-api/src/properties/mod.rs`: `PropertyValue`, `Default`, `get`, `read`, `set`, `update`, `set_if_neq` | Values require `Default + Clone + Interpolatable + 'static`. `update` mutates a clone and writes it back; `set_if_neq` requires `PartialEq` and compares with the current settled value. A `read` callback must not reenter the same property, including through another dependent read. |
| `pax-macro/src/lib.rs`: `get_field_type` and generated derives; `pax-macro/templates/derive_pax.stpl`: coercion/value conversion | `#[pax]` ordinarily supplies `Clone`, `Default`, serialization, and related support. `#[custom(Default)]` opts out of its Default derive. Plain fields still participate in generated type/conversion machinery; the current blanket suggestion of arbitrary implementation state is too broad. Avoid an unsupported resource-handle recipe. |
| `examples/src/game-of-life/src/lib.rs`; `pax-std/src/reference/example_host.rs` | Existing custom-default implementations demonstrate literal initialization. ExampleHost also demonstrates `replace_with` for derived fields and explicit dependency lists; its local-store use is a lead for the later Components chapter, not an application-state prescription here. |
| `pax-runtime-api/src/properties/untyped_property.rs`; `properties_table.rs`: read/set/replacement/update paths; `graph_operations.rs` | Cloning keeps graph identity. `replace_with` preserves existing outbound links and switches inbound dependencies/evaluator. `set` stores a value but retains a computed evaluator; a subsequent dependency change can recompute it. Keep these operations conceptually distinct. The table is thread-local: do not present property handles as a cross-thread messaging interface. |
| `pax-runtime/src/api.rs`: `subscribe`/`clear_subscriptions`; `pax-runtime/src/properties.rs`: `register_node_effect`; `pax-runtime/src/engine/expanded_node.rs`: unmount cleanup | A subscription is a retained computed effect, queued on registration and cleared on unmount. Describe scheduled initial execution, not immediate execution inside `subscribe`, and do not promise one callback for each write. |
| `pax-runtime-api/src/properties/tests.rs` | All 23 focused property tests pass in this worktree. They cover computed updates, replacement/disconnection, cleanup, initial/coalesced effect draining, no-op writes, and cutoff ordering/retention/budgets. |
| Temporary `properties_contract.rs` probe | Additional native checks exercise cloned handles, collection snapshots/updates, lazy caching, preserved field connections, writes to computed fields, and deliberately omitted Rust dependency edges. This is research verification, not a substitute for compiling the final chapter's snippets. |

The existing graph/cutoff material is substantially accurate. The main gaps
are orientation, initialization, collection updates, and subscription timing.
The draft should qualify `get`'s cloning behavior instead of implying a deep
independent copy of every possible value. Serialization similarly records a
property's current value, not its evaluator or graph connections; leave full
persistence workflows to future application recipes.

### Example and media plan

One interactive placeholder and one diagram placeholder are sufficient.
Keep the production briefs in the article until coordinated example work can
provide verified embeds; do not modify the gallery or create a starter here.

- **Interactive panel, preferred:** continue Field notes with a Rust-driven
  progress action and one derived label. The reader changes progress and sees
  the label/bar update; source reveals the input field, write, and computation.
  Alternatives: append notes to a short list to explain collection updates;
  or show two readouts sharing the same handle to explain identity.
- **Diagram, preferred:** progress branches to a template width and a computed
  label; annotate write, invalidation, then read/evaluation. Alternatives:
  a before/after diagram of `replace_with` preserving outgoing connections;
  or a small cutoff diagram showing several pointer positions mapping to one
  bucket. Avoid implying that each write causes a full render or a distinct
  subscription callback.

### Bounded verification before prose review

1. Compile every final Rust/Pax example as a reachable component, including
   the custom default, handler, collection update, and computed mount setup.
   Build the web debug and release cartridges against this worktree.
2. Exercise scalar and collection writes in the running panel. Check that
   derived values follow input changes and that existing field bindings survive
   `replace_with`. Validate subscription initialization, coalescing, and
   unmount cleanup with a focused runtime fixture or live remount test.
3. Rerun the property suite and native assertions. Inspect any observed behavior
   against the final wording; do not imply that a native unit test proves all
   target integrations or that public prose changes fix PAX-995.
4. Build mdBook; check chapter links, retained anchors, and the proposed
   `properties-and-computed-values` anchor. Keep `SUMMARY.md` unchanged until
   the coordinated navigation pass. No deployment or generated API rewrite.

### Editorial checkpoint

Recommended balance: everyday state first, computed properties and subscriptions
next, then the short graph explanation and existing advanced cutoff reference.
Keep cross-component state organization in Components with deliberate links.
Zack approved this balance before the Properties prose pass.

## O. Properties initial prose checkpoint

**Status:** Initial prose accepted by Zack, 2026-09-07. Events and Rust is
now the single active chapter, beginning with research and outline review.

### Draft treatment

- The Field notes panel now has a complete declaration, custom default,
  Advance handler, and template. The next stage adds a Rust computed label
  with `replace_with`; a subscription example follows in the same mount
  method. PAX-993's starter and the website gallery remain independent.
- Collection updates and handle/value copying use small Rust examples.
  Subscriptions explain initial scheduling, coalescing, node lifetime, and
  clear-all behavior. The web logging switch is included so the logging
  example is observable with the default runtime configuration.
- The graph explanation follows everyday state operations. Cutoffs retain
  their precise semantics as advanced reference. Cross-component state,
  lifecycle/event detail, and animation link to their owning chapters.
- One interactive and one diagram placeholder retain production briefs,
  showcase dynamics, and three concrete treatments each. They are future
  example/media work, not completed embeds.
- The seven previous heading anchors are retained, including the old title
  anchor. `properties-and-computed-values` is now a real canonical landing
  anchor. `SUMMARY.md`, generated API docs, and other public chapters were
  not changed by this pass.

### Verification

- All eight Rust fences and two Pax fences were extracted and composed
  according to the article's instructions in a temporary fixture. Both the
  basic and computed panels are reachable components; three local Rust
  examples execute assertions. A fixture check detects drift from the final
  article, including the attribute/field additions and label replacement.
- Native chapter assertions passed for custom defaults, collection writes,
  handle sharing, and cutoff bucket results. All 23 existing property tests
  passed again; the additional native property-contract probe also passed.
- A focused fixture using the actual `NodeContext::subscribe` path passed
  initial scheduling, coalesced writes, `clear_subscriptions`, and unmount
  cleanup. The latter retains the node and source handles and checks that
  queued and subsequent work no longer invokes the callback.
- Debug and optimized release web builds passed. Browser checks in both
  verified the initial quarter-width bar and 25 percent labels, button-driven
  half-width bars and 50 percent labels, and collection growth from one to two
  entries. The release pass also verified progress clamps at one with a full
  bar. Debug console inspection with `?pax_log=info` observed the initial
  subscription message.
- mdBook build, whitespace checks, all 12 chapter links, ten required
  retained/new anchors, and fixture/source parity passed. The rendered
  Properties chapter is available in the local review preview.

Temporary verification artifacts (not publication inputs):

- `/tmp/pax-975-properties-check.8FCwB2`: extracted app, native snippet test,
  debug/release web outputs, extraction recipe, and link/parity check.
- `/tmp/pax-975-paxel-probe.wh0qsR/src/bin/properties_contract.rs`: property
  value/identity/caching assertions from the outline pass.
- `/tmp/pax-975-paxel-probe.wh0qsR/src/bin/subscriptions_contract.rs`: actual
  node-context subscription scheduling and cleanup checks.
- `/tmp/pax-975-docs-review.awrOqW`: rebuilt mdBook preview.

Verification setup note: debug/release builds of one fixture must be
serialized because they generate the same `.pax/cartridge.partial.rs`. An
overlapping attempt mixed generated modes and was discarded; regenerating
and building release in isolation passed. No compiler/runtime workaround or
source change was made. Runtime behavior was exercised through loopback
static servers; this pass makes no hot-reload or dev-session claim.

PAX-995's final handoff is recorded on PAX-975, with its implementation In
Review and unmerged here. The three reserved PAXEL guarantees still require
coordinated integration and the documented verification pass. No publication
or PAX-975 status completion is implied by this chapter checkpoint.

## P. Events and Rust research / outline checkpoint

**Status:** Outline and scope approved by Zack, 2026-09-07. Initial prose
and its verification are recorded in section Q. `SUMMARY.md` remains unchanged.

### Reader outcome and ownership

The reader can connect an interaction to a Rust method, use its event data,
update application state, and choose a suitable lifecycle hook. They can also
reason about which node receives an event and convert pointer coordinates
when building a custom interaction.

Templates and basic Properties are the prerequisites. Keep the small Field
notes panel as the teaching context, with a link to its complete declaration
and template in Properties. This chapter annotates the handler wiring already
introduced there; it should not repeat the full property API explanation or
establish another starter project. PAX-993 continues to own the starter.

Events owns handler signatures, binding locations, lifecycle, event delivery,
and pointer/touch coordinates. Properties owns graph behavior, computations,
and subscriptions. Components owns reusable component contracts, parent/child
state, and named custom events. Native Controls owns forms, two-way bindings,
focus, accessibility, and detailed platform coverage. Routing, Motion, and
Scrolling own their respective behaviors; explain how an event reaches those
operations, then link to the relevant chapter.

### Proposed chapter sequence

1. **Connect an action to Rust.** Start with the Field notes Advance button
   and its `@button_click` binding. Annotate `pub fn`, `&mut self`,
   `&NodeContext`, and `Event<ButtonClick>`, then the property write. Explain
   the Rust method/template connection and that a binding names a handler.
   Distinguish a native Button's activation event from general `@click`/`@tap`
   on custom content. Keep the opening small and directly usable.
2. **Use event data.** A compact event/payload table followed by one text or
   slider example. Cover activation, pointer/touch, keyboard, and common
   control changes; link exact fields and less common events to generated
   API reference. Explain that `@click` and `@tap` both take `Event<Click>`:
   either alone covers mouse and single-touch activation; when both exist
   on the same node, input source selects between them. Include the web
   distinction between textbox input and committed changes without claiming
   identical native commit timing before checking each producer.
3. **Choose the binding's scope.** Compare an inline element binding with
   a component's `@settings` handler. Explain which component supplies
   `self`, and that `NodeContext` describes the node whose handler is running.
   This distinction matters when using bounds or converting coordinates.
   Preview propagation and link to its later section rather than explaining
   the full traversal twice.
4. **Do application work.** Show an ordinary Rust helper called by a handler,
   followed by state writes that expose the result to the UI. Callbacks run
   synchronously; avoid blocking work in the interaction path. Link navigation
   and animation operations to their chapters. Briefly orient data-loading
   and asynchronous work, but do not invent a cross-target executor or teach
   `async fn` as a supported event-handler recipe. A complete network/loading
   example needs its own verified integration path.
5. **Work with component lifecycle.** A four-row table for mount, tick,
   pre-render, and unmount, with one small setup/cleanup example. Teach public
   `on_mount`/`on_tick`/`on_pre_render`/`on_unmount` methods first, then explicit
   `@settings` bindings and short aliases as reference. Explain explicit
   bindings taking precedence and the absence of an event argument. Separate
   value defaults from mounted setup; link subscriptions back to Properties.
   Tick is runtime-driven: do not promise a fixed frame rate or copy the
   game's frame-count-based movement as a general timing recommendation.
6. **Understand event delivery.** Explain hit targeting and template-parent
   bubbling for the common pointer/control events, then default prevention.
   `prevent_default()` requests suppression of a chassis default action;
   it does not stop Pax's remaining handlers or propagation. Keyboard events
   currently dispatch across mounted nodes, with web forwarding gated by DOM
   focus. State this boundary plainly and link focus/control details to Native
   Controls. Avoid implying that every event family bubbles or that default
   cancellation behaves uniformly across targets and browser listener types.
7. **Events and coordinates.** Keep the canonical
   `event-handling-rust.md#events-and-coordinates` landing anchor. Teach window
   mouse/touch coordinates, normalized `local_point`, and conversion through
   the receiving node's bounds. Follow with an advanced touch subsection:
   capture, identifiers, end/cancel cleanup, activation on release, and the
   relationship with a moving Scroller. Preserve the useful existing material
   after checking its target-specific gesture claims. Link layout transforms
   and scrolling details rather than teaching those systems here.
8. **Read further.** Keep the existing Space Game embed as an extended
   keyboard/tick example, with its interaction requirements stated. Link
   Components for custom event contracts, Properties for subscriptions,
   Motion for easing/timelines, Routing for navigation, and Native Controls
   for forms. Do not expand into a game-loop or application-architecture book.

Working title: **Events and Rust**. Retain the old
`event-handling--rust-logic` title anchor if the heading changes. The chapter
remains launch-critical; the deeper reference sections should be easy to skip
without breaking the main learning path.

### Source findings and drafting consequences

| Evidence inspected | Consequence for the draft |
| --- | --- |
| `pax-manifest/src/cartridge_generation/mod.rs`: `event_to_args_map`; `pax-compiler/templates/cartridge_generation/macros.tera`: handler descriptors | Built-in payload types are mapped explicitly; lifecycle has no event payload. Generated callbacks call Rust methods synchronously. A native Button uses `ButtonClick`; click and tap use `Click`. |
| `pax-runtime/src/engine/expanded_node.rs`: `run_event_handlers_for_key`, dispatch macro, `dispatch_pointer_activation` | Component and inline handlers select different state owners, while context comes from the node being handled. Common pointer/control handlers recurse through template parents. Click/tap selection is made at each visited node. Cancellation flags do not stop traversal. |
| `pax-runtime-api/src/events.rs`; web interface `events/listeners.ts`; `pax-runtime/src/engine/mod.rs`: global keyboard dispatch | `Event` shares its cancellation flag across clones and exposes `prevent_default`, with no propagation-stop API. Web listeners consult the result, but some touch listeners are passive; native bridges do not establish a universal cancellation guarantee. Keyboard forwarding on web stops while a DOM element other than the body is focused; runtime delivery is global across mounted nodes. |
| Web interface `classes/native-element-pool.ts`: `attachTextboxListeners`; `pax-chassis-web/src/lib.rs`; `pax-chassis-common/src/core_graphics_c_bridge.rs` | Web DOM `input` produces `TextboxInput`; DOM `change` produces `TextboxChange`. Both chassis bridges route those interrupts to their corresponding handlers. The current `TextboxInput` API comment incorrectly describes commit timing for web; correct that narrowly with the prose pass and inspect native producers before writing a cross-target timing claim. |
| `pax-manifest/src/parsing.rs`: implicit lifecycle discovery; `pax-manifest/tests/tests.rs`; compiler websocket parse-update test | Discovery recognizes public component methods with receiver/context arguments. Explicit bindings win; `on_*` is chosen before the short alias when both exist. Static parsing and source-update tests cover preservation of the binding metadata. |
| `pax-runtime/src/engine/mod.rs`: `tick`; `expanded_node.rs`: lifecycle, mount, unmount | Tick handlers precede pre-render handlers, with effect draining between phases. Mount does not promise that all child mounting/layout is complete. Unmount tears down children and clears node subscriptions. Explain useful lifecycle purposes without exposing implementation ordering as a broader stability guarantee. |
| `pax-runtime/src/api.rs`: `local_point`; runtime capture tests; web and common native chassis touch branches | Mouse/touch conversion includes the receiving node's transform and ancestor Scroller presentation offset. Primary touch capture is established on start and released on end/cancel, with fallback hit testing if capture is absent. This is not a general mouse-capture or arbitrary multi-pointer gesture API. |
| `examples/src/glow-buttons/src/glow_button.{rs,pax}`; `examples/src/space-game/src/lib.{rs,pax}` | Existing examples demonstrate explicit component bindings, local light positioning, touch state cleanup, keyboard state, and tick-driven updates. They supply focused source links; their visual or timing choices are not universal API guarantees. |
| `pax-runtime/src/api.rs`: `dispatch_event`; `properties.rs`: custom event queue; `expanded_node.rs`: custom dispatch; `pax-std/src/forms/combo_box.rs` | Named custom events are queued for end-of-tick dispatch, require a registered receiver, and carry no event payload through this API. Their component ownership and error cases belong with composition; do not present them as arbitrary DOM-style dispatched events. |

### Example and media plan

Use one new interactive placeholder and one diagram placeholder, in addition
to the existing advanced Space Game embed. These are production briefs for
later example work; no gallery ownership or starter decision changes.

- **Interactive, preferred:** the Field notes action with a small event trace.
  Activating the control shows the binding, Rust handler, and resulting value
  together. Alternatives: a textbox contrasts input with committed changes;
  or a transformed pad shows window, normalized-local, and local-pixel
  coordinates under mouse/touch movement. Show source alongside the interaction.
- **Diagram, preferred:** a small component/element tree distinguishes the
  Rust state owner, receiving node, and template-parent delivery path.
  Alternatives: a mount/tick/pre-render/unmount timeline; or a coordinate
  overlay showing how a Scroller changes presentation. Annotate any simplified
  boundary rather than implying every event follows the same path.

### Verification and remaining burden

Research checks passed in this worktree:

- `cargo test --offline -p pax-manifest -p pax-runtime -p pax-compiler lifecycle`:
  five matching tests passed, including implicit/explicit binding, preservation
  through a source update, once-per-tick dispatch, and lifecycle-timeline state.
- `cargo test --offline -p pax-runtime --lib local_point`: the ancestor-Scroller
  coordinate-conversion test passed.
- `cargo test --offline -p pax-runtime --lib touch_capture`: release and removed
  target cleanup passed.

Before prose review, compile every final Rust/Pax fence as reachable code and
build web debug and release cartridges separately. Exercise the first action
and payload example. Add bounded runtime assertions for binding scope,
click/tap selection, bubbling/default flags, and lifecycle timing when the
final examples depend on those details. Verify browser keyboard focus and
input/change delivery; inspect native text and gesture producers for every
cross-target claim. Device-only behavior must remain explicitly untested
unless exercised on that device. Runtime unit tests alone do not prove native
control integration or real touch gesture arbitration.

Build mdBook and check chapter links, old/new anchors, and snippet parity.
Verify the retained Space Game embed/source paths and keyboard usability;
coordinate missing media through the existing example/publication work.
No generated API-page edits, framework fixes, deployment, or PAX-975 status
completion are part of this outline checkpoint.

### Editorial checkpoint

Recommended scope: teach everyday handlers and lifecycle in the main sequence,
with propagation and custom gestures as a clearly signposted deeper section.
Keep reusable custom-event contracts in Components. Limit asynchronous/data
loading coverage here to orientation until a small cross-target recipe is
verified; do not introduce an unproven integration into the launch basics.
Zack approved these scope choices before the prose pass.

## Q. Events and Rust initial prose checkpoint

**Status:** Draft ready for Zack's review, 2026-09-07. Events remains the
single active chapter; Components research/outline follows prose feedback.

### Draft treatment

- The opening annotates the existing Field notes Button handler, then adds
  live title editing and a shared helper for the Button and a keyboard shortcut.
  Properties supplies the complete starting component; later snippets have
  explicit addition/replacement instructions.
- Native Button activation and general click/tap activation are distinguished.
  A compact event/payload table supplies practical entry points into the API.
  Binding scope explains Rust state ownership separately from receiving-node
  context, including the effect of keeping both inline and component bindings.
- Lifecycle follows application work, with logging to make mount and unmount
  observable. Propagation, default prevention, keyboard delivery, coordinates,
  and touch capture follow as deeper reference. Native Scroller gesture prose
  is now specifically scoped to the inspected iOS/iPadOS implementation.
- Custom events remain a short bridge to Components. Asynchronous work gets
  orientation without an unverified cross-target executor recipe. No starter,
  gallery, routing, or pain-points content was changed.
- One interactive and one diagram placeholder include production briefs and
  three concrete treatments each. The existing Space Game embed is retained.
  The old title anchor and the canonical events-and-coordinates anchor work;
  navigation labels await the coordinated topology pass.
- Corrected the `TextboxInput` public Rust comment and regenerated its API
  page using the existing generator. Generation ran into a temporary output
  directory and only the affected events page was applied; it matches that
  generated output exactly. No hand-edit of generated prose or TOC rewrite.

### Verification

- Extracted all five Rust and five Pax fences into a temporary app. The first
  handler and its helper-based replacement are both reachable, as are the
  inline, component-level, and deliberately duplicated binding variants.
  Debug and optimized release web builds passed, run sequentially.
- Browser checks in both builds verified one-step Button updates, two steps
  with both bindings, live title changes, suppression of the global shortcut
  while the Textbox has focus, and the shortcut working again with body focus.
  The debug pad mapped its center to 50 percent progress. Debug removal and
  remounting produced the documented console messages and restored defaults.
- A focused native runtime probe passed state-owner/context identity checks,
  continued same-node and parent delivery despite default prevention,
  click/tap source selection and single-binding fallbacks, global keyboard
  delivery, tick-before-pre-render phases, and removal of lifecycle work
  after unmount.
- All seven focused existing lifecycle, local-coordinate, and capture tests
  passed again. These tests exercise the shared runtime, not native device UI.
- Read the Swift UIKit/AppKit textbox producers and native Scroller touch
  observation code in addition to the web input/change listeners. Live textbox
  input was exercised on web; exact native commit timing and device gestures
  remain source-audited, not device-tested. No full async integration was added.
- Space Game's web build passed. The review embed has its current bundle and
  all three requested source tabs, with matching source/build fingerprints.
  It renders and runs to game-over. Movement/firing were not conclusively
  verified in the short browser check; retain a gameplay/usability check in the
  later example pass. The draft states its keys from source and how to restart.
- mdBook build, whitespace checks, all 18 chapter links, 16 heading/compatibility
  anchors, snippet/source parity, generated API parity, and embedded source
  parity passed. The rendered opening and both tables were inspected; the
  tables fit the chapter width.

Temporary verification artifacts (not publication inputs):

- `/tmp/pax-975-events-check.aur3Rm`: extracted app, reproducible extraction
  and link/parity checks, debug/release bundles, and scoped generator driver.
- `/tmp/pax-975-paxel-probe.wh0qsR/src/bin/events_contract.rs`: focused
  runtime event-delivery and lifecycle assertions.
- `/tmp/pax-975-docs-review.awrOqW`: rebuilt chapter and local Space Game
  bundle/source manifest. Packaging uses the existing example generator's
  helpers and does not publish anything.

Removed 8.7 GiB of rebuildable compilation caches for this fixture and the
Space Game verification build; review bundles and verification recipes are retained.
PAX-995 integration, the starter tutorial, remaining chapters, final navigation,
and publication remain outside this chapter checkpoint. PAX-975 stays In Progress.

### Space Game restart follow-up

At Zack's request, the canonical `examples/src/space-game` now includes a
centered **Play Again** Button below the game-over score. Mount and restart
share one helper that resets the ship, score, difficulty, asteroids, bullets,
held keys, and spawn/fire cooldowns. Cooldowns use the current runtime frame
because that clock continues between rounds. The decorative starfield keeps
scrolling.

- A focused Rust test passes for consecutive resets after long-running rounds,
  including nonzero score/difficulty and existing objects and held keys.
- Debug and optimized release web builds passed. In the docs embed, two
  consecutive game-over/restart cycles restored the ship and zero score
  without reloading the article. The centered button was visually checked.
  Native devices and extended gameplay were not exercised in this follow-up.
- The chapter now points readers to Play Again. Its three source tabs and web
  bundle are regenerated from the canonical example; no second prose-owned
  copy of the game exists. mdBook, 18 chapter links, 16 anchors, and source,
  snippet, and generated-API parity checks passed.
- The browser retained older nested example assets at the existing preview
  origin. The same local server's `localhost:8795` origin supplied the fresh
  assets for the browser checks and review. Nothing was published.

PAX-995 landed during this follow-up; its earlier deferred documentation
integration remains a separate editorial step after this example change.

### Space Game rotated-image rendering detour

Zack's gameplay review exposed invisible asteroids with working collisions.
The asset requests, live image sources, positions, sizes, and ordering were
valid. A temporary copy changing only asteroid rotation to zero rendered the
asteroids, isolating the stencil-clipped image path.

- The mixed retained vector/image renderer accumulated draw runs while
  immediately encoding all stencil-mask changes. It then drew every run
  against the final mask. A saved stencil depth does not preserve the mask
  geometry at that depth. This ordering was introduced by `1b224c051a`
  (PAX-848 GPU performance work), independently of the PAX-995 merge.
- Pending draws now precede every stencil push, pop, or replacement. Runs
  that differ only in rectangular scissors remain batched. The canonical
  game keeps its animated rotation; no asset or template workaround is needed.
- An in-repository Metal pixel regression test reproduces missing rotated
  images before the fix and passes afterward. It checks distinct masks at
  equal depth, nested clipping, oversized image clipping, mixed vectors and
  images, successive retained frames, immediate/deferred submission, and
  direct/mirrored screenshot capture. Run it explicitly on macOS with
  `cargo test -p pax-gpu retained_clip_pixels -- --ignored`; ordinary test
  runs skip this hardware-dependent case.
- That test also exposed screenshot buffers being mapped before their queued
  GPU copy was submitted. Readback now uses wgpu's map-on-submit mechanism,
  keeping capture valid with deferred command submission.
- All 10 ordinary `pax-gpu` tests and the GPU regression passed. Debug and
  optimized release Space Game web builds passed, and browser review showed
  visible asteroids in both builds. The rebuilt docs embed retained the
  centered Play Again control and restarted successfully. The release browser
  check reported no console errors. Native iOS/iPadOS rendering and extended
  gameplay remain untested.
- The local book and canonical example bundle/source tabs were regenerated.
  All 18 chapter links, 16 anchors, source/snippet/API parity checks, and
  whitespace checks passed. The renderer fix restores the documented behavior;
  no public syntax/API or baked program representation changed, and no prose
  workaround or duplicated game source was added. Nothing was published.

## R. Components and Composition — outline and evidence packet

**Editorial checkpoint:** Events accepted on 2026-09-08. Zack now authorizes
an internal outline/evidence step followed directly by a complete article
draft, with review after the draft. Continue one article at a time. This
supersedes the separate pre-prose outline approval for subsequent chapters;
the broader launch scope and publication boundary are unchanged.

**Reader outcome:** Extract the Field notes view into a reusable Rust/template
pair, pass values into it, choose an explicit state owner, and assemble
conditional views, stable keyed lists, and caller-provided child content.
Retain `components-composition.md`; absorb the useful control-flow material
and keep `control-flow.md` as a compatibility guide with its existing anchors.
Defer final sidebar changes to the navigation pass.

### Internal outline

1. A component boundary: Rust state/defaults, associated template, public
   properties, and per-instance state. Keep the Field notes visual language.
2. Extract a small `NoteCard`: complete files and module/re-export wiring,
   two invocations, parent/child expression scope, and input/default precedence.
3. Choose a state owner: ordinary expression input, deliberate `bind:` sharing,
   and why derived inputs should be changed at their source. Link Properties.
4. Control flow and identity: structural `if`/`else if`/`else`, ranges, item/index
   bindings, records, stable unique string/integer keys, and lifecycle effects.
5. Slots: positional header plus remainder content, caller-owned expressions
   and handlers, layout at the insertion site, active slot consumption order.
6. Component actions: a small named-event control with a real registered
   receiver and a two-argument handler; queued delivery and no event payload.
7. Scoped shared state: a domain-specific `Store`, provider setup, nearest-type
   lookup, shared property handles, and a clear missing-provider contract.
8. Read more: Layout next; Events, Properties, Motion, Routing, and current
   canonical examples for deeper use. Keep a small number of media briefs.

### Evidence and bounded verification

| Source / example | Draft consequence / check |
| --- | --- |
| `pax-macro/src/lib.rs`, file resolution and derives; `examples/src/router-playground/src/lib.rs` module/re-export pattern | Use `pub mod`/`pub use` and explicit crate-relative template paths. Each component has its own defaults; only the app root has `#[main]`. Compile the actual split-file pattern. |
| `pax-runtime/src/cartridge.rs`, `build_component_property` / `apply_component_property`; compiler property descriptors | Ordinary expressions derive inputs; `bind:` aliases the actual property handle. Verify parent-to-child propagation, independent defaults, and deliberate write-through. |
| `pax-runtime/src/conditional.rs`; `repeat.rs` keyed reconciliation and fallback handling; `examples/src/glow-buttons`, `transition-grid` | Structural branches control mounted content. Keys are unique strings/integers per loop; groups with retained keys keep identity through reorder. Invalid/duplicate keys warn and use positional fallback. Run focused runtime tests and a reordered interactive fixture. |
| `pax-runtime/src/slot.rs` projection resolver and tests; `examples/src/slot-projection-resolver` | Preserve zero-based slots, earlier active-site consumption, and silent empty remainder. Verify caller scope and fixed/remainder placement in debug/release. Avoid named-slot syntax. |
| `NodeContext::dispatch_event`, `RuntimeContext::flush_custom_events`, generated handler descriptors; `pax-std/src/forms/combo_box.rs` | A binding on the component registers its named receiver. The callback has no typed event argument; dispatch validates the receiver and queues delivery. Build and click the complete named-event pair. |
| `NodeContext::{push_local_store,peek_local_store}`, `RuntimePropertiesStackFrame`, `Store`; router playground chrome store | Lookup follows the runtime property stack by Rust type; nearest provider wins. Share a property handle, keep the store borrow short, handle absence, and do not describe a global singleton. Verify provider/consumer behavior. |

All runnable snippets will be assembled in a temporary verification project,
not a new canonical starter. Build debug and release sequentially, inspect
the running result, and check article links/compatibility anchors. Renderer
detour changes and other approved drafts are preserved. Public API reference
and internal runtime design material remain links, not prerequisites.

## S. Components and Composition — draft checkpoint

Drafted on 2026-09-08 after the internal outline above. The chapter now owns
split-file components, defaults and input scope, state ownership and `bind:`,
structural control flow, keyed identity, slots, named component actions, and
scoped stores. Three media briefs remain placeholders. `control-flow.md` is
a short compatibility guide preserving its `if`, `for`, and keyed-loop
anchors. `SUMMARY.md` and previously accepted chapters are unchanged in this
checkpoint; the final navigation pass still owns sidebar order and labels.

### Verification completed

- All 10 Rust and 9 Pax fences were extracted and assembled into a temporary
  application. Added test-only controls expose card-local state, reorder the
  collection, edit a shared input, invoke a named action, and reset a store.
  They are not additional public snippets or a new canonical starter.
- Current-source web builds passed in debug and optimized release modes.
  Browser checks verified independent defaults, ordinary input propagation
  without write-through, `bind:` write-through, conditional removal/recreation,
  range values, card-local state surviving keyed reorder, fixed/remainder
  slots, caller-owned projected expressions and handlers, named action
  delivery, and a descendant resetting its provider's shared property.
  Release checks exercised the baked representation and reported no console
  errors. The static debug preview reported repeated generic error events
  while served without the development service; those did not prevent the
  interaction checks. Their precise origin was not investigated here.
- Focused runtime suites passed: 8 slot tests, 9 repeat tests, and 4
  conditional tests. These cover additional projection, identity, and exit/
  re-entry behavior beyond the short browser interactions.
- `mdbook build`, 26 chapter/compatibility links, 18 anchors, snippet parity,
  and `git diff --check` passed. The article's opening was reviewed in the
  browser; code blocks and the page fit the current review viewport.
- Verification is web-based on this workstation. Native target builds and
  devices were not exercised for this documentation-only change. Public API
  comments and generated reference need no update: no public API or library
  behavior was changed by this chapter work.

Temporary recipes and the final debug/release app are in
`/tmp/pax-975-components-check.Bpqw8d`. The local preview is at
`http://localhost:8796/components-composition.html`, served from
`/tmp/pax-975-docs-review.awrOqW`. That book's existing Space Game embed and
canonical source tabs were restored after the book rebuild. Nothing was
published, and no commits or ticket status changes were made.

### Follow-up found during slot example verification

An initial version put the remainder `slot()` inside a vertical Stacker
with `gutter=8px`. The caller supplied a Text and Button, each with
`width=100% height=100%`, after the header consumed by `slot(0)`. In the debug
browser, both remainder children occupied the body area instead of separate
rows; the Text also obstructed the Button's activation. This is an observed
projection/container-layout issue, not yet a root-cause diagnosis. The
existing focused slot/repeat suites do not establish this combination.

The draft now uses a body Group with explicitly positioned caller content:
Text height 32px, Button y 48px and height 36px. Header binding and Button
delivery work in both debug and release. This keeps the slots explanation
verifiable without making a claim about automatic layout across that boundary.
Before teaching a Stacker containing remainder slots in Layout, reproduce
the earlier shape in isolation and inspect received children and container
frame ownership. Compare the canonical `slot-projection-resolver` example
too; its remainder bucket uses a similar composition. No runtime workaround
or opportunistic library fix was added here. Keep this follow-up visible to
Zack; do not silently promote the failed layout shape into a docs example.

`design/pain-points.md` remains untouched because of the known editorial
overlap. Next article after review: Layout and Responsiveness. PAX-975
remains In Progress until the complete launch spine is ready.

## T. Events follow-up — custom dispatch ownership

On 2026-09-08, Zack accepted Components and requested a dedicated Events
section for `NodeContext::dispatch_event`, then authorized proceeding to
Layout. Events now owns the complete AdvanceButton sender/invocation/receiver
example, context ownership, queued delivery, required/optional receivers,
payload limitations, and interaction with the original bubbling input.
Components retains its `component-actions` anchor as design guidance with a
link to that canonical explanation. The ending afterthought in Events is
removed. Source evidence: `NodeContext::dispatch_event`, the runtime custom
queue/flush, `ExpandedNode::dispatch_custom_event`, and generated handler
metadata. Recheck the extracted example in debug and release and preserve
the existing Events and Components anchors.

## U. Layout and Responsiveness — internal outline and evidence

**Reader outcome:** Predict the area a node receives, position and align its
content, choose free positioning or a row/column, and adapt the Field notes
cards to a narrow or wide space without changing application state.

**Owned concepts:** parent-local geometry; pixels/percent/mixed units; default
anchors; Group/Frame/Stacker responsibilities; transforms; padding and
content-driven sizing; viewport versus component bounds; responsive settings;
layout participation and breakout overlays. Link template grammar and style
sharing, drawing materials, text measurement, scrolling, and motion to their
canonical chapters. Preserve the existing layout/autosize/padding/role
anchors. Do not change `SUMMARY.md` during this checkpoint.

### Outline

1. The coordinate and size model: logical units, parent or assigned cell,
   ordinary fill defaults, and the different roles of size and position.
2. Alignment: default percentage anchors, concrete 400/100-pixel examples,
   mixed-unit edge insets, and explicit anchors for geometric coordinates.
3. Containers: Group, Frame, and Stacker; equal cells, gutters, and explicit
   cell sizes. Keep draw order separate from placement.
4. Padding and content-driven sizing: small measured stack, axis controls,
   explicit-size precedence, measurement cycles and text settling.
5. Responsive Field notes: same cards in a row or column via conditional
   settings; retain identity, use layout-specific breakpoints, and test short
   as well as narrow spaces. Explain viewport scope and a bounds-derived
   property for reusable components.
6. Transform origins and multi-axis syntax; shared visual settings and
   typography alignment versus element alignment.
7. Breakout content: excluded from flow/hulls while retaining ancestor
   clipping, scroll, and hit-test relationships.
8. Three bounded media briefs and deliberate onward links. No new starter.

### Source and verification plan

- `pax-runtime/src/layout.rs`: size resolution, implicit anchors, padding,
  transform composition, and content hulls. Use numeric assertions for percent
  alignment, explicit anchors, mixed sizes, and transformed bounds.
- `pax-runtime-api/src/layout.rs` / `transform.rs`: public common properties,
  units, axis shorthand contract, and layout role. Verify exact snippet syntax.
- `pax-std/src/core/{group,frame}.rs`, `layout/stacker.rs`, runtime container
  tests: container frames, clipping, equal-cell defaults, measured stacks,
  padding, explicit-axis precedence, and breakout exclusion. Run focused tests.
  Use `autosize=true` alongside Stacker axis overrides: its current layout
  implementation gates the measurement pass on that master flag.
- `pax-runtime-api/src/platform.rs`, globals in runtime engine, conditional
  settings implementation, `responsive-helpers`, `adaptive-cards`, and
  `materials`: viewport facts and component-local bounds. Build and resize
  the actual article snippets at representative narrow, breakpoint, and wide
  sizes in debug and release.
- The prior remainder-slot/Stacker overlap remains a separate follow-up.
  This chapter can teach direct Stacker children without asserting that the
  failing projection combination works. No runtime fix is authorized by this
  prose task, and the issue must remain visible rather than silently lost.

Recent layout commits were discovery leads, including `dc557d147` (padding/
autosize), `6e262a53e` (responsive helpers/settings), and `ba17a0c3d` (later
layout-related integration). Current source and focused checks control the
draft's claims. Native device behavior will be explicitly left unverified
unless exercised during this checkpoint.

## V. Custom dispatch and Layout — draft checkpoint

Completed the Events follow-up and drafted Layout on 2026-09-08. Components
is accepted; its action section now links to the full Events explanation.
Layout follows the internal outline above and retains three media briefs for
later production. The draft includes the distinction between a transformed
visual silhouette and the untransformed bounds used by flow measurement.
`SUMMARY.md` remains unchanged pending the final navigation pass.

### Verification completed

- Extracted the updated Events examples (7 Rust and 6 Pax fences) and all
  Layout examples (13 Pax fences and 1 Rust fence) into temporary applications.
  Current-source web builds passed in debug and optimized release modes.
  The Components extraction now reads the canonical custom-event code from
  Events; its remaining 8 Rust and 8 Pax fences retain source parity.
- Clicked the custom AdvanceButton in debug and release: one activation
  advanced its caller's progress from 25 to 50 percent without changing the
  comparison panels. A separate native contract check verified missing-name
  errors, deferred delivery, two queued deliveries, no redelivery on the next
  tick, and the caller-properties/emitting-node context pair.
- Exercised the responsive board at widths 360, 719, 720, and 1280. Its row/
  column breakpoint and progress preservation passed in debug and release.
  Two instances at widths 320 and 800 in one window also chose their layouts
  from their own allocated bounds. The short-window check confirmed the
  documented need for scrolling when the column exceeds the viewport.
- Mounted all 13 Layout Pax samples in the release fixture. Visual checks
  covered centered alignment, Frame clipping, fixed/flexible Stacker cells,
  the measured 156-pixel stack, and the explicitly anchored breakout panel.
  Native numeric assertions also covered default/explicit anchors, mixed-unit
  positions and sizes, fill defaults, and centered transformed bounds.
- Focused Rust suites passed: 10 runtime layout tests, 12 container tests,
  and 19 Stacker tests (41 total). Release browser checks reported no console
  errors. Native app builds and iOS/iPadOS devices were not exercised.
- `mdbook build` passed. Link/anchor checks passed for Events (19/19),
  Components plus its control-flow compatibility guide (26/18), and Layout
  (21/18). Snippet parity, the three canonical Space Game source tabs, and
  `git diff --check` passed. The rendered Events section and Layout opening
  were reviewed; the pages fit the review viewport, and Layout's code blocks
  had no horizontal overflow there.

Verification recipes and builds are under `/tmp/pax-975-events-check.aur3Rm`
and `/tmp/pax-975-layout-check.I73XHF`; the native contract is in
`/tmp/pax-975-paxel-probe.wh0qsR/src/bin/dispatch_layout_contract.rs`. The local
book preview remains `http://localhost:8796/`, served from
`/tmp/pax-975-docs-review.awrOqW`. Its Space Game bundle and canonical source
tabs were restored after rebuilding. These are temporary review artifacts,
not additional canonical examples or publication output.

No library/API behavior or generated API reference was changed by this
checkpoint. Prior renderer, Space Game, and editorial changes were preserved.
No commits, publication, or ticket status changes were made. Layout is ready
for Zack's draft review; the earlier remainder-slot/Stacker observation remains
an unresolved, separate follow-up.

## W. Text, Fonts, and Images — internal outline and evidence

Zack accepted Layout and authorized the next chapter. Reader outcome: give
content a readable hierarchy, constrain and measure text, select and load a
font deliberately, and place images with an explicit fitting policy.

### Outline

1. Text content and a small Field notes hierarchy; reactive strings link to
   Properties/PAXEL, reusable typography links to Templates.
2. Width, wrapping, omitted-height measurement, clipping, and the distinction
   between block alignment and multiline alignment.
3. Font values: installed family, named modifiers, direct font-file URL,
   Google Fonts' special CSS path, fallback/loading, and native boundaries.
4. Selection, editable text and deliberate `bind:`, trusted Markdown; link
   forms and application handlers to their canonical chapters.
5. Asset-owned Image sources, explicit display bounds, Fit/Fill/Stretch,
   reactive sources, and raw RGBA data.
6. NativeImage's separate URL API and current target-specific source/fit
   behavior. Text alternatives and format limits without accessibility claims.
7. Two bounded media briefs and onward links; preserve existing anchors.

### Evidence and checks

- `pax-std/src/core/text.rs`: Text/TextStyle defaults, font coercion, native
  measurement requests, editable-text property updates, solid-color text
  patches. Font size currently requires pixels; gradient fills become their
  first stop color in native text patches.
- `pax-compiler/files/interfaces/web/src/classes/{text,native-element-pool}.ts`:
  font loading, wrapping, alignment, Markdown, editing, image decoding and
  native image object-fit. The old article's generic stylesheet URL guidance
  is inaccurate: only the Google Fonts CSS URL pattern gets CSS treatment;
  other nonempty font URLs are loaded as font files.
- `pax-compiler/files/swift/pax-swift-common/Sources/{Messages/Messages,Rendering/Rendering}.swift`
  and the macOS/iOS `PaxView*` image loaders: source-backed target caveats.
  Relative bundled font URLs are not resolved to native bundle resources by
  the Web-font loader; native image widgets read local files. macOS's native
  image cover mode currently stretches. Do not claim uniform behavior.
- `pax-std/src/media/{image,native_image}.rs`: source coercion, fit geometry,
  explicit bounds, raw data contract, distinct canvas/native paths. The web
  canvas loader prefixes the document directory to every source, so the old
  article's arbitrary remote Image URL example is not a supported recipe.
- Canonical leads: `space-game`, `photo-picker`, `runtime-settings-themes`,
  and `auto-sized-containers`. Recent commits including `3b5664409` and
  `20337d2c4` are discovery leads; current source and tests control claims.
- Assemble the actual snippets in a temporary app, compile web debug/release,
  inspect wrapping, typography, editing, fitting and local asset loading, and
  run font/image coercion tests. Check links/anchors and mdBook output.

This is documentation work only. Newly exposed implementation limits remain
visible in the chapter and checkpoint rather than expanding into library
fixes. Do not modify the known-overlap pain-points article. Correct adjacent
Text/font API comments and regenerate only their affected reference page.

## X. Text, Fonts, and Images — draft checkpoint

Drafted on 2026-09-08. The article now owns text hierarchy, width/measurement,
wrapping/clipping, text alignment, font values and loading, selection/editing,
trusted Markdown, image assets, fitting, raw RGBA data, and the canvas/native
image boundary. Existing `font-values`, `image-sources`, and title anchors
are preserved; the planned `text-images-and-assets` alias is present. Two
media briefs remain placeholders. Layout is accepted; Drawing and Styling
is the next article after this review.

### Verification completed

- Extracted all 12 Pax fences into a temporary verification application,
  keeping per-snippet component scope and a Notes-like title property for
  the editing example. Final current-source web debug and optimized release
  builds passed. The fixtures are not a new canonical example or starter.
- Browser checks in debug/release covered the hierarchy, wrapped measurement
  (312-pixel width / 63-pixel height versus 952 / 21), fixed clipping without
  ellipsis, centered multiline alignment, italic/bold font settings, bound
  editing that updates the second label, and Markdown emphasis.
- The web font-file recipe used a workstation font copied only into the
  temporary fixture under the documented filename. Its family was applied
  and the caption rendered. This is not a redistributed font asset or a test
  of remote hosting, native font registration, or first-load fallback timing.
- The three fit modes rendered their expected distinct shapes after the
  source was loaded; raw RGBA data rendered; NativeImage reported a decoded
  32-by-32 image with `contain` fitting in both builds. A first-load canvas
  image problem was open at this checkpoint; the authorized detour in Y
  diagnoses and resolves it.
- Five font coercion tests and two image-source coercion tests passed. These
  establish shorthand/default/weight/constructor behavior, not native-device
  display parity. No native application or device tests were run here.
- Corrected adjacent Text/font public comments, including the stale generic
  stylesheet recipe, unconditional selection wording, and gradient color
  limitation. Used the repository's API generator against current rustdoc
  JSON and copied only the regenerated `api/pax-std/core/text.md`; its diff
  contains the corresponding comment changes only. No library behavior was
  changed, and the generated page matches the generator output.
- `mdbook build`, 17 article links, 15 anchors, snippet parity, and
  `git diff --check` passed. Events' 19 links/19 anchors and canonical Space
  Game source tabs were rechecked after restoring its embed. The chapter's
  rendered opening was reviewed; long literal strings were split into
  equivalent PAXEL concatenations to keep code readable at the review width.

Artifacts and verification recipes: `/tmp/pax-975-content-check.DP1RNG`.
Local review: `http://localhost:8796/text-fonts-images.html`; the same book
also hosts temporary `_content_debug/` and `_content_release/` fixtures.
`SUMMARY.md`, the known-overlap articles, and unrelated pending work remain
untouched. Nothing was committed, published, or moved to In Review in Linear.

### Follow-ups exposed by this chapter

**First-load canvas image (resolved in Y):** Starting with a fresh release-page load, advance
from sample 0 to sample 8, which contains the exact bundled Image snippet.
The image remains blank while left on that sample, including a later capture
after other verification work. Advance to sample 9: the same source appears
in all three fit modes. Return to sample 8: it now appears. The debug pass
also showed the initial blank and subsequent loaded fit comparison. NativeImage
loads normally. No artificial animation, property nudge, preload, or remount
workaround was added to the chapter. This needs an isolated runtime/render
diagnosis before calling the image examples launch-verified. A source lead
is the web Image/Data interrupt loading the renderer image without an explicit
node invalidation in that handler; this is not yet a proven root cause.

**Source-backed platform gaps:** generic CSS font URLs, native relative font
assets, arbitrary remote canvas Image URLs, macOS NativeImage Fill behavior,
and iOS/iPadOS unclipped text selection are now described accurately rather
than implied to work uniformly. These are candidates for separate library
work, not documentation workarounds. The lack of a public image `alt` property
is explicit; the earlier inventory's “image alt” lead was not confirmed in
current source. No accessibility completeness claim is made.

The known-overlap `design/pain-points.md` remains untouched; this checkpoint
preserves the findings for later coordination.

## Y. Canvas image first-load fix

Zack authorized an implementation detour on 2026-09-08 after reviewing the
Text, Fonts, and Images checkpoint.

### Cause and relationship to the clipping fix

The web chassis's `Image/Data` completion handler populated the renderer's
image cache but did not invalidate the requesting node or its canvas layer.
An Image's first render can run before asynchronous decoding finishes. That
render leaves the Image node dirty, but the engine clears the layer's dirty
flag at the end of the frame. `has_canvas_render_work()` then reports no work,
so later renders return before consulting the newly populated cache.

This explains both symptoms: a static first mount stays blank; a subsequent
remount or other redraw reveals the cached pixels. The regression test uses
the real Image primitive and engine render gate, and demonstrates that the
old cache-only completion leaves the canvas asleep.

The earlier asteroid fix concerned retained draw batching across stencil
changes. That failure happened during rendering; this one prevents rendering
from being scheduled. The fixes address separate stages of the same path.
The clean-canvas gate is present in `9c933f14ad` (the no-GPU-work-at-rest change),
while the web cache-only completion predates it. This is source provenance,
not a bisect establishing the first affected release.

### Implementation and scope

- `pax-chassis-web/src/image_loading.rs` now owns the web completion path.
  After loading decoded pixels, it marks the live requesting Image node and
  its current canvas layer dirty. `src/lib.rs` calls that helper from the
  existing interrupt handler.
- Late completions after unmount can still populate the cache without
  scheduling stale nodes. Out-of-order responses do not change the current
  source property. Concurrent requests retain their existing per-request
  callbacks; no cache-coalescing or image-lifetime redesign was introduced.
- The Apple bridge already invalidates canvas nodes/layers when image data
  arrives. Its implementation was left unchanged. The web helper is shared
  by the GPU and Piet renderer selection paths.
- No public API, manifest format, generated cartridge representation, or
  authoring workaround was added. The human-facing Image section now
  explains asynchronous loading and its automatic redraw.

### Verification

- Five focused tests exercise delayed completion on an idle canvas, targeted
  node/current-layer invalidation, concurrent same-source requests, unmounted
  nodes, and out-of-order source changes. All pass in debug and release; the
  complete web-chassis debug suite passes (eight tests).
- The existing retained-clipping pixel test passes with mirrored/unmirrored
  and deferred/immediate submission on both frames (eight combinations).
- Rebuilt all twelve chapter snippets for web debug and optimized release.
  Fresh browser loads of each build now show sample 8's bundled image on its
  first mount, without leaving the sample, resizing, or changing properties.
- Rebuilt the canonical Space Game web debug bundle for the local docs
  preview. The browser shows the ship, starfield, and moving asteroids. The
  prior Play Again and retained-clipping changes are preserved.
- `mdbook build`, the chapter's 17 links/15 anchors and twelve-snippet parity,
  Events' 19 links/19 anchors and three canonical source tabs, and
  `git diff --check` pass. The fixed debug/release fixtures are available
  locally at `_image_fixed_debug/` and `_image_fixed_release/` under the
  existing review server; these remain temporary verification artifacts.

The native application targets and a separately forced Piet browser session
were not exercised by this detour. The remaining source-backed platform gaps
in X and the earlier remainder-slot/Stacker observation remain separate work.
No commits, publication, ticket status changes, or known-overlap article edits
were made.

## Z. Drawing and Styling — internal outline and evidence

Zack authorized the next article after the canvas-image fix. Per the updated
editorial process, outline internally and proceed to a draft before review.
Reader outcome: build a coherent vector surface, understand its paint and
geometry, and find the path/SVG and lighting workflows when needed.

### Outline and ownership

1. A small Field notes surface; Rectangle/Ellipse/Line/Path vocabulary and
   explicit geometry. Link Layout and template ordering rather than reteach them.
2. Color channels, alpha versus element opacity, fill/stroke defaults, centered
   strokes and cap/join choices; native Text points back to its own chapter.
3. Corner radii and linear gradients. Move the temporary shape reference from
   PAXEL here, retaining its old anchors as links. Qualify radial backend behavior.
4. A compact class-based visual system; shared palette/theme choices and the
   existing ImportSettings chapter/example, without repeating precedence rules.
5. Path command lists, local points/control handles, closed contours and draw
   ranges; smoothing/Handwriter discovery, motion linked to its canonical chapter.
6. SVG validation, compile-time component import, and editable ejection. Make
   subset limitations and source ownership explicit. Verify actual CLI commands.
7. LightFrame/material orientation with one small lit surface, current defaults,
   scoping and GPU/Piet limits. Defer exhaustive material tuning.
8. Bounded media briefs and read-more links. Keep all agreed feature anchors.

### Source and verification plan

- Current `pax-std/src/drawing/{rectangle,ellipse,line,path,lighting,handwriter}.rs`
  and `pax-runtime-api/src/{color,drawing,unit_value}.rs` establish public types,
  defaults, path grammar, draw ranges, and light/resource semantics.
- `pax-runtime/src/engine/{pax_gpu_render_context,piet_render_context}.rs`,
  `pax-gpu/src/render_context.rs`, and `pax-runtime/src/cartridge.rs` establish
  actual fill lowering and runtime gradient evaluation. Stops need percentages
  on Piet. Radial parameter interpretation currently differs: GPU scales the
  start-to-end vector by radius; Piet treats start/end as origin/center and
  radius as its radius. The old equal-start/end PAXEL radial recipe therefore
  cannot be presented as a portable, verified drawing example.
- Canonical examples: `path-drawing`, `runtime-settings-themes`, `glow-buttons`,
  `materials`, and `example-host`. PAX-967/PAX-966 and completed commit history
  are discovery leads; source overrides old aspirations or superseded designs.
- `pax-compiler/src/svg_import.rs` and `pax-cli/src/svg_import.rs` own the SVG
  subset, diagnostics and file generation. Verify validation/ejection and build
  both SVG forms with the article's actual Pax snippets in web debug/release.
- Run bounded color/drawing/coercion, primitive/path, and SVG importer tests;
  inspect rendered shapes, alpha, gradient direction, draw range, and lighting.
  Check mdBook, article/PAXEL compatibility links and anchors, and snippet parity.

New library behavior is outside this chapter draft. Record any additional
implementation gaps here for Zack; leave the overlapping pain-points file alone.

## AA. Drawing and Styling — draft checkpoint

Drafted on 2026-09-08. The chapter now teaches shapes, color/alpha, centered
strokes and caps/joins, corner radii, linear gradients, shared visual settings,
path commands and draw ranges, SVG import/ejection, and scoped lighting.
It preserves the old title anchor and the planned `shapes-and-gradients`,
`paths-and-svg`, and `lighting-and-materials` destinations. Two interactive
example briefs and one diagram brief include three possible treatments each.

PAXEL's temporary corner/gradient reference now links here, retaining its
existing headings and deep links. Timeline-relative values remain in PAXEL
until the Motion pass. ImportSettings setup and precedence stay in Templates;
opacity/compositing and animation timing keep their separate destinations.

### Verification completed

- Extracted and built all twelve Pax fences plus the SVG component Rust
  declaration with current source, in web debug and optimized release.
  Browser checks covered every sample in both builds: the layered notes
  surface, matching alpha/opacity, round line endpoints, asymmetric corners,
  diagonal/default gradients, inline class override, cubic/closed paths,
  a half-revealed stroke, imported SVG, and a glossy light-reactive surface.
- Validated the canonical `path-drawing` signature SVG, inspected `--stdout`,
  ejected an editable component, and compiled that separate project in web
  debug/release. Its release rendering matches the imported version. A
  repeat ejection correctly refuses to overwrite existing source without
  `--force`; generated Pax matches the CLI stdout output.
- Passed 53 runtime-API tests, 22 drawing-primitive tests, 13 SVG importer
  tests, and seven light-scope runtime tests (95 total). These cover coercion,
  smoothing, path trimming, corner arities, line coverage, SVG subset behavior,
  light containment, ambient handling, and deterministic overflow selection.
- `mdbook build` and `git diff --check` pass. Drawing's 21 links/16 required
  anchors and PAXEL's 22 links/five compatibility anchors pass, as do snippet
  parity and the canonical SVG fixture check. Text's 17 links/15 anchors and
  Events' 19 links/19 anchors plus three Space Game source tabs were rechecked.
- Reviewed the rendered chapter opening at the normal review viewport; its
  code blocks have no horizontal overflow there. The local Space Game embed
  was restored after the book rebuild. No native app or forced-Piet browser
  run was performed, and no cross-backend visual parity is implied.

### Findings retained for follow-up

- **Radial gradient geometry:** the GPU/Piet interpretation mismatch recorded
  in Z remains unresolved. The old equal-center radial snippet was removed
  with the relocated reference, and the new article states the backend limit.
  This source-backed finding was not turned into an ad hoc portable recipe.
- **Stroke reveal caps:** in both browser builds, the half-revealed GPU stroke
  has a flat cut at its new endpoint even with Round caps. The geometry shader
  clips fragments by path progress; it does not generate a new round cap at
  the cut. The article describes that current behavior. No renderer changes
  were made in this chapter pass.
- **Authored helpers:** material constructors such as `Material::glossy(0.6)`
  belong inside `{...}` when assigned in a template. The initial bare helper
  fixture failed static parsing; the corrected article and fixture compile.
- **SVG scaling:** imported points are normalized to bounds while emitted
  stroke widths remain pixels. The article asks readers to check stroke
  weight at their intended display size, as well as the source aspect ratio.

Verification sources and builds: `/tmp/pax-975-drawing-check.LbflIr`.
Local article: `http://localhost:8796/drawing-styling.html`; temporary fixtures
are `_drawing_debug/`, `_drawing_release/`, and `_drawing_ejected/` on the same
server. These are verification apps, not additional canonical examples.

No library/API comments, generated reference, SUMMARY, known-overlap articles,
or unrelated pending changes were modified. No commits, publication, or Linear
status changes were made. Drawing is ready for Zack's draft review; Native
Controls is the next chapter. Earlier unresolved findings remain separate.

## AB. Accessibility and Native Controls — working outline

Drawing and Styling was approved; Zack authorized the next chapter. As agreed,
this outline is an internal intermediate step before a complete draft review.

### Reader outcome and ownership

Build a small form whose native controls edit component properties, choose the
right event boundary for side effects, and understand the current platform and
accessibility contract. Use the approved `accessibility-native-controls.md`
destination, update direct inbound prose links, and preserve the old
`input-native-controls.html` URL and its PhotoPicker/Other Controls anchors.

1. Native controls in the Pax scene: shared layout, platform editing behavior,
   native visual properties, and the difference between ordinary controls and
   PhotoPicker's slotted affordance. Link Layout/Compositing for deeper mechanics.
2. A complete small Field notes form: Textbox, Checkbox, Dropdown, Slider,
   Button; direct `bind:` state and an explicit Rust save/validation handler.
3. Compact control/property/event map, text input versus committed change,
   index-based selections, multiline editing, and styling ownership.
4. Focus, keyboard interaction, and a candid current accessibility boundary:
   platform-native building blocks do not establish full application semantics.
5. PhotoPicker: custom affordance, completion/partial-result handling,
   transient previews versus persistence, permissions and source/target limits.
6. Source-backed support matrix and read-more links; bounded interactive form,
   picker example, and focus/semantics diagram briefs with three treatments each.

### Evidence and verification plan

- `pax-std/src/forms`, `pax-runtime-api/src/events.rs`, web native-element-pool
  and event listeners, Apple Rendering.swift and Messages.swift, and both
  chassis interrupt handlers establish current behavior. PAX-932 (Done) and
  PAX-958 (Backlog), including their empty comment streams, were read as leads.
- Both chassis update bound native properties before TextboxInput,
  CheckboxChange, and SliderChange handlers. Dropdown and RadioList update
  their selected index without a dedicated public change event.
- Apple source implements all six basic controls, including both Textbox
  modes. Styling differs: native sliders do not apply `step`; Apple multiline
  textboxes do not display `placeholder`. These are limits, not new fix scope.
- PhotoPicker's web pipeline copies optional bytes; the current Swift event
  payload contains metadata and temporary file handles, but no bytes. Byte
  limits apply even when copying is disabled. Partial selection can report
  Selected with a warning; web dismissal does not have an explicit cancel
  listener, so a completion callback cannot be promised for every dismissal.
- Web focusable elements receive tabindex values derived from visual z-order;
  the public API lacks an independent tab-order/semantic-label contract.
  Native elements and visible text are a foundation, not an audited reading
  order or annotation system. PAX-958 remains separate, unfinished work.
- Extract actual article snippets into disposable verification apps and build
  web debug/release. Exercise editing, selection, submission, and keyboard
  interaction in the browser. Verify PhotoPicker's compiled affordance and
  existing canonical source, without claiming an Apple device or assistive-
  technology audit. Rebuild mdBook, check links/anchors/compatibility redirect,
  and restore the existing Space Game embed. No publication or library fixes.

## AC. Accessibility and Native Controls — draft checkpoint

Drafted on 2026-09-08 after Zack approved Drawing and Styling. The chapter
now lives at the agreed `accessibility-native-controls.md` destination. It
teaches a complete editable Field notes form, control-specific bindings and
events, multiline editing and styling, keyboard focus, the accessibility
boundary, and PhotoPicker's results, storage, permissions, and platform limits.
Two interactive-example briefs and one diagram brief each have three treatments.

### Navigation and ownership

- SUMMARY now names and links this chapter. Direct inbound links from PAXEL,
  Events, Text/Images, and Drawing use the canonical destination.
- The old Markdown file is a short migration pointer. A source HTML alias
  preserves the old book URL, query string, and fragment. mdBook's stock
  meta-refresh redirect dropped `#photopicker` in the browser check, so the
  alias uses a small location replacement, with a no-JavaScript fallback.
  The chapter retains `input--native-controls`, `photopicker`, and
  `other-controls`, plus the planned `native-controls` and `current-support`.
- Binding mechanics remain in PAXEL/Properties, event delivery in Events,
  layout/compositing in their chapters, and image loading in Text/Images.
  No canonical example source or generated API reference changed in this pass.

### Verification completed

- Extracted all four Pax fences and both Rust fences into disposable apps.
  The complete form and supplementary snippets compile in web debug and
  optimized release; source-to-fixture parity passes.
- Browser checks in both build modes covered Textbox editing, Checkbox state,
  Dropdown selection, Slider values, empty-name validation, and Rust submission
  of all four bound values. Keyboard checkbox activation, slider adjustment,
  and Button activation also work in the tested browser.
- Walked Textbox → Checkbox → Dropdown → Slider → Button with Tab in the
  representative form. This validates that fixture, not a general tab-order
  contract. Supplementary debug/release checks covered multiline text,
  RadioList's bound index, and focus-on-mount. Actual typed input in release
  confirmed the updated bound value in both input and committed-change handlers.
- PhotoPicker's article affordance renders in debug/release. The canonical
  `examples/src/photo-picker` project also builds in both modes, and its release
  opening renders with Library and Camera affordances. The automated chooser
  flow timed out before returning a chooser, so actual file selection, decoded
  preview, cancellation, and permission outcomes were **not** verified through
  the browser. Source inspection is the evidence for their documented behavior.
- The focused Apple Info.plist metadata test passes, covering iOS values,
  iPadOS inheritance, and target overrides. No Apple app/device build, camera
  permission test, or screen-reader audit was performed.
- mdBook and `git diff --check` pass. The new chapter's 23 links and 14 required
  anchors pass. Rechecked Drawing (21 links), PAXEL (22), Text/Images (17), and
  Events (19), including the three canonical Space Game source tabs.
- Verified the old PhotoPicker deep link redirects with both query and fragment
  intact. The rendered chapter's code blocks fit the normal review viewport.
  A requested narrow viewport override did not change the fixture's observed
  1280×720 bounds; it was reset, and no narrow-viewport result is claimed.

### Follow-up findings, without library changes

- **Apple Slider step:** UISlider/NSSlider apply min/max/value but not `step`.
  A continuous slider is implemented; discrete stepping parity is missing.
- **PhotoPicker byte delivery:** Swift's emitted photo entries contain metadata
  and temporary file handles but no data bytes, even with `include_bytes=true`.
  Web supports the optional byte copy. The article preserves that distinction.
- **Picker limits and partial success:** a positive per-photo byte limit is
  enforced even when copying is disabled. Accepted photos can accompany a
  Selected status plus an over-limit warning; the previous prose overstated
  size-limit status as the outcome for every oversized member of a selection.
- **Web picker dismissal:** there is no explicit input `cancel` listener;
  completion cannot be promised for every native-dialog dismissal.
- **Accessibility:** independent semantic labels/roles/order remain unexposed,
  and a nearby Text does not label a control. Web tabindex is tied to visual
  z-order. These observations align with still-open PAX-958 and should inform
  that task's audit; this draft is not an accessibility implementation pass.
- Apple multiline placeholders and Button hover color remain limited as
  described in the article. The known-overlap pain-points file is untouched;
  these findings are recorded here for later coordination.

Verification sources and builds: `/tmp/pax-975-native-check.5MNSTL`.
Local article: `http://localhost:8796/accessibility-native-controls.html`.
Temporary apps: `_native_debug/`, `_native_release/`, `_native_extras_debug/`,
`_native_extras_release/`, and `_photo_picker_release/` on the review server.
The Space Game embed was restored after the final book rebuild.

No commits, publication, Linear status changes, or library changes were made.
Unrelated pending work is preserved. This chapter is ready for Zack's draft
review; Animation and Motion is next in the current review sequence.

## AD. Animation and Motion — working outline

Zack authorized Animation and Motion after reviewing Native Controls. Per the
current editorial agreement, outline internally and proceed to a complete
draft before the next review.

### Reader outcome and ownership

Choose a motion mechanism, author a small timeline, control a shared playhead
from state, and keep enter/exit/reflow behavior legible under rapid changes.
Retain the existing article slug and compatibility headings; add the planned
`timelines-and-easing` and `structural-transitions` anchors. Move the temporary
PAXEL timeline-relative example here, preserving its old heading as a link.

1. A first looping property track: markers, values, outgoing easing, duration,
   loop behavior, frame clock versus monotonic elapsed milliseconds.
2. Named selector tracks and shared playback: complete Rust/Pax replay/pause/
   scrub example. Clarify numeric playhead units and ordinary global clocks
   versus lifecycle-local time; avoid suggesting a named-timeline start API.
3. `$base` and property ownership: relative values, same-component selectors,
   and inline overrides. Keep ordinary settings precedence in Templates/PAXEL.
4. Rust easing queues and cancellation: ease_to, ease_to_later, explicit units,
   current-value interruption, and cancelling before immediate state ownership.
5. Enter/exit with one reusable NoteCard and a conditional host. Component-
   scoped and inline element forms; explain named element selector scope.
6. Takeover versus Restart, retained instance identity, parallel transitions,
   delayed unmount and the five-second exit timeout; no velocity guarantee.
7. Stacker Flow/Ghost and Snap/Ease as independent choices. Use `frames`, not
   the old article's unsupported `duration` field. Named reflow remains reserved.
8. Path reveal/procedural discovery, bounded motion design and reduced-motion
   app choices, example/media briefs, and read-more links.

### Evidence and bounded checks

Read PAX-975/PAX-860 descriptions, status, relations and all comments again;
the user-approved per-chapter workflow overrides the older outline-review gate.
Read completed PAX-866 and PAX-946 with all comments. Current source, rather
than intermediate historical ontology or unfinished-work comments, is canonical.

- `pax-runtime-api/src/animation.rs` and property transition implementation:
  duration clocks, named easing curves, queue reset/cancellation and type limits.
- `pax-runtime/src/cartridge.rs`: keyframe sampling, default looping, outgoing
  easing, bound playhead units, relative values, settings layers, and typed
  takeover/restart tests. Ordinary selector timelines precede inline settings;
  lifecycle overlays have their own lowering and must not be collapsed into a
  blanket rule that every timeline wins over inline properties.
- Manifest transition lowering and ProgramIR/binary/Rust constructor paths:
  finite local lifecycle playback, duration inheritance, and release behavior.
- ExpandedNode/conditional/repeat retention and Stacker source/tests:
  interruption on the same mounted identity, exit timeout, Flow/Ghost and reflow.
- Canonical timeline-playground, transition-grid, pax-logo, marionette, and
  mouse-animation source establish concrete uses; build a bounded subset, not
  every example or every possible rendering backend.

Extract actual article snippets into disposable apps, compile web debug/release,
and inspect playback, scrubbing, stop/replay, relative values, lifecycle reversal,
and representative Stacker changes. Run focused timeline, transition, retention,
reflow and release-format tests. Rebuild the book and check anchors/links. Keep
new findings here, leave the overlapping pain-points/routing files untouched,
and do not turn this chapter into an unapproved runtime-fix detour.

## AE. Animation and Motion — draft checkpoint

The chapter draft is ready for Zack's review. It follows the approved sequence:
an internally prepared outline, source research, then one complete article.
The existing slug and compatibility anchors remain intact. PAXEL's temporary
timeline-relative example now links to Motion's canonical explanation, with
its original heading retained for inbound links.

### Delivered

- A first property timeline, markers and clock units, default looping, and
  outgoing-keyframe easing. Ordinary application clocks are distinguished from
  lifecycle-local clocks without promising a fixed rendering rate.
- A complete Rust/Pax replay, pause, and scrub example with a shared playhead,
  three coordinated tracks, and working `$base`-relative placement.
- Rust easing queues, interruption, and cancellation before immediate edits.
- A reusable NoteCard with finite component enter/exit transitions, inline
  element transitions, selector scope, retained identity, Takeover/Restart,
  delayed unmount, and the current five-second exit timeout.
- Stacker's independent Flow/Ghost and Snap/Ease policies, keyed changes,
  canonical transition-grid discovery, and reserved Named reflow support.
- Path/scroll-driven discovery links, motion choice and reduced-motion limits,
  one diagram brief and two interactive-example briefs. These media remain
  production placeholders; the temporary verification apps are not a new
  canonical example project or a published embed.

### Corrections and authoring findings

- The previous Stacker recipe used `duration`; the actual reflow field is
  `frames`. The draft compiles with `frames: 18`.
- The prior PAXEL relative-timeline example put `y` inline, hiding the ordinary
  selector timeline for that property. The new working example puts its base
  `x` in a settings rule. Lifecycle overlays are documented separately.
- PathElement and BezPath currently use the default Interpolatable behavior;
  compatible command lists alone do not produce coordinate morphing. The draft
  points to numeric-parameter-derived geometry and `draw_end` reveals instead.
  No interpolation implementation was changed or broadened in this task.
- Pause before scrubbing makes property ownership explicit. `set` alone does
  not clear an active Rust easing queue; replay cancels before resetting.
- No unified reduced-motion platform preference is exposed by the current
  public API. Application choices are recommended without implying automatic
  OS preference handling. The known-overlap pain-points file remains untouched.

### Bounded validation

- Extracted all six Pax fences and three Rust fences from the article into two
  disposable apps; both built successfully for web debug and baked release.
  Snippet-to-fixture parity is checked, rather than testing rewritten samples.
- Browser checks in both modes cover replay to 100, pause holding an intermediate
  value, keyboard scrubbing, relative marker placement, component and inline
  element visibility changes, keyed removal/reversal/restoration, and a queued
  pulse reaching 1.00 before settling to 0.20. Screenshots confirm list visual
  ordering rather than relying on retained native-text DOM order.
- Precise interruption and retention semantics are grounded in focused runtime
  tests, not inferred from timing short browser clicks. Timeline-filter tests
  pass (15), transition-filter tests pass (14; these groups overlap), Stacker
  tests pass (19), animation API tests pass (8), and manifest tests pass (5),
  including ProgramIR and binary roundtrips.
- Canonical `examples/src/transition-grid` builds successfully in web debug and
  release. Other referenced motion examples were inspected as source discovery
  references, not exhaustively built across all targets.
- The book builds; Motion's 17 internal links and 19 heading/compatibility
  anchors resolve. PAXEL's preserved timeline-relative anchor and new canonical
  link pass, as do the previous Native, Drawing/PAXEL, and Events checks,
  including the native-controls alias and Space Game source-tab parity.
- No macOS, iOS, or iPadOS device run, frame-rate benchmark, accessibility audit,
  or published-docs deployment is claimed. Planned media remains unfinished.

Verification sources and builds: `/tmp/pax-975-motion-check.6cQNhI`.
Local chapter: `http://localhost:8796/animation-motion.html`.
No commits, library edits, publication, or Linear status changes were made.
The next chapter in the current review sequence is Compositing and Effects.

## AF. Compositing and Effects — working outline

Zack approved proceeding after Motion. Prepare the outline internally, then
draft and verify the chapter before requesting the next review.

Reader outcome: predict how vector, image, text, and native-control surfaces
combine; choose Group, Frame, or Mask; reason about inherited opacity; and
recognize when an effect depends on a particular renderer or native platform.
Keep `compositing-effects.md`, preserve `compositing--effects`, and establish the
planned `native-compositing` anchor. Link Drawing's paint/geometry and Motion's
timing explanations rather than reteaching them.

1. Source order and the mixed rendered/native scene, followed by a concise
   choice among grouping, rectangular clipping, and geometry masking.
2. Frame clipping with rounded corners and overflowing native/vector content.
   Frame adds no fill; nested constraints and visual bounds stay explicit.
3. Mask's exactly-two-child contract: content first, geometry subtree second.
   Demonstrate a filled ellipse and a closed path. Explain hidden source
   geometry, combined coverage, units/transforms, and reactive geometry.
4. Distinguish coverage masking from raster-alpha/luminance masking. Empty
   source coverage does not mean hide-all. Avoid compound-path boolean claims.
5. Inherited opacity and paint alpha multiply. A Group/Frame/Mask does not
   establish a general offscreen, flattened opacity-isolation surface.
6. Native compositing: punch-through coverage, scroller-owned islands, and
   limits of cross-surface translucent gradients, image alpha, and path reveals.
   Show a native EventBlocker underlay for tinting/blocking the background;
   keep routing and accessibility ownership elsewhere.
7. Bounded Apple LiquidGlass example and fallback notes; lighting links back
   to Drawing. General blur, blend modes, and backdrop effects are deferred.
8. Short practical verification path, canonical examples, media briefs, and
   read-more links to Scrolling, Layout, Drawing, Motion, and Native Controls.

Evidence: PAX-975 contract/comments; PAX-831/PAX-883 completed work; PAX-915's
final pause/correction comment and PAX-963's current backlog scope. Reviewed
Mask/Frame/Group/EventBlocker/LiquidGlass source, runtime inherited opacity,
coverage/native-mask and raycast passes, web clip/opacity/mask projection, and
Apple glass/blur fallbacks. Read relevant occlusion, neon-opacity, liquid-glass,
and router examples plus recent compositor history and existing pain points.

Bounded checks: extract article snippets into a disposable app, build web debug
and release, visually inspect clips/masks/opacity and interact with visible
controls/underlays. Build a canonical occlusion example, run focused existing
coverage and GPU clipping tests, and check book links/anchors. Apple native
effect visuals require a separate device run and will not be claimed from web
validation. Record findings here; do not edit the overlapping pain-points file
or turn a documentation draft into an unapproved rendering fix.

## AG. Compositing and Effects — draft checkpoint

The alpha-mask limitation in this initial checkpoint is superseded by AH below,
following Zack's direction to incorporate PAX-993 as landed.

Replaced the nine-line outline with a complete chapter, preserving the existing
slug and title compatibility anchor and adding `native-compositing` as the
canonical destination. No navigation restructuring or other chapter rewrite
was needed for this iteration.

### Delivered and verified against source

- Group/Frame/Mask decision, rounded Frame clipping, and separation of clipping
  from sizing, paint, and mounted state.
- Mask's exactly-two-child contract, content/source order, a filled ellipse,
  a closed cut-corner Path, transformed coverage, and subtree geometry.
- Mask boundaries: no sampled bitmap alpha/luminance, feather, or invert API;
  empty source coverage currently means no clip. Complex contour combinations
  are not sold as a portable boolean-geometry operation.
- Inherited per-descendant opacity, nested multiplication, overlap buildup,
  and the absence of a general Group/Frame/Mask offscreen isolation operation.
- Native punch-through, coverage approximation limits for gradients/images/
  path reveals, scroller islands, and a native EventBlocker dimming surface.
  Pointer blocking is distinguished from modal focus management.
- Apple LiquidGlass scope, Group surfaces, nested opt-out, current native glass
  OS gates and earlier-OS visual-effect fallbacks, and ordinary web fallback.
  UIKit's interactive treatment is not promised for macOS.
- General blur, blend modes, and portable backdrop materials stay outside the
  current public toolkit; this reflects source exports and the final PAX-915
  pause comment/PAX-963 backlog scope, rather than the old experimental branch.
- One interactive Mask brief and one native-layer diagram brief, each with
  three visual treatments. These remain production placeholders.

### Bounded validation

- Extracted all seven Pax fences into a disposable verification app. The Path
  fragment is tested as the second child of the article's Mask, as instructed.
  Web debug and baked release builds both pass; fixture parity is checked.
- Browser screenshots in both modes confirm the rounded Frame's crop, ellipse
  and cut-corner masks, hidden mask-source paint, 50%/25% opacity and overlap,
  vector coverage in front of a native Button, the undimmed foreground panel
  above a dimmed background, and ordinary readable LiquidGlass web fallback.
- A separately instrumented copy of the underlay snippet adds a click counter
  and a conditional blocker. Blocked clicks leave the count at zero; removing
  the blocker lets the underlying button increment it. This instrumentation
  does not change the article's scope into a complete modal tutorial.
- Canonical `occlusion` builds in web debug and release. The release browser
  run shows moving masks clipping native text and controls alongside vectors;
  its Deal button increments from 0 to 1. `neon-opacity` and `liquid-glass` were
  inspected as additional source references, not exhaustively device-tested.
- Both fill-coverage alpha tests pass. The existing retained-clipping pixel
  regression passes with local Metal access, covering image/vector clip order,
  nested clips, retained replay, and the screenshot mirror path. The initial
  sandbox attempt could not find an adapter; the permitted hardware run passed.
- Book build and whitespace checks pass. All 21 internal links and 16 chapter/
  compatibility anchors resolve. Previous Motion, Native, Drawing/PAXEL, and
  Events checks remain green, including the preserved native-controls alias
  and Space Game source-tab parity. The Space Game embed is restored.

No Apple application/device run or forced Piet/browser matrix is claimed.
Native-glass visuals, detailed mask hit geometry, complex compound contours,
and all possible clip/scroll nesting combinations remain outside this bounded
draft verification. Source-established limits are stated in the chapter.

Verification source/builds: `/tmp/pax-975-compositing-check.UuHNnR`.
Local review: `http://localhost:8796/compositing-effects.html`.
The overlapping pain-points file is untouched; findings stay here. No library
edits, commits, publication, or Linear status changes were made. Scrolling and
Viewports is next after Zack's review of this draft.

## AH. Compositing correction — PAX-993 alpha masks

Zack requested present-tense documentation of PAX-993's `Mask alpha=true|false`
contract as landed. The article now distinguishes default geometric coverage
from painted-alpha masking and includes a gradient-alpha reveal example.

Evidence was read directly from `/Users/zack/.codex/worktrees/2383/pax`:
`pax-std/src/core/mask.rs`, its current tests, renderer alpha-mask paths, and
the branch's Mask reference. Commit `d006b2697` introduced alpha support;
the worktree also contains subsequent mask invalidation fixes. The latest
PAX-993 task discussion considers automatic alpha selection and removing
mask-specific feathering, but explicitly leaves the flag unchanged. The draft
therefore teaches the requested flag and source-painted softness, without
advertising a settled feather API or the proposed automatic selection.

- `alpha=false` remains the default geometry mode, including mixed native
  and rendered content. Existing examples retain that meaning.
- `alpha=true` consumes Rectangle/Ellipse/Path fill and stroke alpha, source
  opacity/transforms, and linear/radial gradient alpha with up to eight stops.
  RGB is irrelevant. Overlapping source paints use source-over alpha;
  nested alpha masks multiply, and empty alpha sources hide content.
- Alpha mode is currently GPU canvas-only: native content and Piet are not
  supported. Image/text/native alpha sources and source-side clips remain
  unsupported. Alpha masks do not establish isolated group opacity or change
  hit testing. Bitmap-alpha masking is not inferred from vector alpha support.
- Removed the article's blanket denial of soft masking and qualified its
  geometry-only explanations. Updated the media brief to include separate
  geometry and GPU alpha cases, keeping native controls out of the latter.

Integration boundary: the docs worktree at `bf0fe7a739` still has the older
`Mask {}` implementation. No merge, cherry-pick, source-code copy, or generated
API overwrite was performed. The prose uses the requested landed framing;
the PAX-993 library code and generated API reference still need to arrive
here through Zack's integration workflow. Validation must identify which
source tree it uses rather than claiming the local older library has the API.

Verification: the exact new gradient snippet was extracted into
`/tmp/pax-975-compositing-check.UuHNnR/alpha`, with an explicit dependency on
PAX-993's library source. Its CLI built the app successfully in web debug and
release, with all build output kept in this task's temporary/project output
locations. The release browser check shows the expected opaque middle and
transparent-to-opaque fades at both ends. No PAX-993 source was modified.
The rebuilt article passes all 21 links and 17 anchors, including the new
`painted-alpha-masks` anchor, and all eight snippets match their verification
sources. Existing seven-snippet checks remain attributed to this docs tree;
the new alpha snippet is explicitly attributed to PAX-993. Space Game's local
embed was restored after the book build. No published docs were changed.

## AI. Scrolling and Viewports — working outline

Zack approved Compositing and requested the next chapter. As authorized for
the continuing sequence, this internal outline proceeds directly to a draft.

Reader outcome: build a bounded scroll region, choose explicit or measured
content extents, observe and set its position, and use that position for paging
or motion without taking over the platform's scrolling gesture.

1. Viewport/content/offset model and a complete short vertical example.
   Clarify percentage reference frames and explicit child content sizing.
2. Autosized documents: vertical default, axis overrides, measurable children,
   and the distinction between the resolved pane and the public size input.
3. Two-way `scroll_pos_y` binding with Back to top / Last note controls.
   Separate authoritative position from `@scroll` deltas and native input.
4. Horizontal/nested regions; scroll snapping and Carousel as the higher-level
   page container. Dots indicate position, rather than acting as buttons.
5. Scroll-driven motion: bounded progress from content minus viewport extent;
   reuse the small state example and link timeline ownership to Motion.
6. Native children, clipping, fixed siblings, platform gesture ownership,
   and why rendering culling/tiling does not virtualize a repeated collection.
7. Canonical examples, purposeful media placeholders, and focused read-more
   links. Preserve old title/autosize anchors; drop `scroll-island-mini`.

Evidence read: Scroller public/host source, autosize measurement and tests,
Carousel layout/template, web size/position/snap/page-delegation code, Apple
position clamping and snapping, shared/native scroll event producers,
repeat expansion, and existing Event/Layout/Motion/Compositing chapters.
Canonical examples inspected include `scroll-matrix`, `rounded-scroller-tiles`,
and `scroll-garden` (which now uses Carousel). Recent Scroller history provides
discovery leads, not a performance contract.

PAX-948 remains a separate solution-oriented scrolling/graphics write-up;
this chapter will not absorb its implementation-history scope. PAX-839
(lazy initialization) and PAX-954 (Piet fast-scroll presentation) remain
Backlog; full descriptions and empty comment histories were checked. Avoid
virtualization promises or universal smoothness claims. No status changes.

Bounded validation: extract snippets into a temporary app, build web debug
and release, inspect viewport/content dimensions, exercise native scrolling
and position buttons, verify snapping/Carousel and scroll-driven progress.
Run focused existing measurement/scroll tests and build one canonical
example. Native Apple gestures and extreme fallback scrolling require
separate device checks; do not imply they passed from a web run.

## AJ. Scrolling and Viewports — draft checkpoint

The former short outline is now a full draft. The existing filename and
`autosized-scrollers` anchor remain; `scrolling--viewports` preserves the old
title anchor. No navigation reorganization was made.

Delivered: viewport/content/offset model, explicit and measured content,
two-way position controls, position versus delta events, horizontal snapping,
Carousel's intended API, a scroll-sampled timeline, nested/native viewport
considerations, and bounded collection/performance guidance. Two media briefs
remain explicit placeholders with three possible treatments each. Examples
are verification fixtures, not new canonical apps or gallery changes.

### Verification completed

- Six exact Pax fences and one Rust fence were extracted into
  `/tmp/pax-975-scroll-check.WPP1Yn`. The combined harness adds only sample
  selection and a background; the progress fragment is combined with the
  state example as instructed in the chapter. Web debug and release builds
  pass against this worktree's library, including the baked release path.
- Browser checks in both modes confirmed a 240px viewport / 720px pane,
  a 480px end offset, and the autosized 448px pane. Native scrolling updates
  the bound offset; Last note requests 480 and Back to top requests zero.
  The timeline fills at 480 and is half filled at 240 in both modes.
- The horizontal fixture needed an explicit `anchor_x=0%`: percentage
  positioning otherwise implicitly anchored full-width panels back over one
  another. Corrected the article and rebuilt both modes. Release inspection
  confirms Panel 2 at offset 1232 in a 1232px viewport / 3696px pane with
  `x mandatory` snapping. This is an authoring correction, not a library fix.
- `rounded-scroller-tiles` builds in web debug and release. Its release view
  shows six independently clipped gradient panes; scrolling the first and
  fifth to 720 leaves the other four at zero. Existing source places the
  gradients before its native radius labels, obscuring those labels by normal
  Pax z-order. No canonical example source was changed in this docs turn.
- Focused tests passed: Scroller autosize defaults/overrides (2), placed
  content measurement (1), local-point scroll conversion (1), and layer tiling
  (8). A Carousel-specific test filter found no tests; it is not counted as
  coverage.
- Book build, 17 chapter links, 12 anchors, exact fixture parity, referenced
  example-directory checks, and whitespace checks pass. Existing Compositing,
  Motion, Native Controls, Text, and Events link/snippet checks also pass.
  Space Game's local embed/source tabs were restored after rebuilding.

### Carousel rendering follow-up

**Unresolved; publication gate for the Carousel example.** The exact three-
Rectangle Carousel renders its position dots but none of the supplied page
content in both debug and release. A separate standalone debug app reproduces
the omission without the sample selector, conditional, or root background.
Changing the standalone pages to Group + Text + Rectangle also reproduces it.
The native pane has the expected extent and snapping changes the active dot;
there is only the root canvas and no rendered page/native-text content.

This establishes a rendering/projection symptom, not a precise root cause or
proof of when it regressed. Do not claim it was caused by recent Scroller or
slot changes without further investigation. The chapter marks this as an
explicit draft verification note and directs readers to the verified ordinary
Scroller snapping pattern. Carousel is not counted as a passed visual check;
`scroll-garden` is source-inspected, not runtime-validated by this checkpoint.
Zack can authorize a focused library detour separately. No implementation
shortcut, library change, or new issue was introduced here.

The first build attempts hit mixed-worktree cached `pax-chassis-web` artifacts
from the preceding PAX-993 alpha validation. Rebuilt this worktree's CLI and
cleaned only that package's generated wasm debug/release cache, then rebuilt
successfully from this tree. Future cross-worktree verification should use
separate target directories. No source was removed or merged.

Apple native gestures, narrow-device layouts, nested gesture arbitration, and
extreme Piet fallback scrolling were not device-tested in this checkpoint.
Those remain bounded validation work, not inferred passes. Keep PAX-839 and
PAX-954's outstanding work separate from the article's shipped-behavior claims.
The overlapping pain-points file remains untouched; findings are recorded here.

Local review: `http://localhost:8796/scrolling-viewports.html`.
No commits, publication, or Linear status changes. The draft is ready for
editorial review, with the Carousel gate made visible. How Pax Runs is the
next proposed chapter after review or any authorized detour.

## AK. How Pax Runs — working outline

Scrolling and Viewports is approved. Following Zack's standing direction,
outline and research proceed directly into one chapter draft before review.
The Carousel publication gate remains open; this turn does not expand into
a library detour.

Create `how-pax-runs.md` as the builder-facing runtime chapter approved in the
topology. Its outcome: readers can connect source, running instances, property
updates, platform surfaces, and practical performance measurements. Prerequisites
are the authoring fundamentals and Layout; link to Properties, Events, Motion,
Scrolling, Compositing, and Native Controls for their owned concepts.

Proposed sequence:

1. Compiler, runtime, and chassis; current Rust application logic and four
   runtime targets. Explain cartridge as the program-specific build output,
   without promising a portable plugin file or future language runtime.
2. Source to running instances: templates expand into a scene tree, properties
   connect values, and assets/native services cross platform boundaries.
3. A property change reaches the screen: invalidation, computed reads/effects,
   layout, dirty rendering, and native updates. Separate work skipped from
   work that still runs; no zero-CPU or demand-driven scheduler promise.
4. Shared scene, distinct surfaces/backends: WebGPU, current Piet browser
   policy, Apple GPU chassis, and native controls. Link capability restrictions.
5. Debug versus release: generated program representation and compiled Rust,
   hot-reload boundary, release measurement, and assets outside Wasm sizes.
6. Use the existing Increment example to inspect instances and understand
   event-driven rotation versus its intentionally continuous color updates.
   Finish with a short measurement checklist and deliberate read-more links.

Move the old architecture entry out of the primary learning sequence into a
clearly labeled maintainer appendix, retaining its URL/anchors. Add a historical
design-note notice and replace its unversioned size snapshot with a link to
current measurement guidance. Do not rewrite that whole appendix or perform
the remaining global topology pass. Link the new chapter from What is Pax.

Evidence: compiler preparation and cartridge generation/template; current
`rust_manifest` constructors and transitional ProgramIR; property graph and
tests; expanded-node updates, engine tick/render, web frame loop; browser
surface policy and renderer selection; Apple graphics bridge; canonical
Increment source; CLI build/run/dev help. PAX-851 description and all comments
were read as discovery leads and checked against current code. Its Done status
does not establish literal zero compute at rest. Demand-driven scheduling and
future logic-module documents remain design material.

Important implementation boundary: this checkout's normal release cartridge
generation uses Rust manifest constructors. Do not describe the transitional
ProgramIR binary format as the universal shipped application container. Public
prose can accurately say that release builds bake the program into the artifact.

Bounded verification: focused graph/runtime/browser-policy tests; web debug and
release builds of unchanged canonical Increment; live inspection, logs, and
click-driven rotation; mdBook, new chapter links, preserved appendix anchors,
and affected inbound links. No benchmark or Apple-device certification is
implied. One optional graph/surface visualization brief can remain a clearly
labeled placeholder; this is not a new canonical example or gallery task.

## AL. How Pax Runs — draft checkpoint

Created the builder-facing `how-pax-runs.md` draft. It owns compiler/runtime/
chassis responsibilities, cartridge terminology, instances versus dependencies,
property-to-screen updates, ongoing frame work, native surfaces and rendering
backends, culling versus collection lifetime, and measurement boundaries.
One clearly labeled visualization brief follows the unchanged canonical
Increment example. There is no new starter or delegated gallery work.

The old architecture page is now a labeled maintainer appendix after the API
reference. Its URL and 14 heading anchors remain. A prominent historical-status
notice distinguishes proposed packaging/languages from current behavior;
unversioned footprint numbers were removed in favor of measurement guidance.
The remaining design prose was not rewritten as a second chapter.

The link check found that the approved What is Pax page was still absent from
SUMMARY and therefore not rendered by mdBook. Added its agreed entry before
Getting Started so the new chapter's prerequisite and reciprocal link resolve.
Getting Started remains independently reachable by its existing URL; no
prerequisite gate or redirect was introduced. Other topology work is unchanged.

### Verification completed

- 36 focused tests pass: reactive properties (23), engine mount/lifecycle/
  globals (8), browser rendering policy (3), and release/designtime feature
  rejection (2). The graph tests cover cached reads, dirty effects, redundant
  writes, and propagation cutoffs.
- Unchanged `examples/src/increment` builds for web in debug and release.
  Browser checks in both modes show count 0 becoming 1 and the rectangle
  rotating after a click. The debug inspector confirms a settled quarter-turn
  and distinguishes native Text from canvas Rectangle; continuing color
  changes match the canonical `@pre_render` handler.
- The exact documented run/inspect/log commands were exercised. Inspect at
  depth 3 reports four nodes. Default warning-level logging legitimately
  returned no entries. After reloading with `?pax_log=info`, the same log command
  reported WebGPU selection, adapter, format, and surface limit. The article
  now includes this setting instead of assuming info-level startup logs are
  present by default. No renderer switch or browser-setting change was made.
- mdBook and whitespace checks pass. The new chapter plus changed cross-links
  pass 28 local-link checks, 12 new anchors, and 14 preserved appendix anchors.
  Scrolling's 17-link/12-anchor/snippet check and Events' 19-link/19-anchor/
  Space Game source-tab check also pass. Local Space Game output was restored
  after each book rebuild.
- The stale local Python preview was listening but returning empty responses;
  it was restarted on the same loopback port with a log file instead of its
  orphaned output pipe. The review URL remains unchanged apart from the new
  article slug. This is local preview maintenance, not publication.

No performance benchmark, profiling-mode build, Apple-device run, or live Piet
comparison was performed. `--profiling` behavior was verified from current
help/implementation; debug/release builds were the executed build paths. Backend
policy tests establish selection rules, not universal initialization recovery
or fallback visual parity. The Carousel gate from AJ remains outstanding.

Verification scripts are under `/tmp/pax-975-runtime-check.XhqEY1`; the local
release example is at `http://localhost:8796/_runtime_increment_release/`.
Review: `http://localhost:8796/how-pax-runs.html`.
No library/example-source edits, commits, publication, or Linear status changes.
Stop here for Zack's chapter review before starting another article.

## AM. Developer Workflow and Tools — working outline

Zack authorized this chapter after the remaining-spine check. Research and
outline proceed directly to a draft under the standing sequential process.
Create `developer-workflow.md`; do not start Targets/Build/Deployment in parallel.

Reader outcome: operate one understandable edit/run/inspect/capture loop, select
the intended session, and distinguish source edits, runtime observations, and
real user interactions. Getting Started is the only prerequisite. Own detailed
reload/formatting/dev/docs commands; link to How Pax Runs for execution and
performance, Events for application handlers, and individual feature chapters
for authoring semantics. Keep Rust debugger/LSP configuration and protocol
internals outside the bounded launch chapter.

Outline:

1. Keep one debug session running, save a focused change, interact, inspect,
   capture. Explain the local dev service and its OSS status.
2. Pax versus logic reload: defaults, four modes, target matrix, precedence,
   restart boundaries, source persistence, and failed/superseded builds.
3. Format Pax files and inline templates; check mode and ordinary Rust formatting.
4. Discover/select sessions; project fallback and explicit ID safety; stale
   sessions, mounted browser/app requirement, and timeouts.
5. Inspect the expanded tree, selectors, source IDs, and point hit stacks.
6. Capture one frame or a timed sequence; paths, sampling, scale/coordinates,
   backend limitations, and distinction from profiling.
7. Read web logs and enable info-level renderer diagnostics.
8. Drive actual UI interactions, then observe; name the current CLI injection
   gap and explain that `dev touch` mutates source. Brief, clearly mutating
   source-replacement guidance, without encouraging raw protocol writes.
9. Read the CLI's bundled docs/example source offline; distinguish the snapshot
   from the live website and from a full runnable repository example.
10. Short troubleshooting table and read-more links.

Move only the established formatting/hot-reload explanations from Getting
Started, leaving the same heading anchors and concise forward links. Leave its
web public files/project packaging material for the next chapter and keep the
known-overlap Routing/pain-points files untouched. Update existing formatting/
reload cross-links to the new canonical owner where practical.

Evidence: full CLI command definitions and handlers for dev/docs, session
resolution/registration, web and macOS request handlers, hot-reload policy,
source watcher and feature gating/tests, formatting implementation, embedded
docs build/runtime, and recent relevant history. PAX-975 issue/status/relations
and all comments were refreshed read-only; it remains In Progress.

Confirmed gap: `dev touch` has only apply-component-source and replace-node.
`DevClientRequest` and host request dispatch likewise expose no input-injection
request. Do not invent `dev click`, imply ray-cast sends an event, or reinterpret
`NodeContext::dispatch_event` as a platform gesture driver. The accepted OSS
boundary is unchanged; packaging an ergonomic CLI gesture interface remains a
launch tooling review point. No new feature or Linear issue is authorized here.

Bounded checks: temporary copy of canonical Increment for safe source edits;
web Pax/all/off modes with expected screen changes; inspector/selector/ray-cast;
single and sampled captures; real browser click; info logs; formatting
check/write/check; bundled docs/search/example lookup; focused reload/formatting
tests; web release build; mdBook/link/compatibility-anchor checks. Apple support
is checked against source and existing tests, not asserted as device-tested by
the web fixture. Commands that regenerate APIs or all examples are not run over
the active editorial tree.

## AN. Developer Workflow and Tools — draft checkpoint

Created `developer-workflow.md` and added its navigation entry after How Pax
Runs. The draft covers the edit/run loop, two reload lanes and target support,
formatting, safe session selection, tree/selector/hit inspection, captures,
web logs, actual UI interactions, source mutations, bundled docs/examples,
and practical troubleshooting. One clearly labeled workflow-media brief
remains; no new canonical example or starter was introduced.

Getting Started now links to the detailed workflow owner. Its formatting and
hot-reloading headings/anchors remain with short summaries and forward links;
web public-file and packaging content was left in place for the next chapter.
Templates, Components, and How Pax Runs now link directly to the new canonical
sections. Routing and the overlapping pain-points file remain untouched.

### Verification completed

- Used an isolated source copy of canonical Increment at
  `/tmp/pax-975-workflow-check.T9Lrws`, with only fixture package/path adjustments
  and intentional test edits. Canonical `examples/src/increment` is unchanged.
- Default web run: changing the template label from clicks to saves appeared
  without a page reload. A subsequent Rust +1 to +2 edit produced the expected
  restart-required notice; a live click still advanced by one.
- Web `all`: restart compiled the pending +2 change; a click advanced by two.
  Changing Rust to +3 triggered a build/activation on the same app URL. The new
  revision reset local state, and its next click advanced by three. This is
  observed reload behavior, not a blanket state-preservation promise.
- Web `off`: root inspection and session queries remained available.
  `apply-component-source` successfully wrote the complete replacement source
  to the fixture, changing saves to drafts on disk; the mounted app continued
  to show saves. No source-write shortcut around the disabled lane occurred.
- Session status, type/ID selectors, default-scale ray-cast, and screenshots
  were exercised. A center hit returned native Text above canvas Rectangle.
  Explicit `--session` selected the intended fixture even with a different
  `--path`, confirming precedence. Tooling commands used the running fixture,
  not an unrelated user application.
- `look` produced a 1280x720 PNG; `--scale 0.5` produced 640x360 without
  resizing the app. The five-second / 500ms request produced two images with
  actual capture timestamps about 3.46 seconds apart. A one-second request
  produced only one. The article therefore describes best-effort observation
  and timestamps, not a guaranteed sample count or frame-rate measurement.
  Captured native text and drawn content were visually checked.
- Default logs were quiet as expected; `?pax_log=info` exposed renderer/adapter/
  format diagnostics. The logs include captured browser-console output as
  well as Rust messages. Continuous `--follow` was source/help-checked, not
  left running as an unattended monitor.
- Formatting was checked both on the app and on separate deliberately
  unformatted `.pax`/inline-Rust fixtures. `--check` exited 1 on two dirty files,
  formatting changed both, and the subsequent check passed.
- 16 focused tests pass: reload modes/precedence/metadata (7), mobile mode
  boundaries (1), native revision request/response roundtrips (2), Pax-only/
  logic-only/failed-build behavior (3), and formatting (3).
- Rebuilt the local CLI without running the API/example generators. Bundled
  docs list/search/open find this chapter; example lookup prints unchanged
  canonical Increment source. The final fixture also builds in web release;
  its served output shows 0 drafts becoming 3 drafts on a real browser click.
- mdBook and whitespace checks pass. Checked 81 local links across the five
  affected prose pages, 18 new chapter anchors, and two Getting Started
  compatibility anchors. Existing How Pax Runs/appendix, Events/Space Game,
  Scrolling, Native Controls, and Motion checks also pass. Space Game's local
  runnable embed and source tabs were restored after book builds.

### Remaining boundaries

The CLI input-injection gap identified in AM remains explicit in the chapter.
Browser UI clicks validate the interaction loop; they are not evidence of a
Pax CLI gesture command. A separate tooling decision/implementation is needed
if launch requires a first-class CLI click/tap/key interface. No issue, worktree,
or library feature was created as a side effect of this article.

macOS capture/inspection and logic reload were source-inspected, not live-tested
here. iOS/iPadOS reload behavior was source/test-checked without a device run.
No live `logic`-only session, JPEG capture, replacement-node mutation, mobile
gesture test, or native accessibility audit was performed. These are explicit
limits, not implied passes. The earlier Carousel gate remains outstanding.

Temporary development servers were stopped after checks. The final static
fixture is available under the local review server at
`http://localhost:8796/_workflow_increment_release/`.
Chapter review: `http://localhost:8796/developer-workflow.html`.
No canonical example/library edits, commits, publication, or Linear changes.
Stop for Zack's review; Targets, Build, and Deployment is next when authorized.

## AO. Targets, Build, and Deployment — working outline

Zack approved the next chapter after reviewing/editing Developer Workflow.
Preserve that edited chapter. Outline and draft proceed in sequence without
another outline approval under the standing editorial instruction.

Create `targets-build-deploy.md`. Reader outcome: choose a runtime target and
build workstation, produce the right artifact, host a web release, and
understand the current Apple development/distribution boundary. Prerequisite:
Getting Started. Own target/host matrix, build modes/output, public files,
packaging metadata, web serving and Apple delivery constraints. Link to Routing
for route selection, Text/Images for asset authoring, How Pax Runs for backend/
release internals, and Developer Workflow for development sessions.

Outline: (1) targets/workstations; (2) run/build/release and generated outputs;
(3) release web build and local static preview; (4) assets versus public files;
(5) host root, deep links/base URL, MIME/HTTPS/caching and a static-host recipe;
(6) macOS and mobile preparation/commands/destinations; (7) project metadata,
icons, identity and string-only Info.plist values; (8) Apple release boundary;
(9) bounded preflight checklist and read-more links. Keep Getting Started's
external-CTA first-run path intact and preserve public-files/project-metadata
anchors with short links to their new canonical owner.

Source findings to represent honestly:

- `run` is debug-only; `--release` is a build flag. Web profiling has its own
  output directory. Publish the complete web output, not the source project.
- CLI/runtime targets are web/macOS/iOS/iPadOS; Windows/Linux are development
  workstations for web, not native Pax runtime targets. Source-linked clean-host
  checks are distinct from the release-candidate public-package gate.
- iOS/iPadOS release commands reach an explicit `unimplemented!` before Xcode
  packaging. The embedded diagnostic has stale `.pax/pkg` paths. Do not publish
  those paths or a pretend working IPA/export recipe. Development-team metadata
  exists now; the remaining release packaging gap is not absence of that key.
- Default web HTML resolves bootstrap URLs from `document.baseURI`. Host fallback
  alone is insufficient for nested deep links; a deliberate base URL and route
  topology must agree. There is no CLI base-path flag or current route-HTML
  metadata generator in this checkout. Leave Routing source untouched.
- Browser cache guidance must account for fixed JS/Wasm names; reserve immutable
  caching for genuinely versioned URLs. Pax-docs publication stays with PAX-987.

Verification plan: isolated Increment source copy with public files and metadata;
debug/release web builds, actual static browser load/click, nested path/base URL
checks, public byte equality, focused metadata/public/Apple-selection tests, CLI
help, and mdBook/link/compatibility checks. Native/device/real hosting results
must be stated separately from source inspection; no signing, uploading,
provisioning, or publication is authorized by this prose task.

## AP. Targets, Build, and Deployment — draft checkpoint

Created `targets-build-deploy.md` and added it after Developer Workflow in
SUMMARY. It owns target/workstation distinctions, build modes/artifacts, web
release preview/hosting, public files, base URL and fallback, cache/type policy,
Apple toolchain/destinations, metadata/icons, and distribution boundaries.
The tables and output tree provide the essential visual explanation; no new
canonical example or media production is needed to understand this draft.

Getting Started's first-run instructions remain intact. Its public-files and
project-metadata sections now provide concise forward links with their original
anchors preserved. Developer Workflow was read but not modified; its SHA-256
remains `be8639d75a1545124c0ec927a29452e09204f6347695ed76f459006a5dd6d0bc`.
Routing, pain-points, canonical examples, and library/API source were untouched.

### Completed checks

- Read current CLI build/run/eject flags, web/Apple builder paths, project
  metadata parsing/materialization, public-file copying, bootstrap URL handling,
  route pathname serialization, Apple project templates, workstation harness,
  and recent related history. Refreshed PAX-975/status/relations/comments without
  mutation. External web-hosting/Apple guidance is linked to primary vendor or
  MDN documentation in the article.
- Isolated Increment copy: `/tmp/pax-975-deploy-check.ntvql5`. Canonical source
  unchanged. Fixture adds public files, a distinct package name, and the local
  Pax dependency path. Web debug and release builds pass with the current source.
- Six byte checks confirm three public files (robots, nested index, well-known
  metadata) survive unchanged in debug and release outputs. Local static
  responses include 200 for the public directory index/well-known file, proper
  JS/Wasm types, and 404 for a missing snippet. This is local HTTP verification,
  not a deployed TLS/CDN test.
- Actual release browser interaction: 0 clicks → 1 clicks at the root URL.
  A local harness serving entry HTML at `/check/nested/` reproduces the blank
  startup caused by relative bootstrap URLs. Ran `eject --target web` on the
  fixture, added `<base href="/">` to its source interface, rebuilt release,
  and reloaded that same nested URL: 0 clicks → 1 clicks now works. This checks
  bootstrap/base URL behavior, not an entire routed app or arbitrary subpath
  deployment. The harness only provides fallback for that named test route.
- 37 focused compiler tests pass: public files (7), project metadata/icons (8),
  Apple destination/architecture/identity/launch configuration (20), and release
  development-feature rejection (2).
- macOS debug build passes on this Apple-silicon/Xcode 26.4 workstation with
  the default project target directory. Verified the expected `.app` path;
  its display name is Increment and marketing version is 0.38.3. Xcode reports
  the expected unsigned-debug/entitlement warning. No native app was launched,
  no signing identities changed, and no provisioning/registration was requested.
- Rebuilt CLI bundles the new chapter; `docs search 'Targets, Build'` finds it.
  mdBook and whitespace checks pass. The local checker validates 98 links
  across the new chapter, Getting Started, and preserved Developer Workflow,
  21 chapter headings, and four compatibility anchors. Rendered chapter reviewed
  in the in-app browser; the local Space Game embed/source tabs were restored
  after mdBook rebuilt the preview.

### Findings and remaining validation

1. **Mobile release packaging remains unimplemented.** `building/apple.rs`
   stops before Xcode packaging for release iOS/iPadOS. The diagnostic's old
   `.pax/pkg` paths and claim that team configuration is absent are stale;
   the chapter uses current `.pax/interface` paths and the actual metadata
   contract. No promise of a working manual distribution recipe is made.
2. **Apple packaging ignores custom Cargo output location.** A live macOS
   check with `CARGO_TARGET_DIR` compiled its Rust library successfully, then
   failed at `install_name_tool` because the builder hardcodes the project's
   `target/<triple>/<mode>` dylib path. Retrying with the default target directory
   completes Rust and Xcode packaging. The draft states this limitation; no
   library workaround/fix was introduced. This is a follow-up tooling candidate.
3. **Nested web loading needs an explicit base plus host fallback.** The chapter
   provides the working custom-interface recipe and its maintenance cost.
   There is no current CLI base-path flag or route-HTML generator in this tree.
   Coordinate the eventual Routing pass with PAX-869 instead of anticipating
   its unmerged metadata work.

The first native attempt was blocked by sandboxed macOS networking during font
vendoring; retrying outside the sandbox resolved that environment issue before
the separate target-directory problem was established. No public-package
installation, clean Linux/Windows workstation, live mobile simulator/device,
macOS release, mobile release execution, signing, notarization, archive export,
or upload was performed. The NGINX configuration is source-checked against
vendor docs, not executed here (NGINX is not installed); local Python serving
verifies the web artifacts and the specific fallback/base behavior separately.
Subpath routing, TLS/CDN cache behavior, and complete release distribution remain
launch verification gates. Carousel and the earlier CLI gesture gap remain open.

Review: `http://127.0.0.1:8796/targets-build-deploy.html`.
No commits, merges, publication, or Linear mutations. Stop for Zack's review;
do not start Routing or the first-interface tutorial in this turn.

## AQ. Routing — working outline and anticipated integration

Zack approved Targets and authorized drafting Routing in anticipation of
PAX-869 landing before launch. This supersedes AP's decision to wait for the
route-metadata prerequisite. Read PAX-869 and all comments, and inspect its
actual implementation in the clean `zb/website` worktree (1b54, `64f6984b6`),
without merging it or editing that worktree. Keep integration-dependent
verification separate from checks against the current docs worktree.

Reader outcome: organize a multi-screen app around paths, navigate from Pax or
Rust, reason about nested scope and state lifetime, and give web routes
appropriate entry documents and metadata. Prerequisites: templates, events,
properties, and component composition. Own route matching/scope/navigation and
route metadata here; link to stores, lifecycle motion, and deployment policy.

Working article order:

1. Routes and history: location selects a branch; navigation updates location.
2. Basic shape and declaration-order matching, exact paths, params, terminal
   catch-all, and an explicit fallback.
3. Link and Rust navigation; root-relative URLs and web/native differences.
4. The reactive `route` binding, query/fragment data, and nested scope with a
   concrete URL walkthrough. Pass outer captures into child components before
   an inner router introduces its own binding.
5. State lifetime, retained shells, RouteCard/RouteModal, direct-load behavior,
   and application-owned shared state.
6. PAX-869 web metadata: required literals, inheritance, concrete/symbolic
   routes, indexing limits, site settings, generated artifacts, and fallback.
7. Canonical Router Playground with the actual route outlet and navigation
   source visible; focused things to try; deliberate read-more links.

Preserve the six existing chapter anchors and add `routes-and-history` and
`web-route-metadata` as canonical launch destinations. No new example needed.
Minimally reconcile Targets' base-URL recipe with the incoming generator;
leave other reviewed prose and user edits intact.

Verification: current runtime router tests; canonical web debug/release builds
and browser interaction; source inspection of the PAX-869 parser, compiler,
metadata updater and tests; mdBook and affected links/anchors. Record native
and merge-dependent checks as unverified where not executed. No publication,
library changes, commits, merges, or Linear mutation are part of this draft.

## AR. Routing — draft checkpoint

Rewrote the canonical Routing chapter after AQ's internal outline. It now
teaches location-driven selection, a complete two-page starter, declaration
order and fallback, Link/Rust navigation, native virtual locations, reactive
route fields, nested scope, state lifetime, and retained card/modal routes.
The web metadata section incorporates PAX-869's implementation as anticipated
landed behavior, per Zack's explicit direction. No merge was performed.

Targets' public-files/base-URL sections now agree with that incoming behavior:
generated metadata/entry documents own page heads; the generator sets the
HTML base; ordinary routing no longer asks builders to eject the interface.
The hosting fallback and subdirectory cautions remain. SUMMARY, Getting
Started, Developer Workflow, pain-points, API reference, canonical examples,
and library source were not edited in this turn. Developer Workflow retains
SHA-256 `be8639d75a1545124c0ec927a29452e09204f6347695ed76f459006a5dd6d0bc`.

### Verification completed

- Read current runtime selection/scope/reuse code, Router/Route/Card/Modal/Link
  source, NodeContext navigation, web History API serialization, Swift virtual
  location coordinator, and canonical router-playground components.
- 14 runtime routing tests pass, including parameter remounting, catch-all
  reuse, modal retention/dismissal, exit lifecycle, and hit-test exclusion.
  Five manifest route parsing/serialization tests pass with the required
  `compiler` feature. The initial featureless test invocation ran zero tests;
  only the corrected invocation counts as validation.
- Canonical Router Playground builds for web in debug and release. The first
  attempt hit sandboxed dependency DNS; the network-enabled retry passed.
  No example source changes were needed.
- Live release browser: root → team member → nested settings → Back → Forward
  → top-level fallback. The UI and URL agree on captured team ID, query values,
  fragment, local/global scope, and remainder. This used ordinary static
  serving at the origin root and in-app navigation; it did not prove hard-load
  fallback or the incoming metadata generator.
- The article's complete opening Rust/template pair builds in web debug and
  release as an isolated fixture at `/tmp/pax-975-routing-check.AWHkZh`.
  Later component sketches use illustrative application-owned screen types;
  their corresponding patterns are covered by canonical examples/source,
  not a claim that every excerpt is a standalone program.
- PAX-869 source inspection: literal metadata parsing/validation, static
  topology collection, inherited metadata, concrete entry generation, HTML
  base, release site-URL validation, ProgramIR stripping, and browser head
  updates. Read its existing release catalog and `/blog/index.html` output.
  Its worktree was read-only and remained clean at `64f6984b6`.
- Executed that branch's exact TypeScript metadata updater in a small DOM
  test double: six cases cover root, concrete blog, trailing slash plus query/
  fragment, catch-all tail, parameterized route, and unknown default. Titles,
  robots directives, and canonical links match the article. This is a focused
  source test, not a newly rebuilt website or full browser integration test.
- mdBook and whitespace checks pass. The checker validates 74 local links
  across Routing and Targets, all six old Routing anchors plus the two launch
  anchors, and byte-for-byte correspondence for seven canonical source tabs.
  The preview includes freshly built Router Playground release output and the
  restored Space Game embed. Reviewed rendered prose and live embedded UI.

### Integration and publication gates

1. Rebuild the metadata snippets against the combined branch after PAX-869
   lands. Test generated root/literal entries, query/history head updates,
   preserved-URL fallback, public-file collisions, and missing asset responses
   in debug and release. Current f8d8 library source does not yet implement the
   feature, so this draft is not permission to publish it ahead of the merge.
2. The incoming updater makes symbolic parameter routes and catch-all tails
   `noindex` with no canonical link. The fallback document initially exposes
   its own head on unknown URLs until JavaScript updates it. The prose states
   both limits; do not infer per-record static pages, SSR, sitemap generation,
   or server-owned 404 semantics from this feature.
3. A site URL subpath sets an HTML base but does not strip that prefix in either
   runtime routing or metadata matching. End-to-end subdirectory mounting is
   not established. The canonical launch deployment remains domain-root.
4. **Routed docs embeds need a coordinated initialization/base policy.** The
   current standalone-built playground first sees
   `/_pax_examples/router-playground/app/index.html` and selects its fallback.
   Clicking Menu → Landing works and preserves the outer docs URL; the chapter
   now gives that instruction. PAX-869's origin-root generated base also needs
   explicit testing under the docs embed directory before publication. Avoid
   an unconditional mount-time navigation or ad-hoc generated-bundle rewrite
   that would discard legitimate direct links. Coordinate the durable embedding
   contract with the docs/example build pipeline and PAX-987.

No live native routing, OS deep-link integration, new native build, full
post-merge metadata build, actual host rewrite, or public deployment was
performed in this turn. Existing Apple packaging, Carousel, and dev gesture
gates remain unchanged. PAX-975 stays In Progress; no Linear mutations.

Review: `http://127.0.0.1:8796/routing.html`.
Stop for Zack's article review. Next is the PAX-993-backed first-interface
tutorial or the agreed final launch-spine integration pass; do not start
another chapter in this turn.

## AS. Userland extension points — working outline

Zack requested custom route-branch coverage after Route branch kinds, plus
primitive authoring in Components and Composition with a cross-link. Existing
public prose has no primitive-authoring section; generated/internal runtime
reference is not a substitute for explaining this extension boundary.

Source findings: `#[route_branch(path = "...", default = "...", modal = true)]`
is discovered from ordinary userland Pax types by static analysis; the parser
retains their component shell and presentation settings. Matching remains the
existing router's job. A primitive instead uses `#[primitive("...")]` and an
`InstanceNode` implementation; generated factories call the supplied Rust
path, without adding an enum variant. New native widget protocols or compiler
syntax can still require engine/chassis work.

Plan: add a complete slotted `PanelRoute` with custom path/fallback names;
explain optional retained-background policy and unchanged matching semantics.
Add an advanced end-of-chapter Components section covering when a primitive
is justified, declaration/runtime-instance/expanded-node responsibilities,
a minimal non-drawing registration scaffold, rendering/native obligations,
and engine/std source landmarks. Keep route branches template-backed and link
to the primitive discussion only to explain the distinct lower-level option.

Verify both userland extensions together in a temporary application with web
debug/release builds, source/API and generated-factory inspection, live route
navigation, focused parser/runtime tests, and mdBook/link checks. No library,
canonical-example, API-output, gallery, publication, or git-operation changes.

## AT. Userland extension points — draft checkpoint

Added `routing.md#custom-route-branches` immediately after Route branch kinds,
with a complete template-backed `PanelRoute`, renamed path/fallback fields,
slotted content, and the optional type-level modal retention policy. It links
to the new advanced `components-composition.md#authoring-primitives` section.
That section explains the component/primitive/data-type distinction, a small
non-drawing primitive registration scaffold, per-expanded-node state, rendering
and lifecycle responsibilities, and native/compiler extension boundaries. It
links to the engine and standard-library implementations for further study.

Validation:

- The combined temporary application at
  `/tmp/pax-975-extensibility-check.zBmAYM` builds for web in debug and release.
  Its generated factories use `ComponentInstance` for `PanelRoute` and the
  application-defined `RuntimeMarkerInstance` for the primitive.
- Live debug and release navigation verifies the custom panel presentation,
  captured parameter, multiple slotted children, and fallback. Debug browser
  Back restores the preceding route.
- Five manifest route tests and fourteen runtime router tests pass.
- The focused checker validates 79 local links, both new anchors, three
  documented snippets against the built fixture, both generated factories,
  and the restored Router Playground and Space Game preview embeds.
- mdBook, the CLI rebuild/docs search, and `git diff --check` pass. Both new
  sections were inspected in the rendered review site. Source-link paths were
  verified against local files; remote GitHub retrieval returned cache misses.

The primitive fixture deliberately does not draw or create a native widget.
It validates registration and integration, not a custom renderer, native
chassis implementation, or independently published dependency crate. Custom
modal policy was checked against source and existing tests, not a new live
custom-modal fixture. Existing metadata merge, embed initialization, native,
and publication gates remain unchanged.

Only the two human-facing articles and this planning record changed in this
turn. No library, canonical example, API output, Linear, publication, or git
operations were performed. Stop for Zack's review of these additions.

## AU. Primitives — standalone chapter checkpoint

Zack requested a dedicated Primitives chapter immediately after Components
and Composition, with a clearer introduction to this advanced alternative
and an Authoring primitives section after the introduction. The chapter now
owns the primitive extension boundary, registration scaffold, runtime
responsibilities, and engine/std source examples at `primitives.md`.

The structure is: introduction; Components and primitives; Authoring
primitives (registration, behavior, native integration, source examples);
Read more. Components retains its template-backed focus and a short pointer
to the new chapter. Routing links directly to
`primitives.md#authoring-primitives`. Five former section anchors on the
Components page remain as compatibility anchors beside the chapter link.
This supersedes the primitive section placement in AS/AT; their source and
implementation validation evidence still applies.

Validation: mdBook and the CLI rebuild pass, and `pax-cli docs search
Primitives --limit 3` finds the new chapter first as chapter 9. The focused
checker passes 91 local links, the Components → Primitives → Routing order,
the five preserved anchors and their new destinations, three unchanged code
snippets against the previously built fixture, and both restored preview
embeds. The rendered introduction was inspected in the browser. Whitespace
checks pass. No implementation changed, so runtime tests and application
builds were not repeated; the earlier debug/release fixture results apply to
the unchanged scaffold. Existing integration and publication gates remain.

Review: `http://127.0.0.1:8796/primitives.html`.

## AV. Navigation and reference — working outline

Zack approved Primitives and authorized the next step. All currently drafted
human-facing chapters have now had an initial review; the next bounded piece
is reference orientation and navigation integration, not another feature
chapter or a new starter.

PAX-993 was rechecked: Linear remains In Progress, its only published handoff
still describes Ink & Light, while the current registry at `e4f2b2a30` defaults
to Living Quilt. The dedicated task is actively working on rendering and
motion; its worktree contains runtime/GPU changes. There is no updated stable
first-edit handoff. Its source was inspected read-only. Keep the accepted
tutorial outline queued, without inventing a replacement project or freezing
the changing first-run screen.

Plan:

1. Make the existing API landing page useful to builders: when to use it,
   task-oriented entry points, reading Rust signatures alongside template
   examples, and matching reference to the project's release.
2. Give internal generated APIs their own clearly labeled Maintainer Reference
   branch, with the historical cartridge appendix beneath it. Explain current
   source contracts versus design proposals, and link to Primitives and How
   Pax Runs for the human-facing introductions.
3. Keep generated landing-page prose in generator-owned source templates and
   regenerate only the affected indexes. Test regeneration and navigation so
   subsequent API generation preserves this structure; do not refresh unrelated
   type reference pages or overwrite prior source-comment fixes.
4. Remove the control-flow compatibility page from the reading sequence; link
   directly to Components from Templates/PAXEL and preserve old web URLs and
   fragments. Align the PAXEL → Properties sequence with the articles' next links.
5. Build and inspect the book; audit local links across the learning path and
   the new reference entries, verify compatibility redirects and CLI discovery,
   and consolidate the remaining launch gates. Preserve manual prose edits,
   especially Developer Workflow. Do not merge, publish, or change Linear state.

## AW. Navigation and reference — draft checkpoint

The API Reference landing page now pairs common builder tasks with exact
reference modules and their guide chapters, explains how to read a generated
declaration, and sets a release-matching/target-support boundary. Maintainer
Reference is a separate top-level branch, with the historical Runtime and
Cartridge Notes nested beneath it. Its introduction distinguishes current
source contracts from design proposals and directs primitive authors to the
human-facing chapter first.

The two introductions are maintained in `pax-docs/templates/` and included by
the API generator. The affected indexes and generated navigation were rendered
through that generator's functions using an isolated driver; unrelated type
reference pages were not regenerated or edited. The generator now owns the
whole maintainer child list, including the appendix, so a later generation
preserves the hierarchy. A rendered-book check caught an mdBook behavior in
the first draft: splitting the child list around API-END dropped the earlier
internal children. Keeping a single contiguous list fixes that; the final
check confirms every sidebar destination is emitted.

The reading sequence now follows Templates → PAXEL → Properties → Events →
Components → Primitives. Control Flow is no longer a redundant chapter stop;
Templates and PAXEL link directly to its canonical sections in Components.
The old Control Flow URL maps its previous section fragments to those sections
and retains query parameters. Old API landing-page heading anchors also remain.

### Validation

- Three API-generator tests pass: landing-page orientation/anchors and output
  tracking, public-only behavior, and idempotent regeneration with a separate
  maintainer branch and preserved non-generated content.
- mdBook and the local CLI rebuild pass. CLI searches find API Reference and
  Maintainer Reference as the first result for their respective names.
- The focused navigation checker validates 644 local article links across
  22 pages, all 121 sidebar destinations, seven redirect cases, ten preserved
  API/primitive anchors, reading order, and both restored preview embeds.
- Browser review confirms both introductions and the new sidebar. A live old
  `control-flow.html?from=bookmark#for-loops` bookmark resolves to
  `components-composition.html?from=bookmark#ranges-and-collections`.
- The local server was restarted at port 8796. The browser retained an older
  sidebar asset under `127.0.0.1`; the fresh `localhost` origin shows the current
  navigation. Review uses `http://localhost:8796/api/index.html`.
- Developer Workflow still has SHA-256
  `be8639d75a1545124c0ec927a29452e09204f6347695ed76f459006a5dd6d0bc`.
  No existing example or library implementation was changed. Whitespace checks
  pass. Remote URLs, every generated type-page cross-link, application builds,
  native targets, and published-package commands were not revalidated here.

### Remaining launch work

| Gate | Next bounded action | Ownership/boundary |
| --- | --- | --- |
| First successful edit | Obtain the stable PAX-993 command/tree/first-edit handoff, finish the accepted tutorial, and connect it after Getting Started. | PAX-993 owns the project; PAX-975 owns prose and the tested edit sequence. |
| Combined-branch accuracy | Re-run the route metadata/hosting and painted-alpha examples after PAX-869/PAX-993 land, including docs-embed initialization. Reconcile any incoming API changes. | No merge or future-API redesign is authorized by this docs pass. |
| Commands and targets | Finish the previously recorded clean published-CLI/workstation, Apple packaging/launch, and developer-tool event-driving gates; resolve or accurately bound Carousel and other documented limitations. | Use the per-chapter verification records and PAX-906 release evidence; do not infer cross-target success from web builds. |
| Media and examples | Triage the 31 remaining planned media/example slots across 14 chapters into essential launch proofs versus optional enhancements. Fill a bounded essential set from canonical source; remove deferred placeholders from launch-facing output. | Coordinate with PAX-976/example work; this is not a requirement to author 31 new examples or edit the website gallery. |
| Publication handoff | Rebuild the finished versioned book and selected embeds, check final links and version behavior, then hand the source to PAX-987. | PAX-987 owns upload/deployment; no publication occurred here. |

Stop for Zack's review of the reference/navigation pass. The tutorial remains
queued; the next unblocked docs pass can prioritize the essential media/proof
set while the starter and integration dependencies settle. PAX-975 remains
In Progress, not ready for its final In Review transition.

## AX. Existing-example integration — Transition Grid review checkpoint

Zack approved a reuse-first collateral pass: defer new teaching projects,
diagrams, and recordings; adjust existing examples where they closely fit a
chapter. The shortlist is Space Game, Router Playground, Transition Grid,
Auto-sized Containers, Path Drawing, and Occlusion. This checkpoint implements
Transition Grid only, plus shared embed controls. ExampleHost redesign and
the website gallery remain outside this iteration.

### Implemented

- Animation's `#try-it-transition-grid` section embeds the canonical example
  with six source files, instructions for comparing Flow/Ghost and Snap/Ease,
  and a keyed-reordering exercise. Components links to it instead of promising
  a second new keyed-list specimen. The prose distinguishes parent-owned tile
  counters from child-local animated color state.
- The canonical example retains its existing policies, lifecycle timelines,
  tile interaction, and palette. Widths now fit the viewport, button labels
  are shorter, tiles are more compact, and a Scroller keeps a growing list
  reachable. There is no copied example project in docs.
- Shared HTML embeds expose Run, Restart, and Open standalone. No iframe is
  created before Run. Restart replaces its browsing context at the initial
  URL while preserving the selected HTML source tab. Source stays available
  without running WASM. Existing Space Game and Router Playground embeds use
  these controls too. No ExampleHost code was changed.

### Verification and remaining boundary

- Five dependency-free embed-script tests pass via
  `node --test pax-docs/tests/example-embed.test.mjs`: deferred startup, source
  availability, version-relative standalone link, restart/context replacement,
  selected-source preservation, stale load event isolation, invalid height,
  and missing-build/metadata failure handling.
- All four existing Transition Grid Rust tests pass. Fresh debug and optimized
  release web builds pass with cached dependencies (`CARGO_NET_OFFLINE=true`).
- Browser checks cover the actual debug embed at desktop and 390px page width:
  Run, source tabs, Restart, standalone link, tile taps/recycling, reverse,
  insertion/removal, and scrolling to new content. The release build also
  renders and passes the tap/reverse/remove/insert sequence. The narrow hint
  exposed a literal backslash-n; the final source uses ordinary wrapping.
- mdBook passes; the navigation check reports 646 local links across 22
  articles, 121 sidebar destinations, seven redirect cases, and no errors.
  The two replaced placeholders leave 29 to curate or remove in subsequent
  passes. They are not commitments to produce new collateral.
- Review: `http://localhost:8796/animation-motion.html#try-it-transition-grid`.
  The normal generator currently packages debug web bundles. Static debug
  previews attempt unavailable developer WebSocket connections; release
  packaging is a remaining publication-pipeline gate, not a claim that this
  checkpoint has shipped release assets. Resize/list-growth checks also emit
  tile-window diagnostic warnings; no missing tiles were observed in these
  checks. Apple targets were not rebuilt or visually verified here.
- The latest PAX-993 comment now hands off Living Quilt as the default starter
  on its unmerged branch. That supersedes the older Ink & Light default;
  tutorial/source integration remains a separate checkpoint.

Stop for review of this integrated example before applying the pattern to the
other candidates. No publishing, commits, merges, or Linear status changes.

### Review adjustment: automatic startup

Zack requested automatic initialization instead of the Run gate. The shared
embed now mounts immediately with Restart and Open standalone available from
the beginning. Routine loading/opened status copy is removed; only a load
error reveals a status message. The Animation instructions and embed tests
follow this revised behavior. This supersedes the deferred-startup UX above;
source tabs and full-context restart are unchanged.

### Review adjustment: dark styling and a contained tile viewport

Zack requested balanced title/counter/hint spacing, paired policy controls
inside rounded groups, separation above the tile panel, and fixed controls
while tiles scroll. The canonical template now uses explicit header anchors,
16px spacing around the counter's layout box, 2px gaps within each tab pair,
and a 20px gap before the independent tile Scroller. The instruction box stays
visible after interactions. The palette is dark grayscale with cyan/violet
selection strokes and a changing bright stroke on each tile.

True gradient strokes are unavailable in the current `Stroke` API, which
accepts `Color`. Zack explicitly chose solid accent strokes for this pass;
no layered gradient-border approximation or engine change was introduced.

The fresh debug/release web builds pass. Desktop and 390px browser review
confirm the grouped controls and readable palette. Growing the list to 13
tiles and scrolling to the last tile leaves the header at the same position.
The docs embed/source metadata is regenerated from the canonical example.

### Review adjustment: early examples and reader-facing frame headings

Default placement for live examples is immediately after the chapter intro,
unless an example has a clear conceptual home further down. Space Game now
precedes Events' "Connect an action to Rust"; Router Playground moves before
"Routes and history" and retains its `#example` anchor. Transition Grid stays
under container-owned motion, with a prominent link from the chapter intro.
Frame headings use "Example: <name>" and omit monorepo-relative run commands.
CLI source-reading output labels its repository commands as requiring a Pax
source checkout; Routing directs readers to Open standalone instead.

The current CLI docs bundle is unsuitable as a runnable-project archive:
it selects `.pax`/`.rs` source, defaults to at most 12 files, truncates files
over 200 KB, and excludes Cargo manifests/assets. PAX-993's separate bundled
project machinery is the appropriate future foundation for an example runner.
No `pax-cli example` command was added in this bounded presentation pass.
