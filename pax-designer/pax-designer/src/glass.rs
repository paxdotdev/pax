use pax_lang::api::*;
use pax_lang::*;
use pax_std::primitives::Rectangle;

#[derive(Pax)]
#[custom(Default)]
#[file("glass.pax")]
pub struct Glass {
    pub show_selection_controls: Property<bool>,
    pub control_points: Property<Vec<ControlPoint>>,
}

impl Default for Glass {
    fn default() -> Self {
        Self {
            show_selection_controls: Box::new(PropertyLiteral::new(true)),
            control_points: Box::new((PropertyLiteral::new(vec![
                ControlPoint {
                    x: 300.0,
                    y: 100.0,
                },
                ControlPoint {
                    x: 350.0,
                    y: 100.0,
                },
                ControlPoint {
                    x: 400.0,
                    y: 100.0,
                },
                ControlPoint {
                    x: 300.0,
                    y: 150.0,
                },
                // anchor point
                ControlPoint {
                    x: 400.0,
                    y: 150.0,
                },
                ControlPoint {
                    x: 300.0,
                    y: 200.0,
                },
                ControlPoint {
                    x: 350.0,
                    y: 200.0,
                },
                ControlPoint {
                    x: 400.0,
                    y: 200.0,
                },
            ]))),
        }
    }
}

#[derive(Pax)]
#[custom(Imports)]
pub struct ControlPoint {
    pub x: f64,
    pub y: f64,
}