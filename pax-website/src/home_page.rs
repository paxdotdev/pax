#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("home_page.pax")]
pub struct HomePage {
    pub hero_height: Property<f64>,
    pub framework_height: Property<f64>,
    pub authoring_height: Property<f64>,
    pub runtime_height: Property<f64>,
    pub gallery_height: Property<f64>,
    pub resource_height: Property<f64>,
    pub hero_sources: Property<Vec<ExampleSource>>,
    pub authoring_sources: Property<Vec<ExampleSource>>,
    pub framework_sources: Property<Vec<ExampleSource>>,
    pub runtime_sources: Property<Vec<ExampleSource>>,
    pub feature_gallery_sources: Property<Vec<ExampleSource>>,
    pub resource_sources: Property<Vec<ExampleSource>>,
}

impl HomePage {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let mut hero_sources = section_sources(
            "src/hero_section.pax",
            include_str!("hero_section.pax"),
            "src/hero_section.rs",
            include_str!("hero_section.rs"),
        );
        // include_str! is relative to this source file, not the build's cwd.
        hero_sources.extend(section_sources(
            "src/rotating_headline.pax",
            include_str!("rotating_headline.pax"),
            "src/rotating_headline.rs",
            include_str!("rotating_headline.rs"),
        ));
        hero_sources.extend(section_sources(
            "pax-logo/src/animated_pax_logo.pax",
            include_str!("../../examples/src/pax-logo/src/animated_pax_logo.pax"),
            "pax-logo/src/animated_pax_logo.rs",
            include_str!("../../examples/src/pax-logo/src/animated_pax_logo.rs"),
        ));
        hero_sources.extend(section_sources(
            "pax-logo/src/animated_pax_logo_banner.pax",
            include_str!("../../examples/src/pax-logo/src/animated_pax_logo_banner.pax"),
            "pax-logo/src/animated_pax_logo_banner.rs",
            include_str!("../../examples/src/pax-logo/src/animated_pax_logo_banner.rs"),
        ));
        hero_sources.extend(section_sources(
            "pax-logo/src/animated_pax_logo_post.pax",
            include_str!("../../examples/src/pax-logo/src/animated_pax_logo_post.pax"),
            "pax-logo/src/animated_pax_logo_post.rs",
            include_str!("../../examples/src/pax-logo/src/animated_pax_logo_post.rs"),
        ));
        self.hero_sources.set(hero_sources);
        let mut resource_sources = section_sources(
            "src/resource_section.pax",
            include_str!("resource_section.pax"),
            "src/resource_section.rs",
            include_str!("resource_section.rs"),
        );
        resource_sources.extend(section_sources(
            "src/calculator_preview.pax",
            include_str!("calculator_preview.pax"),
            "src/calculator_preview.rs",
            include_str!("calculator_preview.rs"),
        ));
        resource_sources.extend(section_sources(
            "calculator/src/lib.pax",
            include_str!("../../examples/src/calculator/src/lib.pax"),
            "calculator/src/lib.rs",
            include_str!("../../examples/src/calculator/src/lib.rs"),
        ));
        resource_sources.extend(section_sources(
            "calculator/src/key.pax",
            include_str!("../../examples/src/calculator/src/key.pax"),
            "calculator/src/key.rs",
            include_str!("../../examples/src/calculator/src/key.rs"),
        ));
        resource_sources.extend(section_sources(
            "src/device_frame.pax",
            include_str!("device_frame.pax"),
            "src/device_frame.rs",
            include_str!("device_frame.rs"),
        ));
        for (label, code) in [
            (
                "calculator/src/model.rs",
                include_str!("../../examples/src/calculator/src/model.rs"),
            ),
            (
                "calculator/src/expression.rs",
                include_str!("../../examples/src/calculator/src/expression.rs"),
            ),
            (
                "calculator/src/graph.rs",
                include_str!("../../examples/src/calculator/src/graph.rs"),
            ),
            (
                "calculator/src/layout.rs",
                include_str!("../../examples/src/calculator/src/layout.rs"),
            ),
        ] {
            resource_sources.push(ExampleSource {
                label: label.into(),
                language: "rust".into(),
                code: code.into(),
            });
        }
        self.resource_sources.set(resource_sources);
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
                label: "feature_gallery/studies.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("feature_gallery/studies.pax").to_string(),
            },
            ExampleSource {
                label: "feature_gallery/studies.rs".to_string(),
                language: "rust".to_string(),
                code: include_str!("feature_gallery/studies.rs").to_string(),
            },
            ExampleSource {
                label: "feature_gallery/paint_demo.pax".to_string(),
                language: "pax".to_string(),
                code: include_str!("feature_gallery/paint_demo.pax").to_string(),
            },
            ExampleSource {
                label: "feature_gallery/paint_demo.rs".to_string(),
                language: "rust".to_string(),
                code: include_str!("feature_gallery/paint_demo.rs").to_string(),
            },
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
