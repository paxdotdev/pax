#![allow(unused_imports)]

#[cfg(feature = "app")]
use pax_kit::math::Point2;
use pax_kit::*;

#[cfg(feature = "app")]
pub mod acrostic;
#[cfg(feature = "app")]
mod ai_prompt;
#[cfg(feature = "app")]
mod analytics;
#[cfg(feature = "app")]
pub mod authoring_section;
#[cfg(feature = "app")]
pub mod brand_mark;
#[cfg(feature = "app")]
pub mod calculator_preview;
#[cfg(feature = "app")]
pub mod circuit_board;
#[cfg(feature = "app")]
mod circuit_logo;
#[cfg(feature = "app")]
mod circuit_package;
#[cfg(feature = "app")]
mod circuit_pin;
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
pub mod fluent_rain;
#[cfg(feature = "app")]
pub mod framework_section;
#[cfg(feature = "app")]
pub mod glass_action;
#[cfg(feature = "app")]
pub mod get_started;
#[cfg(feature = "app")]
pub mod hero_letter;
#[cfg(feature = "app")]
mod hero_outlines;
#[cfg(feature = "app")]
pub mod hero_section;
#[cfg(feature = "app")]
pub mod home_page;
#[cfg(feature = "app")]
pub mod hero_curtain;
#[cfg(feature = "app")]
mod logo_portal;
#[cfg(feature = "app")]
pub mod mobile_source_sheet;
#[cfg(feature = "app")]
pub mod modularity_section;
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
pub mod framed_study;
#[cfg(feature = "app")]
mod section_layout;
#[cfg(feature = "app")]
mod scroll_story;
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
pub use get_started::GetStarted;
#[cfg(feature = "app")]
pub use hero_letter::{HeroCluster, HeroLetter};
#[cfg(feature = "app")]
pub use hero_section::HeroSection;
#[cfg(feature = "app")]
pub use home_page::HomePage;
#[cfg(feature = "app")]
pub use hero_curtain::HeroCurtain;
#[cfg(feature = "app")]
pub use mobile_source_sheet::MobileSourceSheet;
#[cfg(feature = "app")]
pub use modularity_section::ModularitySection;
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
pub use framed_study::FramedStudy;
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
    pub light_mode: Property<bool>,
    pub page_scroll_y: Property<f64>,
    pub source_overlay_open: Property<bool>,
    pub pointer_light_x: Property<f64>,
    pub pointer_light_y: Property<f64>,
    pub pointer_light_active: Property<bool>,
}

#[cfg(feature = "app")]
impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        site_theme::connect_preference(ctx, &self.light_mode);
    }

    pub fn move_pointer_light(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        self.place_pointer_light(ctx, event.mouse.x, event.mouse.y);
    }

    pub fn touch_pointer_light(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self.place_pointer_light(ctx, touch.x, touch.y);
        }
    }

    pub fn drag_pointer_light(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if let Some(touch) = event.touches.first() {
            self.place_pointer_light(ctx, touch.x, touch.y);
        }
    }

    fn place_pointer_light(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let point = ctx.local_point(Point2::new(x, y));
        let (width, height) = ctx.bounds_self.get();
        // No capture or cancellation: lighting must not interfere with links,
        // controls, or the browser's native touch scrolling.
        self.pointer_light_x.local().set_if_neq(point.x * width);
        self.pointer_light_y.local().set_if_neq(point.y * height);
        self.pointer_light_active.local().set_if_neq(true);
    }
}
