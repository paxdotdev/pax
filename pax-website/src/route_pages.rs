#![allow(unused_imports)]

use crate::SiteTheme;
use calculator::Calculator;
use led_oscilloscope::LedOscilloscope;
use pax_kit::*;
use paxflix::Paxflix;

#[pax]
#[file("not_found_page.pax")]
pub struct NotFoundPage {
    pub light_mode: Property<bool>,
}

/// Full-page demo, entered through a document navigation on web so its only
/// Paxflix instance participates in the initial yielded mount.
#[pax]
#[file("paxflix_page.pax")]
pub struct PaxflixPage {
    pub light_mode: Property<bool>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

/// A dedicated calculator session, independent of the desktop homepage embed.
#[pax]
#[file("calculator_page.pax")]
pub struct CalculatorPage {
    pub light_mode: Property<bool>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

/// A dedicated oscilloscope session with its source viewer and safe-area layout.
#[pax]
#[file("playground_page.pax")]
pub struct PlaygroundPage {
    pub light_mode: Property<bool>,
    pub source_open: Property<bool>,
    pub sources: Property<Vec<ExampleSource>>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

impl PlaygroundPage {
    pub fn mount(&mut self, ctx: &NodeContext) {
        crate::analytics::watch_drawer(ctx, &self.source_open, "oscilloscope");
        let files = [
            (
                "led-oscilloscope/src/lib.pax",
                include_str!("../../examples/led-oscilloscope/src/lib.pax"),
            ),
            (
                "led-oscilloscope/src/lib.rs",
                include_str!("../../examples/led-oscilloscope/src/lib.rs"),
            ),
            (
                "led-oscilloscope/src/knob.pax",
                include_str!("../../examples/led-oscilloscope/src/knob.pax"),
            ),
            (
                "led-oscilloscope/src/knob.rs",
                include_str!("../../examples/led-oscilloscope/src/knob.rs"),
            ),
            (
                "led-oscilloscope/src/display.rs",
                include_str!("../../examples/led-oscilloscope/src/display.rs"),
            ),
        ];
        self.sources.set(
            files
                .into_iter()
                .map(|(label, code)| ExampleSource {
                    label: label.into(),
                    language: if label.ends_with(".rs") {
                        "rust"
                    } else {
                        "pax"
                    }
                    .into(),
                    code: code.into(),
                })
                .collect(),
        );
    }
}
