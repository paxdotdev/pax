#![allow(unused_imports)]

use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("home_page.pax")]
pub struct HomePage {
    pub framework_height: Property<f64>,
    pub authoring_height: Property<f64>,
    pub runtime_height: Property<f64>,
    pub gallery_height: Property<f64>,
    pub compact: Property<bool>,
    pub hero_sources: Property<Vec<ExampleSource>>,
    pub authoring_sources: Property<Vec<ExampleSource>>,
    pub framework_sources: Property<Vec<ExampleSource>>,
    pub runtime_sources: Property<Vec<ExampleSource>>,
    pub feature_gallery_sources: Property<Vec<ExampleSource>>,
    pub resource_sources: Property<Vec<ExampleSource>>,
}

impl HomePage {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let mut hero_sources = section_sources(
            "src/hero_section.pax",
            include_str!("hero_section.pax"),
            "src/hero_section.rs",
            include_str!("hero_section.rs"),
        );
        // include_str! is relative to this source file, not the build's cwd.
        hero_sources.extend(section_sources(
            "living-quilt/src/lib.pax",
            include_str!("../../examples/src/living-quilt/src/lib.pax"),
            "living-quilt/src/lib.rs",
            include_str!("../../examples/src/living-quilt/src/lib.rs"),
        ));
        hero_sources.push(ExampleSource {
            label: "living-quilt/src/quilt_tile.rs".to_string(),
            language: "rust".to_string(),
            code: include_str!("../../examples/src/living-quilt/src/quilt_tile.rs").to_string(),
        });
        self.hero_sources.set(hero_sources);
        self.authoring_sources.set(section_sources(
            "src/authoring_section.pax",
            include_str!("authoring_section.pax"),
            "src/authoring_section.rs",
            include_str!("authoring_section.rs"),
        ));
        self.framework_sources.set(section_sources(
            "framework_section.pax",
            include_str!("framework_section.pax"),
            "framework_section.rs",
            include_str!("framework_section.rs"),
        ));
        self.runtime_sources.set(section_sources(
            "runtime_section.pax",
            include_str!("runtime_section.pax"),
            "runtime_section.rs",
            include_str!("runtime_section.rs"),
        ));
        self.feature_gallery_sources.set(vec![
            ExampleSource {
                label: "feature_gallery/gallery.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("feature_gallery/gallery.pax").to_string(),
            },
            ExampleSource {
                label: "feature_gallery/marquee.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("feature_gallery/marquee.pax").to_string(),
            },
            ExampleSource {
                label: "feature_gallery/card.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("feature_gallery/card.pax").to_string(),
            },
            ExampleSource {
                label: "feature_gallery/mod.rs".to_string(),
                language: "rust".to_string(),
                code: include_str!("feature_gallery/mod.rs").to_string(),
            },
        ]);
        self.resource_sources.set(section_sources(
            "resource_section.pax",
            include_str!("resource_section.pax"),
            "resource_section.rs",
            include_str!("resource_section.rs"),
        ));
        for sources in [
            &self.framework_sources,
            &self.authoring_sources,
            &self.runtime_sources,
        ] {
            sources.update(|files| {
                files.extend(section_sources(
                    "device_frame.pax",
                    include_str!("device_frame.pax"),
                    "device_frame.rs",
                    include_str!("device_frame.rs"),
                ));
                for (label, code) in [
                    ("path_study.pax", include_str!("path_study.pax")),
                    ("material_study.pax", include_str!("material_study.pax")),
                    (
                        "native_scroll_study.pax",
                        include_str!("native_scroll_study.pax"),
                    ),
                    ("demo_studies.rs", include_str!("demo_studies.rs")),
                ] {
                    files.push(ExampleSource {
                        label: label.into(),
                        language: if label.ends_with(".rs") {
                            "rust"
                        } else {
                            "pax"
                        }
                        .into(),
                        code: code.into(),
                    });
                }
            });
        }
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        self.compact
            .set_if_neq(ctx.bounds_self.get().0 < COMPACT_BREAKPOINT_PX);
    }
}

fn section_sources(
    pax_label: &str,
    pax_code: &str,
    rust_label: &str,
    rust_code: &str,
) -> Vec<ExampleSource> {
    vec![
        ExampleSource {
            label: pax_label.to_string(),
            language: "pax".to_string(),
            code: pax_code.to_string(),
        },
        ExampleSource {
            label: rust_label.to_string(),
            language: "rust".to_string(),
            code: rust_code.to_string(),
        },
    ]
}
