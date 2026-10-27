#![allow(unused_imports)]

use crate::SiteTheme;
use calculator::Calculator;
use pax_kit::*;
use paxflix::Paxflix;

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

/// A dedicated calculator session, without the homepage's other live examples.
#[pax]
#[file("calculator_page.pax")]
pub struct CalculatorPage {
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}
