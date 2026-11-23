#![allow(unused_imports)]

use crate::SiteTheme;
use pax_kit::*;

#[pax]
#[file("home_page.pax")]
pub struct HomePage {
    pub light_mode: Property<bool>,
    pub scroll_y: Property<f64>,
    pub hero_span: Property<f64>,
    pub authoring_span: Property<f64>,
    pub runtime_span: Property<f64>,
    pub hero_exit: Property<f64>,
    pub runtime_progress: Property<f64>,
    pub hero_source_open: Property<bool>,
    pub source_overlay_open: Property<bool>,
    pub source_sheets_mounted: Property<usize>,
    pub compact_sources: Property<bool>,
    pub hero_source_selected: Property<usize>,
    // All source presentations explicitly share this page-scoped preference.
    pub runtime_source_open: Property<bool>,
    pub runtime_source_selected: Property<usize>,
    pub authoring_source_open: Property<bool>,
    pub authoring_source_selected: Property<usize>,
    pub authoring_height: Property<f64>,
    pub authoring_sources: Property<Vec<ExampleSource>>,
    pub modularity_source_open: Property<bool>,
    pub modularity_source_selected: Property<usize>,
    pub gallery_source_open: Property<bool>,
    pub gallery_source_selected: Property<usize>,
    pub open_source_source_open: Property<bool>,
    pub open_source_source_selected: Property<usize>,
    pub resource_source_open: Property<bool>,
    pub resource_source_selected: Property<usize>,
    pub hero_height: Property<f64>,
    pub headline_cap_top: Property<f64>,
    pub modularity_height: Property<f64>,
    pub runtime_height: Property<f64>,
    pub gallery_height: Property<f64>,
    pub open_source_height: Property<f64>,
    pub resource_height: Property<f64>,
    pub hero_sources: Property<Vec<ExampleSource>>,
    pub modularity_sources: Property<Vec<ExampleSource>>,
    pub runtime_sources: Property<Vec<ExampleSource>>,
    pub feature_gallery_sources: Property<Vec<ExampleSource>>,
    pub open_source_sources: Property<Vec<ExampleSource>>,
    pub resource_sources: Property<Vec<ExampleSource>>,
}

