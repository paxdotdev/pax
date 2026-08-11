#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("feature_card.pax")]
pub struct FeatureCard {
    pub ordinal: Property<String>,
    pub category: Property<String>,
    pub title: Property<String>,
    pub summary: Property<String>,
    pub docs_label: Property<String>,
    pub docs_url: Property<String>,
    pub visual_kind: Property<usize>,
}
