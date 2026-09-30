//! Curated links to the prose and generated API book for every inspectable part.
pub struct DocRef {
    pub title: &'static str,
    /// Book-relative path without an extension; also resolves to a local .md source.
    pub page: &'static str,
}

pub fn for_part(id: &str) -> &'static [DocRef] {
    match id {
        "source" => &[
            DocRef {
                title: "Getting started",
                page: "getting-started",
            },
            DocRef {
                title: "Templates & UI structure",
                page: "template-language",
            },
            DocRef {
                title: "Events & Rust",
                page: "event-handling-rust",
            },
        ],
        "construct.analyze" => &[
            DocRef {
                title: "Templates & UI structure",
                page: "template-language",
            },
            DocRef {
                title: "Data binding & expressions",
                page: "data-binding-expressions",
            },
            DocRef {
                title: "API · Language interpreter",
                page: "api/internal/pax-language/interpreter",
            },
        ],
        "construct.connect" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Cartridge generation",
                page: "api/internal/pax-manifest/cartridge_generation",
            },
            DocRef {
                title: "Targets, build & deployment",
                page: "targets-build-deploy",
            },
        ],
        "cartridge" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Cartridge",
                page: "api/internal/pax-runtime/cartridge",
            },
            DocRef {
                title: "API · Cartridge generation",
                page: "api/internal/pax-manifest/cartridge_generation",
            },
        ],
        "construct.instantiate" => &[
            DocRef {
                title: "Components & composition",
                page: "components-composition",
            },
            DocRef {
                title: "API · Cartridge",
                page: "api/internal/pax-runtime/cartridge",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
        ],
        "instance.behavior" => &[
            DocRef {
                title: "Components & composition",
                page: "components-composition",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
            DocRef {
                title: "API · Cartridge",
                page: "api/internal/pax-runtime/cartridge",
            },
        ],
        "construct.expand" => &[
            DocRef {
                title: "Components & composition",
                page: "components-composition",
            },
            DocRef {
                title: "API · Runtime engine",
                page: "api/internal/pax-runtime/engine",
            },
            DocRef {
                title: "API · Properties",
                page: "api/pax-runtime-api/properties",
            },
        ],
        "scene" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "Components & composition",
                page: "components-composition",
            },
            DocRef {
                title: "API · Runtime engine",
                page: "api/internal/pax-runtime/engine",
            },
        ],
        "operate.drive" => &[
            DocRef {
                title: "Events & Rust",
                page: "event-handling-rust",
            },
            DocRef {
                title: "API · Events",
                page: "api/pax-runtime-api/events",
            },
            DocRef {
                title: "API · Node context",
                page: "api/internal/pax-runtime/api",
            },
        ],
        "operate.react.properties" => &[
            DocRef {
                title: "State & properties",
                page: "state-properties",
            },
            DocRef {
                title: "Data binding & expressions",
                page: "data-binding-expressions",
            },
            DocRef {
                title: "API · Properties",
                page: "api/pax-runtime-api/properties",
            },
        ],
        "operate.react.evaluate" => &[
            DocRef {
                title: "Data binding & expressions",
                page: "data-binding-expressions",
            },
            DocRef {
                title: "API · Language interpreter",
                page: "api/internal/pax-language/interpreter",
            },
            DocRef {
                title: "API · Properties",
                page: "api/pax-runtime-api/properties",
            },
        ],
        "operate.react.dirty" => &[
            DocRef {
                title: "State & properties",
                page: "state-properties",
            },
            DocRef {
                title: "API · Properties",
                page: "api/pax-runtime-api/properties",
            },
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
        ],
        "operate.react.effects" => &[
            DocRef {
                title: "State & properties",
                page: "state-properties",
            },
            DocRef {
                title: "API · Properties",
                page: "api/pax-runtime-api/properties",
            },
            DocRef {
                title: "API · Runtime engine",
                page: "api/internal/pax-runtime/engine",
            },
        ],
        "operate.structure" => &[
            DocRef {
                title: "Templates & UI structure",
                page: "template-language",
            },
            DocRef {
                title: "Routing",
                page: "routing",
            },
            DocRef {
                title: "Components & composition",
                page: "components-composition",
            },
        ],
        "operate.geometry" => &[
            DocRef {
                title: "Layout & responsiveness",
                page: "layout-responsiveness",
            },
            DocRef {
                title: "API · Runtime layout",
                page: "api/internal/pax-runtime/layout",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
        ],
        "operate.geometry.index" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
            DocRef {
                title: "API · Layer surfaces",
                page: "api/internal/pax-runtime/engine/layer_surface",
            },
        ],
        "operate.opacity" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
        ],
        "operate.project" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
            DocRef {
                title: "API · Native messages",
                page: "api/internal/pax-message/index",
            },
        ],
        "present.plan.layers" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Occlusion",
                page: "api/internal/pax-runtime/engine/occlusion",
            },
            DocRef {
                title: "Scrolling & viewports",
                page: "scrolling-viewports",
            },
        ],
        "present.plan.coverage" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Frame",
                page: "api/pax-std/core/frame",
            },
            DocRef {
                title: "API · Mask",
                page: "api/pax-std/core/mask",
            },
        ],
        "present.plan.tiles" => &[
            DocRef {
                title: "Scrolling & viewports",
                page: "scrolling-viewports",
            },
            DocRef {
                title: "API · Layer tiling",
                page: "api/internal/pax-runtime/engine/layer_tiling",
            },
            DocRef {
                title: "API · Layer surfaces",
                page: "api/internal/pax-runtime/engine/layer_surface",
            },
        ],
        "present.plan.occlusion" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Occlusion",
                page: "api/internal/pax-runtime/engine/occlusion",
            },
            DocRef {
                title: "API · Native messages",
                page: "api/internal/pax-message/index",
            },
        ],
        "present.work.replay" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Layer surfaces",
                page: "api/internal/pax-runtime/engine/layer_surface",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
        ],
        "present.work.dirty" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Runtime engine",
                page: "api/internal/pax-runtime/engine",
            },
            DocRef {
                title: "API · Layer surfaces",
                page: "api/internal/pax-runtime/engine/layer_surface",
            },
        ],
        "present.lights" => &[
            DocRef {
                title: "Drawing & styling",
                page: "drawing-styling",
            },
            DocRef {
                title: "API · Lighting",
                page: "api/pax-std/drawing/lighting",
            },
            DocRef {
                title: "API · Drawing & paints",
                page: "api/pax-runtime-api/drawing",
            },
        ],
        "present.work.cull" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · GPU runtime adapter",
                page: "api/internal/pax-runtime/engine/pax_gpu_render_context",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
        ],
        "present.patch" => &[
            DocRef {
                title: "Accessibility & native controls",
                page: "accessibility-native-controls",
            },
            DocRef {
                title: "API · Native messages",
                page: "api/internal/pax-message/index",
            },
            DocRef {
                title: "API · Instance rendering",
                page: "api/internal/pax-runtime/rendering",
            },
        ],
        "present.draw.adapter" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · GPU runtime adapter",
                page: "api/internal/pax-runtime/engine/pax_gpu_render_context",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
        ],
        "present.draw.retained" => &[
            DocRef {
                title: "Drawing & styling",
                page: "drawing-styling",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
            DocRef {
                title: "API · GPU backend",
                page: "api/internal/pax-gpu/render_backend",
            },
        ],
        "present.draw.vector" => &[
            DocRef {
                title: "Drawing & styling",
                page: "drawing-styling",
            },
            DocRef {
                title: "API · Drawing & paints",
                page: "api/pax-runtime-api/drawing",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
        ],
        "present.draw.materials" => &[
            DocRef {
                title: "Drawing & styling",
                page: "drawing-styling",
            },
            DocRef {
                title: "API · Drawing & paints",
                page: "api/pax-runtime-api/drawing",
            },
            DocRef {
                title: "API · Lighting",
                page: "api/pax-std/drawing/lighting",
            },
        ],
        "present.draw.textures" => &[
            DocRef {
                title: "Text, fonts & images",
                page: "text-fonts-images",
            },
            DocRef {
                title: "API · Image",
                page: "api/pax-std/media/image",
            },
            DocRef {
                title: "API · GPU backend",
                page: "api/internal/pax-gpu/render_backend",
            },
        ],
        "present.draw.stencil" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Stencil renderer",
                page: "api/internal/pax-gpu/render_backend/stencil",
            },
            DocRef {
                title: "API · Mask",
                page: "api/pax-std/core/mask",
            },
        ],
        "present.draw.capture" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
            DocRef {
                title: "API · GPU backend",
                page: "api/internal/pax-gpu/render_backend",
            },
        ],
        "present.draw.alpha" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Mask",
                page: "api/pax-std/core/mask",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
        ],
        "present.draw.capture.opacity" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · GPU render context",
                page: "api/internal/pax-gpu/render_context",
            },
            DocRef {
                title: "API · GPU backend",
                page: "api/internal/pax-gpu/render_backend",
            },
        ],
        "present.draw.submit" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · GPU backend",
                page: "api/internal/pax-gpu/render_backend",
            },
            DocRef {
                title: "API · GPU runtime adapter",
                page: "api/internal/pax-runtime/engine/pax_gpu_render_context",
            },
        ],
        "feedback.clock" => &[
            DocRef {
                title: "Animation & motion",
                page: "animation-motion",
            },
            DocRef {
                title: "API · Animation",
                page: "api/pax-runtime-api/animation",
            },
            DocRef {
                title: "API · Runtime engine",
                page: "api/internal/pax-runtime/engine",
            },
        ],
        "feedback.input" => &[
            DocRef {
                title: "Events & Rust",
                page: "event-handling-rust",
            },
            DocRef {
                title: "API · Events",
                page: "api/pax-runtime-api/events",
            },
            DocRef {
                title: "API · Native messages",
                page: "api/internal/pax-message/index",
            },
        ],
        "chassis.bridge" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Native messages",
                page: "api/internal/pax-message/index",
            },
            DocRef {
                title: "Targets, build & deployment",
                page: "targets-build-deploy",
            },
        ],
        "chassis.native" => &[
            DocRef {
                title: "Accessibility & native controls",
                page: "accessibility-native-controls",
            },
            DocRef {
                title: "Text, fonts & images",
                page: "text-fonts-images",
            },
            DocRef {
                title: "API · Scroller",
                page: "api/pax-std/core/scroller",
            },
        ],
        "chassis.masks" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "API · Occlusion",
                page: "api/internal/pax-runtime/engine/occlusion",
            },
            DocRef {
                title: "API · Mask",
                page: "api/pax-std/core/mask",
            },
        ],
        "chassis.surfaces" => &[
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
            DocRef {
                title: "API · Layer tiling",
                page: "api/internal/pax-runtime/engine/layer_tiling",
            },
            DocRef {
                title: "API · Layer surfaces",
                page: "api/internal/pax-runtime/engine/layer_surface",
            },
        ],
        "present.compose" => &[
            DocRef {
                title: "Compositing & effects",
                page: "compositing-effects",
            },
            DocRef {
                title: "Accessibility & native controls",
                page: "accessibility-native-controls",
            },
            DocRef {
                title: "How Pax runs",
                page: "how-pax-runs",
            },
        ],
        "feedback.resource" => &[
            DocRef {
                title: "Text, fonts & images",
                page: "text-fonts-images",
            },
            DocRef {
                title: "API · Image",
                page: "api/pax-std/media/image",
            },
            DocRef {
                title: "State & properties",
                page: "state-properties",
            },
        ],
        _ => &[],
    }
}
