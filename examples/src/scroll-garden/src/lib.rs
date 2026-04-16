#![allow(unused_imports)]

use pax_kit::*;

mod kaleidoscope;
use crate::kaleidoscope::{KaleidoscopeSide, KaleidoscopeVariant};
mod kaleidoscope_stream;
use crate::kaleidoscope_stream::KaleidoscopeStream;
mod tendril;
mod tendril_limb;

const SCROLL_HEIGHT_MULTIPLIER: f64 = 7.5;
const TIMELINE_FRAMES: f64 = 1000.0;
const PANEL_BREAKPOINT: f64 = 720.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub scroll_y: Property<f64>,
    pub scroll_playhead: Property<f64>,
    pub panel_width: Property<f64>,
    pub panel_x_left: Property<f64>,
    pub panel_x_right: Property<f64>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds_for_playhead = ctx.bounds_self.clone();
        let bounds_for_playhead_calc = bounds_for_playhead.clone();
        let scroll_y = self.scroll_y.clone();
        let scroll_y_calc = scroll_y.clone();
        self.scroll_playhead.replace_with(Property::computed(
            move || {
                let (_, height) = bounds_for_playhead_calc.get();
                if height <= 0.0 {
                    return 0.0;
                }
                let max_scroll = (SCROLL_HEIGHT_MULTIPLIER - 1.0).max(1.0) * height;
                let progress = (scroll_y_calc.get() / max_scroll).clamp(0.0, 1.0);
                progress * TIMELINE_FRAMES
            },
            &[bounds_for_playhead.untyped(), scroll_y.untyped()],
        ));

        let bounds_for_width = ctx.bounds_self.clone();
        let bounds_for_width_calc = bounds_for_width.clone();
        self.panel_width.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_for_width_calc.get();
                if width <= PANEL_BREAKPOINT {
                    width * 0.86
                } else {
                    width * 0.52
                }
            },
            &[bounds_for_width.untyped()],
        ));

        let bounds_for_left = ctx.bounds_self.clone();
        let bounds_for_left_calc = bounds_for_left.clone();
        self.panel_x_left.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_for_left_calc.get();
                if width <= PANEL_BREAKPOINT {
                    width * 0.07
                } else {
                    width * 0.10
                }
            },
            &[bounds_for_left.untyped()],
        ));

        let bounds_for_right = ctx.bounds_self.clone();
        let bounds_for_right_calc = bounds_for_right.clone();
        self.panel_x_right.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_for_right_calc.get();
                let panel_width = if width <= PANEL_BREAKPOINT {
                    width * 0.86
                } else {
                    width * 0.52
                };
                let gutter = if width <= PANEL_BREAKPOINT {
                    width * 0.07
                } else {
                    width * 0.10
                };
                (width - panel_width - gutter).max(0.0)
            },
            &[bounds_for_right.untyped()],
        ));
    }
}
