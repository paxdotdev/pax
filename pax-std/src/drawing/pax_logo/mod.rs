//! Reusable Pax branding with a consumer-controlled animation playhead.
//!
//! `AnimatedPaxLogo` contains only the logo. Parent components own layout,
//! background, controls, and initial playback. `StaticPaxLogo` draws its settled
//! pose without animation masks. Both use a 920.72 × 463.95 coordinate system;
//! scale an enclosing `Group` to fit another layout.

pub mod animated_pax_logo;
pub mod animated_pax_logo_banner;
pub mod animated_pax_logo_post;
pub mod static_pax_logo;

pub use animated_pax_logo::*;
pub use animated_pax_logo_banner::*;
pub use animated_pax_logo_post::*;
pub use static_pax_logo::*;