impl HomePage {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let viewport = ctx.bounds_self.clone();
        let scroll = self.scroll_y.local();
        let hero = self.hero_height.local();
        let runtime = self.runtime_height.local();
        let authoring = self.authoring_height.local();
        let deps = [
            viewport.untyped(),
            scroll.untyped(),
            hero.untyped(),
            runtime.untyped(),
            authoring.untyped(),
        ];
        let hero_span = self.hero_span.local();
        let authoring_span = self.authoring_span.local();
        let runtime_span = self.runtime_span.local();
        let hero_exit = self.hero_exit.local();
        let runtime_progress = self.runtime_progress.local();
        let update_story = move || {
            let story = crate::scroll_story::OpeningStory::sample(
                scroll.get(),
                viewport.get(),
                hero.get(),
                authoring.get(),
                runtime.get(),
            );
            hero_span.set_if_neq(story.hero_span);
            authoring_span.set_if_neq(story.authoring_span);
            runtime_span.set_if_neq(story.runtime_span);
            hero_exit.set_if_neq(story.hero_exit);
            runtime_progress.set_if_neq(story.runtime_progress);
        };
        update_story();
        ctx.subscribe(&deps, update_story);
        for (open, section) in [
            (&self.hero_source_open, "hero"),
            (&self.runtime_source_open, "paxflix"),
            (&self.authoring_source_open, "cube"),
            (&self.modularity_source_open, "modularity"),
            (&self.gallery_source_open, "toolkit"),
            (&self.open_source_source_open, "open_source"),
            (&self.resource_source_open, "footer"),
        ] {
            crate::analytics::watch_drawer(ctx, open, section);
        }
        let bounds = ctx.bounds_self.clone();
        let compact = self.compact_sources.local();
        let overlay_open = self.source_overlay_open.local();
        let mounted = self.source_sheets_mounted.local();
        let open = [
            self.hero_source_open.local(),
            self.runtime_source_open.local(),
            self.authoring_source_open.local(),
            self.modularity_source_open.local(),
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
        hero_sources.extend(get_started_sources());
        hero_sources.extend(section_sources(
            "src/hero_curtain.pax",
            include_str!("hero_curtain.pax"),
            "src/hero_curtain.rs",
            include_str!("hero_curtain.rs"),
        ));
        hero_sources.push(ExampleSource {
            label: "src/home_page.pax".into(),
            language: "pax".into(),
            code: include_str!("home_page.pax").into(),
        });
        hero_sources.extend(section_sources(
            "src/glass_action.pax",
            include_str!("glass_action.pax"),
            "src/glass_action.rs",
            include_str!("glass_action.rs"),
        ));
        hero_sources.push(ExampleSource {
            label: "src/site_theme.rs".into(),
            language: "rust".into(),
            code: include_str!("site_theme.rs").into(),
        });
        hero_sources.push(ExampleSource {
            label: "src/scroll_story.rs".into(),
            language: "rust".into(),
            code: include_str!("scroll_story.rs").into(),
        });
        // include_str! is relative to this source file, not the build's cwd.
        hero_sources.extend(section_sources(
            "src/site_logo.pax",
            include_str!("site_logo.pax"),
            "src/site_logo.rs",
            include_str!("site_logo.rs"),
        ));
        hero_sources.extend(section_sources(
            "src/site_shell.pax",
            include_str!("site_shell.pax"),
            "src/site_shell.rs",
            include_str!("site_shell.rs"),
        ));
        hero_sources.extend(section_sources(
            "src/lib.pax",
            include_str!("lib.pax"),
            "src/lib.rs",
            include_str!("lib.rs"),
        ));
        hero_sources.extend(section_sources(
            "src/site_header.pax",
            include_str!("site_header.pax"),
            "src/site_header.rs",
            include_str!("site_header.rs"),
        ));
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
            "pax-std/src/drawing/pax_logo/static_pax_logo.pax",
            include_str!("../../pax-std/src/drawing/pax_logo/static_pax_logo.pax"),
            "pax-std/src/drawing/pax_logo/static_pax_logo.rs",
            include_str!("../../pax-std/src/drawing/pax_logo/static_pax_logo.rs"),
        ));
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
        for (label, language, code) in [
            (
                "pax-std/src/shaders/flashy.rs",
                "rust",
                include_str!("../../pax-std/src/shaders/flashy.rs"),
            ),
            (
                "pax-std/src/shaders/flashy.wgsl",
                "wgsl",
                include_str!("../../pax-std/src/shaders/flashy.wgsl"),
            ),
            (
                "pax-std/src/shaders/radiance.rs",
                "rust",
                include_str!("../../pax-std/src/shaders/radiance.rs"),
            ),
            (
                "pax-std/src/shaders/radiance_common.wgsl",
                "wgsl",
                include_str!("../../pax-std/src/shaders/radiance_common.wgsl"),
            ),
            (
                "pax-std/src/shaders/radiance_projection.wgsl",
                "wgsl",
                include_str!("../../pax-std/src/shaders/radiance_projection.wgsl"),
            ),
            (
                "pax-std/src/shaders/radiance_composite.wgsl",
                "wgsl",
                include_str!("../../pax-std/src/shaders/radiance_composite.wgsl"),
            ),
        ] {
            hero_sources.push(ExampleSource {
                label: label.into(),
                language: language.into(),
                code: code.into(),
            });
        }
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
        resource_sources.extend(get_started_sources());
        resource_sources.extend(section_sources(
            "framework_section.pax",
            include_str!("framework_section.pax"),
            "framework_section.rs",
            include_str!("framework_section.rs"),
        ));
        resource_sources.extend(section_sources(
            "src/framed_study.pax",
            include_str!("framed_study.pax"),
            "src/framed_study.rs",
            include_str!("framed_study.rs"),
        ));
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
            resource_sources.push(ExampleSource {
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
        self.resource_sources.set(resource_sources);
        let mut modularity_sources = section_sources(
            "src/modularity_section.pax",
            include_str!("modularity_section.pax"),
            "src/modularity_section.rs",
            include_str!("modularity_section.rs"),
        );
        modularity_sources.extend(section_sources(
            "src/circuit_board.pax",
            include_str!("circuit_board.pax"),
            "src/circuit_board.rs",
            include_str!("circuit_board.rs"),
        ));
        modularity_sources.extend(section_sources(
            "src/circuit_package.pax",
            include_str!("circuit_package.pax"),
            "src/circuit_package.rs",
            include_str!("circuit_package.rs"),
        ));
        modularity_sources.extend(section_sources(
            "src/circuit_pin.pax",
            include_str!("circuit_pin.pax"),
            "src/circuit_pin.rs",
            include_str!("circuit_pin.rs"),
        ));
        modularity_sources.extend(section_sources(
            "src/circuit_logo.pax",
            include_str!("circuit_logo.pax"),
            "src/circuit_logo.rs",
            include_str!("circuit_logo.rs"),
        ));
        for (label, code) in [
            ("src/circuit_layout.rs", include_str!("circuit_layout.rs")),
            ("src/circuit_motion.rs", include_str!("circuit_motion.rs")),
            ("src/circuit_traffic.rs", include_str!("circuit_traffic.rs")),
        ] {
            modularity_sources.push(ExampleSource {
                label: label.into(),
                language: "rust".into(),
                code: code.into(),
            });
        }
        self.modularity_sources.set(modularity_sources);
        let mut runtime_sources = section_sources(
            "runtime_section.pax",
            include_str!("runtime_section.pax"),
            "runtime_section.rs",
            include_str!("runtime_section.rs"),
        );
        runtime_sources.extend(section_sources(
            "runtime_glass.pax",
            include_str!("runtime_glass.pax"),
            "runtime_glass.rs",
            include_str!("runtime_glass.rs"),
        ));
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
        self.authoring_sources.set(
            increment_cube::sources()
                .into_iter()
                .filter(|source| matches!(source.label.as_str(), "increment.pax" | "lib.rs"))
                .collect(),
        );
        runtime_sources.push(ExampleSource {
            label: "glass-bricks/glass.wgsl".into(),
            language: "wgsl".into(),
            code: include_str!("../../examples/fixtures/glass-bricks/src/glass.wgsl").into(),
        });
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
    }
}

