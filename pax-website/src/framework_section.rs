#![allow(unused_imports)]

use crate::{FramedStudy, SiteTheme};
use led_oscilloscope::LedOscilloscope;
use pax_kit::*;

#[pax]
#[file("framework_section.pax")]
pub struct FrameworkSection {
    pub light_mode: Property<bool>,
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub scope_height: Property<f64>,
    pub scope_visited: Property<bool>,
    pub scene_progress: Property<f64>,
}

impl FrameworkSection {
    pub fn approach(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        // Defer the initial lamp geometry until approaching the footer. Keep the
        // instrument afterwards so a visitor's message and knob settings survive.
        if event.current.in_proximity {
            self.scope_visited.set_if_neq(true);
        }
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "framework_content",
                    &self.content_height,
                    &Property::new(0.0),
                ));
        }
    }
}
