#![allow(unused_imports)]

use pax_kit::*;

#[cfg(feature = "app")]
pub mod acrostic;
#[cfg(feature = "app")]
pub mod authoring_section;
#[cfg(feature = "app")]
pub mod brand_mark;
#[cfg(feature = "app")]
pub mod calculator_preview;
// Parked experiment: retained with tests, but not mounted or wired into the site.
#[cfg(feature = "app")]
pub mod chromatic_contour;
// Parked pinstripe treatment; no mounted instances, clock, or lighting resources.
#[cfg(feature = "app")]
pub mod cmy_ribbon;
#[cfg(feature = "app")]
pub mod demo_studies;
#[cfg(feature = "app")]
pub mod device_frame;
#[cfg(feature = "app")]
pub mod feature_gallery;
#[cfg(feature = "app")]
pub mod framework_section;
#[cfg(feature = "app")]
pub mod glass_action;
#[cfg(feature = "app")]
pub mod hero_letter;
#[cfg(feature = "app")]
mod hero_outlines;
#[cfg(feature = "app")]
pub mod hero_section;
#[cfg(feature = "app")]
pub mod home_page;
#[cfg(feature = "app")]
pub mod mobile_source_sheet;
pub mod nav_logo;
#[cfg(feature = "app")]
pub mod open_source_section;
#[cfg(feature = "app")]
pub mod paxflix_showcase;
#[cfg(feature = "app")]
pub mod resource_section;
#[cfg(feature = "app")]
pub mod rotating_headline;
#[cfg(feature = "app")]
pub mod route_pages;
#[cfg(feature = "app")]
pub mod runtime_section;
#[cfg(feature = "app")]
pub mod screenshot_study;
#[cfg(feature = "app")]
mod section_layout;
#[cfg(feature = "app")]
mod scroll_story;
#[cfg(feature = "app")]
pub mod shared_definition_demo;
#[cfg(feature = "app")]
pub mod site_logo;
pub mod site_header;
#[cfg(feature = "app")]
pub mod site_shell;
pub mod site_theme;

#[cfg(feature = "app")]
pub use acrostic::{Acrostic, AcrosticGlyph, AcrosticRow, RowJourney};
#[cfg(feature = "app")]
pub use authoring_section::AuthoringSection;
#[cfg(feature = "app")]
pub use brand_mark::BrandMark;
#[cfg(feature = "app")]
pub use calculator_preview::CalculatorPreview;
#[cfg(feature = "app")]
pub use demo_studies::{MaterialStudy, NativeScrollStudy, PathStudy};
#[cfg(feature = "app")]
pub use device_frame::DeviceFrame;
#[cfg(feature = "app")]
pub use feature_gallery::{FeatureCard, FeatureGallery};
#[cfg(feature = "app")]
pub use framework_section::FrameworkSection;
#[cfg(feature = "app")]
pub use glass_action::GlassAction;
#[cfg(feature = "app")]
pub use hero_letter::{HeroCluster, HeroLetter};
#[cfg(feature = "app")]
pub use hero_section::HeroSection;
#[cfg(feature = "app")]
pub use home_page::HomePage;
#[cfg(feature = "app")]
pub use mobile_source_sheet::MobileSourceSheet;
pub use nav_logo::NavLogo;
#[cfg(feature = "app")]
pub use open_source_section::OpenSourceSection;
#[cfg(feature = "app")]
pub use paxflix_showcase::PaxflixShowcase;
#[cfg(feature = "app")]
pub use resource_section::ResourceSection;
#[cfg(feature = "app")]
pub use rotating_headline::RotatingHeadline;
#[cfg(feature = "app")]
pub use route_pages::{CalculatorPage, NotFoundPage, PaxflixPage, PlaygroundPage};
#[cfg(feature = "app")]
pub use runtime_section::RuntimeSection;
#[cfg(feature = "app")]
pub use screenshot_study::ScreenshotStudy;
#[cfg(feature = "app")]
pub use shared_definition_demo::{RelationshipPreview, SharedDefinitionDemo};
#[cfg(feature = "app")]
pub use site_logo::SiteLogo;
pub use site_header::SiteHeader;
#[cfg(feature = "app")]
pub use site_shell::SiteShell;
pub use site_theme::SiteTheme;

#[cfg(feature = "app")]
#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub page_scroll_y: Property<f64>,
    pub hero_source_open: Property<bool>,
    pub source_overlay_open: Property<bool>,
}