fn get_started_sources() -> Vec<ExampleSource> {
    let mut sources = section_sources(
        "src/get_started.pax",
        include_str!("get_started.pax"),
        "src/get_started.rs",
        include_str!("get_started.rs"),
    );
    sources.push(ExampleSource {
        label: "src/ai_prompt.rs".into(),
        language: "rust".into(),
        code: include_str!("ai_prompt.rs").into(),
    });
    sources
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

#[cfg(test)]
mod content_tests {
    #[test]
    fn homepage_pairs_examples_in_the_agreed_editorial_order() {
        let page = include_str!("home_page.pax");
        let sections = [
            "<HeroSection",
            "<AuthoringSection",
            "<RuntimeSection",
            "<FeatureGallery",
            "<ModularitySection",
            "<OpenSourceSection",
            "<ResourceSection",
        ];
        let positions: Vec<_> = sections
            .iter()
            .map(|section| {
                assert_eq!(page.matches(section).count(), 1);
                page.find(section).unwrap()
            })
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(page.contains("open=bind:self.authoring_source_open"));
        assert!(page.contains("source_open=bind:self.authoring_source_open"));
        // Keep the circuit-board integration with its copy after the toolkit,
        // including when the separately developed visual is merged.
        assert!(!include_str!("authoring_section.pax").contains("<ModularitySection"));
        assert!(!page.contains("<FrameworkSection"));
    }

    #[test]
    fn oscilloscope_follows_the_get_started_actions_and_precedes_the_footer() {
        let resources = include_str!("resource_section.pax");
        assert_eq!(resources.matches("<FrameworkSection").count(), 1);
        let scope = resources.find("<FrameworkSection").unwrap();
        assert!(resources.find("Browse examples →").unwrap() < scope);
        assert!(scope < resources.find("[Docs](https://docs.pax.dev/)").unwrap());
        assert!(resources.find("<GetStarted").unwrap() < scope);
    }

    #[test]
    fn install_is_in_the_hero_and_removed_copy_stays_removed() {
        let hero = include_str!("hero_section.pax");
        let footer = include_str!("resource_section.pax");
        assert_eq!(hero.matches("<GetStarted").count(), 1);
        assert_eq!(footer.matches("<GetStarted").count(), 1);
        assert!(include_str!("get_started.pax").contains("cargo install pax-cli"));
        assert!(footer.contains(
            "macOS, Windows (alpha), Linux (alpha), iOS, iPadOS, Android (alpha), and the web."
        ));
        assert!(!hero.contains("(alpha)"));
        assert!(!hero.contains("Other targets are in beta."));
        assert!(!hero.contains("This website was built in Pax"));
        assert!(!include_str!("framework_section.pax").contains("Dream it. Build it."));
        assert!(!include_str!("authoring_section.pax").contains("artificial minds"));
    }
}
