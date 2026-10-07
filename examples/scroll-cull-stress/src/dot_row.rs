#![allow(unused_imports)]

use pax_kit::*;

const DOT_COLUMNS: usize = 30;
const DOT_DIAMETER: f64 = 34.0;
const DOT_GAP: f64 = 8.0;
const DOT_MARGIN_X: f64 = 12.0;
const DOT_MARGIN_Y: f64 = 2.0;

#[pax]
#[custom(Default)]
#[file("dot_row.pax")]
pub struct DotRow {
    pub row: Property<usize>,
    pub card: Property<usize>,
    pub elements: Property<Vec<PathElement>>,
}

impl Default for DotRow {
    fn default() -> Self {
        Self {
            row: Property::new(0),
            card: Property::new(0),
            elements: Property::new(build_row_path()),
        }
    }
}

fn build_row_path() -> Vec<PathElement> {
    let mut elements = Vec::new();
    let radius = DOT_DIAMETER * 0.5;
    let kappa = radius * 0.552_284_749_830_793_6;
    let center_y = DOT_MARGIN_Y + radius;

    for col in 0..DOT_COLUMNS {
        let center_x = DOT_MARGIN_X + radius + col as f64 * (DOT_DIAMETER + DOT_GAP);
        let left = center_x - radius;
        let right = center_x + radius;
        let top = center_y - radius;
        let bottom = center_y + radius;

        elements.push(PathElement::Point(px(right), px(center_y)));
        elements.push(PathElement::Cubic(
            px(right),
            px(center_y + kappa),
            px(center_x + kappa),
            px(bottom),
        ));
        elements.push(PathElement::Point(px(center_x), px(bottom)));
        elements.push(PathElement::Cubic(
            px(center_x - kappa),
            px(bottom),
            px(left),
            px(center_y + kappa),
        ));
        elements.push(PathElement::Point(px(left), px(center_y)));
        elements.push(PathElement::Cubic(
            px(left),
            px(center_y - kappa),
            px(center_x - kappa),
            px(top),
        ));
        elements.push(PathElement::Point(px(center_x), px(top)));
        elements.push(PathElement::Cubic(
            px(center_x + kappa),
            px(top),
            px(right),
            px(center_y - kappa),
        ));
        elements.push(PathElement::Point(px(right), px(center_y)));
        elements.push(PathElement::Close);
    }

    elements
}

fn px(value: f64) -> Size {
    Size::Pixels(Numeric::F64(value))
}
