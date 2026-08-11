#![allow(unused_imports)]

use pax_kit::*;

pub mod brand_mark;
pub mod builder_proof_section;
pub mod feature_card;
pub mod feature_gallery_section;
pub mod framework_section;
pub mod hero_section;
pub mod home_page;
pub mod resource_section;
pub mod route_pages;
pub mod runtime_section;
pub mod site_shell;
pub mod site_theme;

pub use brand_mark::BrandMark;
pub use builder_proof_section::BuilderProofSection;
pub use feature_card::FeatureCard;
pub use feature_gallery_section::FeatureGallerySection;
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
