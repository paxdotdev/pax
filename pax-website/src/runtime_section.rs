#![allow(unused_imports)]

use crate::{PaxflixShowcase, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("runtime_section.pax")]
pub struct RuntimeSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub showcase_height: Property<f64>,
    pub scene_progress: Property<f64>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn paxflix_has_its_own_row_and_calculator_follows_the_oscilloscope() {
        let runtime = include_str!("runtime_section.pax");
        assert_eq!(runtime.matches("<PaxflixShowcase").count(), 1);
        assert!(!runtime.contains("<CalculatorPreview"));
        let authoring = include_str!("authoring_section.pax");
        assert!(!authoring.contains("<CalculatorPreview"));
        let demos = include_str!("framework_section.pax");
        assert_eq!(demos.matches("<CalculatorPreview").count(), 1);
        assert!(
            demos.find("<LedOscilloscope").unwrap() < demos.find("<CalculatorPreview").unwrap()
        );
        assert!(demos.contains("compact=true live={!self.compact}"));
        assert!(!include_str!("resource_section.pax").contains("<CalculatorPreview"));
    }

    #[test]
    fn hero_introduces_rust_in_the_supporting_copy_not_a_third_heading_line() {
        let headline = include_str!("rotating_headline.pax");
        assert!(!headline.contains(">with Rust</Text>"));
        let hero = include_str!("hero_section.pax");
        assert!(hero.contains("From a single Rust codebase, ship to native macOS"));
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
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "runtime_content",
                &self.content_height,
            ));
        }
    }
}
