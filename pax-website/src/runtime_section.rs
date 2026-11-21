#![allow(unused_imports)]

use crate::{fluent_rain::FluentRain, PaxflixShowcase, SiteTheme};
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
    fn rain_is_a_background_of_the_autosized_copy_not_the_demo() {
        let runtime = include_str!("runtime_section.pax");
        let intro = runtime.split_once("<Group id=fluent_intro").unwrap().1;
        let (copy, rest) = intro.split_once("</Group>").unwrap();
        assert!(copy.contains(">Fluent in UI</Text>"));
        assert!(
            copy.contains("<FluentRain light_mode={self.light_mode} width=100% height=100% layout_role=LayoutRole::Breakout")
        );
        assert!(!copy.contains("<PaxflixShowcase"));
        assert!(rest.contains("<PaxflixShowcase"));
        let rain = include_str!("fluent_rain.pax");
        assert_eq!(rain.matches("<WetPane").count(), 1);
        let captured = rain
            .split_once("<WetPane")
            .unwrap()
            .1
            .split_once("</WetPane>")
            .unwrap()
            .0;
        assert!(captured.contains("<Rectangle"));
        assert!(!captured.contains("<Text"));
        assert!(!captured.contains("slot("));
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
                    &Property::new(80.0),
                ));
        }
    }
}
