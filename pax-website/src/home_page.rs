#![allow(unused_imports)]

use crate::{LogoAnchor, SiteTheme};
use pax_kit::*;

#[pax]
#[file("home_page.pax")]
pub struct HomePage {
    pub logo_anchor: Property<LogoAnchor>,
    pub scroll_y: Property<f64>,
    pub hero_source_open: Property<bool>,
    pub source_overlay_open: Property<bool>,
    pub source_sheets_mounted: Property<usize>,
    pub compact_sources: Property<bool>,
    pub hero_source_selected: Property<usize>,
    pub hero_source_light: Property<bool>,
    pub runtime_source_open: Property<bool>,
    pub runtime_source_selected: Property<usize>,
    pub runtime_source_light: Property<bool>,
    pub framework_source_open: Property<bool>,
    pub framework_source_selected: Property<usize>,
    pub framework_source_light: Property<bool>,
    pub authoring_source_open: Property<bool>,
    pub authoring_source_selected: Property<usize>,
    pub authoring_source_light: Property<bool>,
    pub gallery_source_open: Property<bool>,
    pub gallery_source_selected: Property<usize>,
    pub gallery_source_light: Property<bool>,
    pub open_source_source_open: Property<bool>,
    pub open_source_source_selected: Property<usize>,
    pub open_source_source_light: Property<bool>,
    pub resource_source_open: Property<bool>,
    pub resource_source_selected: Property<usize>,
    pub resource_source_light: Property<bool>,
    pub hero_height: Property<f64>,
    pub framework_height: Property<f64>,
    pub authoring_height: Property<f64>,
    pub runtime_height: Property<f64>,
    pub gallery_height: Property<f64>,
    pub open_source_height: Property<f64>,
    pub resource_height: Property<f64>,
    pub hero_sources: Property<Vec<ExampleSource>>,
    pub authoring_sources: Property<Vec<ExampleSource>>,
    pub framework_sources: Property<Vec<ExampleSource>>,
    pub runtime_sources: Property<Vec<ExampleSource>>,
    pub feature_gallery_sources: Property<Vec<ExampleSource>>,
    pub open_source_sources: Property<Vec<ExampleSource>>,
    pub resource_sources: Property<Vec<ExampleSource>>,
}

