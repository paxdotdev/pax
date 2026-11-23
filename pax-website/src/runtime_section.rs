#![allow(unused_imports)]

use crate::{runtime_glass::RuntimeGlass, PaxflixShowcase, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("runtime_section.pax")]
pub struct RuntimeSection {
    pub light_mode: Property<bool>,
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub showcase_height: Property<f64>,
    pub scene_progress: Property<f64>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn glass_spans_the_runtime_and_modularity_uses_the_plain_site_background() {
        let runtime = include_str!("runtime_section.pax");
        let (content, background) = runtime.rsplit_once("</Stacker>").unwrap();
        assert!(content.contains(">Fluent in UI</Text>"));
        assert!(content.contains("<PaxflixShowcase"));
        assert_eq!(runtime.matches("<RuntimeGlass ").count(), 1);
        assert!(background.contains("<RuntimeGlass "));
        assert!(!runtime.contains("<ModularityRain"));

        let modularity = include_str!("modularity_section.pax");
        let (content, background) = modularity.rsplit_once("</Stacker>").unwrap();
        assert!(content.contains(">Plug, Extend, and Play</Text>"));
        assert!(content.contains("<CircuitBoard"));
        assert!(!modularity.contains("<ModularityRain"));
        assert!(background.contains("<Rectangle class=\"site_ink\""));

        for (paint, shader) in [
            (include_str!("runtime_glass.pax"), "GlassBricks"),
            (include_str!("modularity_rain.pax"), "WetPane"),
        ] {
            let opening = format!("<{shader} ");
            let closing = format!("</{shader}>");
            let captured = paint
                .split_once(&opening)
                .unwrap()
                .1
                .split_once(&closing)
                .unwrap()
                .0;
            assert!(captured.contains("<Rectangle"));
            assert!(!captured.contains("<Text"));
            assert!(!captured.contains("slot("));
        }
    }

    #[test]
    fn paxflix_has_its_own_row_and_calculator_is_not_on_the_homepage() {
        let runtime = include_str!("runtime_section.pax");
        assert_eq!(runtime.matches("<PaxflixShowcase").count(), 1);
        assert!(!runtime.contains("<CalculatorPreview"));
        let authoring = include_str!("authoring_section.pax");
        assert!(!authoring.contains("<CalculatorPreview"));
        let demos = include_str!("framework_section.pax");
        assert!(!demos.contains("<CalculatorPreview"));
        assert_eq!(demos.matches("<LedOscilloscope").count(), 1);
        assert!(!include_str!("resource_section.pax").contains("<CalculatorPreview"));
        // Removing the homepage showcase must not break existing demo URLs.
        assert!(include_str!("calculator_page.pax").contains("<Calculator"));
    }

    #[test]
    fn hero_introduces_rust_in_the_supporting_copy_not_a_third_heading_line() {
        let headline = include_str!("rotating_headline.pax");
        assert!(!headline.contains(">with Rust</Text>"));
        let hero = include_str!("hero_section.pax");
        assert!(hero.contains(">Declare UIs in Pax and write logic in Rust.</Text>"));
        assert!(hero
            .contains(">Ship to native mobile, desktop, and web from a single codebase.</Text>"));
        assert!(hero.contains("self.font_px * 2.52"));
    }
}

impl RuntimeSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, COMPACT_BREAKPOINT_PX);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "runtime_content",
                    &self.content_height,
                    &Property::new(24.0),
                ));
        }
    }
}
