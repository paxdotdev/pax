#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("font_comparison_row.pax")]
pub struct FontComparisonRow {
    pub title: Property<String>,
    pub original_font: Property<HandwriterFont>,
    pub curved_font: Property<HandwriterFont>,
}