impl HomePage {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let compact = self.compact_sources.local();
        let overlay_open = self.source_overlay_open.local();
        let mounted = self.source_sheets_mounted.local();
        let open = [
            self.hero_source_open.local(),
            self.runtime_source_open.local(),
            self.framework_source_open.local(),
            self.authoring_source_open.local(),
            self.gallery_source_open.local(),
            self.open_source_source_open.local(),
            self.resource_source_open.local(),
        ];
        let mut deps = vec![bounds.untyped(), mounted.untyped()];
        deps.extend(open.iter().map(|value| value.untyped()));
        let update = move || {
            let is_compact = bounds.get().0 < 600.0;
            compact.set_if_neq(is_compact);
            overlay_open.set_if_neq(
                mounted.get() > 0 || (is_compact && open.iter().any(|value| value.get())),
            );
        };
        update();
        ctx.subscribe(&deps, update);
        let mut hero_sources = section_sources(
            "src/hero_section.pax",
            include_str!("hero_section.pax"),
            "src/hero_section.rs",
            include_str!("hero_section.rs"),
        );
        // include_str! is relative to this source file, not the build's cwd.
        hero_sources.extend(section_sources(
            "src/nav_logo.pax",
            include_str!("nav_logo.pax"),
            "src/nav_logo.rs",
            include_str!("nav_logo.rs"),
        ));
        hero_sources.extend(section_sources(
            "src/site_shell.pax",
            include_str!("site_shell.pax"),
            "src/site_shell.rs",
            include_str!("site_shell.rs"),
        ));
        hero_sources.push(ExampleSource {
            label: "src/logo_home.pax".into(),
            language: "pax".into(),
            code: include_str!("logo_home.pax").into(),
        });
        hero_sources.extend(section_sources(
            "src/rotating_headline.pax",
            include_str!("rotating_headline.pax"),
            "src/rotating_headline.rs",
            include_str!("rotating_headline.rs"),
        ));
        hero_sources.extend(section_sources(
            "src/hero_letter.pax",
            include_str!("hero_letter.pax"),
            "src/hero_letter.rs",
            include_str!("hero_letter.rs"),
        ));
        hero_sources.push(ExampleSource {
            label: "src/hero_outlines.rs".into(),
            language: "rust".into(),
            code: include_str!("hero_outlines.rs").into(),
        });
        hero_sources.push(ExampleSource {
            label: "src/hero_outlines_data.rs".into(),
            language: "rust".into(),
            code: include_str!("hero_outlines_data.rs").into(),
        });
        hero_sources.extend(section_sources(
            "pax-std/src/drawing/pax_logo/animated_pax_logo.pax",
            include_str!("../../pax-std/src/drawing/pax_logo/animated_pax_logo.pax"),
            "pax-std/src/drawing/pax_logo/animated_pax_logo.rs",
            include_str!("../../pax-std/src/drawing/pax_logo/animated_pax_logo.rs"),
        ));
        hero_sources.extend(section_sources(
            "pax-std/src/drawing/pax_logo/animated_pax_logo_banner.pax",
            include_str!("../../pax-std/src/drawing/pax_logo/animated_pax_logo_banner.pax"),
            "pax-std/src/drawing/pax_logo/animated_pax_logo_banner.rs",
            include_str!("../../pax-std/src/drawing/pax_logo/animated_pax_logo_banner.rs"),
        ));
        hero_sources.extend(section_sources(
            "pax-std/src/drawing/pax_logo/animated_pax_logo_post.pax",
            include_str!("../../pax-std/src/drawing/pax_logo/animated_pax_logo_post.pax"),
            "pax-std/src/drawing/pax_logo/animated_pax_logo_post.rs",
            include_str!("../../pax-std/src/drawing/pax_logo/animated_pax_logo_post.rs"),
        ));
        self.hero_sources.set(hero_sources);
        let mut open_source_sources = section_sources(
            "src/open_source_section.pax",
            include_str!("open_source_section.pax"),
            "src/open_source_section.rs",
            include_str!("open_source_section.rs"),
        );
        open_source_sources.extend(section_sources(
            "src/acrostic.pax",
            include_str!("acrostic.pax"),
            "src/acrostic.rs",
            include_str!("acrostic.rs"),
        ));
        for (label, language, code) in [
            (
                "src/acrostic_row.pax",
                "pax",
                include_str!("acrostic_row.pax"),
            ),
            (
                "src/hero_outlines.rs",
                "rust",
                include_str!("hero_outlines.rs"),
            ),
            (
                "src/acrostic_outlines_data.rs",
                "rust",
                include_str!("acrostic_outlines_data.rs"),
            ),
        ] {
            open_source_sources.push(ExampleSource {
                label: label.into(),
                language: language.into(),
                code: code.into(),
            });
        }
        self.open_source_sources.set(open_source_sources);
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
            "src/calculator_page.pax",
            include_str!("calculator_page.pax"),
            "src/route_pages.rs",
            include_str!("route_pages.rs"),
        ));
        resource_sources.extend(section_sources(
            "calculator/src/lib.pax",
            include_str!("../../examples/calculator/src/lib.pax"),
            "calculator/src/lib.rs",
            include_str!("../../examples/calculator/src/lib.rs"),
        ));
        resource_sources.extend(section_sources(
            "calculator/src/key.pax",
            include_str!("../../examples/calculator/src/key.pax"),
            "calculator/src/key.rs",
            include_str!("../../examples/calculator/src/key.rs"),
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
                include_str!("../../examples/calculator/src/model.rs"),
            ),
            (
                "calculator/src/expression.rs",
                include_str!("../../examples/calculator/src/expression.rs"),
            ),
            (
                "calculator/src/graph.rs",
                include_str!("../../examples/calculator/src/graph.rs"),
            ),
            (
                "calculator/src/layout.rs",
                include_str!("../../examples/calculator/src/layout.rs"),
            ),
        ] {
            resource_sources.push(ExampleSource {
                label: label.into(),
                language: "rust".into(),
                code: code.into(),
            });
        }
        self.resource_sources.set(resource_sources);
        let mut authoring_sources = section_sources(
            "src/authoring_section.pax",
            include_str!("authoring_section.pax"),
            "src/authoring_section.rs",
            include_str!("authoring_section.rs"),
        );
        authoring_sources.extend(section_sources(
            "src/shared_definition_demo.pax",
            include_str!("shared_definition_demo.pax"),
            "src/shared_definition_demo.rs",
            include_str!("shared_definition_demo.rs"),
        ));
        authoring_sources.push(ExampleSource {
            label: "src/relationship_preview.pax".into(),
            language: "pax".into(),
            code: include_str!("relationship_preview.pax").into(),
        });
        self.authoring_sources.set(authoring_sources);
        self.framework_sources.set(section_sources(
            "framework_section.pax",
            include_str!("framework_section.pax"),
            "framework_section.rs",
            include_str!("framework_section.rs"),
        ));
        self.framework_sources.update(|files| {
            for (label, code) in [
                (
                    "led-oscilloscope/lib.pax",
                    include_str!("../../examples/led-oscilloscope/src/lib.pax"),
                ),
                (
                    "led-oscilloscope/lib.rs",
                    include_str!("../../examples/led-oscilloscope/src/lib.rs"),
                ),
                (
                    "led-oscilloscope/knob.pax",
                    include_str!("../../examples/led-oscilloscope/src/knob.pax"),
                ),
                (
                    "led-oscilloscope/knob.rs",
                    include_str!("../../examples/led-oscilloscope/src/knob.rs"),
                ),
                (
                    "led-oscilloscope/display.rs",
                    include_str!("../../examples/led-oscilloscope/src/display.rs"),
                ),
                (
                    "led-oscilloscope/message_editor.pax",
                    include_str!("../../examples/led-oscilloscope/src/message_editor.pax"),
                ),
                (
                    "led-oscilloscope/message_editor.rs",
                    include_str!("../../examples/led-oscilloscope/src/message_editor.rs"),
                ),
                (
                    "led-oscilloscope/signature_pad.pax",
                    include_str!("../../examples/led-oscilloscope/src/signature_pad.pax"),
                ),
                (
                    "led-oscilloscope/signature_pad.rs",
                    include_str!("../../examples/led-oscilloscope/src/signature_pad.rs"),
                ),
                (
                    "led-oscilloscope/ink.rs",
                    include_str!("../../examples/led-oscilloscope/src/ink.rs"),
                ),
                (
                    "led-oscilloscope/font.rs",
                    include_str!("../../examples/led-oscilloscope/src/font.rs"),
                ),
                (
                    "led-oscilloscope/geometry.rs",
                    include_str!("../../examples/led-oscilloscope/src/geometry.rs"),
                ),
                (
                    "led-oscilloscope/led_glow.rs",
                    include_str!("../../examples/led-oscilloscope/src/led_glow.rs"),
                ),
                (
                    "led-oscilloscope/foreground.rs",
                    include_str!("../../examples/led-oscilloscope/src/foreground.rs"),
                ),
            ] {
                files.push(ExampleSource {
                    label: label.into(),
                    code: code.into(),
                    language: if label.ends_with(".rs") {
                        "rust"
                    } else {
                        "pax"
                    }
                    .into(),
                });
            }
        });
        let mut runtime_sources = section_sources(
            "runtime_section.pax",
            include_str!("runtime_section.pax"),
            "runtime_section.rs",
            include_str!("runtime_section.rs"),
        );
        runtime_sources.extend(section_sources(
            "paxflix_showcase.pax",
            include_str!("paxflix_showcase.pax"),
            "paxflix_showcase.rs",
            include_str!("paxflix_showcase.rs"),
        ));
        runtime_sources.extend(section_sources(
            "device_frame.pax",
            include_str!("device_frame.pax"),
            "device_frame.rs",
            include_str!("device_frame.rs"),
        ));
        for (label, code) in [
            (
                "paxflix/lib.pax",
                include_str!("../../examples/paxflix/src/lib.pax"),
            ),
            (
                "paxflix/lib.rs",
                include_str!("../../examples/paxflix/src/lib.rs"),
            ),
            (
                "paxflix/theme.pax",
                include_str!("../../examples/paxflix/src/theme.pax"),
            ),
            (
                "paxflix/movie_shelf.pax",
                include_str!("../../examples/paxflix/src/movie_shelf.pax"),
            ),
            (
                "paxflix/movie_shelf.rs",
                include_str!("../../examples/paxflix/src/movie_shelf.rs"),
            ),
            (
                "paxflix/movie_card.pax",
                include_str!("../../examples/paxflix/src/movie_card.pax"),
            ),
            (
                "paxflix/movie_card.rs",
                include_str!("../../examples/paxflix/src/movie_card.rs"),
            ),
            (
                "paxflix/movie_detail.pax",
                include_str!("../../examples/paxflix/src/movie_detail.pax"),
            ),
            (
                "paxflix/movie_detail.rs",
                include_str!("../../examples/paxflix/src/movie_detail.rs"),
            ),
            (
                "paxflix/film_artwork.pax",
                include_str!("../../examples/paxflix/src/film_artwork.pax"),
            ),
            (
                "paxflix/film_artwork.rs",
                include_str!("../../examples/paxflix/src/film_artwork.rs"),
            ),
            (
                "paxflix/profile_menu.pax",
                include_str!("../../examples/paxflix/src/profile_menu.pax"),
            ),
            (
                "paxflix/profile_menu.rs",
                include_str!("../../examples/paxflix/src/profile_menu.rs"),
            ),
            (
                "paxflix/catalog.rs",
                include_str!("../../examples/paxflix/src/catalog.rs"),
            ),
        ] {
            runtime_sources.push(ExampleSource {
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
        self.runtime_sources.set(runtime_sources);
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
        self.authoring_sources.update(|files| {
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
