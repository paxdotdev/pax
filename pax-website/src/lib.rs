#![allow(unused_imports)]

use pax_kit::*;

pub mod authoring_section;
pub mod brand_mark;
pub mod calculator_section;
// Parked experiment: retained with tests, but not mounted or wired into the site.
pub mod chromatic_contour;
// Parked pinstripe treatment; no mounted instances, clock, or lighting resources.
pub mod cmy_ribbon;
pub mod demo_studies;
pub mod device_frame;
pub mod feature_gallery;
pub mod framework_section;
pub mod hero_section;
pub mod home_page;
pub mod resource_section;
pub mod rotating_headline;
pub mod route_pages;
pub mod runtime_section;
mod section_layout;
pub mod site_shell;
pub mod site_theme;

pub use authoring_section::AuthoringSection;
pub use brand_mark::BrandMark;
pub use calculator_section::CalculatorSection;
pub use demo_studies::{MaterialStudy, NativeScrollStudy, PathStudy};
pub use device_frame::DeviceFrame;
pub use feature_gallery::{FeatureCard, FeatureGallery};
pub use framework_section::FrameworkSection;
pub use hero_section::HeroSection;
pub use home_page::HomePage;
pub use resource_section::ResourceSection;
pub use rotating_headline::RotatingHeadline;
pub use route_pages::NotFoundPage;
pub use runtime_section::RuntimeSection;
pub use site_shell::SiteShell;
pub use site_theme::SiteTheme;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}
