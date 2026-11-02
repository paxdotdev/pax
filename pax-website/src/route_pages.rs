#![allow(unused_imports)]

use crate::SiteTheme;
use calculator::Calculator;
use led_oscilloscope::LedOscilloscope;
use pax_kit::*;
use paxflix::Paxflix;
use space_game::SpaceGame;

#[pax]
#[file("not_found_page.pax")]
pub struct NotFoundPage {}

/// Full-page demo, entered through a document navigation on web so its only
/// Paxflix instance participates in the initial yielded mount.
#[pax]
#[file("paxflix_page.pax")]
pub struct PaxflixPage {
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

/// A dedicated calculator session, separate from the screenshot-only homepage.
#[pax]
#[file("calculator_page.pax")]
pub struct CalculatorPage {
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

/// Live creative examples mount only after leaving the screenshot-backed homepage.
#[pax]
#[file("playground_page.pax")]
pub struct PlaygroundPage {
    pub game: Property<bool>,
    pub sources: Property<Vec<ExampleSource>>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

impl PlaygroundPage {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        let files = if self.game.get() {
            vec![
                (
                    "space-game/src/lib.pax",
                    include_str!("../../examples/fixtures/space-game/src/lib.pax"),
                ),
                (
                    "space-game/src/lib.rs",
                    include_str!("../../examples/fixtures/space-game/src/lib.rs"),
                ),
                (
                    "space-game/src/animation.rs",
                    include_str!("../../examples/fixtures/space-game/src/animation.rs"),
                ),
            ]
        } else {
            vec![
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
            ]
        };
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
