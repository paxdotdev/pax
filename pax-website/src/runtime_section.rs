#![allow(unused_imports)]

use crate::{CalculatorPreview, PaxflixShowcase, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("runtime_section.pax")]
pub struct RuntimeSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub showcase_height: Property<f64>,
    pub calculator_height: Property<f64>,
    pub scene_progress: Property<f64>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_gives_paxflix_a_full_row_and_keeps_the_calculator_below() {
        let runtime = include_str!("runtime_section.pax");
        assert_eq!(runtime.matches("<PaxflixShowcase").count(), 1);
        assert_eq!(runtime.matches("<CalculatorPreview").count(), 1);
        assert_eq!(runtime.matches("compact={self.compact}").count(), 2);
        assert!(!runtime.contains("Some(62%)"));
        assert!(
            runtime.find("<PaxflixShowcase").unwrap() < runtime.find("<CalculatorPreview").unwrap()
        );
        assert!(!include_str!("resource_section.pax").contains("<CalculatorPreview"));
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
