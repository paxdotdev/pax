#![allow(unused_imports)]

use pax_kit::*;

pub mod authoring_section;
pub mod brand_mark;
pub mod feature_gallery;
pub mod framework_section;
pub mod hero_section;
pub mod home_page;
pub mod resource_section;
pub mod route_pages;
pub mod runtime_section;
pub mod site_shell;
pub mod site_theme;

pub use authoring_section::AuthoringSection;
pub use brand_mark::BrandMark;
pub use feature_gallery::{FeatureCard, FeatureGallery};
pub use framework_section::FrameworkSection;
pub use hero_section::HeroSection;
pub use home_page::HomePage;
pub use resource_section::ResourceSection;
pub use route_pages::{BlogRouteSeam, NotFoundPage};
pub use runtime_section::RuntimeSection;
pub use site_shell::SiteShell;
pub use site_theme::SiteTheme;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}
