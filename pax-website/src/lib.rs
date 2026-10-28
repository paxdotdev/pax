#![allow(unused_imports)]

use pax_kit::*;

pub mod acrostic;
pub mod authoring_section;
pub mod brand_mark;
pub mod calculator_preview;
// Parked experiment: retained with tests, but not mounted or wired into the site.
pub mod chromatic_contour;
// Parked pinstripe treatment; no mounted instances, clock, or lighting resources.
pub mod cmy_ribbon;
pub mod demo_studies;
pub mod device_frame;
pub mod feature_gallery;
pub mod framework_section;
pub mod hero_letter;
mod hero_outlines;
pub mod hero_section;
pub mod home_page;
pub mod mobile_source_sheet;
pub mod nav_logo;
pub mod open_source_section;
pub mod paxflix_showcase;
pub mod resource_section;
pub mod rotating_headline;
pub mod route_pages;
pub mod runtime_section;
mod section_layout;
pub mod shared_definition_demo;
pub mod site_shell;
pub mod site_theme;

pub use acrostic::{Acrostic, AcrosticGlyph, AcrosticRow, RowJourney};
pub use authoring_section::AuthoringSection;
pub use brand_mark::BrandMark;
pub use calculator_preview::CalculatorPreview;
pub use demo_studies::{MaterialStudy, NativeScrollStudy, PathStudy};
pub use device_frame::DeviceFrame;
pub use feature_gallery::{FeatureCard, FeatureGallery};
pub use framework_section::FrameworkSection;
pub use hero_letter::{HeroCluster, HeroLetter};
pub use hero_section::HeroSection;
pub use home_page::HomePage;
pub use mobile_source_sheet::MobileSourceSheet;
pub use nav_logo::{LogoAnchor, LogoHome, NavLogo};
pub use open_source_section::OpenSourceSection;
pub use paxflix_showcase::PaxflixShowcase;
pub use resource_section::ResourceSection;
pub use rotating_headline::RotatingHeadline;
pub use route_pages::{CalculatorPage, NotFoundPage, PaxflixPage};
pub use runtime_section::RuntimeSection;
pub use shared_definition_demo::{RelationshipPreview, SharedDefinitionDemo};
pub use site_shell::SiteShell;
pub use site_theme::SiteTheme;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub logo_anchor: Property<LogoAnchor>,
    pub page_scroll_y: Property<f64>,
    pub hero_source_open: Property<bool>,
    pub source_overlay_open: Property<bool>,
}
